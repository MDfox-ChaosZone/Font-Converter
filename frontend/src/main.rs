mod api;
mod i18n;

use std::collections::HashSet;

use font_converter_shared::{
    BatchSummary, ConversionKind, FolderConversionMode, ItemStatus, ProgressEvent, QueueItem,
    ScanResult, ScanWarning,
};
use i18n::{Locale, Message, Theme};
use leptos::{ev, leptos_dom::helpers::window_event_listener, prelude::*};
use wasm_bindgen_futures::spawn_local;

const MIN_COLUMN_WIDTHS: [i32; 5] = [68, 120, 140, 88, 56];

#[derive(Clone, Copy)]
struct ColumnResize {
    index: usize,
    start_x: i32,
    start_width: i32,
}

fn main() {
    mount_to_body(App);
}

#[component]
fn App() -> impl IntoView {
    let locale = RwSignal::new(Locale::load());
    let theme = RwSignal::new(Theme::load());
    theme.get_untracked().apply();
    let items = RwSignal::new(Vec::<QueueItem>::new());
    let warnings = RwSignal::new(Vec::<ScanWarning>::new());
    let summary = RwSignal::new(BatchSummary::default());
    let active_batch = RwSignal::new(None::<String>);
    let scanning = RwSignal::new(false);
    let dragging = RwSignal::new(false);
    let output_directory = RwSignal::new(None::<String>);
    let pending_folder = RwSignal::new(None::<String>);
    let pending_folder_scan = RwSignal::new(None::<ScanResult>);
    let folder_conversion_mode = RwSignal::new(FolderConversionMode::Both);
    let error = RwSignal::new(None::<String>);
    let column_widths = RwSignal::new([78_i32, 160, 180, 96, 62]);
    let resizing = RwSignal::new(None::<ColumnResize>);

    let pointer_move = window_event_listener(ev::pointermove, move |event| {
        let Some(resize) = resizing.get_untracked() else {
            return;
        };
        let next = resized_column_width(
            resize.start_width,
            resize.start_x,
            event.client_x(),
            MIN_COLUMN_WIDTHS[resize.index],
        );
        column_widths.update(|widths| widths[resize.index] = next);
    });
    let pointer_up = window_event_listener(ev::pointerup, move |_| resizing.set(None));
    on_cleanup(move || {
        pointer_move.remove();
        pointer_up.remove();
    });

    let add_paths = move |paths: Vec<String>| {
        if paths.is_empty() || scanning.get_untracked() || active_batch.get_untracked().is_some() {
            return;
        }
        scanning.set(true);
        error.set(None);
        spawn_local(async move {
            match api::collect_inputs(paths, output_directory.get_untracked(), None).await {
                Ok(result) => merge_scan_result(items, warnings, summary, result),
                Err(message) => error.set(Some(message)),
            }
            scanning.set(false);
        });
    };

    api::setup_drag_drop_listener(move |is_dragging, paths| {
        dragging.set(is_dragging);
        if !paths.is_empty() {
            add_paths(paths);
        }
    });
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
                Ok(paths) => {
                    if let Some(path) = paths.into_iter().next() {
                        if scanning.get_untracked() || active_batch.get_untracked().is_some() {
                            return;
                        }
                        scanning.set(true);
                        error.set(None);
                        pending_folder_scan.set(None);
                        let scan = api::collect_inputs(
                            vec![path.clone()],
                            output_directory.get_untracked(),
                            Some(FolderConversionMode::Both),
                        )
                        .await;
                        match scan {
                            Ok(result) => {
                                let has_font_to_woff2 = result.items.iter().any(|item| {
                                    FolderConversionMode::FontToWoff2.accepts(item.conversion)
                                });
                                let has_woff2_to_font = result.items.iter().any(|item| {
                                    FolderConversionMode::Woff2ToFont.accepts(item.conversion)
                                });
                                if has_font_to_woff2 && has_woff2_to_font {
                                    folder_conversion_mode.set(FolderConversionMode::Both);
                                    pending_folder_scan.set(Some(result));
                                    pending_folder.set(Some(path));
                                } else {
                                    merge_scan_result(items, warnings, summary, result);
                                }
                            }
                            Err(message) => error.set(Some(message)),
                        }
                        scanning.set(false);
                    }
                }
                Err(message) => error.set(Some(message)),
            }
        });
    };
    let cancel_folder_selection = Callback::new(move |_: ()| {
        pending_folder.set(None);
        pending_folder_scan.set(None);
    });
    let confirm_folder_selection = Callback::new(move |_: ()| {
        let Some(path) = pending_folder.get_untracked() else {
            return;
        };
        if scanning.get_untracked() || active_batch.get_untracked().is_some() {
            return;
        }
        pending_folder.set(None);
        scanning.set(true);
        error.set(None);
        let mode = folder_conversion_mode.get_untracked();
        let pending_scan = pending_folder_scan.get_untracked();
        pending_folder_scan.set(None);
        if let Some(result) = pending_scan {
            merge_scan_result(items, warnings, summary, filter_scan_result(result, mode));
            scanning.set(false);
            return;
        }
        spawn_local(async move {
            match api::collect_inputs(vec![path], output_directory.get_untracked(), Some(mode))
                .await
            {
                Ok(result) => merge_scan_result(items, warnings, summary, result),
                Err(message) => error.set(Some(message)),
            }
            scanning.set(false);
        });
    });
    let retarget_outputs = move |next_directory: Option<String>| {
        if scanning.get_untracked() || active_batch.get_untracked().is_some() {
            return;
        }
        output_directory.set(next_directory.clone());
        let paths = items.with_untracked(|items| {
            items
                .iter()
                .map(|item| item.input_path.clone())
                .collect::<Vec<_>>()
        });
        if paths.is_empty() {
            return;
        }
        scanning.set(true);
        error.set(None);
        spawn_local(async move {
            match api::collect_inputs(paths, next_directory, None).await {
                Ok(result) => replace_scan_result(items, warnings, summary, result),
                Err(message) => error.set(Some(message)),
            }
            scanning.set(false);
        });
    };
    let choose_output_folder = move |_| {
        spawn_local(async move {
            match api::pick_folder().await {
                Ok(paths) => {
                    if let Some(path) = paths.into_iter().next() {
                        retarget_outputs(Some(path));
                    }
                }
                Err(message) => error.set(Some(message)),
            }
        });
    };
    let reset_output_folder = move |_| retarget_outputs(None);
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
        summary.set(BatchSummary::from_items(&items.get_untracked()));
    };
    let clear_all = move |_| {
        items.set(Vec::new());
        warnings.set(Vec::new());
        summary.set(BatchSummary::default());
        error.set(None);
    };
    let remove_item = Callback::new(move |id: String| {
        if active_batch.get_untracked().is_some() {
            return;
        }
        items.update(|items| items.retain(|item| item.id != id));
        summary.set(BatchSummary::from_items(&items.get_untracked()));
    });
    let open_output_folder = Callback::new(move |output_path: String| {
        error.set(None);
        spawn_local(async move {
            if let Err(message) = api::open_output_folder(output_path).await {
                error.set(Some(message));
            }
        });
    });
    let set_locale = move |next: Locale| {
        locale.set(next);
        next.save();
    };
    let set_theme = move |next: Theme| {
        theme.set(next);
        next.save();
        next.apply();
    };

    let pending_count = move || summary.get().queued;
    let finished_count = move || completed_count(&summary.get());
    let has_finished = move || finished_count() > 0;
    let progress_percent = move || {
        let current = summary.get();
        completed_count(&current)
            .saturating_mul(100)
            .checked_div(current.total)
            .unwrap_or_default()
    };
    let queue_columns_style = move || {
        let widths = column_widths.get();
        format!(
            "--direction-column: {}px; --name-column: {}px; --path-column: {}px; \
             --size-column: {}px; --status-column: {}px",
            widths[0], widths[1], widths[2], widths[3], widths[4]
        )
    };

    view! {
        <main class="app-shell">
            <header class="topbar">
                <div class="brand">
                    <div class="brand-mark">"FC"</div>
                    <div>
                        <h1>"Font Converter"</h1>
                        <p>{move || locale.get().t(Message::Tagline)}</p>
                    </div>
                </div>
                <div class="topbar-actions">
                    <div class="theme-switcher" role="group" aria-label=move || locale.get().t(Message::Theme)>
                        <button
                            type="button"
                            class:active=move || theme.get() == Theme::System
                            on:click=move |_| set_theme(Theme::System)
                        >{move || locale.get().t(Message::ThemeSystem)}</button>
                        <button
                            type="button"
                            class:active=move || theme.get() == Theme::Light
                            on:click=move |_| set_theme(Theme::Light)
                        >{move || locale.get().t(Message::ThemeLight)}</button>
                        <button
                            type="button"
                            class:active=move || theme.get() == Theme::Dark
                            on:click=move |_| set_theme(Theme::Dark)
                        >{move || locale.get().t(Message::ThemeDark)}</button>
                    </div>
                    <div class="language" aria-label=move || locale.get().t(Message::Language)>
                        <button
                            type="button"
                            class:active=move || locale.get() == Locale::ZhCn
                            on:click=move |_| set_locale(Locale::ZhCn)
                        >"中文"</button>
                        <button
                            type="button"
                            class:active=move || locale.get() == Locale::En
                            on:click=move |_| set_locale(Locale::En)
                        >"EN"</button>
                    </div>
                </div>
            </header>

            <div class="workspace">
                <section
                    class="drop-zone"
                    class:busy=move || scanning.get()
                    class:dragging=move || dragging.get()
                >
                    <div class="drop-content">
                        <div class="drop-icon" aria-hidden="true">"Aa"</div>
                        <h2>{move || locale.get().t(Message::DropTitle)}</h2>
                        <p class="drop-hint">{move || locale.get().t(Message::DropHint)}</p>
                        <div
                            class="format-pills"
                            aria-label=move || locale.get().t(Message::SupportedFormats)
                        >
                            <span class="format-pill">"TTF / OTF → WOFF2"</span>
                            <span class="reverse-format">
                                <span class="format-pill">"WOFF2 → TTF / OTF"</span>
                                <span
                                    class="help-tooltip"
                                    tabindex="0"
                                    aria-label=move || locale.get().t(Message::AutoDetectHint)
                                >
                                    "?"
                                    <span role="tooltip">
                                        {move || locale.get().t(Message::AutoDetectHint)}
                                    </span>
                                </span>
                            </span>
                        </div>

                        <div class="picker-actions">
                            <button
                                class="button secondary"
                                type="button"
                                on:click=choose_files
                                disabled=move || active_batch.get().is_some() || scanning.get()
                            >
                                {move || locale.get().t(Message::SelectFiles)}
                            </button>
                            <button
                                class="button secondary"
                                type="button"
                                on:click=choose_folder
                                disabled=move || active_batch.get().is_some() || scanning.get()
                            >
                                {move || locale.get().t(Message::SelectFolder)}
                            </button>
                        </div>

                        <div class="output-destination">
                            <button
                                class="output-destination-copy"
                                type="button"
                                title=move || locale.get().t(Message::ChooseOutputFolder)
                                on:click=choose_output_folder
                                disabled=move || active_batch.get().is_some() || scanning.get()
                            >
                                <span>{move || locale.get().t(Message::OutputFolder)}</span>
                                <strong title=move || {
                                    output_directory
                                        .get()
                                        .unwrap_or_else(|| locale.get().t(Message::SourceFolder).into())
                                }>
                                    {move || {
                                        output_directory
                                            .get()
                                            .map(|path| file_name(&path))
                                            .unwrap_or_else(|| locale.get().t(Message::SourceFolder).into())
                                    }}
                                </strong>
                            </button>
                            <div class="output-destination-actions">
                                <button
                                    type="button"
                                    class="destination-button"
                                    title=move || locale.get().t(Message::ChooseOutputFolder)
                                    aria-label=move || locale.get().t(Message::ChooseOutputFolder)
                                    on:click=choose_output_folder
                                    disabled=move || active_batch.get().is_some() || scanning.get()
                                >
                                    <FolderIcon />
                                </button>
                                <Show when=move || output_directory.get().is_some()>
                                    <button
                                        type="button"
                                        class="destination-button reset"
                                        title=move || locale.get().t(Message::ResetOutputFolder)
                                        aria-label=move || locale.get().t(Message::ResetOutputFolder)
                                        on:click=reset_output_folder
                                        disabled=move || active_batch.get().is_some() || scanning.get()
                                    >
                                        <span aria-hidden="true">"↺"</span>
                                    </button>
                                </Show>
                            </div>
                        </div>

                        <div class="scan-state" role="status" aria-live="polite">
                            <Show when=move || scanning.get()>
                                <span class="scanning-note">
                                    <span class="spinner" aria-hidden="true"></span>
                                    {move || locale.get().t(Message::Scanning)}
                                </span>
                            </Show>
                        </div>
                    </div>
                </section>

                <section class="queue-card" aria-label=move || locale.get().t(Message::Queue)>
                    <div class="queue-toolbar">
                            <div class="queue-overview">
                            <div class="queue-title-line">
                                <h2>{move || locale.get().t(Message::Queue)}</h2>
                            </div>
                            <Show when=move || { summary.get().total > 0 }>
                                <p class="completion" role="status" aria-live="polite">
                                    {move || locale.get().t(Message::Completed)}
                                    " "
                                    <strong>{finished_count}</strong>
                                    " / "
                                    <strong>{move || summary.get().total}</strong>
                                </p>
                                <div class="progress-track" aria-hidden="true">
                                    <span style:width=move || format!("{}%", progress_percent())></span>
                                </div>
                                <BatchSummaryView locale summary />
                            </Show>
                        </div>
                        <div class="queue-actions">
                            <Show when=move || {
                                active_batch.get().is_none() && has_finished()
                            }>
                                <button class="button ghost" type="button" on:click=clear_completed>
                                    {move || locale.get().t(Message::ClearCompleted)}
                                </button>
                            </Show>
                            <Show when=move || {
                                active_batch.get().is_none() && !items.read().is_empty()
                            }>
                                <button class="button ghost" type="button" on:click=clear_all>
                                    {move || locale.get().t(Message::ClearAll)}
                                </button>
                            </Show>
                            <Show
                                when=move || active_batch.get().is_some()
                                fallback=move || view! {
                                    <button
                                        class="button primary"
                                        type="button"
                                        on:click=start
                                        disabled=move || pending_count() == 0 || scanning.get()
                                    >
                                        {move || locale.get().t(Message::Start)}
                                        <Show when=move || { pending_count() > 0 }>
                                            " ("
                                            {pending_count}
                                            ")"
                                        </Show>
                                    </button>
                                }
                            >
                                <button class="button danger" type="button" on:click=cancel>
                                    {move || locale.get().t(Message::Cancel)}
                                </button>
                            </Show>
                        </div>
                    </div>

                    <div class="queue-notices">
                        {move || error.get().map(|message| view! {
                            <div class="alert error" role="alert" aria-live="assertive">
                                <strong>{locale.get().t(Message::CommandFailed)}</strong>
                                <span>{message}</span>
                            </div>
                        })}

                        {move || (!warnings.read().is_empty()).then(|| view! {
                            <details class="alert warnings">
                                <summary>
                                    {locale.get().t(Message::Warnings)}
                                    " ("
                                    {warnings.read().len()}
                                    ")"
                                </summary>
                                <ul>
                                    <For
                                        each=move || warnings.get()
                                        key=|warning| format!("{}:{}", warning.path, warning.message)
                                        children=|warning| view! {
                                            <li>
                                                <code>{warning.path}</code>
                                                " — "
                                                {warning.message}
                                            </li>
                                        }
                                    />
                                </ul>
                            </details>
                        })}
                    </div>

                    <Show
                        when=move || !items.read().is_empty()
                        fallback=move || view! {
                            <div class="empty-state">
                                <h3>{locale.get().t(Message::EmptyTitle)}</h3>
                                <p>{locale.get().t(Message::EmptyHint)}</p>
                            </div>
                        }
                    >
                        <div class="queue-list" style=queue_columns_style>
                            <div class="queue-head">
                                <ColumnHeading
                                    locale
                                    label=Message::ConversionDirection
                                    index=0
                                    column_widths
                                    resizing
                                />
                                <ColumnHeading
                                    locale
                                    label=Message::File
                                    index=1
                                    column_widths
                                    resizing
                                />
                                <ColumnHeading
                                    locale
                                    label=Message::Path
                                    index=2
                                    column_widths
                                    resizing
                                />
                                <ColumnHeading
                                    locale
                                    label=Message::SizeChange
                                    index=3
                                    column_widths
                                    resizing
                                    class="size-change"
                                />
                                <ColumnHeading
                                    locale
                                    label=Message::Status
                                    index=4
                                    column_widths
                                    resizing
                                />
                                <span class="visually-hidden">{move || locale.get().t(Message::Actions)}</span>
                            </div>
                            <For
                                each=move || items.get()
                                key=|item| format!(
                                    "{}:{:?}:{:?}:{:?}",
                                    item.id, item.status, item.output_bytes, item.message
                                )
                                children=move |item| view! {
                                    <QueueRow
                                        locale
                                        item
                                        active_batch
                                        on_remove=remove_item
                                        on_open_output=open_output_folder
                                    />
                                }
                            />
                        </div>
                    </Show>
                </section>
            </div>

            <Show when=move || pending_folder.get().is_some()>
                <FolderDirectionDialog
                    locale
                    mode=folder_conversion_mode
                    pending_folder
                    on_cancel=cancel_folder_selection
                    on_confirm=confirm_folder_selection
                />
            </Show>
        </main>
    }
}

#[component]
fn FolderDirectionDialog(
    locale: RwSignal<Locale>,
    mode: RwSignal<FolderConversionMode>,
    pending_folder: RwSignal<Option<String>>,
    on_cancel: Callback<()>,
    on_confirm: Callback<()>,
) -> impl IntoView {
    view! {
        <div class="folder-dialog-backdrop">
            <section
                class="folder-dialog"
                role="dialog"
                aria-modal="true"
                aria-labelledby="folder-dialog-title"
            >
                <h2 id="folder-dialog-title">
                    {move || locale.get().t(Message::FolderDirectionTitle)}
                </h2>
                <p
                    class="folder-dialog-path"
                    title=move || pending_folder.get().unwrap_or_default()
                >
                    {move || {
                        pending_folder
                            .get()
                            .map(|path| file_name(&path))
                            .unwrap_or_default()
                    }}
                </p>
                <div
                    class="folder-direction-options"
                    role="radiogroup"
                    aria-label=move || locale.get().t(Message::ConversionDirection)
                >
                    <button
                        type="button"
                        role="radio"
                        class="folder-direction-option"
                        class:active=move || {
                            mode.get() == FolderConversionMode::FontToWoff2
                        }
                        aria-checked=move || {
                            mode.get() == FolderConversionMode::FontToWoff2
                        }
                        on:click=move |_| mode.set(FolderConversionMode::FontToWoff2)
                    >
                        {move || locale.get().t(Message::FolderFontToWoff2)}
                    </button>
                    <button
                        type="button"
                        role="radio"
                        class="folder-direction-option"
                        class:active=move || {
                            mode.get() == FolderConversionMode::Woff2ToFont
                        }
                        aria-checked=move || {
                            mode.get() == FolderConversionMode::Woff2ToFont
                        }
                        on:click=move |_| mode.set(FolderConversionMode::Woff2ToFont)
                    >
                        {move || locale.get().t(Message::FolderWoff2ToFont)}
                    </button>
                    <button
                        type="button"
                        role="radio"
                        class="folder-direction-option"
                        class:active=move || mode.get() == FolderConversionMode::Both
                        aria-checked=move || mode.get() == FolderConversionMode::Both
                        on:click=move |_| mode.set(FolderConversionMode::Both)
                    >
                        {move || locale.get().t(Message::FolderBoth)}
                    </button>
                </div>
                <div class="folder-dialog-actions">
                    <button
                        class="button ghost"
                        type="button"
                        on:click=move |_| on_cancel.run(())
                    >
                        {move || locale.get().t(Message::Cancel)}
                    </button>
                    <button
                        class="button primary"
                        type="button"
                        on:click=move |_| on_confirm.run(())
                    >
                        {move || locale.get().t(Message::ScanFolder)}
                    </button>
                </div>
            </section>
        </div>
    }
}

#[component]
fn FolderIcon() -> impl IntoView {
    view! {
        <svg
            class="folder-icon"
            viewBox="0 0 24 24"
            fill="none"
            stroke="currentColor"
            stroke-width="1.8"
            stroke-linecap="round"
            stroke-linejoin="round"
            aria-hidden="true"
        >
            <path d="M3.75 6.25h5.1l1.65 2h9.75c.97 0 1.75.78 1.75 1.75v7.75c0 .97-.78 1.75-1.75 1.75H3.75A1.75 1.75 0 0 1 2 17.75V8c0-.97.78-1.75 1.75-1.75Z" />
            <path d="M2.5 10h19" />
        </svg>
    }
}

#[component]
fn ColumnHeading(
    locale: RwSignal<Locale>,
    label: Message,
    index: usize,
    column_widths: RwSignal<[i32; 5]>,
    resizing: RwSignal<Option<ColumnResize>>,
    #[prop(optional, into)] class: String,
) -> impl IntoView {
    view! {
        <span class=format!("column-heading {class}")>
            {move || locale.get().t(label)}
            <button
                class="column-resizer"
                type="button"
                title=move || locale.get().t(Message::ResizeColumn)
                aria-label=move || locale.get().t(Message::ResizeColumn)
                on:pointerdown=move |event| {
                    event.prevent_default();
                    resizing.set(Some(ColumnResize {
                        index,
                        start_x: event.client_x(),
                        start_width: column_widths.get_untracked()[index],
                    }));
                }
            ></button>
        </span>
    }
}

#[component]
fn QueueRow(
    locale: RwSignal<Locale>,
    item: QueueItem,
    active_batch: RwSignal<Option<String>>,
    on_remove: Callback<String>,
    on_open_output: Callback<String>,
) -> impl IntoView {
    let status_class = format!("status {}", status_class(&item.status));
    let input_name = file_name(&item.input_path);
    let size_change = format_size_change(item.input_bytes, item.output_bytes);
    let status = item.status.clone();
    let item_id = item.id.clone();
    let output_path = item.output_path.clone();
    let can_open_output = item.status == ItemStatus::Succeeded;
    let input_title = display_path(&item.input_path);
    let output_title = display_path(&item.output_path);
    view! {
        <article class="queue-row">
            <div class="direction-cell">
                <div class="file-badge">{conversion_badge(item.conversion)}</div>
            </div>
            <div class="font-name-cell">
                <strong title=input_title.clone()>{input_name}</strong>
            </div>
            <div class="path-cell">
                <span title=input_title>{display_path(&item.input_path)}</span>
                <small title=output_title>
                    <span aria-hidden="true">"→ "</span>
                    {display_path(&item.output_path)}
                </small>
                {item.message.map(|message| view! {
                    <em>{move || localized_message(locale.get(), &message)}</em>
                })}
            </div>
            <div class="size-change">
                <span>{format_bytes(item.input_bytes)} " → " {format_bytes(item.output_bytes)}</span>
                <strong>{size_change}</strong>
            </div>
            <span class=status_class aria-live="polite">
                {move || status_label(locale.get(), &status)}
            </span>
            <div class="row-actions">
                <button
                    class="open-output"
                    type="button"
                    disabled=!can_open_output
                    title=move || locale.get().t(Message::OpenOutputFolder)
                    aria-label=move || locale.get().t(Message::OpenOutputFolder)
                    on:click=move |_| on_open_output.run(output_path.clone())
                >
                    <FolderIcon />
                </button>
                <button
                    class="remove-item"
                    type="button"
                    disabled=move || active_batch.get().is_some()
                    title=move || locale.get().t(Message::Remove)
                    aria-label=move || locale.get().t(Message::Remove)
                    on:click=move |_| on_remove.run(item_id.clone())
                >
                    <span aria-hidden="true">"×"</span>
                </button>
            </div>
        </article>
    }
}

#[component]
fn BatchSummaryView(locale: RwSignal<Locale>, summary: RwSignal<BatchSummary>) -> impl IntoView {
    view! {
        <div class="summary" aria-live="polite">
            <Show when=move || { summary.get().queued > 0 }>
                <span class="summary-item queued">
                    {move || locale.get().t(Message::Queued)}
                    " "
                    {move || summary.get().queued}
                </span>
            </Show>
            <Show when=move || { summary.get().running > 0 }>
                <span class="summary-item running">
                    {move || locale.get().t(Message::Running)}
                    " "
                    {move || summary.get().running}
                </span>
            </Show>
            <Show when=move || { summary.get().succeeded > 0 }>
                <span class="summary-item succeeded">
                    {move || locale.get().t(Message::Succeeded)}
                    " "
                    {move || summary.get().succeeded}
                </span>
            </Show>
            <Show when=move || { summary.get().skipped > 0 }>
                <span class="summary-item skipped">
                    {move || locale.get().t(Message::Skipped)}
                    " "
                    {move || summary.get().skipped}
                </span>
            </Show>
            <Show when=move || { summary.get().failed > 0 }>
                <span class="summary-item failed">
                    {move || locale.get().t(Message::Failed)}
                    " "
                    {move || summary.get().failed}
                </span>
            </Show>
            <Show when=move || { summary.get().cancelled > 0 }>
                <span class="summary-item cancelled">
                    {move || locale.get().t(Message::Cancelled)}
                    " "
                    {move || summary.get().cancelled}
                </span>
            </Show>
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
        let mut known_outputs = current
            .iter()
            .map(|item| item.output_path.to_lowercase())
            .collect::<HashSet<_>>();
        for mut item in result.items {
            if !known.insert(item.input_path.clone()) {
                continue;
            }
            if !known_outputs.insert(item.output_path.to_lowercase()) {
                item.status = ItemStatus::Skipped;
                item.message = Some("Output path conflicts with another queued font".into());
            }
            current.push(item);
        }
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

fn replace_scan_result(
    items: RwSignal<Vec<QueueItem>>,
    warnings: RwSignal<Vec<ScanWarning>>,
    summary: RwSignal<BatchSummary>,
    result: ScanResult,
) {
    warnings.set(result.warnings);
    items.set(result.items);
    summary.set(BatchSummary::from_items(&items.get_untracked()));
}

fn filter_scan_result(result: ScanResult, mode: FolderConversionMode) -> ScanResult {
    ScanResult {
        items: result
            .items
            .into_iter()
            .filter(|item| mode.accepts(item.conversion))
            .collect(),
        warnings: result.warnings,
    }
}

fn completed_count(summary: &BatchSummary) -> usize {
    summary.succeeded + summary.skipped + summary.failed + summary.cancelled
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
        ConversionKind::TtfToWoff2 => "TTF→WOFF2",
        ConversionKind::OtfToWoff2 => "OTF→WOFF2",
        ConversionKind::Woff2ToTtf => "WOFF2→TTF",
        ConversionKind::Woff2ToOtf => "WOFF2→OTF",
    }
}

fn resized_column_width(start_width: i32, start_x: i32, current_x: i32, minimum: i32) -> i32 {
    (start_width + current_x - start_x).max(minimum)
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
    path.rsplit(['\\', '/'])
        .find(|part| !part.is_empty())
        .unwrap_or(path)
        .to_owned()
}

fn display_path(path: &str) -> String {
    if let Some(path) = path.strip_prefix(r"\\?\UNC\") {
        format!(r"\\{path}")
    } else {
        path.strip_prefix(r"\\?\").unwrap_or(path).to_owned()
    }
}

fn localized_message(locale: Locale, message: &str) -> String {
    if locale == Locale::En {
        return message.into();
    }

    match message {
        "Output file already exists" => "输出文件已存在".into(),
        "Output path conflicts with another queued font" => "输出路径与队列中的另一字体冲突".into(),
        "Cancelled before conversion started" => "转换开始前已取消".into(),
        "Invalid or changed input font" => "输入字体无效或已发生变化".into(),
        "Invalid or changed conversion path" => "转换路径无效或已发生变化".into(),
        "Input does not contain a valid WOFF2 header" => "输入文件不包含有效的 WOFF2 文件头".into(),
        "The WOFF2 output size is invalid or exceeds the 128 MB safety limit" => {
            "WOFF2 解压后大小无效或超过 128 MB 安全限制".into()
        }
        "Google WOFF2 rejected the file or failed to decompress it" => {
            "Google WOFF2 拒绝该文件或解压失败".into()
        }
        "Input font is empty" => "输入字体为空".into(),
        "Google WOFF2 could not determine a safe output size" => {
            "Google WOFF2 无法确定安全的输出大小".into()
        }
        "Google WOFF2 rejected the font or failed to encode it" => {
            "Google WOFF2 拒绝该字体或编码失败".into()
        }
        _ => message.into(),
    }
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

fn format_size_change(input: Option<u64>, output: Option<u64>) -> String {
    let (Some(input), Some(output)) = (input, output) else {
        return "—".into();
    };
    if input == 0 {
        return "—".into();
    }

    let change = (output as f64 - input as f64) / input as f64 * 100.0;
    if change.abs() < 0.05 {
        "0.0%".into()
    } else {
        format!("{change:+.1}%")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn size_change_handles_compression_and_expansion() {
        assert_eq!(format_size_change(Some(100), Some(60)), "-40.0%");
        assert_eq!(format_size_change(Some(100), Some(220)), "+120.0%");
        assert_eq!(format_size_change(Some(100), None), "—");
        assert_eq!(format_size_change(Some(0), Some(10)), "—");
    }

    #[test]
    fn column_resize_tracks_pointer_delta_and_respects_minimum() {
        assert_eq!(resized_column_width(120, 300, 340, 90), 160);
        assert_eq!(resized_column_width(120, 300, 100, 90), 90);
    }

    #[test]
    fn completed_count_excludes_queued_and_running() {
        let summary = BatchSummary {
            total: 8,
            queued: 1,
            running: 1,
            succeeded: 2,
            skipped: 1,
            failed: 2,
            cancelled: 1,
        };
        assert_eq!(completed_count(&summary), 6);
    }

    #[test]
    fn extracts_windows_file_names_in_wasm_compatible_way() {
        assert_eq!(
            file_name(r"\\?\C:\Fonts\Example Font.ttf"),
            "Example Font.ttf"
        );
        assert_eq!(file_name("/fonts/example.woff2"), "example.woff2");
        assert_eq!(display_path(r"\\?\C:\Fonts\font.ttf"), r"C:\Fonts\font.ttf");
        assert_eq!(
            display_path(r"\\?\UNC\server\fonts\font.otf"),
            r"\\server\fonts\font.otf"
        );
    }

    #[test]
    fn localizes_known_queue_messages() {
        assert_eq!(
            localized_message(Locale::ZhCn, "Output file already exists"),
            "输出文件已存在"
        );
        assert_eq!(
            localized_message(Locale::En, "Output file already exists"),
            "Output file already exists"
        );
    }
}
