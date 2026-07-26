use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    },
};

use tauri::{AppHandle, Emitter};
use ttf2woff2_gui_shared::{BatchSummary, ItemStatus, PROGRESS_EVENT, ProgressEvent, QueueItem};
use uuid::Uuid;

use crate::converter::{self, ConversionError};

#[derive(Clone, Default)]
pub struct BatchManager {
    batches: Arc<Mutex<HashMap<String, Arc<AtomicBool>>>>,
}

impl BatchManager {
    pub fn start(&self, app: AppHandle, mut items: Vec<QueueItem>) -> String {
        let batch_id = Uuid::new_v4().to_string();
        let cancellation = Arc::new(AtomicBool::new(false));
        self.batches
            .lock()
            .expect("batch manager lock poisoned")
            .insert(batch_id.clone(), cancellation.clone());

        let manager = self.clone();
        let task_batch_id = batch_id.clone();
        tauri::async_runtime::spawn_blocking(move || {
            run_batch(&app, &task_batch_id, &mut items, &cancellation);
            manager
                .batches
                .lock()
                .expect("batch manager lock poisoned")
                .remove(&task_batch_id);
        });

        batch_id
    }

    pub fn cancel(&self, batch_id: &str) -> bool {
        let cancellation = self
            .batches
            .lock()
            .expect("batch manager lock poisoned")
            .get(batch_id)
            .cloned();
        if let Some(cancellation) = cancellation {
            cancellation.store(true, Ordering::Release);
            true
        } else {
            false
        }
    }
}

fn run_batch(app: &AppHandle, batch_id: &str, items: &mut [QueueItem], cancellation: &AtomicBool) {
    for index in 0..items.len() {
        if items[index].status == ItemStatus::Skipped {
            emit(app, batch_id, Some(items[index].clone()), items, false);
            continue;
        }

        if cancellation.load(Ordering::Acquire) {
            for cancelled_index in index..items.len() {
                if !items[cancelled_index].status.is_finished() {
                    items[cancelled_index].status = ItemStatus::Cancelled;
                    items[cancelled_index].message =
                        Some("Cancelled before conversion started".into());
                    emit(
                        app,
                        batch_id,
                        Some(items[cancelled_index].clone()),
                        items,
                        false,
                    );
                }
            }
            break;
        }

        let input = PathBuf::from(&items[index].input_path);
        let expected_output = input.with_extension("woff2");
        if !valid_item(&input, &expected_output, &items[index].output_path) {
            items[index].status = ItemStatus::Failed;
            items[index].message = Some("Invalid or changed conversion path".into());
            emit(app, batch_id, Some(items[index].clone()), items, false);
            continue;
        }

        items[index].status = ItemStatus::Running;
        items[index].message = None;
        emit(app, batch_id, Some(items[index].clone()), items, false);

        match converter::convert(&input, &expected_output) {
            Ok(output) => {
                items[index].status = ItemStatus::Succeeded;
                items[index].input_bytes = Some(output.input_bytes);
                items[index].output_bytes = Some(output.output_bytes);
            }
            Err(ConversionError::AlreadyExists) => {
                items[index].status = ItemStatus::Skipped;
                items[index].message = Some("Output file already exists".into());
            }
            Err(ConversionError::Failed(message)) => {
                items[index].status = ItemStatus::Failed;
                items[index].message = Some(message);
            }
        }
        emit(app, batch_id, Some(items[index].clone()), items, false);
    }

    emit(app, batch_id, None, items, true);
}

fn valid_item(input: &Path, expected_output: &Path, supplied_output: &str) -> bool {
    input.is_file()
        && input
            .extension()
            .and_then(|value| value.to_str())
            .is_some_and(|value| value.eq_ignore_ascii_case("ttf"))
        && expected_output == Path::new(supplied_output)
}

fn emit(
    app: &AppHandle,
    batch_id: &str,
    item: Option<QueueItem>,
    items: &[QueueItem],
    finished: bool,
) {
    let _ = app.emit(
        PROGRESS_EVENT,
        ProgressEvent {
            batch_id: batch_id.into(),
            item,
            summary: BatchSummary::from_items(items),
            finished,
        },
    );
}
