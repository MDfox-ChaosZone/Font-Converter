use std::{
    collections::HashMap,
    path::PathBuf,
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, AtomicUsize, Ordering},
    },
    thread,
};

use fontbridge_shared::{BatchSummary, ItemStatus, PROGRESS_EVENT, ProgressEvent, QueueItem};
use tauri::{AppHandle, Emitter};
use uuid::Uuid;

use crate::{
    converter::{self, ConversionError},
    scanner,
};

const MAX_PARALLEL_CONVERSIONS: usize = 4;

#[derive(Clone, Default)]
pub struct BatchManager {
    batches: Arc<Mutex<HashMap<String, Arc<AtomicBool>>>>,
}

impl BatchManager {
    pub fn start(&self, app: AppHandle, items: Vec<QueueItem>) -> String {
        let batch_id = Uuid::new_v4().to_string();
        let cancellation = Arc::new(AtomicBool::new(false));
        self.batches
            .lock()
            .expect("batch manager lock poisoned")
            .insert(batch_id.clone(), cancellation.clone());

        let manager = self.clone();
        let task_batch_id = batch_id.clone();
        tauri::async_runtime::spawn_blocking(move || {
            run_batch(&app, &task_batch_id, items, &cancellation);
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

fn run_batch(app: &AppHandle, batch_id: &str, items: Vec<QueueItem>, cancellation: &AtomicBool) {
    let total = items.len();
    let items = Arc::new(Mutex::new(items));
    let next_index = AtomicUsize::new(0);

    thread::scope(|scope| {
        for _ in 0..parallelism_for(total) {
            let items = Arc::clone(&items);
            let next_index = &next_index;
            scope.spawn(move || {
                loop {
                    if cancellation.load(Ordering::Acquire) {
                        break;
                    }
                    let index = next_index.fetch_add(1, Ordering::AcqRel);
                    if index >= total {
                        break;
                    }

                    let item = { items.lock().expect("batch items lock poisoned")[index].clone() };
                    if item.status == ItemStatus::Skipped {
                        emit_snapshot(app, batch_id, Some(item), &items, false);
                        continue;
                    }
                    if cancellation.load(Ordering::Acquire) {
                        break;
                    }

                    let input = PathBuf::from(&item.input_path);
                    let output = PathBuf::from(&item.output_path);
                    let Ok((expected_kind, default_output)) = scanner::conversion_for(&input)
                    else {
                        update_item(
                            app,
                            batch_id,
                            &items,
                            index,
                            ItemStatus::Failed,
                            Some("Invalid or changed input font".into()),
                            None,
                            None,
                        );
                        continue;
                    };
                    if expected_kind != item.conversion
                        || default_output.file_name() != output.file_name()
                    {
                        update_item(
                            app,
                            batch_id,
                            &items,
                            index,
                            ItemStatus::Failed,
                            Some("Invalid or changed conversion path".into()),
                            None,
                            None,
                        );
                        continue;
                    }

                    update_item(
                        app,
                        batch_id,
                        &items,
                        index,
                        ItemStatus::Running,
                        None,
                        None,
                        None,
                    );

                    match converter::convert(&input, &output, expected_kind) {
                        Ok(converted) => update_item(
                            app,
                            batch_id,
                            &items,
                            index,
                            ItemStatus::Succeeded,
                            None,
                            Some(converted.input_bytes),
                            Some(converted.output_bytes),
                        ),
                        Err(ConversionError::AlreadyExists) => update_item(
                            app,
                            batch_id,
                            &items,
                            index,
                            ItemStatus::Skipped,
                            Some("Output file already exists".into()),
                            None,
                            std::fs::metadata(&output)
                                .ok()
                                .map(|metadata| metadata.len()),
                        ),
                        Err(ConversionError::Failed(message)) => update_item(
                            app,
                            batch_id,
                            &items,
                            index,
                            ItemStatus::Failed,
                            Some(message),
                            None,
                            None,
                        ),
                    }
                }
            });
        }
    });

    if cancellation.load(Ordering::Acquire) {
        let cancelled = {
            let mut items = items.lock().expect("batch items lock poisoned");
            let mut cancelled = Vec::new();
            for item in items.iter_mut() {
                if item.status == ItemStatus::Queued {
                    item.status = ItemStatus::Cancelled;
                    item.message = Some("Cancelled before conversion started".into());
                    cancelled.push(item.clone());
                }
            }
            cancelled
        };
        for item in cancelled {
            emit_snapshot(app, batch_id, Some(item), &items, false);
        }
    }

    emit_snapshot(app, batch_id, None, &items, true);
}

#[allow(clippy::too_many_arguments)]
fn update_item(
    app: &AppHandle,
    batch_id: &str,
    items: &Arc<Mutex<Vec<QueueItem>>>,
    index: usize,
    status: ItemStatus,
    message: Option<String>,
    input_bytes: Option<u64>,
    output_bytes: Option<u64>,
) {
    let item = {
        let mut items = items.lock().expect("batch items lock poisoned");
        let item = &mut items[index];
        item.status = status;
        item.message = message;
        if let Some(input_bytes) = input_bytes {
            item.input_bytes = Some(input_bytes);
        }
        if let Some(output_bytes) = output_bytes {
            item.output_bytes = Some(output_bytes);
        }
        item.clone()
    };
    emit_snapshot(app, batch_id, Some(item), items, false);
}

fn emit_snapshot(
    app: &AppHandle,
    batch_id: &str,
    item: Option<QueueItem>,
    items: &Arc<Mutex<Vec<QueueItem>>>,
    finished: bool,
) {
    let snapshot = items.lock().expect("batch items lock poisoned").clone();
    emit(app, batch_id, item, &snapshot, finished);
}

fn parallelism_for(item_count: usize) -> usize {
    thread::available_parallelism()
        .map(usize::from)
        .unwrap_or(1)
        .min(MAX_PARALLEL_CONVERSIONS)
        .min(item_count.max(1))
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

#[cfg(test)]
mod tests {
    use super::{MAX_PARALLEL_CONVERSIONS, parallelism_for};

    #[test]
    fn parallelism_is_bounded_by_the_queue_and_worker_limit() {
        assert_eq!(parallelism_for(0), 1);
        assert_eq!(parallelism_for(1), 1);
        assert!(parallelism_for(2) <= 2);
        assert!(parallelism_for(100) <= MAX_PARALLEL_CONVERSIONS);
        assert!(parallelism_for(100) >= 1);
    }
}
