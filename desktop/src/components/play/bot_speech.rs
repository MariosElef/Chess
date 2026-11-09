use gpui_kit::{
    component::{h_flex, label::Label},
    prelude::FluentBuilder as _,
    *,
};

/// How big the bot's picture is beside what it says.
const PICTURE: Pixels = px(88.);
/// The bubble's text, dark on white whatever the theme.
const BUBBLE_TEXT: u32 = 0x00_26_26_26;

/// A bot at the top of the game's panel: its picture with its rating, and what it said last.
#[derive(IntoElement)]
pub struct BotSpeech {
    picture: &'static str,
    rating: u32,
    line: Option<&'static str>,
}

impl BotSpeech {
    pub fn new(picture: &'static str, rating: u32) -> Self {
        Self {
            picture,
            rating,
            line: None,
        }
    }

    pub fn line(mut self, line: Option<&'static str>) -> Self {
        self.line = line;
        self
    }
}

impl RenderOnce for BotSpeech {
    fn render(self, _: &mut Window, _: &mut App) -> impl IntoElement {
        h_flex()
            .w_full()
            .py_2()
            .gap_4()
            .items_center()
            .child(portrait(self.picture, self.rating))
            .when_some(self.line, |this, line| this.child(bubble(line)))
    }
}

/// The bot's picture, with its rating in a small dark badge across its bottom edge.
fn portrait(picture: &'static str, rating: u32) -> Div {
    div()
        .relative()
        .flex_shrink_0()
        .size(PICTURE)
        .child(img(picture).size_full().rounded(px(10.)))
        .child(
            div()
                .absolute()
                .bottom(px(-6.))
                .left_0()
                .right_0()
                .flex()
                .justify_center()
                .child(
                    div()
                        .px_1p5()
                        .rounded(px(4.))
                        .bg(rgba(0x00_00_00_cc))
                        .text_xs()
                        .text_color(white())
                        .child(Label::new(rating.to_string())),
                ),
        )
}

/// What the bot says, in a white bubble.
fn bubble(line: &'static str) -> Div {
    div()
        .flex_1()
        .min_w_0()
        .px_4()
        .py_3()
        .rounded(px(12.))
        .bg(white())
        .text_color(rgb(BUBBLE_TEXT))
        .child(line)
}
