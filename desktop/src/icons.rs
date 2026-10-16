use gpui::{AssetSource, Result, SharedString, Svg, svg};
use std::borrow::Cow;

/// The handful of line icons the window needs, embedded as SVG source.
/// GPUI renders an SVG as an alpha mask, so `text_color` sets its color.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Icon {
    Plus,
    Folder,
    Trash,
    Close,
    Alert,
    Minus,
    Check,
    Reveal,
    More,
}

impl Icon {
    const ALL: [Icon; 9] = [
        Icon::Plus,
        Icon::Folder,
        Icon::Trash,
        Icon::Close,
        Icon::Alert,
        Icon::Minus,
        Icon::Check,
        Icon::Reveal,
        Icon::More,
    ];
    fn name(self) -> &'static str {
        match self {
            Icon::Plus => "icons/plus.svg",
            Icon::Folder => "icons/folder.svg",
            Icon::Trash => "icons/trash.svg",
            Icon::Close => "icons/close.svg",
            Icon::Alert => "icons/alert.svg",
            Icon::Minus => "icons/minus.svg",
            Icon::Check => "icons/check.svg",
            Icon::Reveal => "icons/reveal.svg",
            Icon::More => "icons/more.svg",
        }
    }
    fn body(self) -> &'static str {
        match self {
            Icon::Plus => "<path d='M12 5v14M5 12h14'/>",
            Icon::Folder => {
                "<path d='M3 7.5A2.5 2.5 0 0 1 5.5 5H9l2 2.5h7.5A2.5 2.5 0 0 1 21 10v6.5A2.5 2.5 0 0 1 18.5 19h-13A2.5 2.5 0 0 1 3 16.5z'/>"
            }
            Icon::Trash => {
                "<path d='M4 7h16M10 11v6M14 11v6M6 7l1 12a2 2 0 0 0 2 2h6a2 2 0 0 0 2-2l1-12M9 7V4h6v3'/>"
            }
            Icon::Close => "<path d='M6 6l12 12M18 6L6 18'/>",
            Icon::Alert => {
                "<path d='M10.3 4.2 2.6 17.5A2 2 0 0 0 4.3 20.5h15.4a2 2 0 0 0 1.7-3L13.7 4.2a2 2 0 0 0-3.4 0zM12 9.5v4M12 17h.01'/>"
            }
            Icon::Minus => "<path d='M5 12h14'/>",
            Icon::Check => "<path d='M5 12.5l4.5 4.5L19 7.5'/>",
            Icon::Reveal => {
                "<path d='M14 4h6v6M20 4l-9 9M18 14v4a2 2 0 0 1-2 2H6a2 2 0 0 1-2-2V8a2 2 0 0 1 2-2h4'/>"
            }
            Icon::More => "<path d='M5 12h.01M12 12h.01M19 12h.01' stroke-width='3'/>",
        }
    }
    pub fn svg(self) -> Svg {
        svg().path(self.name())
    }
}

pub struct Assets;
impl AssetSource for Assets {
    fn load(&self, path: &str) -> Result<Option<Cow<'static, [u8]>>> {
        Ok(Icon::ALL
            .iter()
            .find(|icon| icon.name() == path)
            .map(|icon| {
                Cow::Owned(
                    format!(
                        "<svg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 24 24' fill='none' \
                     stroke='black' stroke-width='2' stroke-linecap='round' \
                     stroke-linejoin='round'>{}</svg>",
                        icon.body()
                    )
                    .into_bytes(),
                )
            }))
    }
    fn list(&self, path: &str) -> Result<Vec<SharedString>> {
        Ok(Icon::ALL
            .iter()
            .map(|icon| icon.name())
            .filter(|name| name.starts_with(path))
            .map(SharedString::from)
            .collect())
    }
}
