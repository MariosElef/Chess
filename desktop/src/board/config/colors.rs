use gpui_kit::{App, Hsla, component::ActiveTheme as _, rgb};

/// The board's own colour roles, one set per theme mode, so no view writes a colour literal.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BoardColors {
    pub light_square: Hsla,
    pub dark_square: Hsla,
}

impl BoardColors {
    pub fn current(cx: &App) -> Self {
        if cx.theme().is_dark() {
            Self {
                light_square: rgb(0xb8c1cc).into(),
                dark_square: rgb(0x6f7f93).into(),
            }
        } else {
            Self {
                light_square: rgb(0xe9e5dc).into(),
                dark_square: rgb(0x9fb1c4).into(),
            }
        }
    }
}
