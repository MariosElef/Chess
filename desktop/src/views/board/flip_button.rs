use gpui_kit::{
    assets::IconName,
    component::button::{Button, ButtonVariants},
    *,
};

use crate::i18n::tr;

/// Turns the page's board round: the small icon beside the board's top-right corner.
pub fn flip_button<T: 'static>(
    id: &'static str,
    cx: &mut Context<T>,
    flip: fn(&mut T, &mut Context<T>),
) -> Button {
    Button::new(id)
        .ghost()
        .icon(IconName::ArrowUpDown)
        .tooltip(tr!("board.flip_board"))
        .on_click(cx.listener(move |page, _, _, cx| flip(page, cx)))
}
