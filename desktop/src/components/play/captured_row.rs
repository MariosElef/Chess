use chess_domain::{Board, Piece, PieceKind, Side};
use gpui_kit::{
    component::{ActiveTheme as _, h_flex, label::Label},
    prelude::FluentBuilder as _,
    *,
};

use crate::{
    components::board::{BoardStyle, piece_image},
    enums::settings::PieceSet,
    services::play::side_label,
};

/// The pieces one side has taken, small, with "+N" when that side is ahead in material.
#[derive(IntoElement)]
pub struct CapturedRow {
    side: Side,
    taken: Vec<PieceKind>,
    advantage: i32,
}

impl CapturedRow {
    /// What `side` has captured on `board`, and by how much it leads.
    pub fn for_side(board: &Board, side: Side) -> Self {
        Self {
            side,
            taken: board.captured(side.opponent()),
            advantage: advantage(board.material_balance(), side),
        }
    }
}

impl RenderOnce for CapturedRow {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let lost_side = self.side.opponent();
        let pieces = BoardStyle::current(cx).pieces;
        h_flex()
            .h_6()
            .gap_1()
            .child(Label::new(side_label(self.side)).text_sm().w_12())
            .children(
                self.taken
                    .iter()
                    .map(|&kind| small_piece(Piece::new(lost_side, kind), pieces)),
            )
            .when(self.advantage > 0, |this| {
                this.child(advantage_text(self.advantage, cx))
            })
    }
}

/// The balance seen from `side`: positive when it has more material.
fn advantage(white_minus_black: i32, side: Side) -> i32 {
    match side {
        Side::White => white_minus_black,
        Side::Black => -white_minus_black,
    }
}

fn small_piece(piece: Piece, pieces: PieceSet) -> Img {
    img(piece_image(piece, pieces).path).size_5()
}

fn advantage_text(advantage: i32, cx: &App) -> Div {
    div()
        .text_sm()
        .text_color(cx.theme().muted_foreground)
        .child(format!("+{advantage}"))
}

#[cfg(test)]
mod tests {
    use chess_domain::Side;

    use super::advantage;

    #[test]
    fn the_balance_is_read_from_each_side() {
        assert_eq!(advantage(3, Side::White), 3);
        assert_eq!(advantage(3, Side::Black), -3);
    }
}
