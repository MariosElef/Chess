use chess_domain::Side;
use gpui_kit::{
    component::{ActiveTheme as _, h_flex},
    prelude::FluentBuilder as _,
    *,
};

use crate::services::board::side_name;

/// How it is going, on a white card as chess.com's: a square of the colour to move, then the
/// status.
pub fn status_card(status: &str, good: bool, side: Option<Side>, cx: &App) -> Div {
    let theme = cx.theme();
    let text = if good {
        theme.success
    } else {
        rgb(0x0011_1111).into()
    };
    h_flex()
        .gap_3()
        .p_4()
        .rounded(px(10.))
        .bg(white())
        .border_1()
        .border_color(cx.theme().border)
        .text_color(text)
        .font_weight(FontWeight::SEMIBOLD)
        .when_some(side, |this, side| {
            let fill = match side {
                Side::White => white(),
                Side::Black => rgb(0x0030_3030).into(),
            };
            this.child(
                div()
                    .size_5()
                    .flex_shrink_0()
                    .rounded(px(4.))
                    .border_2()
                    .border_color(rgb(0x0030_3030))
                    .bg(fill),
            )
        })
        .child(div().flex_1().child(status.to_owned()))
        .when_some(side.filter(|_| status.is_empty()), |this, side| {
            this.child(format!("{} to move", side_name(side)))
        })
}
