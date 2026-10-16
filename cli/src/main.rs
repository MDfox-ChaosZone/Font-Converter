use clap::{Parser, ValueEnum};
use font_converter_engine::{Event, Existing, Mode, Options, Summary, scan, start};
use serde::Serialize;
use std::{
    path::PathBuf,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
};
#[derive(Clone, Copy, ValueEnum)]
enum Direction {
    Auto,
    Encode,
    Decode,
}
#[derive(Clone, Copy, ValueEnum)]
enum Policy {
    Skip,
    Error,
    Overwrite,
}
#[derive(Parser)]
#[command(
    version,
    about = "Convert TTF/OTF and WOFF2 fonts with the native conversion engine"
)]
struct Args {
    #[arg(long, value_enum, default_value = "auto")]
    mode: Direction,
    #[arg(short = 'o', long)]
    output_dir: Option<PathBuf>,
    #[arg(long, value_enum, default_value = "skip")]
    existing: Policy,
    #[arg(short = 'j', long, default_value_t = font_converter_engine::default_jobs() as u8, value_parser = clap::value_parser!(u8).range(1..=32))]
    jobs: u8,
    /// Scan without creating directories or output files.
    #[arg(long)]
    dry_run: bool,
    /// Write a single version 2 JSON report to stdout.
    #[arg(long)]
    json: bool,
    #[arg(required = true, value_name = "PATH")]
    paths: Vec<PathBuf>,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Report {
    format_version: u32,
    options: Options,
    items: Vec<font_converter_engine::Item>,
    warnings: Vec<font_converter_engine::Warning>,
    summary: Summary,
    interrupted: bool,
    dry_run: bool,
}
fn main() {
    std::process::exit(run(Args::parse()));
}
fn run(args: Args) -> i32 {
    let options = Options {
        mode: match args.mode {
            Direction::Auto => Mode::Auto,
            Direction::Encode => Mode::Encode,
            Direction::Decode => Mode::Decode,
        },
        existing: match args.existing {
            Policy::Skip => Existing::Skip,
            Policy::Error => Existing::Error,
            Policy::Overwrite => Existing::Overwrite,
        },
        output_dir: args.output_dir,
        jobs: args.jobs as usize,
    };
    let cancelled = Arc::new(AtomicBool::new(false));
    let token = cancelled.clone();
    if let Err(error) = ctrlc::set_handler(move || {
        token.store(true, Ordering::Release);
    }) {
        eprintln!("Cannot register interrupt handler: {error}");
        return 1;
    }
    let mut plan = scan(&args.paths, &options);
    if !args.dry_run && !plan.items.is_empty() {
        match start(plan.clone(), options.clone(), cancelled.clone()) {
            Ok(batch) => {
                let mut finished = false;
                for event in batch.events {
                    match event {
                        Event::Item { item, .. } if !args.json => eprintln!(
                            "{:?}: {}{}",
                            item.status,
                            item.input.display(),
                            item.message.map(|s| format!(" — {s}")).unwrap_or_default()
                        ),
                        Event::Finished { items, .. } => {
                            plan.items = items;
                            finished = true;
                            break;
                        }
                        _ => {}
                    }
                }
                if !finished {
                    for item in &mut plan.items {
                        if matches!(
                            item.status,
                            font_converter_engine::Status::Queued
                                | font_converter_engine::Status::Running
                        ) {
                            item.fail(font_converter_engine::Error::new(
                                font_converter_engine::ErrorCode::ConversionFailed,
                                "Conversion worker stopped unexpectedly",
                            ));
                        }
                    }
                }
            }
            Err(error) => {
                eprintln!("{error}");
                return 1;
            }
        }
    }
    let report = Report {
        format_version: 2,
        options,
        summary: Summary::from_items(&plan.items),
        items: plan.items,
        warnings: plan.warnings,
        interrupted: cancelled.load(Ordering::Acquire),
        dry_run: args.dry_run,
    };
    if args.json {
        println!(
            "{}",
            serde_json::to_string(&report).expect("Report serialization")
        );
    } else {
        for warning in &report.warnings {
            eprintln!(
                "{:?}: {} — {}",
                warning.error_code,
                warning.path.display(),
                warning.message
            );
        }
        println!(
            "{} total; {} succeeded; {} skipped; {} failed; {} cancelled{}",
            report.summary.total,
            report.summary.succeeded,
            report.summary.skipped,
            report.summary.failed,
            report.summary.cancelled,
            if args.dry_run { " (dry run)" } else { "" }
        );
    }
    if report.interrupted {
        130
    } else if report.summary.failed > 0 || !report.warnings.is_empty() {
        1
    } else if report.items.is_empty() {
        2
    } else {
        0
    }
}
