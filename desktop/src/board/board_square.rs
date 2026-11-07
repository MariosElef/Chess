use chess_domain::{Piece, Square};
use gpui_kit::{prelude::FluentBuilder as _, *};

use super::config::{BoardColors, piece_image};

#[derive(IntoElement)]
pub struct BoardSquare {
    square: Square,
    piece: Option<Piece>,
    colors: BoardColors,
}

impl BoardSquare {
    pub fn new(square: Square, piece: Option<Piece>, colors: BoardColors) -> Self {
        Self {
            square,
            piece,
            colors,
        }
    }
}

impl RenderOnce for BoardSquare {
    fn render(self, _: &mut Window, _: &mut App) -> impl IntoElement {
        let background = if self.square.is_light() {
            self.colors.light_square
        } else {
            self.colors.dark_square
        };

        div()
            .flex_1()
            .flex()
            .items_center()
            .justify_center()
            .bg(background)
            .when_some(self.piece, |this, piece| {
                this.child(img(piece_image(piece).path).size_full())
            })
    }
}
