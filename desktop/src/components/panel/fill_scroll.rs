use gpui_kit::*;

/// A box that takes the room left in its panel, never less than `min`, and scrolls inside.
///
/// Its content sits out of the flow, so a panel that itself scrolls never grows to fit it: the
/// rows around it stay in view, and the panel only scrolls once even `min` does not fit.
pub fn fill_scroll(id: impl Into<ElementId>, min: Rems, content: impl IntoElement) -> Div {
    div().w_full().flex_1().min_h(min).relative().child(
        div()
            .id(id)
            .absolute()
            .size_full()
            .overflow_y_scroll()
            .child(content),
    )
}
