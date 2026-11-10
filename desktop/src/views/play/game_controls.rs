//! What Play's own games and online games share: the buttons along the bottom of their panels.
use gpui_kit::*;

use crate::components::panel::ChoiceTile;

/// A game button at the bottom of the panel: a solid tile that calls `act` on the page.
pub fn game_tile<T: 'static>(
    (id, label): (&'static str, String),
    picked: bool,
    cx: &mut Context<T>,
    act: fn(&mut T, &mut Context<T>),
) -> ChoiceTile {
    let page = cx.entity().downgrade();
    ChoiceTile::new(id, label, picked, move |_, cx| {
        let _ = page.update(cx, act);
    })
}
