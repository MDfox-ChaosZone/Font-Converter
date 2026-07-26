mod api;
mod i18n;

use std::collections::HashSet;

use i18n::{Locale, Message};
use leptos::prelude::*;
use ttf2woff2_gui_shared::{
    BatchSummary, ConversionKind, ItemStatus, ProgressEvent, QueueItem, ScanResult, ScanWarning,
};
use wasm_bindgen_futures::spawn_local;

fn main() {
    mount_to_body(App);
}

#[component]
fn App() -> impl IntoView {
    let locale = RwSignal::new(Locale::load());
    let items = RwSignal::new(Vec::<QueueItem>::new());
    let warnings = RwSignal::new(Vec::<ScanWarning>::new());
    let summary = RwSignal::new(BatchSummary::default());
    let active_batch = RwSignal::new(None::<String>);
    let scanning = RwSignal::new(false);
    let error = RwSignal::new(None::<String>);

    let add_paths = move |paths: Vec<String>| {
        if paths.is_empty() {
            return;
        }
        scanning.set(true);
        error.set(None);
        spawn_local(async move {
            match api::collect_inputs(paths).await {
                Ok(result) => merge_scan_result(items, warnings, summary, result),
                Err(message) => error.set(Some(message)),
            }
            scanning.set(false);
        });
    };

    api::setup_drag_drop_listener(add_paths);
    api::setup_progress_listener(move |event: ProgressEvent| {
        if active_batch.get_untracked().as_deref() == Some("") {
            active_batch.set(Some(event.batch_id.clone()));
        }
        if let Some(updated) = event.item {
            items.update(|items| {
                if let Some(item) = items.iter_mut().find(|item| item.id == updated.id) {
                    *item = updated;
                }
            });
        }
        summary.set(BatchSummary::from_items(&items.get_untracked()));
        let event_matches_active = active_batch
            .get_untracked()
            .as_deref()
            .is_some_and(|id| id.is_empty() || id == event.batch_id);
        if event.finished && event_matches_active {
            active_batch.set(None);
        }
    });

    let choose_files = move |_| {
        spawn_local(async move {
            match api::pick_files().await {
                Ok(paths) => add_paths(paths),
                Err(message) => error.set(Some(message)),
            }
        });
    };
    let choose_folder = move |_| {
        spawn_local(async move {
            match api::pick_folder().await {
                Ok(paths) => add_paths(paths),
                Err(message) => error.set(Some(message)),
            }
        });
    };
    let start = move |_| {
        let pending = items.with_untracked(|items| {
            items
                .iter()
                .filter(|item| item.status == ItemStatus::Queued)
                .cloned()
                .collect::<Vec<_>>()
        });
        if pending.is_empty() {
            return;
        }
        error.set(None);
        active_batch.set(Some(String::new()));
        spawn_local(async move {
            match api::start_conversion(pending).await {
                Ok(batch_id) => {
                    if active_batch.get_untracked().is_some() {
                        active_batch.set(Some(batch_id));
                    }
                }
                Err(message) => {
                    active_batch.set(None);
                    error.set(Some(message));
                }
            }
        });
    };
    let cancel = move |_| {
        let Some(batch_id) = active_batch.get_untracked() else {
            return;
        };
        spawn_local(async move {
            if let Err(message) = api::cancel_conversion(batch_id).await {
                error.set(Some(message));
            }
        });
    };
    let clear_completed = move |_| {
        items.update(|items| items.retain(|item| !item.status.is_finished()));
        if items.read().is_empty() {
            summary.set(BatchSummary::default());
        }
    };
    let remove_item = Callback::new(move |id: String| {
        if active_batch.get_untracked().is_some() {
            return;
        }
        items.update(|items| items.retain(|item| item.id != id));
        summary.set(BatchSummary::from_items(&items.get_untracked()));
    });
    let set_locale = move |next: Locale| {
        locale.set(next);
        next.save();
    };

    let pending_count = move || {
        items.with(|items| {
            items
                .iter()
                .filter(|item| item.status == ItemStatus::Queued)
                .count()
        })
    };

    view! {
        <main class="app-shell">
            <header class="topbar">
                <div class="brand">
                    <div class="brand-mark">"W2"</div>
                    <div>
                        <h1>"ttf2woff2-GUI"</h1>
                        <p>{move || locale.get().t(Message::Tagline)}</p>
                    </div>
                </div>
                <div class="language" aria-label=move || locale.get().t(Message::Language)>
                    <button
                        class:active=move || locale.get() == Locale::ZhCn
                        on:click=move |_| set_locale(Locale::ZhCn)
                    >"中文"</button>
                    <button
                        class:active=move || locale.get() == Locale::En
                        on:click=move |_| set_locale(Locale::En)
                    >"EN"</button>
                </div>
            </header>

            <section class="drop-zone" class:busy=move || scanning.get()>
                <div class="drop-icon" aria-hidden="true">"Aa"</div>
                <h2>{move || locale.get().t(Message::DropTitle)}</h2>
                <p>{move || locale.get().t(Message::DropHint)}</p>
                <div class="format-pills" aria-label=move || locale.get().t(Message::SupportedFormats)>
                    <span>"TTF → WOFF2"</span>
                    <span>"OTF → WOFF2"</span>
                    <span>"WOFF2 → TTF / OTF"</span>
                </div>
                <div class="picker-actions">
                    <button class="button secondary" on:click=choose_files disabled=move || active_batch.get().is_some()>
                        {move || locale.get().t(Message::SelectFiles)}
                    </button>
                    <button class="button secondary" on:click=choose_folder disabled=move || active_batch.get().is_some()>
                        {move || locale.get().t(Message::SelectFolder)}
                    </button>
                </div>
                <span class="safe-note">"✓ " {move || locale.get().t(Message::SafeOutput)}</span>
            </section>

            {move || error.get().map(|message| view! {
                <div class="alert error" role="alert">
                    <strong>{locale.get().t(Message::CommandFailed)}</strong>
                    <span>{message}</span>
                </div>
            })}

            {move || (!warnings.read().is_empty()).then(|| view! {
                <details class="alert warnings">
                    <summary>{locale.get().t(Message::Warnings)} " (" {warnings.read().len()} ")"</summary>
                    <ul>
                        <For
                            each=move || warnings.get()
                            key=|warning| format!("{}:{}", warning.path, warning.message)
                            children=|warning| view! {
                                <li><code>{warning.path}</code> " — " {warning.message}</li>
                            }
                        />
                    </ul>
                </details>
            })}

            <section class="queue-card">
                <div class="queue-toolbar">
                    <div>
                        <h2>{move || locale.get().t(Message::Total)} " · " {move || items.read().len()}</h2>
                        <BatchSummaryView locale summary />
                    </div>
                    <div class="queue-actions">
                        <button
                            class="button ghost"
                            on:click=clear_completed
                            disabled=move || active_batch.get().is_some() || !items.read().iter().any(|item| item.status.is_finished())
                        >{move || locale.get().t(Message::ClearCompleted)}</button>
                        <Show
                            when=move || active_batch.get().is_some()
                            fallback=move || view! {
                                <button
                                    class="button primary"
                                    on:click=start
                                    disabled=move || pending_count() == 0 || scanning.get()
                                >{move || locale.get().t(Message::Start)} " (" {pending_count} ")"</button>
                            }
                        >
                            <button class="button danger" on:click=cancel>
                                {move || locale.get().t(Message::Cancel)}
                            </button>
                        </Show>
                    </div>
                </div>

                <Show
                    when=move || !items.read().is_empty()
                    fallback=move || view! {
                        <div class="empty-state">
                            <div>"W2"</div>
                            <h3>{locale.get().t(Message::EmptyTitle)}</h3>
                            <p>{locale.get().t(Message::EmptyHint)}</p>
                        </div>
                    }
                >
                    <div class="queue-list">
                        <div class="queue-head">
                            <span>{move || locale.get().t(Message::File)}</span>
                            <span class="input-size">{move || locale.get().t(Message::InputSize)}</span>
                            <span class="output-size">{move || locale.get().t(Message::OutputSize)}</span>
                            <span>{move || locale.get().t(Message::Status)}</span>
                            <span>{move || locale.get().t(Message::Actions)}</span>
                        </div>
                        <For
                            each=move || items.get()
                            key=|item| format!(
                                "{}:{:?}:{:?}:{:?}",
                                item.id, item.status, item.output_bytes, item.message
                            )
                            children=move |item| view! {
                                <QueueRow locale item active_batch on_remove=remove_item />
                            }
                        />
                    </div>
                </Show>
            </section>
        </main>
    }
}

#[component]
fn QueueRow(
    locale: RwSignal<Locale>,
    item: QueueItem,
    active_batch: RwSignal<Option<String>>,
    on_remove: Callback<String>,
) -> impl IntoView {
    let status_class = format!("status {}", status_class(&item.status));
    let input_name = file_name(&item.input_path);
    let output_name = file_name(&item.output_path);
    let status = item.status.clone();
    let item_id = item.id.clone();
    view! {
        <article class="queue-row">
            <div class="file-cell">
                <div class="file-badge">{conversion_badge(item.conversion)}</div>
                <div class="file-info">
                    <strong title=item.input_path.clone()>{input_name}</strong>
                    <span title=item.output_path.clone()>
                        {move || locale.get().t(Message::Output)} ": " {output_name}
                    </span>
                    {item.message.map(|message| view! { <small>{message}</small> })}
                </div>
            </div>
            <span class="size input-size">{format_bytes(item.input_bytes)}</span>
            <span class="size output-size">{format_bytes(item.output_bytes)}</span>
            <span class=status_class>{move || status_label(locale.get(), &status)}</span>
            <button
                class="remove-item"
                type="button"
                disabled=move || active_batch.get().is_some()
                title=move || locale.get().t(Message::Remove)
                on:click=move |_| on_remove.run(item_id.clone())
            >
                {move || locale.get().t(Message::Remove)}
            </button>
        </article>
    }
}

#[component]
fn BatchSummaryView(locale: RwSignal<Locale>, summary: RwSignal<BatchSummary>) -> impl IntoView {
    view! {
        <div class="summary">
            <span class="success-dot"></span>
            <span>{move || locale.get().t(Message::Succeeded)} " " {move || summary.get().succeeded}</span>
            <span class="skip-dot"></span>
            <span>{move || locale.get().t(Message::Skipped)} " " {move || summary.get().skipped}</span>
            <span class="fail-dot"></span>
            <span>{move || locale.get().t(Message::Failed)} " " {move || summary.get().failed}</span>
        </div>
    }
}

fn merge_scan_result(
    items: RwSignal<Vec<QueueItem>>,
    warnings: RwSignal<Vec<ScanWarning>>,
    summary: RwSignal<BatchSummary>,
    result: ScanResult,
) {
    items.update(|current| {
        let mut known = current
            .iter()
            .map(|item| item.input_path.clone())
            .collect::<HashSet<_>>();
        current.extend(
            result
                .items
                .into_iter()
                .filter(|item| known.insert(item.input_path.clone())),
        );
    });
    warnings.update(|current| {
        let mut known = current
            .iter()
            .map(|warning| (warning.path.clone(), warning.message.clone()))
            .collect::<HashSet<_>>();
        current.extend(
            result
                .warnings
                .into_iter()
                .filter(|warning| known.insert((warning.path.clone(), warning.message.clone()))),
        );
    });
    summary.set(BatchSummary::from_items(&items.get_untracked()));
}

fn status_label(locale: Locale, status: &ItemStatus) -> &'static str {
    locale.t(match status {
        ItemStatus::Queued => Message::Queued,
        ItemStatus::Running => Message::Running,
        ItemStatus::Succeeded => Message::Succeeded,
        ItemStatus::Skipped => Message::Skipped,
        ItemStatus::Failed => Message::Failed,
        ItemStatus::Cancelled => Message::Cancelled,
    })
}

fn conversion_badge(conversion: ConversionKind) -> &'static str {
    match conversion {
        ConversionKind::TtfToWoff2 => "TTF→W2",
        ConversionKind::OtfToWoff2 => "OTF→W2",
        ConversionKind::Woff2ToTtf => "W2→TTF",
        ConversionKind::Woff2ToOtf => "W2→OTF",
    }
}

fn status_class(status: &ItemStatus) -> &'static str {
    match status {
        ItemStatus::Queued => "queued",
        ItemStatus::Running => "running",
        ItemStatus::Succeeded => "succeeded",
        ItemStatus::Skipped => "skipped",
        ItemStatus::Failed => "failed",
        ItemStatus::Cancelled => "cancelled",
    }
}

fn file_name(path: &str) -> String {
    std::path::Path::new(path)
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_else(|| path.into())
}

fn format_bytes(bytes: Option<u64>) -> String {
    let Some(bytes) = bytes else {
        return "—".into();
    };
    if bytes < 1024 {
        format!("{bytes} B")
    } else if bytes < 1024 * 1024 {
        format!("{:.1} KB", bytes as f64 / 1024.0)
    } else {
        format!("{:.1} MB", bytes as f64 / (1024.0 * 1024.0))
    }
}
