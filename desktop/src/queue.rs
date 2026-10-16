use crate::{
    Workbench,
    icons::Icon,
    settings::tr,
    theme::{ActivePalette, Palette},
    ui,
};
use font_converter_engine::{Item, Status};
use gpui::{prelude::*, *};
use std::ops::Range;

pub const ROW_HEIGHT: f32 = 44.;

pub fn status(zh: bool, value: Status) -> &'static str {
    match value {
        Status::Queued => tr(zh, "Queued", "排队"),
        Status::Running => tr(zh, "Converting", "转换中"),
        Status::Succeeded => tr(zh, "Done", "完成"),
        Status::Skipped => tr(zh, "Skipped", "跳过"),
        Status::Failed => tr(zh, "Failed", "失败"),
        Status::Cancelled => tr(zh, "Cancelled", "已取消"),
    }
}
pub fn size(bytes: u64) -> String {
    if bytes < 1024 {
        format!("{bytes} B")
    } else if bytes < 1024 * 1024 {
        format!("{:.1} KiB", bytes as f64 / 1024.)
    } else {
        format!("{:.1} MiB", bytes as f64 / 1048576.)
    }
}
pub fn path_text(path: &std::path::Path) -> String {
    let text = path.to_string_lossy();
    if cfg!(windows) {
        if let Some(unc) = text.strip_prefix(r"\\?\UNC\") {
            return format!(r"\\{unc}");
        }
        text.strip_prefix(r"\\?\").unwrap_or(&text).to_owned()
    } else {
        text.into_owned()
    }
}
fn file_name(item: &Item) -> String {
    item.input
        .file_name()
        .unwrap_or_default()
        .to_string_lossy()
        .into_owned()
}

/// 16 px status mark at the start of a row: the only status indicator.
fn mark(value: Status, p: &Palette) -> AnyElement {
    let icon = |icon: Icon, color: Hsla| {
        icon.svg()
            .size(px(15.))
            .text_color(color)
            .into_any_element()
    };
    match value {
        Status::Queued => div()
            .size(px(12.))
            .rounded_full()
            .border_1()
            .border_color(p.faint)
            .into_any_element(),
        Status::Running => div()
            .size(px(12.))
            .rounded_full()
            .border_2()
            .border_color(Palette::tint(p.accent, 0.35))
            .flex()
            .items_center()
            .justify_center()
            .child(div().size(px(4.)).rounded_full().bg(p.accent))
            .into_any_element(),
        Status::Succeeded => icon(Icon::Check, p.success),
        Status::Failed => icon(Icon::Alert, p.danger),
        Status::Skipped => icon(Icon::Minus, p.warning),
        Status::Cancelled => icon(Icon::Minus, p.faint),
    }
}

impl Workbench {
    pub(crate) fn render_rows(
        &mut self,
        range: Range<usize>,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) -> Vec<AnyElement> {
        #[cfg(test)]
        self.rendered_rows
            .set(self.rendered_rows.get() + range.len());
        let p = cx.palette();
        let zh = self.zh();
        let locked = self.batch.is_some();
        range
            .filter_map(|ix| {
                let item = self.items.get(ix)?;
                let selected = self.selected == Some(ix);
                let formats = format!(
                    "{} → {}",
                    item.input_format.extension().to_uppercase(),
                    item.output_format.extension().to_uppercase()
                );
                // One muted line on the right: what happens, or what happened.
                let (detail, detail_color) = match (item.status, item.output_bytes) {
                    (Status::Failed, _) => (
                        item.message
                            .clone()
                            .unwrap_or_else(|| status(zh, item.status).into()),
                        p.danger,
                    ),
                    (Status::Skipped, _) => (
                        item.message
                            .clone()
                            .unwrap_or_else(|| status(zh, item.status).into()),
                        p.muted,
                    ),
                    (Status::Succeeded, Some(out)) if item.input_bytes > 0 => {
                        let change = (out as f64 / item.input_bytes as f64 - 1.) * 100.;
                        (
                            format!(
                                "{formats} · {} → {} ({change:+.0}%)",
                                size(item.input_bytes),
                                size(out)
                            ),
                            p.muted,
                        )
                    }
                    _ => (format!("{formats} · {}", size(item.input_bytes)), p.muted),
                };
                let tip = match &item.message {
                    Some(message) => format!("{}\n{message}", path_text(&item.input)),
                    None => path_text(&item.input),
                };
                Some(
                    div()
                        .id(("row", ix))
                        .group("row")
                        .role(Role::Row)
                        .aria_label(SharedString::from(format!(
                            "{}, {}",
                            file_name(item),
                            status(zh, item.status)
                        )))
                        .aria_selected(selected)
                        .relative()
                        .h(px(ROW_HEIGHT))
                        .w_full()
                        .px_4()
                        .flex()
                        .items_center()
                        .gap_3()
                        .border_b_1()
                        .border_color(Palette::tint(p.border, 0.6))
                        .when_else(
                            selected,
                            |el| el.bg(p.selected),
                            |el| el.hover(move |s| s.bg(p.hover)),
                        )
                        .tooltip(ui::tooltip(tip))
                        .on_click(cx.listener(move |this, _, window, cx| {
                            window.focus(&this.focus, cx);
                            this.select(ix, cx);
                        }))
                        .child(
                            div()
                                .size(px(16.))
                                .flex_none()
                                .flex()
                                .items_center()
                                .justify_center()
                                .child(mark(item.status, &p)),
                        )
                        .child(
                            div()
                                .flex_1()
                                .min_w_0()
                                .flex()
                                .items_baseline()
                                .gap_2()
                                .child(
                                    div()
                                        .flex_shrink_0()
                                        .max_w(relative(0.7))
                                        .truncate()
                                        .text_size(px(13.5))
                                        .child(file_name(item)),
                                )
                                .child(
                                    div()
                                        .flex_1()
                                        .min_w_0()
                                        .whitespace_nowrap()
                                        .overflow_hidden()
                                        .text_ellipsis_start()
                                        .text_xs()
                                        .text_color(p.faint)
                                        .child(path_text(
                                            item.input.parent().unwrap_or(&item.input),
                                        )),
                                ),
                        )
                        .child(
                            div()
                                .flex_none()
                                .max_w(px(320.))
                                .truncate()
                                .text_xs()
                                .text_color(detail_color)
                                .group_hover("row", |s| s.invisible())
                                .child(detail),
                        )
                        // Row actions appear over the detail text on hover.
                        .child(
                            div()
                                .absolute()
                                .top_0()
                                .bottom_0()
                                .right_0()
                                .pl_4()
                                .pr_2()
                                .flex()
                                .items_center()
                                .gap_1()
                                .invisible()
                                .group_hover("row", |s| s.visible())
                                .child(
                                    ui::Button::new(("reveal", ix))
                                        .ghost()
                                        .small()
                                        .icon(Icon::Reveal)
                                        .aria(tr(zh, "Show in folder", "打开所在位置"))
                                        .tooltip(tr(
                                            zh,
                                            "Show in folder · Ctrl+R",
                                            "打开所在位置 · Ctrl+R",
                                        ))
                                        .on_click(cx.listener(move |this, _, _, cx| {
                                            this.selected = Some(ix);
                                            this.reveal_selected(cx);
                                        })),
                                )
                                .when(!locked, |el| {
                                    el.child(
                                        ui::Button::new(("remove", ix))
                                            .ghost()
                                            .small()
                                            .icon(Icon::Close)
                                            .aria(tr(zh, "Remove", "移除"))
                                            .tooltip(tr(zh, "Remove · Delete", "移除 · Delete"))
                                            .on_click(cx.listener(move |this, _, _, cx| {
                                                this.remove(ix, cx)
                                            })),
                                    )
                                }),
                        )
                        .into_any_element(),
                )
            })
            .collect()
    }
}
