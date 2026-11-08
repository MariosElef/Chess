use gpui_kit::{component::ActiveTheme as _, *};

/// A circle with the person's initial, tinted with the theme's primary colour.
pub fn avatar(name: &str, size: Pixels, cx: &App) -> Div {
    initial_tile(name, size, cx).rounded_full()
}

/// A square with the person's initial, tinted as `avatar`: a friend's picture, as chess.com's.
pub fn square_avatar(name: &str, size: Pixels, cx: &App) -> Div {
    initial_tile(name, size, cx).rounded(px(4.))
}

/// The initial on the primary colour's tint, at the size given, before its corners are set.
fn initial_tile(name: &str, size: Pixels, cx: &App) -> Div {
    let primary = cx.theme().primary;
    div()
        .size(size)
        .flex_shrink_0()
        .flex()
        .items_center()
        .justify_center()
        .bg(primary.opacity(0.15))
        .text_color(primary)
        .text_size(size * 0.45)
        .font_weight(FontWeight::SEMIBOLD)
        .child(initial_of(name))
}

/// The first letter of the name, capital.
fn initial_of(name: &str) -> String {
    name.trim()
        .chars()
        .next()
        .map(|first| first.to_uppercase().collect())
        .unwrap_or_default()
}
