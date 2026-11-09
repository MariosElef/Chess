use gpui_kit::{component::ActiveTheme as _, prelude::FluentBuilder as _, *};

use crate::models::account::Face;

/// A circle with the person's picture, or their initial on the primary colour's tint.
pub fn avatar(name: &str, picture: Option<&str>, size: Pixels, cx: &App) -> Div {
    pictured(initial_tile(name, size, cx), picture, (size, size / 2.))
}

/// A square with the person's picture or initial, as `avatar`: a friend's, as chess.com's.
pub fn square_avatar(name: &str, picture: Option<&str>, size: Pixels, cx: &App) -> Div {
    pictured(initial_tile(name, size, cx), picture, (size, SQUARE_CORNER))
}

/// A player's picture or initial on a board strip, beside their name.
pub fn strip_avatar(face: &Face, cx: &App) -> Div {
    square_avatar(&face.name, face.picture.as_deref(), px(32.), cx)
}

/// A square avatar's corners, as chess.com rounds a friend's picture.
const SQUARE_CORNER: Pixels = px(4.);

/// The picture over the initial, each rounded by `corner`, as GPUI clips only to rectangles.
fn pictured(tile: Div, picture: Option<&str>, (size, corner): (Pixels, Pixels)) -> Div {
    tile.relative()
        .rounded(corner)
        .when_some(picture, |this, path| {
            this.child(
                img(SharedString::from(path.to_owned()))
                    .absolute()
                    .top_0()
                    .left_0()
                    .size(size)
                    .rounded(corner)
                    .object_fit(ObjectFit::Cover),
            )
        })
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
