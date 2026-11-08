use chess_domain::Side;
use gpui_kit::*;

/// The dark side's colour, on chips and the graph's ground, and the ink on the light side.
pub const DARK_SIDE: u32 = 0x0040_3e3c;
const LIGHT_INK: u32 = 0x0030_2e2c;
/// The light chip's edge, so it keeps its shape on a light panel.
const LIGHT_EDGE: u32 = 0x00c8_c8c8;

/// A number on a chip in a side's colours: white with dark ink, or dark with white ink, as
/// chess.com shows evaluations, accuracy and ratings.
pub fn side_chip(text: String, side: Side) -> Div {
    let chip = div()
        .rounded(px(4.))
        .border_1()
        .text_center()
        .font_weight(FontWeight::BOLD)
        .child(text);
    match side {
        Side::White => chip
            .bg(white())
            .border_color(rgb(LIGHT_EDGE))
            .text_color(rgb(LIGHT_INK)),
        Side::Black => chip
            .bg(rgb(DARK_SIDE))
            .border_color(rgb(DARK_SIDE))
            .text_color(white()),
    }
}
