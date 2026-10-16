//! Small, self-drawn controls. They cover exactly what the window uses, so the
//! application does not need a general component library.
use crate::{icons::Icon, theme::ActivePalette};
use gpui::{prelude::*, *};
use std::rc::Rc;

type ClickHandler = Rc<dyn Fn(&ClickEvent, &mut Window, &mut App)>;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Variant {
    Primary,
    Secondary,
    Ghost,
}

#[derive(IntoElement)]
pub struct Button {
    id: ElementId,
    label: Option<SharedString>,
    icon: Option<Icon>,
    variant: Variant,
    small: bool,
    disabled: bool,
    tooltip: Option<SharedString>,
    aria: Option<SharedString>,
    on_click: Option<ClickHandler>,
}

impl Button {
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            id: id.into(),
            label: None,
            icon: None,
            variant: Variant::Secondary,
            small: false,
            disabled: false,
            tooltip: None,
            aria: None,
            on_click: None,
        }
    }
    pub fn label(mut self, label: impl Into<SharedString>) -> Self {
        self.label = Some(label.into());
        self
    }
    pub fn icon(mut self, icon: Icon) -> Self {
        self.icon = Some(icon);
        self
    }
    pub fn primary(mut self) -> Self {
        self.variant = Variant::Primary;
        self
    }
    pub fn ghost(mut self) -> Self {
        self.variant = Variant::Ghost;
        self
    }
    pub fn small(mut self) -> Self {
        self.small = true;
        self
    }
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }
    pub fn tooltip(mut self, text: impl Into<SharedString>) -> Self {
        self.tooltip = Some(text.into());
        self
    }
    /// Accessible name for icon-only buttons.
    pub fn aria(mut self, text: impl Into<SharedString>) -> Self {
        self.aria = Some(text.into());
        self
    }
    pub fn on_click(
        mut self,
        handler: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_click = Some(Rc::new(handler));
        self
    }
}

impl RenderOnce for Button {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let p = cx.palette();
        let (bg, fg, hover, border) = match self.variant {
            Variant::Primary => (p.accent, p.accent_text, p.accent_hover, p.accent),
            Variant::Secondary => (p.surface, p.text, p.hover, p.border),
            Variant::Ghost => (transparent_black(), p.muted, p.hover, transparent_black()),
        };
        let height = px(if self.small { 28. } else { 34. });
        let icon_size = px(if self.small { 14. } else { 16. });
        let hover_text = if self.variant == Variant::Ghost {
            p.text
        } else {
            fg
        };
        let name = self.aria.clone().or_else(|| self.label.clone());
        div()
            .id(self.id)
            .role(Role::Button)
            .when_some(name, |el, name| el.aria_label(name))
            .when(!self.disabled, |el| el.focusable().tab_stop(true))
            .h(height)
            .flex_none()
            .when(self.label.is_some(), |el| {
                el.px(px(if self.small { 10. } else { 14. }))
            })
            .when(self.label.is_none(), |el| el.w(height))
            .flex()
            .items_center()
            .justify_center()
            .gap(px(6.))
            .rounded(px(8.))
            .border_1()
            .border_color(border)
            .bg(bg)
            .text_color(fg)
            .text_size(px(if self.small { 12.5 } else { 13.5 }))
            .font_weight(FontWeight::MEDIUM)
            .whitespace_nowrap()
            .focus_visible(move |s| s.border_color(p.accent).border_2())
            .when(self.variant == Variant::Secondary, |el| el.shadow_xs())
            .when_else(
                self.disabled,
                |el| el.opacity(0.45),
                |el| {
                    el.cursor_pointer()
                        .hover(move |s| s.bg(hover).text_color(hover_text))
                        .active(|s| s.opacity(0.85))
                        .when_some(self.on_click, |el, handler| {
                            el.on_click(move |event, window, cx| {
                                cx.stop_propagation();
                                handler(event, window, cx)
                            })
                        })
                },
            )
            .when_some(self.tooltip, |el, text| el.tooltip(tooltip(text)))
            .when_some(self.icon, |el, icon| {
                el.child(icon.svg().size(icon_size).flex_none().text_color(fg))
            })
            .when_some(self.label, |el, label| el.child(label))
    }
}

pub struct Tooltip(SharedString);
impl Render for Tooltip {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let p = cx.palette();
        div()
            .max_w(px(460.))
            .px_2()
            .py_1()
            .rounded(px(6.))
            .bg(if p.dark { rgb(0x2b3341) } else { rgb(0x1f2533) })
            .text_color(rgb(0xf3f5f9))
            .text_xs()
            .shadow_md()
            .child(self.0.clone())
    }
}

pub fn tooltip(
    text: impl Into<SharedString>,
) -> impl Fn(&mut Window, &mut App) -> AnyView + 'static {
    let text = text.into();
    move |_, cx| cx.new(|_| Tooltip(text.clone())).into()
}

/// A row of mutually exclusive options; replaces drop-down menus for short
/// choices, so every option stays visible and one click away.
pub fn segmented<T: Copy + PartialEq + 'static>(
    id: &'static str,
    label: &'static str,
    options: Vec<(T, SharedString)>,
    value: T,
    disabled: bool,
    on_select: impl Fn(T, &mut Window, &mut App) + 'static,
    cx: &App,
) -> impl IntoElement {
    let p = cx.palette();
    let on_select = Rc::new(on_select);
    div()
        .id(id)
        .role(Role::RadioGroup)
        .aria_label(label)
        .flex()
        .w_full()
        .p(px(3.))
        .gap(px(2.))
        .rounded(px(9.))
        .bg(p.subtle)
        .border_1()
        .border_color(p.border)
        .when(disabled, |el| el.opacity(0.5))
        .children(
            options
                .into_iter()
                .enumerate()
                .map(|(index, (option, text))| {
                    let selected = option == value;
                    let on_select = on_select.clone();
                    div()
                        .id((id, index))
                        .role(Role::RadioButton)
                        .aria_label(text.clone())
                        .aria_toggled(if selected {
                            Toggled::True
                        } else {
                            Toggled::False
                        })
                        .flex_1()
                        .min_w_0()
                        .h(px(28.))
                        .flex()
                        .items_center()
                        .justify_center()
                        .rounded(px(7.))
                        .text_size(px(12.5))
                        .whitespace_nowrap()
                        .overflow_hidden()
                        .focus_visible(move |s| s.border_1().border_color(p.accent))
                        .when_else(
                            selected,
                            |el| {
                                el.bg(p.raised)
                                    .text_color(p.text)
                                    .font_weight(FontWeight::MEDIUM)
                                    .shadow_xs()
                            },
                            |el| el.text_color(p.muted),
                        )
                        .when(!disabled && !selected, |el| {
                            el.focusable()
                                .tab_stop(true)
                                .cursor_pointer()
                                .hover(move |s| s.text_color(p.text))
                                .on_click(move |_, window, cx| on_select(option, window, cx))
                        })
                        .child(text)
                }),
        )
}
