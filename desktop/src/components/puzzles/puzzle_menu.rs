use std::rc::Rc;

use gpui_kit::{
    component::{ActiveTheme as _, h_flex, slider::SliderState, v_flex},
    *,
};

use super::{puzzle_pictures::BATTLE_PICTURE, range_picker::RangePicker, rush_menu::RUSH_PICTURE};
use crate::{
    components::{
        panel::{MenuTile, StartButton},
        shell::PUZZLES_PICTURE,
    },
    models::puzzles::RatingRange,
};

type OnClick = Rc<dyn Fn(&mut Window, &mut App)>;

/// The Puzzles panel: the puzzle rating, Practice at a chosen difficulty, and the ways into
/// Puzzle Rush and Puzzle Battle.
#[derive(IntoElement)]
pub struct PuzzleMenu {
    rating_line: String,
    slider: Entity<SliderState>,
    range: RatingRange,
    on_practice: OnClick,
    on_rush: OnClick,
    on_battle: OnClick,
}

impl PuzzleMenu {
    pub fn new(rating_line: String, (slider, range): (Entity<SliderState>, RatingRange)) -> Self {
        Self {
            rating_line,
            slider,
            range,
            on_practice: Rc::new(|_, _| {}),
            on_rush: Rc::new(|_, _| {}),
            on_battle: Rc::new(|_, _| {}),
        }
    }

    pub fn on_practice(mut self, handler: impl Fn(&mut Window, &mut App) + 'static) -> Self {
        self.on_practice = Rc::new(handler);
        self
    }

    pub fn on_rush(mut self, handler: impl Fn(&mut Window, &mut App) + 'static) -> Self {
        self.on_rush = Rc::new(handler);
        self
    }

    pub fn on_battle(mut self, handler: impl Fn(&mut Window, &mut App) + 'static) -> Self {
        self.on_battle = Rc::new(handler);
        self
    }
}

impl RenderOnce for PuzzleMenu {
    /// As chess.com lays it out: the heading with the rating, the difficulty and Solve
    /// Puzzles, then a tile for each other way to play.
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let (on_practice, on_rush, on_battle) = (self.on_practice, self.on_rush, self.on_battle);
        v_flex()
            .w_full()
            .gap_3()
            .child(heading(self.rating_line, cx))
            .child(RangePicker::new(self.slider, self.range))
            .child(StartButton::new(
                "start-practice",
                "Solve Puzzles",
                move |window, cx| on_practice(window, cx),
            ))
            .child(
                MenuTile::new("open-rush", "Puzzle Rush", move |window, cx| {
                    on_rush(window, cx)
                })
                .picture(RUSH_PICTURE),
            )
            .child(
                MenuTile::new("open-battle", "Puzzle Battle", move |window, cx| {
                    on_battle(window, cx)
                })
                .picture(BATTLE_PICTURE),
            )
    }
}

/// The puzzles picture and the page's name, large, with the player's rating under them.
fn heading(rating_line: String, cx: &App) -> Div {
    v_flex()
        .items_center()
        .gap_1()
        .py_3()
        .child(
            h_flex()
                .gap_3()
                .child(img(PUZZLES_PICTURE).size_10())
                .child(
                    div()
                        .text_3xl()
                        .font_weight(FontWeight::EXTRA_BOLD)
                        .child("Puzzles"),
                ),
        )
        .child(muted(rating_line, cx))
}

fn muted(text: String, cx: &App) -> Div {
    div()
        .text_sm()
        .text_color(cx.theme().muted_foreground)
        .child(text)
}
