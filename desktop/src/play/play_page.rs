use chess_domain::Board;
use gpui_kit::{
    component::{label::Label, v_flex},
    *,
};

use crate::board::BoardGrid;

#[derive(IntoElement)]
pub struct PlayPage;

impl RenderOnce for PlayPage {
    fn render(self, _: &mut Window, _: &mut App) -> impl IntoElement {
        v_flex()
            .size_full()
            .p_6()
            .gap_4()
            .child(Label::new("Play").text_2xl())
            .child(
                div()
                    .flex_1()
                    .min_h_0()
                    .flex()
                    .justify_center()
                    .child(BoardGrid::new(Board::starting())),
            )
    }
}
