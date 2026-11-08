use gpui_kit::{component::h_flex, *};

use super::choice_tile::ChoiceTile;

/// A row of buttons along the bottom of a panel: solid tiles sharing the width.
pub fn action_bar(tiles: Vec<ChoiceTile>) -> Div {
    h_flex()
        .w_full()
        .gap_2()
        .children(tiles.into_iter().map(|tile| div().flex_1().child(tile)))
}
