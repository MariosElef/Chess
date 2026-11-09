use std::rc::Rc;

use gpui_kit::{
    component::{ActiveTheme as _, h_flex, v_flex},
    *,
};

use super::{
    puzzle_pictures::{BATTLE_PICTURE, CUSTOM_PICTURE},
    rush_menu::RUSH_PICTURE,
};
use crate::components::{
    panel::{MenuTile, StartButton},
    shell::PUZZLES_PICTURE,
};

type OnClick = Rc<dyn Fn(&mut Window, &mut App)>;

/// The Puzzles panel: the puzzle rating, rated puzzles at it, and Custom Practice, Puzzle Rush
/// and Puzzle Battle.
#[derive(IntoElement)]
pub struct PuzzleMenu {
    rating_line: String,
    on_practice: OnClick,
    on_custom: OnClick,
    on_rush: OnClick,
    on_battle: OnClick,
}

impl PuzzleMenu {
    pub fn new(rating_line: String) -> Self {
        Self {
            rating_line,
            on_practice: Rc::new(|_, _| {}),
            on_custom: Rc::new(|_, _| {}),
            on_rush: Rc::new(|_, _| {}),
            on_battle: Rc::new(|_, _| {}),
        }
    }

    pub fn on_practice(mut self, handler: impl Fn(&mut Window, &mut App) + 'static) -> Self {
        self.on_practice = Rc::new(handler);
        self
    }

    pub fn on_custom(mut self, handler: impl Fn(&mut Window, &mut App) + 'static) -> Self {
        self.on_custom = Rc::new(handler);
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
    /// The heading with the rating and Solve Puzzles at it, then a tile per other mode.
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let (on_practice, on_custom) = (self.on_practice, self.on_custom);
        let (on_rush, on_battle) = (self.on_rush, self.on_battle);
        v_flex()
            .w_full()
            .gap_3()
            .child(heading(self.rating_line, cx))
            .child(StartButton::new(
                "start-practice",
                "Solve Puzzles",
                move |window, cx| on_practice(window, cx),
            ))
            .child(
                MenuTile::new("start-custom", "Custom Practice", move |window, cx| {
                    on_custom(window, cx)
                })
                .picture(CUSTOM_PICTURE),
            )
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
