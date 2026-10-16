use gpui::{App, Global, Hsla, WindowAppearance, rgb};

/// Application palette. Every surface reads colors from here, so switching
/// light/dark only swaps this global.
#[derive(Clone, Copy)]
pub struct Palette {
    pub dark: bool,
    pub background: Hsla,
    pub surface: Hsla,
    /// Selected segment / raised chip on top of `subtle`.
    pub raised: Hsla,
    pub subtle: Hsla,
    pub hover: Hsla,
    pub selected: Hsla,
    pub border: Hsla,
    pub text: Hsla,
    pub muted: Hsla,
    pub faint: Hsla,
    pub accent: Hsla,
    pub accent_hover: Hsla,
    pub accent_text: Hsla,
    pub accent_soft: Hsla,
    pub success: Hsla,
    pub danger: Hsla,
    pub warning: Hsla,
}
impl Global for Palette {}

fn c(hex: u32) -> Hsla {
    rgb(hex).into()
}

impl Palette {
    pub fn new(dark: bool) -> Self {
        if dark {
            Self {
                dark,
                background: c(0x0e1116),
                surface: c(0x171c24),
                raised: c(0x2c3546),
                subtle: c(0x1d232d),
                hover: c(0x222a36),
                selected: c(0x1f2c46),
                border: c(0x272f3b),
                text: c(0xe7eaf0),
                muted: c(0x97a1b2),
                faint: c(0x5f6878),
                accent: c(0x7b93ff),
                accent_hover: c(0x91a5ff),
                accent_text: c(0x0b1020),
                accent_soft: c(0x232f52),
                success: c(0x4cc38a),
                danger: c(0xf0676e),
                warning: c(0xe5b24e),
            }
        } else {
            Self {
                dark,
                background: c(0xf6f7f9),
                surface: c(0xffffff),
                raised: c(0xffffff),
                subtle: c(0xf3f4f7),
                hover: c(0xf0f2f6),
                selected: c(0xe9eeff),
                border: c(0xe1e4ea),
                text: c(0x1a2130),
                muted: c(0x5f6b7f),
                faint: c(0x9aa3b2),
                accent: c(0x3a5bd9),
                accent_hover: c(0x2f4cc0),
                accent_text: c(0xffffff),
                accent_soft: c(0xe4e9ff),
                success: c(0x13805c),
                danger: c(0xcc3340),
                warning: c(0xa86b0c),
            }
        }
    }
    pub fn tint(color: Hsla, alpha: f32) -> Hsla {
        Hsla { a: alpha, ..color }
    }
}

pub fn is_dark(preference: &str, system: WindowAppearance) -> bool {
    match preference {
        "dark" => true,
        "light" => false,
        _ => matches!(
            system,
            WindowAppearance::Dark | WindowAppearance::VibrantDark
        ),
    }
}

pub fn apply(dark: bool, cx: &mut App) {
    cx.set_global(Palette::new(dark));
}

pub trait ActivePalette {
    fn palette(&self) -> Palette;
}
impl ActivePalette for App {
    fn palette(&self) -> Palette {
        self.try_global::<Palette>()
            .copied()
            .unwrap_or_else(|| Palette::new(false))
    }
}
