use chess_domain::Square;
use gpui_kit::{App, Hsla, component::ActiveTheme as _, hsla, rgb};

use crate::enums::board::SquareHighlight;

/// The board's own colour roles, one set per theme mode, so no view writes a colour literal.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BoardColors {
    pub light_square: Hsla,
    pub dark_square: Hsla,
    pub selected_square: Hsla,
    /// The two squares of the move that led to the shown position.
    pub last_move_square: Hsla,
    /// The king's square while it is in check.
    pub check_square: Hsla,
    /// A square the player marked with a right-click; softer than check so the two read apart.
    pub marked_square: Hsla,
    /// The squares of a queued premove, chess.com's red.
    pub premove_square: Hsla,
    /// Arrows the player drew with a right-drag.
    pub arrow: Hsla,
    /// The engine's best move, a different colour from the player's own arrows.
    pub engine_arrow: Hsla,
    /// The dots and rings marking where the picked-up piece can go.
    pub move_hint: Hsla,
    /// White's and Black's parts of the analysis eval bar.
    pub eval_white: Hsla,
    pub eval_black: Hsla,
    /// Dims the board behind the promotion picker.
    pub overlay: Hsla,
}

impl BoardColors {
    pub fn current(cx: &App) -> Self {
        if cx.theme().is_dark() {
            Self::dark_mode()
        } else {
            Self::light_mode()
        }
    }

    pub fn background(&self, square: Square, highlight: SquareHighlight) -> Hsla {
        match highlight {
            SquareHighlight::None => self.square(square),
            SquareHighlight::LastMove => self.last_move_square,
            SquareHighlight::Marked => self.marked_square,
            SquareHighlight::Premove => self.premove_square,
            SquareHighlight::Selected => self.selected_square,
            SquareHighlight::Check => self.check_square,
        }
    }

    pub fn square(&self, square: Square) -> Hsla {
        if square.is_light() {
            self.light_square
        } else {
            self.dark_square
        }
    }

    /// The coordinate letters take the other square colour, so they read on either.
    pub fn coordinate(&self, square: Square) -> Hsla {
        if square.is_light() {
            self.dark_square
        } else {
            self.light_square
        }
    }

    fn light_mode() -> Self {
        Self {
            light_square: rgb(0xe9e5dc).into(),
            dark_square: rgb(0x9fb1c4).into(),
            selected_square: rgb(0xf0d27a).into(),
            last_move_square: rgb(0xe3d79b).into(),
            check_square: rgb(0xd9483b).into(),
            marked_square: rgb(0xeb8f80).into(),
            premove_square: rgb(0xd96c5f).into(),
            arrow: hsla(0.09, 0.95, 0.5, 0.8),
            engine_arrow: hsla(0.6, 0.75, 0.45, 0.75),
            move_hint: hsla(0., 0., 0., 0.25),
            eval_white: rgb(0xf4f2ee).into(),
            eval_black: rgb(0x3a3a3c).into(),
            overlay: hsla(0., 0., 0., 0.45),
        }
    }

    fn dark_mode() -> Self {
        Self {
            light_square: rgb(0xb8c1cc).into(),
            dark_square: rgb(0x6f7f93).into(),
            selected_square: rgb(0xc9a94e).into(),
            last_move_square: rgb(0xb4a86a).into(),
            check_square: rgb(0xb5342a).into(),
            marked_square: rgb(0xc9776a).into(),
            premove_square: rgb(0xb85a4e).into(),
            arrow: hsla(0.09, 0.9, 0.5, 0.8),
            engine_arrow: hsla(0.6, 0.7, 0.55, 0.75),
            move_hint: hsla(0., 0., 0., 0.3),
            eval_white: rgb(0xe8e6e1).into(),
            eval_black: rgb(0x1f1f21).into(),
            overlay: hsla(0., 0., 0., 0.55),
        }
    }
}
