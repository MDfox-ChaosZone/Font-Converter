#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
mod icons;
mod queue;
mod settings;
mod theme;
mod ui;
mod view;
use font_converter_engine::{Batch, Event, Existing, Item, Mode, Plan, Status, Warning};
use gpui::{prelude::*, *};
use settings::{Settings, tr};
use std::{
    collections::HashSet,
    path::PathBuf,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
        mpsc::{self, Receiver},
    },
    time::Duration,
};

actions!(
    font_converter,
    [
        AddFiles,
        AddFolder,
        Begin,
        CancelBatch,
        ClearQueue,
        SelectPrevious,
        SelectNext,
        SelectFirst,
        SelectLast,
        RevealSelected,
        RemoveSelected,
        FocusNext,
        FocusPrevious,
    ]
);

/// How often worker results are drained into the UI while a scan or batch runs.
/// Nothing polls while the window is idle.
const POLL: Duration = Duration::from_millis(33);

struct ScrollDrag {
    mouse_y: Pixels,
    offset: Pixels,
}

struct Workbench {
    settings: Settings,
    options: font_converter_engine::Options,
    /// Files and folders the user added; every scan starts from these.
    paths: Vec<PathBuf>,
    /// Inputs removed from the queue individually; filtered out of later scans.
    excluded: HashSet<PathBuf>,
    items: Vec<Item>,
    warnings: Vec<Warning>,
    warnings_open: bool,
    menu_open: bool,
    scan: Option<Receiver<Plan>>,
    /// Set when the inputs change while a scan is still running.
    rescan_pending: bool,
    batch: Option<Batch>,
    polling: bool,
    notice: Option<String>,
    selected: Option<usize>,
    exit_after_batch: bool,
    prompting: bool,
    focus: FocusHandle,
    list: UniformListScrollHandle,
    scroll_drag: Option<ScrollDrag>,
    #[cfg(test)]
    rendered_rows: std::rc::Rc<std::cell::Cell<usize>>,
    _appearance: Subscription,
}

impl Workbench {
    fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let (settings, notice) = Settings::load();
        let focus = cx.focus_handle();
        window.focus(&focus, cx);
        theme::apply(theme::is_dark(&settings.theme, window.appearance()), cx);
        let appearance = cx.observe_window_appearance(window, |this, window, cx| {
            if this.settings.theme == "system" {
                theme::apply(theme::is_dark("system", window.appearance()), cx);
                cx.notify();
            }
        });
        let weak = cx.entity().downgrade();
        window.on_window_should_close(cx, move |window, cx| {
            weak.update(cx, |this, cx| this.should_close(window, cx))
                .unwrap_or(true)
        });
        Self {
            options: font_converter_engine::Options {
                jobs: settings.jobs,
                mode: settings.mode,
                existing: settings.existing,
                ..Default::default()
            },
            settings,
            paths: vec![],
            excluded: HashSet::new(),
            items: vec![],
            warnings: vec![],
            warnings_open: false,
            menu_open: false,
            scan: None,
            rescan_pending: false,
            batch: None,
            polling: false,
            notice,
            selected: None,
            exit_after_batch: false,
            prompting: false,
            focus,
            list: UniformListScrollHandle::new(),
            scroll_drag: None,
            #[cfg(test)]
            rendered_rows: Default::default(),
            _appearance: appearance,
        }
    }

    fn busy(&self) -> bool {
        self.scan.is_some() || self.batch.is_some()
    }
    fn zh(&self) -> bool {
        self.settings.zh()
    }

    fn should_close(&mut self, window: &mut Window, cx: &mut Context<Self>) -> bool {
        if self.batch.is_none() {
            cx.quit();
            return true;
        }
        if !self.prompting {
            self.prompting = true;
            let zh = self.zh();
            let answer = window.prompt(
                PromptLevel::Warning,
                tr(zh, "Conversion is running", "转换正在进行"),
                Some(tr(
                    zh,
                    "Files already being converted finish safely before exit.",
                    "正在处理的文件会完成后再退出。",
                )),
                &[
                    tr(zh, "Keep converting", "继续转换"),
                    tr(zh, "Cancel and exit", "取消后退出"),
                ],
                cx,
            );
            cx.spawn(async move |weak, cx| {
                if let Ok(choice) = answer.await {
                    let _ = weak.update(cx, |this, cx| {
                        this.prompting = false;
                        if choice == 1 {
                            this.exit_after_batch = true;
                            this.cancel();
                            if this.batch.is_none() {
                                cx.quit();
                            }
                        }
                    });
                }
            })
            .detach();
        }
        false
    }

    fn persist(&mut self) {
        if let Err(error) = self.settings.save() {
            self.notice = Some(error);
        }
    }

    /// Drains worker channels on a timer, only while a scan or batch is active.
    fn ensure_polling(&mut self, cx: &mut Context<Self>) {
        if self.polling {
            return;
        }
        self.polling = true;
        cx.spawn(async move |weak, cx| {
            loop {
                cx.background_executor().timer(POLL).await;
                let keep = weak.update(cx, |this, cx| {
                    this.tick(cx);
                    this.polling = this.busy();
                    this.polling
                });
                if !matches!(keep, Ok(true)) {
                    break;
                }
            }
        })
        .detach();
    }

    fn rescan(&mut self, cx: &mut Context<Self>) {
        if self.batch.is_some() {
            return;
        }
        if self.scan.is_some() {
            self.rescan_pending = true;
            return;
        }
        self.selected = None;
        if self.paths.is_empty() {
            self.items.clear();
            self.warnings.clear();
            cx.notify();
            return;
        }
        let paths = self.paths.clone();
        let options = self.options.clone();
        let (tx, rx) = mpsc::channel();
        self.scan = Some(rx);
        std::thread::spawn(move || {
            let _ = tx.send(font_converter_engine::scan(&paths, &options));
        });
        self.ensure_polling(cx);
        cx.notify();
    }

    fn add(&mut self, paths: Vec<PathBuf>, cx: &mut Context<Self>) {
        if self.batch.is_some() {
            return;
        }
        for path in paths {
            self.excluded.remove(&path);
            if !self.paths.contains(&path) {
                self.paths.push(path);
            }
        }
        self.rescan(cx);
    }

    fn clear(&mut self, cx: &mut Context<Self>) {
        if self.batch.is_some() {
            return;
        }
        self.paths.clear();
        self.excluded.clear();
        self.warnings_open = false;
        self.rescan(cx);
    }

    fn remove(&mut self, index: usize, cx: &mut Context<Self>) {
        if self.busy() || index >= self.items.len() {
            return;
        }
        let item = self.items.remove(index);
        self.excluded.insert(item.input);
        renumber(&mut self.items);
        self.selected = match self.selected {
            Some(s) if s > index => Some(s - 1),
            Some(s) if s == index => None,
            other => other,
        };
        cx.notify();
    }

    fn choose(&mut self, files: bool, output: bool, cx: &mut Context<Self>) {
        if self.batch.is_some() {
            return;
        }
        let selection = cx.prompt_for_paths(PathPromptOptions {
            files,
            directories: !files,
            multiple: !output,
            prompt: Some(tr(self.zh(), "Select", "选择").into()),
        });
        cx.spawn(async move |weak, cx| match selection.await {
            Ok(Ok(Some(paths))) => {
                let _ = weak.update(cx, |this, cx| {
                    if output {
                        this.set_output(paths.into_iter().next(), cx);
                    } else {
                        this.add(paths, cx);
                    }
                });
            }
            Ok(Err(error)) => {
                let _ = weak.update(cx, |this, cx| {
                    this.notice = Some(error.to_string());
                    cx.notify();
                });
            }
            _ => {}
        })
        .detach();
    }

    fn begin(&mut self, cx: &mut Context<Self>) {
        if self.busy() || !self.items.iter().any(|item| item.status == Status::Queued) {
            return;
        }
        match font_converter_engine::start(
            Plan {
                items: self.items.clone(),
                warnings: vec![],
            },
            self.options.clone(),
            Arc::new(AtomicBool::new(false)),
        ) {
            Ok(batch) => {
                self.batch = Some(batch);
                self.ensure_polling(cx);
            }
            Err(error) => self.notice = Some(error.to_string()),
        }
        cx.notify();
    }

    fn cancel(&self) {
        if let Some(batch) = &self.batch {
            batch.cancel.store(true, Ordering::Release);
        }
    }

    fn cancelling(&self) -> bool {
        self.batch
            .as_ref()
            .is_some_and(|batch| batch.cancel.load(Ordering::Acquire))
    }

    fn tick(&mut self, cx: &mut Context<Self>) {
        let mut changed = false;
        if let Some(receiver) = &self.scan {
            match receiver.try_recv() {
                Ok(mut plan) => {
                    plan.items
                        .retain(|item| !self.excluded.contains(&item.input));
                    renumber(&mut plan.items);
                    self.items = plan.items;
                    self.warnings = plan.warnings;
                    self.scan = None;
                    changed = true;
                }
                Err(mpsc::TryRecvError::Disconnected) => {
                    self.scan = None;
                    self.notice = Some(
                        tr(
                            self.zh(),
                            "Scan worker stopped unexpectedly",
                            "扫描任务异常终止",
                        )
                        .into(),
                    );
                    changed = true;
                }
                Err(mpsc::TryRecvError::Empty) => {}
            }
            if self.scan.is_none() && std::mem::take(&mut self.rescan_pending) {
                self.rescan(cx);
            }
        }
        let mut finished = false;
        if let Some(batch) = &self.batch {
            // Bounded per tick so a burst of tiny files never stalls a frame.
            for _ in 0..512 {
                match batch.events.try_recv() {
                    Ok(Event::Item { batch: id, item }) if id == batch.id => {
                        if let Some(row) = self.items.get_mut(item.id) {
                            *row = item;
                        }
                        changed = true;
                    }
                    Ok(Event::Finished { batch: id, items }) if id == batch.id => {
                        self.items = items;
                        finished = true;
                        changed = true;
                        break;
                    }
                    Ok(_) => {}
                    Err(mpsc::TryRecvError::Empty) => break,
                    Err(mpsc::TryRecvError::Disconnected) => {
                        for item in &mut self.items {
                            if matches!(item.status, Status::Queued | Status::Running) {
                                item.fail(font_converter_engine::Error::new(
                                    font_converter_engine::ErrorCode::ConversionFailed,
                                    "Conversion worker stopped unexpectedly",
                                ));
                            }
                        }
                        self.notice = Some(
                            tr(
                                self.zh(),
                                "Conversion worker stopped unexpectedly",
                                "转换任务异常终止",
                            )
                            .into(),
                        );
                        finished = true;
                        changed = true;
                        break;
                    }
                }
            }
        }
        if finished {
            self.batch = None;
            if self.exit_after_batch {
                cx.quit();
            }
        }
        if changed {
            cx.notify();
        }
    }

    fn select(&mut self, index: usize, cx: &mut Context<Self>) {
        if self.items.is_empty() {
            return;
        }
        let index = index.min(self.items.len() - 1);
        self.selected = Some(index);
        self.list.scroll_to_item(index, ScrollStrategy::Nearest);
        cx.notify();
    }
    fn step_selection(&mut self, delta: isize, cx: &mut Context<Self>) {
        let next = match self.selected {
            Some(current) => current.saturating_add_signed(delta),
            None if delta < 0 => usize::MAX,
            None => 0,
        };
        self.select(next, cx);
    }
    fn reveal_selected(&self, cx: &mut App) {
        if let Some(item) = self.selected.and_then(|ix| self.items.get(ix)) {
            let target = if item.output.exists() {
                &item.output
            } else {
                &item.input
            };
            cx.reveal_path(target);
        }
    }

    fn set_mode(&mut self, mode: Mode, cx: &mut Context<Self>) {
        if self.batch.is_some() || self.options.mode == mode {
            return;
        }
        self.options.mode = mode;
        self.settings.mode = mode;
        self.persist();
        self.rescan(cx);
    }
    fn set_existing(&mut self, existing: Existing, cx: &mut Context<Self>) {
        if self.batch.is_some() || self.options.existing == existing {
            return;
        }
        self.options.existing = existing;
        self.settings.existing = existing;
        self.persist();
        self.rescan(cx);
    }
    fn set_jobs(&mut self, jobs: usize, cx: &mut Context<Self>) {
        if self.batch.is_some() || !(1..=32).contains(&jobs) {
            return;
        }
        self.options.jobs = jobs;
        self.settings.jobs = jobs;
        self.persist();
        cx.notify();
    }
    fn set_output(&mut self, dir: Option<PathBuf>, cx: &mut Context<Self>) {
        if self.batch.is_some() {
            return;
        }
        self.options.output_dir = dir;
        self.rescan(cx);
    }
    fn set_theme(&mut self, value: &'static str, window: &mut Window, cx: &mut Context<Self>) {
        self.settings.theme = value.into();
        theme::apply(theme::is_dark(value, window.appearance()), cx);
        self.persist();
        cx.notify();
    }
    fn set_language(&mut self, value: &'static str, cx: &mut Context<Self>) {
        self.settings.language = value.into();
        self.persist();
        cx.notify();
    }
}

/// Batch events address rows by `Item::id`, so ids must stay equal to indices.
fn renumber(items: &mut [Item]) {
    for (index, item) in items.iter_mut().enumerate() {
        item.id = index;
    }
}

impl Focusable for Workbench {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus.clone()
    }
}

fn bind_keys(cx: &mut App) {
    cx.bind_keys([
        KeyBinding::new("ctrl-o", AddFiles, Some("Workbench")),
        KeyBinding::new("ctrl-shift-o", AddFolder, Some("Workbench")),
        KeyBinding::new("ctrl-enter", Begin, Some("Workbench")),
        KeyBinding::new("escape", CancelBatch, Some("Workbench")),
        KeyBinding::new("up", SelectPrevious, Some("Workbench")),
        KeyBinding::new("down", SelectNext, Some("Workbench")),
        KeyBinding::new("home", SelectFirst, Some("Workbench")),
        KeyBinding::new("end", SelectLast, Some("Workbench")),
        KeyBinding::new("ctrl-r", RevealSelected, Some("Workbench")),
        KeyBinding::new("delete", RemoveSelected, Some("Workbench")),
        KeyBinding::new("tab", FocusNext, Some("Workbench")),
        KeyBinding::new("shift-tab", FocusPrevious, Some("Workbench")),
    ]);
}

fn main() {
    gpui_platform::application()
        .with_assets(icons::Assets)
        .run(|cx| {
            cx.set_app_identity("io.github.mdfox-chaoszone.fontconverter", "Font Converter");
            bind_keys(cx);
            let bounds = Bounds::centered(None, size(px(920.), px(620.)), cx);
            let window = cx
                .open_window(
                    WindowOptions {
                        window_bounds: Some(WindowBounds::Windowed(bounds)),
                        window_min_size: Some(size(px(640.), px(420.))),
                        titlebar: Some(TitlebarOptions {
                            title: Some("Font Converter".into()),
                            ..Default::default()
                        }),
                        ..Default::default()
                    },
                    |window, cx| cx.new(|cx| Workbench::new(window, cx)),
                )
                .expect("Open Font Converter window");
            // Fonts passed on the command line ("Open with", or dropped onto the
            // executable) start in the queue.
            let paths: Vec<PathBuf> = std::env::args_os().skip(1).map(PathBuf::from).collect();
            if !paths.is_empty() {
                let _ = window.update(cx, |view, _, cx| view.add(paths, cx));
            }
            cx.activate(true);
        });
}

#[cfg(test)]
mod tests {
    use super::*;
    use core::prelude::v1::test;
    fn row(id: usize) -> Item {
        Item {
            id,
            input: PathBuf::from(format!("font{id}.ttf")),
            output: PathBuf::from(format!("font{id}.woff2")),
            input_format: font_converter_engine::Format::Ttf,
            output_format: font_converter_engine::Format::Woff2,
            status: Status::Queued,
            input_bytes: 100,
            output_bytes: None,
            error_code: None,
            message: None,
        }
    }
    fn batch(id: u64, rx: Receiver<Event>) -> Batch {
        Batch {
            id,
            cancel: Arc::new(AtomicBool::new(false)),
            events: rx,
        }
    }
    #[gpui::test]
    fn stale_batch_events_are_ignored_and_completion_unlocks(cx: &mut TestAppContext) {
        let (view, cx) = cx.add_window_view(Workbench::new);
        let (tx, rx) = mpsc::channel();
        view.update(cx, |this, _| {
            this.items = vec![row(0)];
            this.batch = Some(batch(42, rx));
            assert!(this.busy());
        });
        let mut completed = row(0);
        completed.status = Status::Succeeded;
        tx.send(Event::Finished {
            batch: 41,
            items: vec![completed.clone()],
        })
        .unwrap();
        view.update(cx, |this, cx| {
            this.tick(cx);
            assert!(this.busy());
            assert_eq!(this.items[0].status, Status::Queued);
        });
        tx.send(Event::Item {
            batch: 42,
            item: completed.clone(),
        })
        .unwrap();
        tx.send(Event::Finished {
            batch: 42,
            items: vec![completed],
        })
        .unwrap();
        view.update(cx, |this, cx| {
            this.tick(cx);
            assert!(!this.busy());
            assert_eq!(this.items[0].status, Status::Succeeded);
        });
    }
    #[gpui::test]
    fn busy_controls_reject_changes_and_cancel_sets_token(cx: &mut TestAppContext) {
        let (view, cx) = cx.add_window_view(Workbench::new);
        let (_tx, rx) = mpsc::channel();
        let token = Arc::new(AtomicBool::new(false));
        view.update(cx, |this, _| {
            this.items = vec![row(0)];
            this.batch = Some(Batch {
                id: 1,
                cancel: token.clone(),
                events: rx,
            });
        });
        view.update(cx, |this, cx| {
            let mode = this.options.mode;
            this.set_mode(
                if mode == Mode::Encode {
                    Mode::Decode
                } else {
                    Mode::Encode
                },
                cx,
            );
            assert_eq!(this.options.mode, mode);
            this.add(vec![PathBuf::from("ignored")], cx);
            assert!(this.paths.is_empty());
            this.remove(0, cx);
            assert_eq!(this.items.len(), 1);
            this.cancel();
            assert!(token.load(Ordering::Acquire));
            assert!(this.cancelling());
        });
    }
    #[gpui::test]
    fn failed_worker_unlocks_and_marks_pending_tasks(cx: &mut TestAppContext) {
        let (view, cx) = cx.add_window_view(Workbench::new);
        let (tx, rx) = mpsc::channel();
        drop(tx);
        view.update(cx, |this, cx| {
            this.items = vec![row(0)];
            this.batch = Some(batch(1, rx));
            this.tick(cx);
            assert!(!this.busy());
            assert_eq!(this.items[0].status, Status::Failed);
        });
    }
    #[gpui::test]
    fn removing_rows_keeps_ids_aligned_with_indices(cx: &mut TestAppContext) {
        let (view, cx) = cx.add_window_view(Workbench::new);
        view.update(cx, |this, cx| {
            this.items = (0..4).map(row).collect();
            this.selected = Some(3);
            this.remove(1, cx);
            assert_eq!(this.items.len(), 3);
            assert!(
                this.items
                    .iter()
                    .enumerate()
                    .all(|(ix, item)| item.id == ix)
            );
            assert_eq!(this.selected, Some(2));
            assert!(this.excluded.contains(&PathBuf::from("font1.ttf")));
        });
    }
    #[gpui::test]
    fn ten_thousand_rows_render_lazily_and_keyboard_moves_selection(cx: &mut TestAppContext) {
        cx.update(bind_keys);
        let (view, cx) = cx.add_window_view(Workbench::new);
        view.update(cx, |this, cx| {
            this.items = (0..10000).map(row).collect();
            cx.notify();
        });
        cx.run_until_parked();
        cx.simulate_keystrokes("down down");
        view.update(cx, |this, _| {
            let rendered = this.rendered_rows.get();
            assert!(
                rendered > 0 && rendered < 10000,
                "Expected virtualized rendering, got {rendered} rows"
            );
            assert_eq!(this.selected, Some(1));
        });
        cx.simulate_keystrokes("end");
        view.update(cx, |this, _| assert_eq!(this.selected, Some(9999)));
    }
    #[gpui::test]
    fn platform_file_drop_reaches_workbench_without_writing_outputs(cx: &mut TestAppContext) {
        let (view, cx) = cx.add_window_view(Workbench::new);
        let path =
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../tests/fixtures/Abel-Regular.ttf");
        cx.update(|window, cx| {
            window.draw(cx).clear(cx);
            let position = point(px(600.), px(300.));
            window.dispatch_event(
                FileDropEvent::Entered {
                    position,
                    paths: ExternalPaths([path.clone()].into_iter().collect()),
                }
                .to_platform_input(),
                cx,
            );
            window.draw(cx).clear(cx);
            window.dispatch_event(FileDropEvent::Submit { position }.to_platform_input(), cx);
        });
        view.update(cx, |this, _| {
            assert_eq!(this.paths, vec![path]);
            assert!(this.scan.is_some());
            assert!(this.batch.is_none());
        });
    }
}
