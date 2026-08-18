use std::{
    fs,
    path::{Path, PathBuf},
    process,
    sync::{
        Arc,
        atomic::{AtomicBool, AtomicUsize, Ordering},
        mpsc,
    },
    thread,
};

use clap::{Parser, ValueEnum};
use font_converter_core::{converter, scanner};
use font_converter_shared::{
    BatchSummary, ErrorCode, FolderConversionMode, ItemStatus, QueueItem, ScanWarning,
};
use serde::Serialize;

const EXIT_SUCCESS: i32 = 0;
const EXIT_CONVERSION_FAILED: i32 = 1;
const EXIT_USAGE_OR_NO_INPUT: i32 = 2;
const EXIT_INTERRUPTED: i32 = 130;
const REPORT_FORMAT_VERSION: u32 = 1;
const MAX_JOBS: usize = 32;

#[derive(Clone, Copy, Debug, ValueEnum)]
enum Mode {
    Auto,
    Encode,
    Decode,
}

impl From<Mode> for FolderConversionMode {
    fn from(value: Mode) -> Self {
        match value {
            Mode::Auto => Self::Both,
            Mode::Encode => Self::FontToWoff2,
            Mode::Decode => Self::Woff2ToFont,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, ValueEnum)]
enum ExistingPolicy {
    Skip,
    Error,
    Overwrite,
}

impl From<ExistingPolicy> for scanner::ExistingOutputPolicy {
    fn from(value: ExistingPolicy) -> Self {
        match value {
            ExistingPolicy::Skip => Self::Skip,
            ExistingPolicy::Error => Self::Error,
            ExistingPolicy::Overwrite => Self::Overwrite,
        }
    }
}

#[derive(Debug, Parser)]
#[command(
    name = "font-converter-cli",
    version,
    about = "Convert TTF, OTF, and WOFF2 fonts without a graphical interface"
)]
struct Args {
    /// Put generated files in this directory, creating it when needed.
    #[arg(short = 'o', long = "output-dir", value_name = "DIR")]
    output_directory: Option<PathBuf>,

    /// Select both directions, encode TTF/OTF, or decode WOFF2.
    #[arg(long, value_enum, default_value_t = Mode::Auto)]
    mode: Mode,

    /// Choose how to handle output files that already exist.
    #[arg(long, value_enum, default_value_t = ExistingPolicy::Skip)]
    existing: ExistingPolicy,

    /// Scan and report planned work without writing output files.
    #[arg(long)]
    dry_run: bool,

    /// Number of conversions to run concurrently.
    #[arg(
        short = 'j',
        long = "jobs",
        value_name = "N",
        value_parser = parse_jobs,
        default_value_t = default_jobs()
    )]
    jobs: usize,

    /// Print one JSON report instead of human-readable progress.
    #[arg(long)]
    json: bool,

    /// Font files or directories to scan recursively.
    #[arg(value_name = "PATH", required = true)]
    paths: Vec<PathBuf>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct Report {
    format_version: u32,
    items: Vec<QueueItem>,
    warnings: Vec<ScanWarning>,
    summary: BatchSummary,
    interrupted: bool,
    dry_run: bool,
}

enum WorkerEvent {
    Started(usize),
    Finished(usize, QueueItem),
}

fn default_jobs() -> usize {
    thread::available_parallelism()
        .map(usize::from)
        .unwrap_or(1)
        .min(4)
}

fn parse_jobs(value: &str) -> Result<usize, String> {
    let jobs = value
        .parse::<usize>()
        .map_err(|_| "jobs must be a positive integer".to_string())?;
    if !(1..=MAX_JOBS).contains(&jobs) {
        Err(format!("jobs must be between 1 and {MAX_JOBS}"))
    } else {
        Ok(jobs)
    }
}

fn main() {
    let exit_code = match run() {
        Ok(exit_code) => exit_code,
        Err(error) => {
            eprintln!("font-converter-cli: {error}");
            EXIT_USAGE_OR_NO_INPUT
        }
    };
    process::exit(exit_code);
}

fn run() -> Result<i32, String> {
    let args = Args::parse();
    let interrupted = Arc::new(AtomicBool::new(false));
    let signal_state = Arc::clone(&interrupted);
    ctrlc::set_handler(move || {
        signal_state.store(true, Ordering::Release);
    })
    .map_err(|error| format!("cannot install Ctrl+C handler: {error}"))?;

    let paths = args
        .paths
        .iter()
        .map(|path| path.to_string_lossy().into_owned())
        .collect::<Vec<_>>();
    let mut scan = scanner::collect_for_cli(
        &paths,
        args.output_directory.as_deref(),
        args.mode.into(),
        args.existing.into(),
    );

    if !args.json {
        for warning in &scan.warnings {
            eprintln!("warning: {}: {}", warning.path, warning.message);
        }
    }

    if args.dry_run {
        print_dry_run(&scan.items, args.json);
    } else {
        execute_conversions(
            &mut scan.items,
            args.jobs,
            args.existing == ExistingPolicy::Overwrite,
            Arc::clone(&interrupted),
            args.json,
        );
    }

    let was_interrupted = interrupted.load(Ordering::Acquire);
    if was_interrupted {
        cancel_queued_items(&mut scan.items);
    }

    let report = Report {
        format_version: REPORT_FORMAT_VERSION,
        summary: BatchSummary::from_items(&scan.items),
        items: scan.items,
        warnings: scan.warnings,
        interrupted: was_interrupted,
        dry_run: args.dry_run,
    };

    print_report(&report, args.json)?;

    if was_interrupted {
        Ok(EXIT_INTERRUPTED)
    } else if report.items.is_empty() {
        Ok(EXIT_USAGE_OR_NO_INPUT)
    } else if report.summary.failed > 0 || !report.warnings.is_empty() {
        Ok(EXIT_CONVERSION_FAILED)
    } else {
        Ok(EXIT_SUCCESS)
    }
}

fn execute_conversions(
    items: &mut [QueueItem],
    jobs: usize,
    overwrite: bool,
    interrupted: Arc<AtomicBool>,
    json: bool,
) {
    for item in items
        .iter()
        .filter(|item| item.status != ItemStatus::Queued)
    {
        if !json {
            print_status(item, item.message.as_deref().unwrap_or("skipped"));
        }
    }

    let queued_indices = items
        .iter()
        .enumerate()
        .filter_map(|(index, item)| (item.status == ItemStatus::Queued).then_some(index))
        .collect::<Vec<_>>();
    if queued_indices.is_empty() {
        return;
    }

    let worker_count = jobs.min(queued_indices.len());
    let source_items = Arc::new(items.to_vec());
    let queued_indices = Arc::new(queued_indices);
    let next = Arc::new(AtomicUsize::new(0));
    let (sender, receiver) = mpsc::channel();

    thread::scope(|scope| {
        for _ in 0..worker_count {
            let source_items = Arc::clone(&source_items);
            let queued_indices = Arc::clone(&queued_indices);
            let next = Arc::clone(&next);
            let interrupted = Arc::clone(&interrupted);
            let sender = sender.clone();
            scope.spawn(move || {
                loop {
                    if interrupted.load(Ordering::Acquire) {
                        break;
                    }
                    let queued_position = next.fetch_add(1, Ordering::AcqRel);
                    let Some(&index) = queued_indices.get(queued_position) else {
                        break;
                    };
                    if interrupted.load(Ordering::Acquire) {
                        break;
                    }

                    let mut item = source_items[index].clone();
                    if sender.send(WorkerEvent::Started(index)).is_err() {
                        break;
                    }
                    item.status = ItemStatus::Running;
                    convert_item(&mut item, overwrite);
                    if sender.send(WorkerEvent::Finished(index, item)).is_err() {
                        break;
                    }
                }
            });
        }
        drop(sender);

        for event in receiver {
            match event {
                WorkerEvent::Started(index) => {
                    if !json {
                        println!(
                            "Converting {} -> {}",
                            items[index].input_path, items[index].output_path
                        );
                    }
                }
                WorkerEvent::Finished(index, item) => {
                    if !json {
                        print_finished(&item);
                    }
                    items[index] = item;
                }
            }
        }
    });
}

fn convert_item(item: &mut QueueItem, overwrite: bool) {
    let input = Path::new(&item.input_path);
    let output = Path::new(&item.output_path);
    if let Err(error) = fs::metadata(input) {
        let code = if error.kind() == std::io::ErrorKind::NotFound {
            ErrorCode::InputNotFound
        } else {
            ErrorCode::InputUnreadable
        };
        set_failed(item, code, format!("Cannot access input font: {error}"));
        return;
    }
    let result = match scanner::conversion_for(input) {
        Ok((conversion, expected_output))
            if conversion == item.conversion
                && expected_output.file_name() == output.file_name() =>
        {
            let Some(parent) = output.parent() else {
                set_failed(
                    item,
                    ErrorCode::OutputUnwritable,
                    "Output path has no parent directory".into(),
                );
                return;
            };
            if let Err(error) = fs::create_dir_all(parent) {
                set_failed(
                    item,
                    ErrorCode::OutputUnwritable,
                    format!("Cannot create output directory: {error}"),
                );
                return;
            }
            converter::convert_with_overwrite(input, output, conversion, overwrite)
        }
        _ => Err(converter::ConversionError::Failed(
            "Invalid or changed input font or output path".into(),
        )),
    };

    match result {
        Ok(converted) => {
            item.status = ItemStatus::Succeeded;
            item.input_bytes = Some(converted.input_bytes);
            item.output_bytes = Some(converted.output_bytes);
            item.error_code = None;
            item.message = None;
        }
        Err(converter::ConversionError::AlreadyExists) => {
            item.status = ItemStatus::Skipped;
            item.output_bytes = fs::metadata(output).ok().map(|metadata| metadata.len());
            item.error_code = Some(ErrorCode::OutputExists);
            item.message = Some("Output file already exists".into());
        }
        Err(converter::ConversionError::Failed(message)) => {
            let code = if message.contains("256 MB safety limit") {
                ErrorCode::InputTooLarge
            } else if message.contains("rejected")
                || message.contains("valid WOFF2")
                || message.contains("WOFF2 header")
            {
                ErrorCode::InvalidFont
            } else {
                ErrorCode::ConversionFailed
            };
            set_failed(item, code, message);
        }
    }
}

fn set_failed(item: &mut QueueItem, error_code: ErrorCode, message: String) {
    item.status = ItemStatus::Failed;
    item.error_code = Some(error_code);
    item.message = Some(message);
}

fn cancel_queued_items(items: &mut [QueueItem]) {
    for item in items {
        if matches!(item.status, ItemStatus::Queued | ItemStatus::Running) {
            item.status = ItemStatus::Cancelled;
            item.error_code = Some(ErrorCode::Cancelled);
            item.message = Some("Cancelled before conversion started".into());
        }
    }
}

fn print_dry_run(items: &[QueueItem], json: bool) {
    if json {
        return;
    }
    for item in items {
        if item.status == ItemStatus::Queued {
            println!("Would convert {} -> {}", item.input_path, item.output_path);
        } else {
            print_status(item, item.message.as_deref().unwrap_or("skipped"));
        }
    }
}

fn print_finished(item: &QueueItem) {
    match item.status {
        ItemStatus::Succeeded => println!(
            "  succeeded ({} → {} bytes)",
            item.input_bytes.unwrap_or(0),
            item.output_bytes.unwrap_or(0)
        ),
        _ => print_status(item, item.message.as_deref().unwrap_or("conversion failed")),
    }
}

fn print_report(report: &Report, json: bool) -> Result<(), String> {
    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(report).map_err(|error| error.to_string())?
        );
    } else {
        if report.dry_run {
            println!(
                "Dry run: {} queued, {} skipped, {} failed",
                report.summary.queued, report.summary.skipped, report.summary.failed
            );
        } else {
            println!(
                "Finished: {} succeeded, {} skipped, {} failed, {} cancelled",
                report.summary.succeeded,
                report.summary.skipped,
                report.summary.failed,
                report.summary.cancelled
            );
        }
    }
    Ok(())
}

fn print_status(item: &QueueItem, message: &str) {
    let label = match item.status {
        ItemStatus::Queued => "queued",
        ItemStatus::Running => "running",
        ItemStatus::Succeeded => "succeeded",
        ItemStatus::Skipped => "skipped",
        ItemStatus::Failed => "failed",
        ItemStatus::Cancelled => "cancelled",
    };
    println!("  {label}: {message}");
}
