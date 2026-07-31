use std::{
    path::{Path, PathBuf},
    process,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
};

use clap::Parser;
use font_converter_core::{converter, scanner};
use font_converter_shared::{BatchSummary, ItemStatus, QueueItem, ScanWarning};
use serde::Serialize;

const EXIT_SUCCESS: i32 = 0;
const EXIT_CONVERSION_FAILED: i32 = 1;
const EXIT_USAGE_OR_NO_INPUT: i32 = 2;
const EXIT_INTERRUPTED: i32 = 130;

#[derive(Debug, Parser)]
#[command(
    name = "font-converter-cli",
    version,
    about = "Convert TTF, OTF, and WOFF2 fonts without a graphical interface"
)]
struct Args {
    /// Put all generated files in this existing directory.
    #[arg(short = 'o', long = "output-dir", value_name = "DIR")]
    output_directory: Option<PathBuf>,

    /// Print one JSON report instead of human-readable progress.
    #[arg(long)]
    json: bool,

    /// Suppress human-readable progress output.
    #[arg(short, long)]
    quiet: bool,

    /// Font files or directories to scan recursively.
    #[arg(value_name = "PATH", required = true)]
    paths: Vec<PathBuf>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct Report {
    items: Vec<QueueItem>,
    warnings: Vec<ScanWarning>,
    summary: BatchSummary,
    interrupted: bool,
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
    let mut scan = scanner::collect(&paths, args.output_directory.as_deref());
    let mut failed = false;

    if !args.json {
        for warning in &scan.warnings {
            eprintln!("warning: {}: {}", warning.path, warning.message);
        }
    }

    for item in &mut scan.items {
        if item.status != ItemStatus::Queued {
            if !args.json && !args.quiet {
                print_status(item, item.message.as_deref().unwrap_or("skipped"));
            }
            continue;
        }

        if interrupted.load(Ordering::Acquire) {
            item.status = ItemStatus::Cancelled;
            item.message = Some("Cancelled before conversion started".into());
            continue;
        }

        if !args.json && !args.quiet {
            println!("Converting {} -> {}", item.input_path, item.output_path);
        }

        let input = Path::new(&item.input_path);
        let output = Path::new(&item.output_path);
        let result = match scanner::conversion_for(input) {
            Ok((conversion, expected_output))
                if conversion == item.conversion
                    && expected_output.file_name() == output.file_name() =>
            {
                converter::convert(input, output, conversion)
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
                item.message = None;
                if !args.json && !args.quiet {
                    println!(
                        "  succeeded ({} → {} bytes)",
                        converted.input_bytes, converted.output_bytes
                    );
                }
            }
            Err(converter::ConversionError::AlreadyExists) => {
                item.status = ItemStatus::Skipped;
                item.message = Some("Output file already exists".into());
                if !args.json && !args.quiet {
                    print_status(item, "output file already exists");
                }
            }
            Err(converter::ConversionError::Failed(message)) => {
                failed = true;
                item.status = ItemStatus::Failed;
                item.message = Some(message.clone());
                if !args.json && !args.quiet {
                    print_status(item, &message);
                }
            }
        }
    }

    let was_interrupted = interrupted.load(Ordering::Acquire);
    if was_interrupted {
        for item in &mut scan.items {
            if item.status == ItemStatus::Queued {
                item.status = ItemStatus::Cancelled;
                item.message = Some("Cancelled before conversion started".into());
            }
        }
    }

    let report = Report {
        summary: BatchSummary::from_items(&scan.items),
        items: scan.items,
        warnings: scan.warnings,
        interrupted: was_interrupted,
    };

    if args.json {
        println!(
            "{}",
            serde_json::to_string_pretty(&report).map_err(|error| error.to_string())?
        );
    } else if !args.quiet {
        println!(
            "Finished: {} succeeded, {} skipped, {} failed, {} cancelled",
            report.summary.succeeded,
            report.summary.skipped,
            report.summary.failed,
            report.summary.cancelled
        );
    }

    if was_interrupted {
        Ok(EXIT_INTERRUPTED)
    } else if failed || report.summary.failed > 0 {
        Ok(EXIT_CONVERSION_FAILED)
    } else if report.items.is_empty() {
        Ok(EXIT_USAGE_OR_NO_INPUT)
    } else {
        Ok(EXIT_SUCCESS)
    }
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
    println!("  {label}: {}", message);
}
