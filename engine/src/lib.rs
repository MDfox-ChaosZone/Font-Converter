mod codec;
mod scan;
pub use codec::{convert, identify};
pub use scan::scan;
use serde::{Deserialize, Serialize};
use std::{
    path::PathBuf,
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, AtomicU64, Ordering},
        mpsc::{self, Receiver},
    },
    thread,
};
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Mode {
    #[default]
    Auto,
    Encode,
    Decode,
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Existing {
    #[default]
    Skip,
    Error,
    Overwrite,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Format {
    Ttf,
    Otf,
    Woff2,
}
impl Format {
    pub fn extension(self) -> &'static str {
        match self {
            Self::Ttf => "ttf",
            Self::Otf => "otf",
            Self::Woff2 => "woff2",
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Status {
    Queued,
    Running,
    Succeeded,
    Skipped,
    Failed,
    Cancelled,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ErrorCode {
    InputNotFound,
    InputUnreadable,
    InvalidFont,
    UnsupportedFormat,
    InputTooLarge,
    OutputExists,
    OutputConflict,
    OutputUnwritable,
    ConversionFailed,
    Cancelled,
}
#[derive(Debug, thiserror::Error)]
#[error("{message}")]
pub struct Error {
    pub code: ErrorCode,
    pub message: String,
}
impl Error {
    pub fn new(code: ErrorCode, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
        }
    }
}
pub fn default_jobs() -> usize {
    thread::available_parallelism()
        .map(usize::from)
        .unwrap_or(1)
        .min(4)
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Options {
    pub mode: Mode,
    pub output_dir: Option<PathBuf>,
    pub existing: Existing,
    pub jobs: usize,
}
impl Default for Options {
    fn default() -> Self {
        Self {
            mode: Mode::Auto,
            output_dir: None,
            existing: Existing::Skip,
            jobs: default_jobs(),
        }
    }
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Item {
    pub id: usize,
    pub input: PathBuf,
    pub output: PathBuf,
    pub input_format: Format,
    pub output_format: Format,
    pub status: Status,
    pub input_bytes: u64,
    pub output_bytes: Option<u64>,
    pub error_code: Option<ErrorCode>,
    pub message: Option<String>,
}
impl Item {
    pub fn fail(&mut self, error: Error) {
        self.status = Status::Failed;
        self.error_code = Some(error.code);
        self.message = Some(error.message);
    }
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Warning {
    pub path: PathBuf,
    pub error_code: ErrorCode,
    pub message: String,
}
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Plan {
    pub items: Vec<Item>,
    pub warnings: Vec<Warning>,
}
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Summary {
    pub total: usize,
    pub queued: usize,
    pub running: usize,
    pub succeeded: usize,
    pub skipped: usize,
    pub failed: usize,
    pub cancelled: usize,
}
impl Summary {
    pub fn from_items(items: &[Item]) -> Self {
        let mut result = Self {
            total: items.len(),
            ..Self::default()
        };
        for item in items {
            match item.status {
                Status::Queued => result.queued += 1,
                Status::Running => result.running += 1,
                Status::Succeeded => result.succeeded += 1,
                Status::Skipped => result.skipped += 1,
                Status::Failed => result.failed += 1,
                Status::Cancelled => result.cancelled += 1,
            }
        }
        result
    }
}
#[derive(Clone, Debug)]
pub enum Event {
    Item { batch: u64, item: Item },
    Finished { batch: u64, items: Vec<Item> },
}
pub struct Batch {
    pub id: u64,
    pub cancel: Arc<AtomicBool>,
    pub events: Receiver<Event>,
}
static NEXT_BATCH: AtomicU64 = AtomicU64::new(1);
pub fn start(plan: Plan, options: Options, cancel: Arc<AtomicBool>) -> Result<Batch, Error> {
    if !(1..=32).contains(&options.jobs) {
        return Err(Error::new(
            ErrorCode::ConversionFailed,
            "Jobs must be between 1 and 32",
        ));
    }
    let id = NEXT_BATCH.fetch_add(1, Ordering::Relaxed);
    let (tx, events) = mpsc::channel();
    let token = cancel.clone();
    thread::spawn(move || {
        let items = Mutex::new(plan.items);
        let next = Mutex::new(0usize);
        thread::scope(|scope| {
            for _ in 0..options.jobs {
                let items = &items;
                let next = &next;
                let tx = &tx;
                let token = &token;
                let options = &options;
                scope.spawn(move || {
                    loop {
                        let work = {
                            let mut cursor = next.lock().unwrap();
                            let mut list = items.lock().unwrap();
                            if token.load(Ordering::Acquire) {
                                break;
                            }
                            while *cursor < list.len() && list[*cursor].status != Status::Queued {
                                *cursor += 1;
                            }
                            if *cursor == list.len() {
                                break;
                            }
                            let index = *cursor;
                            *cursor += 1;
                            list[index].status = Status::Running;
                            (index, list[index].clone())
                        };
                        let (index, mut item) = work;
                        let _ = tx.send(Event::Item {
                            batch: id,
                            item: item.clone(),
                        });
                        match convert(&item, options.existing) {
                            Ok(bytes) => {
                                item.status = Status::Succeeded;
                                item.output_bytes = Some(bytes);
                            }
                            Err(error)
                                if error.code == ErrorCode::OutputExists
                                    && options.existing == Existing::Skip =>
                            {
                                item.status = Status::Skipped;
                                item.message = Some(error.message);
                            }
                            Err(error) => item.fail(error),
                        }
                        items.lock().unwrap()[index] = item.clone();
                        let _ = tx.send(Event::Item { batch: id, item });
                    }
                });
            }
        });
        let mut list = items.into_inner().unwrap();
        for item in &mut list {
            if item.status == Status::Queued {
                item.status = Status::Cancelled;
                item.error_code = Some(ErrorCode::Cancelled);
            }
        }
        let _ = tx.send(Event::Finished {
            batch: id,
            items: list,
        });
    });
    Ok(Batch { id, cancel, events })
}
