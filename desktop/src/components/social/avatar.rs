use chess_domain::{Piece, PieceKind, Side};
use gpui_kit::{component::ActiveTheme as _, *};

use crate::{components::board::piece_image, enums::settings::PieceSet};

/// A circle with the person's initial, tinted with the theme's primary colour.
pub fn avatar(name: &str, size: Pixels, cx: &App) -> Div {
    let primary = cx.theme().primary;
    div()
        .size(size)
        .flex_shrink_0()
        .flex()
        .items_center()
        .justify_center()
        .rounded_full()
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

/// The light grey behind a stand-in picture, as chess.com's.
const STAND_IN_GROUND: u32 = 0x00c4_c2bf;

/// A square picture for a player with none of their own: a white pawn on light grey.
pub fn pawn_avatar(size: Pixels) -> Div {
    let pawn = Piece {
        side: Side::White,
        kind: PieceKind::Pawn,
    };
    div()
        .size(size)
        .flex_shrink_0()
        .flex()
        .items_center()
        .justify_center()
        .rounded(px(4.))
        .bg(rgb(STAND_IN_GROUND))
        .child(img(piece_image(pawn, PieceSet::default()).path).size(size * 0.7))
}
