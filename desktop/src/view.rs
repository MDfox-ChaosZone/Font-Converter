use crate::{
    icons::Icon,
    queue,
    theme::{ActivePalette, Palette},
    ui::{self, Button},
    *,
};
use font_converter_engine::Summary;

/// Scrollbar thumb placement for the queue list, from its last layout.
struct ScrollGeometry {
    handle: ScrollHandle,
    max: Pixels,
    thumb: Pixels,
    track: Pixels,
    scrolled: Pixels,
}

impl Workbench {
    fn toolbar(&self, total: usize, cx: &mut Context<Self>) -> impl IntoElement {
        let zh = self.zh();
        let locked = self.batch.is_some();
        let weak = cx.entity().downgrade();
        div()
            .px_5()
            .h(px(60.))
            .flex_none()
            .flex()
            .items_center()
            .gap_2()
            .child(
                div()
                    .w(px(320.))
                    .flex_shrink(1.)
                    .min_w(px(220.))
                    .child(ui::segmented(
                        "mode",
                        tr(zh, "Conversion", "转换方向"),
                        vec![
                            (Mode::Auto, tr(zh, "Auto", "自动").into()),
                            (Mode::Encode, "→ WOFF2".into()),
                            (Mode::Decode, "→ TTF / OTF".into()),
                        ],
                        self.options.mode,
                        locked,
                        move |mode, _, cx| {
                            let _ = weak.update(cx, |this, cx| this.set_mode(mode, cx));
                        },
                        cx,
                    )),
            )
            .child(div().flex_1())
            .when(total > 0 || !self.paths.is_empty(), |el| {
                el.child(
                    Button::new("clear")
                        .ghost()
                        .icon(Icon::Trash)
                        .aria(tr(zh, "Clear queue", "清空队列"))
                        .tooltip(tr(zh, "Clear queue", "清空队列"))
                        .disabled(locked)
                        .on_click(cx.listener(|this, _, _, cx| this.clear(cx))),
                )
            })
            .child(
                Button::new("add-folder")
                    .ghost()
                    .icon(Icon::Folder)
                    .label(tr(zh, "Add folder", "添加文件夹"))
                    .tooltip("Ctrl+Shift+O")
                    .disabled(locked)
                    .on_click(cx.listener(|this, _, _, cx| this.choose(false, false, cx))),
            )
            .child(
                Button::new("add-files")
                    .icon(Icon::Plus)
                    .label(tr(zh, "Add fonts", "添加字体"))
                    .tooltip("Ctrl+O")
                    .disabled(locked)
                    .on_click(cx.listener(|this, _, _, cx| this.choose(true, false, cx))),
            )
            .child(
                div()
                    .relative()
                    .child(
                        Button::new("more")
                            .ghost()
                            .icon(Icon::More)
                            .aria(tr(zh, "More options", "更多选项"))
                            .when_some(
                                (!self.menu_open).then(|| tr(zh, "More options", "更多选项")),
                                |button, tip| button.tooltip(tip),
                            )
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.menu_open = !this.menu_open;
                                cx.notify();
                            })),
                    )
                    .when(self.menu_open, |el| el.child(self.menu(cx))),
            )
    }

    /// Settings that are rarely changed, in a small popover under the ⋯ button.
    fn menu(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let p = cx.palette();
        let zh = self.zh();
        let locked = self.batch.is_some();
        let jobs = self.options.jobs;
        let weak = cx.entity().downgrade();
        let row = |label: &'static str, control: AnyElement| {
            div()
                .flex()
                .flex_col()
                .gap(px(6.))
                .child(div().text_xs().text_color(p.muted).child(label))
                .child(control)
        };
        let set_existing = {
            let weak = weak.clone();
            move |existing, _: &mut Window, cx: &mut App| {
                let _ = weak.update(cx, |this, cx| this.set_existing(existing, cx));
            }
        };
        let set_theme = {
            let weak = weak.clone();
            move |theme, window: &mut Window, cx: &mut App| {
                let _ = weak.update(cx, |this, cx| this.set_theme(theme, window, cx));
            }
        };
        let set_language = move |language, _: &mut Window, cx: &mut App| {
            let _ = weak.update(cx, |this, cx| this.set_language(language, cx));
        };
        let theme_value: &'static str = match self.settings.theme.as_str() {
            "light" => "light",
            "dark" => "dark",
            _ => "system",
        };
        deferred(
            div()
                .id("menu")
                .role(Role::Dialog)
                .aria_label(tr(zh, "More options", "更多选项"))
                .occlude()
                .absolute()
                .top(px(40.))
                .right_0()
                .w(px(280.))
                .p_4()
                .flex()
                .flex_col()
                .gap_4()
                .rounded(px(12.))
                .border_1()
                .border_color(p.border)
                .bg(p.surface)
                .shadow_lg()
                .on_mouse_down_out(cx.listener(|this, _, _, cx| {
                    this.menu_open = false;
                    cx.notify();
                }))
                .child(row(
                    tr(zh, "If output exists", "输出文件已存在时"),
                    ui::segmented(
                        "existing",
                        tr(zh, "If output exists", "输出文件已存在时"),
                        vec![
                            (Existing::Skip, tr(zh, "Skip", "跳过").into()),
                            (Existing::Overwrite, tr(zh, "Overwrite", "覆盖").into()),
                            (Existing::Error, tr(zh, "Error", "报错").into()),
                        ],
                        self.options.existing,
                        locked,
                        set_existing,
                        cx,
                    )
                    .into_any_element(),
                ))
                .child(
                    div()
                        .id("jobs")
                        .role(Role::SpinButton)
                        .aria_label(tr(zh, "Parallel files", "并行文件数"))
                        .aria_numeric_value(jobs as f64)
                        .aria_min_numeric_value(1.)
                        .aria_max_numeric_value(32.)
                        .flex()
                        .items_center()
                        .gap_2()
                        .child(div().flex_1().text_xs().text_color(p.muted).child(tr(
                            zh,
                            "Parallel files",
                            "并行文件数",
                        )))
                        .child(
                            Button::new("jobs-down")
                                .ghost()
                                .small()
                                .icon(Icon::Minus)
                                .aria(tr(zh, "Fewer", "减少"))
                                .disabled(locked || jobs <= 1)
                                .on_click(
                                    cx.listener(move |this, _, _, cx| this.set_jobs(jobs - 1, cx)),
                                ),
                        )
                        .child(
                            div()
                                .w(px(24.))
                                .text_center()
                                .text_size(px(13.))
                                .font_weight(FontWeight::SEMIBOLD)
                                .child(jobs.to_string()),
                        )
                        .child(
                            Button::new("jobs-up")
                                .ghost()
                                .small()
                                .icon(Icon::Plus)
                                .aria(tr(zh, "More", "增加"))
                                .disabled(locked || jobs >= 32)
                                .on_click(
                                    cx.listener(move |this, _, _, cx| this.set_jobs(jobs + 1, cx)),
                                ),
                        ),
                )
                .child(div().h(px(1.)).bg(p.border))
                .child(row(
                    tr(zh, "Appearance", "外观"),
                    ui::segmented(
                        "theme",
                        tr(zh, "Appearance", "外观"),
                        vec![
                            ("system", tr(zh, "System", "跟随系统").into()),
                            ("light", tr(zh, "Light", "浅色").into()),
                            ("dark", tr(zh, "Dark", "深色").into()),
                        ],
                        theme_value,
                        false,
                        set_theme,
                        cx,
                    )
                    .into_any_element(),
                ))
                .child(row(
                    tr(zh, "Language", "语言"),
                    ui::segmented(
                        "language",
                        tr(zh, "Language", "语言"),
                        vec![("en", "English".into()), ("zh-CN", "简体中文".into())],
                        if zh { "zh-CN" } else { "en" },
                        false,
                        set_language,
                        cx,
                    )
                    .into_any_element(),
                )),
        )
        .with_priority(1)
    }

    fn banners(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let p = cx.palette();
        let zh = self.zh();
        let banner = |color: Hsla| {
            div()
                .mx_5()
                .mb_3()
                .px_3()
                .py_2()
                .rounded(px(8.))
                .bg(Palette::tint(color, 0.1))
                .flex()
                .flex_col()
                .gap_2()
                .text_size(px(13.))
        };
        div()
            .flex()
            .flex_col()
            .when_some(self.notice.clone(), |el, notice| {
                el.child(
                    banner(p.danger).child(
                        div()
                            .flex()
                            .items_center()
                            .gap_2()
                            .child(
                                Icon::Alert
                                    .svg()
                                    .size(px(14.))
                                    .flex_none()
                                    .text_color(p.danger),
                            )
                            .child(div().flex_1().min_w_0().child(notice))
                            .child(
                                Button::new("dismiss-notice")
                                    .ghost()
                                    .small()
                                    .icon(Icon::Close)
                                    .aria(tr(zh, "Dismiss", "关闭"))
                                    .on_click(cx.listener(|this, _, _, cx| {
                                        this.notice = None;
                                        cx.notify();
                                    })),
                            ),
                    ),
                )
            })
            .when(!self.warnings.is_empty(), |el| {
                let open = self.warnings_open;
                el.child(
                    banner(p.warning)
                        .child(
                            div()
                                .id("toggle-warnings")
                                .role(Role::Button)
                                .aria_expanded(open)
                                .flex()
                                .items_center()
                                .gap_2()
                                .cursor_pointer()
                                .on_click(cx.listener(|this, _, _, cx| {
                                    this.warnings_open = !this.warnings_open;
                                    cx.notify();
                                }))
                                .child(
                                    Icon::Alert
                                        .svg()
                                        .size(px(14.))
                                        .flex_none()
                                        .text_color(p.warning),
                                )
                                .child(div().flex_1().min_w_0().child(format!(
                                    "{} {}",
                                    self.warnings.len(),
                                    tr(zh, "items could not be added", "个项目未能加入")
                                )))
                                .child(div().text_xs().text_color(p.muted).child(if open {
                                    tr(zh, "Hide", "收起")
                                } else {
                                    tr(zh, "Show", "查看")
                                })),
                        )
                        .when(open, |el| {
                            el.child(
                                div()
                                    .id("warning-list")
                                    .max_h(px(120.))
                                    .overflow_y_scroll()
                                    .flex()
                                    .flex_col()
                                    .gap_1()
                                    .text_xs()
                                    .text_color(p.muted)
                                    .children(self.warnings.iter().map(|w| {
                                        format!("{} — {}", queue::path_text(&w.path), w.message)
                                    })),
                            )
                        }),
                )
            })
    }

    fn empty_state(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let p = cx.palette();
        let zh = self.zh();
        div()
            .id("empty")
            .size_full()
            .flex()
            .flex_col()
            .items_center()
            .justify_center()
            .gap_1()
            .cursor_pointer()
            .on_click(cx.listener(|this, _, _, cx| this.choose(true, false, cx)))
            .child(
                div()
                    .text_size(px(15.))
                    .font_weight(FontWeight::MEDIUM)
                    .child(tr(
                        zh,
                        "Drop fonts or folders here",
                        "把字体或文件夹拖到这里",
                    )),
            )
            .child(div().text_size(px(13.)).text_color(p.faint).child(tr(
                zh,
                "TTF, OTF and WOFF2 · or click to choose",
                "支持 TTF、OTF、WOFF2 · 也可点击选择",
            )))
    }

    fn scroll_geometry(&self) -> Option<ScrollGeometry> {
        let handle = self.list.0.borrow().base_handle.clone();
        let viewport = handle.bounds().size.height;
        let max = handle.max_offset().y;
        if max <= px(0.) || viewport <= px(0.) {
            return None;
        }
        let thumb = (viewport * (viewport / (viewport + max))).max(px(28.));
        let scrolled = (-handle.offset().y).clamp(px(0.), max);
        Some(ScrollGeometry {
            handle,
            max,
            thumb,
            track: viewport - thumb,
            scrolled,
        })
    }

    /// Thin overlay scrollbar for the virtualized list; draggable.
    fn scrollbar(&self, cx: &mut Context<Self>) -> Option<impl IntoElement> {
        let p = cx.palette();
        let ScrollGeometry {
            max,
            thumb,
            track,
            scrolled,
            ..
        } = self.scroll_geometry()?;
        let top = track * (scrolled / max);
        let dragging = self.scroll_drag.is_some();
        Some(
            div()
                .id("scroll-thumb")
                .absolute()
                .top(top)
                .right(px(3.))
                .w(px(6.))
                .h(thumb)
                .rounded(px(3.))
                .bg(Palette::tint(p.muted, if dragging { 0.55 } else { 0.3 }))
                .hover(move |s| s.bg(Palette::tint(p.muted, 0.55)))
                .on_mouse_down(
                    MouseButton::Left,
                    cx.listener(move |this, event: &MouseDownEvent, _, cx| {
                        cx.stop_propagation();
                        this.scroll_drag = Some(ScrollDrag {
                            mouse_y: event.position.y,
                            offset: scrolled,
                        });
                        cx.notify();
                    }),
                ),
        )
    }

    fn drag_scroll(&mut self, event: &MouseMoveEvent, cx: &mut Context<Self>) {
        let Some(drag) = &self.scroll_drag else {
            return;
        };
        if event.pressed_button != Some(MouseButton::Left) {
            self.scroll_drag = None;
            cx.notify();
            return;
        }
        let Some(ScrollGeometry {
            handle, max, track, ..
        }) = self.scroll_geometry()
        else {
            return;
        };
        if track <= px(0.) {
            return;
        }
        let target =
            (drag.offset + (event.position.y - drag.mouse_y) * (max / track)).clamp(px(0.), max);
        handle.set_offset(point(px(0.), -target));
        cx.notify();
    }

    fn footer(&self, summary: &Summary, cx: &mut Context<Self>) -> impl IntoElement {
        let p = cx.palette();
        let zh = self.zh();
        let active = self.batch.is_some();
        let done = summary.succeeded + summary.skipped + summary.failed + summary.cancelled;
        let progress = if summary.total == 0 {
            0.
        } else {
            done as f32 / summary.total as f32
        };
        let title: SharedString = if self.scan.is_some() {
            tr(zh, "Looking for fonts…", "正在查找字体…").into()
        } else if active {
            format!(
                "{} {done} / {}",
                tr(zh, "Converting", "正在转换"),
                summary.total
            )
            .into()
        } else if summary.total == 0 {
            tr(zh, "No fonts yet", "尚未添加字体").into()
        } else if summary.queued > 0 {
            format!(
                "{} {}",
                summary.queued,
                tr(zh, "fonts ready", "个字体待转换")
            )
            .into()
        } else {
            let mut parts = vec![format!(
                "{} {}",
                summary.succeeded,
                tr(zh, "converted", "个已转换")
            )];
            for (count, label) in [
                (summary.skipped, tr(zh, "skipped", "跳过")),
                (summary.failed, tr(zh, "failed", "失败")),
                (summary.cancelled, tr(zh, "cancelled", "取消")),
            ] {
                if count > 0 {
                    parts.push(format!("{count} {label}"));
                }
            }
            parts.join(" · ").into()
        };
        let output = self
            .options
            .output_dir
            .as_ref()
            .map(|dir| {
                dir.file_name()
                    .map(|name| name.to_string_lossy().into_owned())
                    .unwrap_or_else(|| queue::path_text(dir))
            })
            .unwrap_or_else(|| tr(zh, "next to source files", "源文件所在目录").into());
        let output_tip = self
            .options
            .output_dir
            .as_deref()
            .map(queue::path_text)
            .unwrap_or_else(|| tr(zh, "Choose an output folder", "选择输出文件夹").into());
        let cancelling = self.cancelling();
        div()
            .relative()
            .flex_none()
            .h(px(64.))
            .px_5()
            .flex()
            .items_center()
            .gap_4()
            .border_t_1()
            .border_color(p.border)
            .when(active || (done > 0 && summary.total > 0), |el| {
                el.child(
                    div()
                        .id("progress")
                        .role(Role::ProgressIndicator)
                        .aria_label(tr(zh, "Conversion progress", "转换进度"))
                        .aria_numeric_value((progress * 100.).round() as f64)
                        .absolute()
                        .top(px(-1.))
                        .left_0()
                        .h(px(2.))
                        .w(relative(progress))
                        .bg(if active { p.accent } else { p.success }),
                )
            })
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .flex()
                    .flex_col()
                    .gap(px(2.))
                    .child(
                        div()
                            .text_size(px(13.5))
                            .font_weight(FontWeight::MEDIUM)
                            .truncate()
                            .child(title),
                    )
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap_1()
                            .text_xs()
                            .text_color(p.muted)
                            .child(tr(zh, "Save to", "保存到"))
                            .child(
                                div()
                                    .id("output")
                                    .role(Role::Link)
                                    .aria_label(SharedString::from(format!(
                                        "{} {output}",
                                        tr(zh, "Save to", "保存到")
                                    )))
                                    .min_w_0()
                                    .truncate()
                                    .text_color(p.text)
                                    .tooltip(ui::tooltip(output_tip))
                                    .when(!active, |el| {
                                        el.focusable()
                                            .tab_stop(true)
                                            .cursor_pointer()
                                            .hover(move |s| s.text_color(p.accent).underline())
                                            .focus_visible(move |s| s.text_color(p.accent))
                                            .on_click(cx.listener(|this, _, _, cx| {
                                                this.choose(false, true, cx)
                                            }))
                                    })
                                    .child(output),
                            )
                            .when(self.options.output_dir.is_some() && !active, |el| {
                                el.child(
                                    div()
                                        .id("reset-output")
                                        .role(Role::Button)
                                        .aria_label(tr(
                                            zh,
                                            "Save next to source files",
                                            "恢复为源文件目录",
                                        ))
                                        .tooltip(ui::tooltip(tr(
                                            zh,
                                            "Save next to source files",
                                            "恢复为源文件目录",
                                        )))
                                        .cursor_pointer()
                                        .on_click(
                                            cx.listener(|this, _, _, cx| this.set_output(None, cx)),
                                        )
                                        .child(Icon::Close.svg().size(px(12.)).text_color(p.faint)),
                                )
                            }),
                    ),
            )
            .child(if active {
                Button::new("cancel")
                    .label(if cancelling {
                        tr(zh, "Finishing…", "正在收尾…")
                    } else {
                        tr(zh, "Cancel", "取消")
                    })
                    .tooltip("Esc")
                    .disabled(cancelling)
                    .on_click(cx.listener(|this, _, _, cx| {
                        this.cancel();
                        cx.notify();
                    }))
            } else {
                Button::new("start")
                    .primary()
                    .label(tr(zh, "Convert", "开始转换"))
                    .tooltip("Ctrl+Enter")
                    .disabled(self.busy() || summary.queued == 0)
                    .on_click(cx.listener(|this, _, _, cx| this.begin(cx)))
            })
    }
}

impl Render for Workbench {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let p = cx.palette();
        let summary = Summary::from_items(&self.items);
        let list = if !self.items.is_empty() {
            div()
                .relative()
                .size_full()
                .child(
                    uniform_list(
                        "queue-rows",
                        self.items.len(),
                        cx.processor(Self::render_rows),
                    )
                    .track_scroll(&self.list)
                    .size_full(),
                )
                .children(self.scrollbar(cx))
                .into_any_element()
        } else if self.scan.is_some() {
            div().into_any_element()
        } else {
            self.empty_state(cx).into_any_element()
        };
        div()
            .id("workbench")
            .key_context("Workbench")
            .track_focus(&self.focus)
            .role(Role::Application)
            .aria_label("Font Converter")
            .size_full()
            .flex()
            .flex_col()
            .bg(p.background)
            .text_color(p.text)
            .text_size(px(14.))
            .on_action(cx.listener(|this, _: &AddFiles, _, cx| this.choose(true, false, cx)))
            .on_action(cx.listener(|this, _: &AddFolder, _, cx| this.choose(false, false, cx)))
            .on_action(cx.listener(|this, _: &Begin, _, cx| this.begin(cx)))
            .on_action(cx.listener(|this, _: &CancelBatch, _, cx| {
                if this.menu_open {
                    this.menu_open = false;
                } else {
                    this.cancel();
                }
                cx.notify();
            }))
            .on_action(cx.listener(|this, _: &ClearQueue, _, cx| this.clear(cx)))
            .on_action(cx.listener(|this, _: &SelectPrevious, _, cx| this.step_selection(-1, cx)))
            .on_action(cx.listener(|this, _: &SelectNext, _, cx| this.step_selection(1, cx)))
            .on_action(cx.listener(|this, _: &SelectFirst, _, cx| this.select(0, cx)))
            .on_action(cx.listener(|this, _: &SelectLast, _, cx| this.select(usize::MAX, cx)))
            .on_action(cx.listener(|this, _: &RevealSelected, _, cx| this.reveal_selected(cx)))
            .on_action(cx.listener(|this, _: &RemoveSelected, _, cx| {
                if let Some(index) = this.selected {
                    this.remove(index, cx);
                }
            }))
            .on_action(cx.listener(|_, _: &FocusNext, window, cx| window.focus_next(cx)))
            .on_action(cx.listener(|_, _: &FocusPrevious, window, cx| window.focus_prev(cx)))
            .on_drop(cx.listener(|this, paths: &ExternalPaths, _, cx| {
                this.add(paths.paths().to_vec(), cx)
            }))
            .on_mouse_move(
                cx.listener(|this, event: &MouseMoveEvent, _, cx| this.drag_scroll(event, cx)),
            )
            .on_mouse_up(
                MouseButton::Left,
                cx.listener(|this, _, _, cx| {
                    if this.scroll_drag.take().is_some() {
                        cx.notify();
                    }
                }),
            )
            .child(self.toolbar(summary.total, cx))
            .child(self.banners(cx))
            .child(
                div()
                    .id("queue")
                    .mx_5()
                    .mb_4()
                    .flex_1()
                    .min_h_0()
                    .overflow_hidden()
                    .rounded(px(10.))
                    .border_1()
                    .border_color(p.border)
                    .bg(p.surface)
                    .drag_over::<ExternalPaths>(move |style, _, _, _| {
                        style.border_color(p.accent).bg(p.accent_soft)
                    })
                    .child(list),
            )
            .child(self.footer(&summary, cx))
    }
}
