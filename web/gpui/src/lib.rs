use gpui::{prelude::*, *};
use std::time::Duration;
use wasm_bindgen::prelude::*;

#[wasm_bindgen(js_namespace = fontConverter)]
extern "C" {
    fn choose();
    fn pending() -> bool;
    fn ready() -> bool;
    fn status() -> String;
    fn download();
}

struct Converter;

fn control(label: &'static str, enabled: bool) -> Stateful<Div> {
    div()
        .id(label)
        .rounded(px(6.))
        .px(px(18.))
        .py(px(10.))
        .bg(if enabled {
            rgb(0x245ea6)
        } else {
            rgb(0x8296ae)
        })
        .text_color(rgb(0xffffff))
        .when(enabled, |el| el.cursor_pointer())
        .child(label)
}

impl Render for Converter {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .size_full()
            .bg(rgb(0xf7f8fa))
            .p(px(32.))
            .flex()
            .flex_col()
            .gap(px(12.))
            .font_family("Source Code Pro")
            .text_color(rgb(0x172438))
            .child(div().text_size(px(26.)).child("Font Converter"))
            .child("TTF / OTF  ↔  WOFF2")
            .child(
                div()
                    .flex()
                    .gap(px(8.))
                    .child(
                        control("Choose font", !pending()).when(!pending(), |button| {
                            button.on_click(cx.listener(|_, _, _, cx| {
                                choose();
                                cx.notify();
                                cx.spawn(async move |view, cx| {
                                    let mut last = status();
                                    while pending() {
                                        cx.background_executor()
                                            .timer(Duration::from_millis(50))
                                            .await;
                                        let message = status();
                                        if message != last {
                                            last = message;
                                            if view.update(cx, |_, cx| cx.notify()).is_err() {
                                                return;
                                            }
                                        }
                                    }
                                    let _ = view.update(cx, |_, cx| cx.notify());
                                })
                                .detach();
                            }))
                        }),
                    )
                    .child(
                        control("Download", ready() && !pending())
                            .when(ready() && !pending(), |button| {
                                button.on_click(|_, _, _| download())
                            }),
                    ),
            )
            .child(status())
    }
}

#[wasm_bindgen(start)]
pub fn start() {
    gpui_platform::web_init();
    gpui_platform::single_threaded_web().run(|cx| {
        cx.text_system()
            .add_fonts(vec![
                include_bytes!("../../../tests/fixtures/SourceCodePro-Regular.otf")
                    .as_slice()
                    .into(),
            ])
            .expect("bundled font");
        cx.open_window(WindowOptions::default(), |_, cx| cx.new(|_| Converter))
            .expect("browser window");
    });
}
