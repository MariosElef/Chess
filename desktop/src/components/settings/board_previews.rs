//! Small pictures of the board choices: a corner of a board in a theme's colours, and a few
//! pieces of a set standing on the chosen board.
use chess_domain::{Piece, PieceKind, Side};
use gpui_kit::{
    component::{ActiveTheme as _, h_flex, v_flex},
    *,
};

use crate::{
    components::board::{piece_image, square_colors},
    enums::settings::{BoardTheme, PieceSet},
};

/// Squares along each side of the theme preview.
const THEME_SQUARES: u8 = 4;
const THEME_SQUARE_SIZE: Pixels = px(18.);
const PIECE_SQUARE_SIZE: Pixels = px(36.);

/// The pieces each set is shown with: one of every kind, alternating colours.
const SHOWN_PIECES: [Piece; 4] = [
    Piece::new(Side::White, PieceKind::King),
    Piece::new(Side::Black, PieceKind::Queen),
    Piece::new(Side::White, PieceKind::Knight),
    Piece::new(Side::Black, PieceKind::Pawn),
];

/// A four-by-four corner of a board in `theme`'s colours.
pub fn theme_preview(theme: BoardTheme, cx: &App) -> Div {
    let squares = square_colors(theme, cx.theme().is_dark());
    v_flex()
        .rounded(cx.theme().radius)
        .overflow_hidden()
        .children((0..THEME_SQUARES).map(|row| {
            h_flex().children(
                (0..THEME_SQUARES).map(|column| {
                    preview_square(is_light(row, column), squares, THEME_SQUARE_SIZE)
                }),
            )
        }))
}

/// A row of `set`'s pieces, standing on squares of the board theme in use.
pub fn piece_set_preview(set: PieceSet, board: BoardTheme, cx: &App) -> Div {
    let squares = square_colors(board, cx.theme().is_dark());
    h_flex()
        .rounded(cx.theme().radius)
        .overflow_hidden()
        .children(SHOWN_PIECES.into_iter().zip(0..).map(|(piece, column)| {
            preview_square(is_light(0, column), squares, PIECE_SQUARE_SIZE)
                .child(img(piece_image(piece, set).path).size_full())
        }))
}

/// Squares alternate like a board's: the top-left one is light.
fn is_light(row: u8, column: u8) -> bool {
    (row + column).is_multiple_of(2)
}

fn preview_square(light: bool, (light_color, dark_color): (Hsla, Hsla), size: Pixels) -> Div {
    div()
        .size(size)
        .bg(if light { light_color } else { dark_color })
}

#[cfg(test)]
mod tests {
    use super::is_light;

    #[test]
    fn the_corner_square_is_light_and_neighbours_alternate() {
        assert!(is_light(0, 0));
        assert!(!is_light(0, 1));
        assert!(!is_light(1, 0));
        assert!(is_light(1, 1));
    }
}
