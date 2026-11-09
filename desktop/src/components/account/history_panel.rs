use std::{rc::Rc, time::Duration};

use gpui_kit::{
    assets::IconName,
    component::{
        ActiveTheme as _, Icon, Sizable as _,
        button::{Button, ButtonVariants},
        h_flex, v_flex,
    },
    prelude::FluentBuilder as _,
    *,
};

use super::rating_card::FALL;
use crate::{
    components::{
        analysis::REVIEW_PICTURE,
        panel::START_GREEN,
        play::{format_clock, speed_picture},
        puzzles::BATTLE_PICTURE,
        shell::PUZZLES_PICTURE,
        social::EmptyState,
    },
    enums::{account::HistoryTab, play::PlayerResult},
    models::{
        play::PlayedGame,
        puzzles::{PlayedAttempt, PlayedBattle},
    },
    services::{
        account::category_of,
        play::{game_line, result_word},
        time::ago_text,
    },
};

pub type OnOpen = Rc<dyn Fn(String, bool, &mut Window, &mut App)>;
type OnTab = Rc<dyn Fn(HistoryTab, &mut Window, &mut App)>;
type OnMore = Rc<dyn Fn(&mut Window, &mut App)>;

/// The player's history as chess.com's archive lists it: games, battles and puzzles, by tab.
#[derive(IntoElement)]
pub struct HistoryPanel {
    tab: HistoryTab,
    rows: Vec<AnyElement>,
    /// More are on the server: Load more shows.
    more: bool,
    loading: bool,
    problem: Option<String>,
    on_tab: OnTab,
    on_more: OnMore,
}

impl HistoryPanel {
    pub fn new(
        (tab, rows): (HistoryTab, Vec<AnyElement>),
        (more, loading, problem): (bool, bool, Option<String>),
        on_tab: impl Fn(HistoryTab, &mut Window, &mut App) + 'static,
        on_more: impl Fn(&mut Window, &mut App) + 'static,
    ) -> Self {
        Self {
            tab,
            rows,
            more,
            loading,
            problem,
            on_tab: Rc::new(on_tab),
            on_more: Rc::new(on_more),
        }
    }
}

impl RenderOnce for HistoryPanel {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = cx.theme();
        let empty = self.rows.is_empty() && !self.loading && self.problem.is_none();
        let first_load = self.rows.is_empty() && self.loading;
        let muted = theme.muted_foreground;
        let tabs: Vec<Stateful<Div>> = HistoryTab::ALL
            .into_iter()
            .map(|tab| tab_title(tab, (tab == self.tab, Rc::clone(&self.on_tab)), cx))
            .collect();
        let on_more = Rc::clone(&self.on_more);
        v_flex()
            .w_full()
            .gap_3()
            .child(
                h_flex()
                    .gap_6()
                    .border_b_1()
                    .border_color(theme.border)
                    .child(
                        div()
                            .pb_2()
                            .text_lg()
                            .font_weight(FontWeight::BOLD)
                            .child("History"),
                    )
                    .children(tabs),
            )
            .child(
                v_flex()
                    .w_full()
                    .gap_1()
                    .p_2()
                    .rounded(px(8.))
                    .bg(theme.muted)
                    .when_some(self.problem, |this, problem| {
                        this.child(
                            div()
                                .p_2()
                                .text_sm()
                                .text_color(theme.danger)
                                .child(problem),
                        )
                    })
                    .when(empty, |this| this.child(empty_state(self.tab)))
                    .when(first_load, |this| {
                        this.child(div().p_3().text_sm().text_color(muted).child("Loading…"))
                    })
                    .children(self.rows)
                    .when(self.more, |this| {
                        this.child(
                            Button::new("history-more")
                                .ghost()
                                .w_full()
                                .label("Load more")
                                .loading(self.loading)
                                .on_click(move |_, window, cx| on_more(window, cx)),
                        )
                    }),
            )
    }
}

/// A tab's name along the heading, underlined when chosen.
fn tab_title(tab: HistoryTab, (chosen, on_tab): (bool, OnTab), cx: &App) -> Stateful<Div> {
    let theme = cx.theme();
    let (bright, muted) = (theme.foreground, theme.muted_foreground);
    let id = format!("history-tab-{}", tab.label().to_lowercase());
    let selector = id.clone();
    div()
        .id(SharedString::from(id))
        .debug_selector(move || selector.clone())
        .pb_2()
        .border_b_2()
        .cursor_pointer()
        .font_weight(FontWeight::SEMIBOLD)
        .when(chosen, |this| this.border_color(bright).text_color(bright))
        .when(!chosen, |this| {
            this.border_color(transparent_black())
                .text_color(muted)
                .hover(move |this| this.text_color(bright))
        })
        .child(tab.label())
        .on_click(move |_, window, cx| on_tab(tab, window, cx))
}

/// What an empty tab says.
fn empty_state(tab: HistoryTab) -> EmptyState {
    match tab {
        HistoryTab::Games => EmptyState::new(
            IconName::ChessKnight,
            "No games yet",
            "Your finished online games show up here.",
        ),
        HistoryTab::Battles => EmptyState::new(
            IconName::Swords,
            "No battles yet",
            "Your finished Puzzle Battles show up here.",
        ),
        HistoryTab::Puzzles => EmptyState::new(
            IconName::Puzzle,
            "No puzzles yet",
            "Puzzles you try in Practice and Puzzle Rush show up here.",
        ),
    }
}

/// A battle: the opponent, the score with yours first, the result, the rating change and when.
pub fn battle_row(battle: &PlayedBattle, cx: &App) -> Div {
    let muted = cx.theme().muted_foreground;
    let kind = if battle.rated { "Rated" } else { "Casual" };
    row_frame(cx)
        .child(picture_slot(Some(BATTLE_PICTURE)))
        .child(
            v_flex()
                .flex_1()
                .min_w_0()
                .child(
                    div()
                        .font_weight(FontWeight::SEMIBOLD)
                        .child(battle.opponent.clone()),
                )
                .child(div().text_xs().text_color(muted).child(format!(
                    "{} – {} · {kind}",
                    battle.my_score, battle.their_score
                ))),
        )
        .child(result_chip(battle.result, cx))
        .child(change_slot(battle.rating_change))
        .child(ago_slot(battle.ago_s, cx))
}

/// A puzzle attempt: solved or missed, its rating, how long it took, the change and when.
pub fn attempt_row(attempt: &PlayedAttempt, cx: &App) -> Div {
    let muted = cx.theme().muted_foreground;
    let (word, colour): (&str, Hsla) = if attempt.solved {
        ("Solved", rgb(START_GREEN).into())
    } else {
        ("Missed", rgb(FALL).into())
    };
    let rating = attempt
        .puzzle_rating
        .map_or_else(|| "Puzzle".to_owned(), |rating| format!("Puzzle {rating}"));
    let kind = if attempt.rated { "Rated" } else { "Not rated" };
    let took = format_clock(Duration::from_millis(attempt.duration_ms.into()));
    row_frame(cx)
        .child(picture_slot(Some(PUZZLES_PICTURE)))
        .child(
            v_flex()
                .flex_1()
                .min_w_0()
                .child(div().font_weight(FontWeight::SEMIBOLD).child(rating))
                .child(
                    div()
                        .text_xs()
                        .text_color(muted)
                        .child(format!("{took} · {kind}")),
                ),
        )
        .child(
            div()
                .w(px(64.))
                .flex_shrink_0()
                .text_sm()
                .font_weight(FontWeight::BOLD)
                .text_color(colour)
                .child(word),
        )
        .child(change_slot(attempt.rating_change))
        .child(ago_slot(attempt.ago_s, cx))
}

/// A row's frame: padded, rounded, lit on hover.
fn row_frame(cx: &App) -> Div {
    let hover = cx.theme().accent;
    h_flex()
        .gap_3()
        .px_3()
        .py_2()
        .rounded(px(6.))
        .hover(move |this| this.bg(hover))
}

/// The row's picture, centred in a fixed square so the rows line up.
fn picture_slot(path: Option<&'static str>) -> Div {
    div()
        .size_8()
        .flex_shrink_0()
        .flex()
        .items_center()
        .justify_center()
        .when_some(path, |this, path| this.child(img(path).size_6()))
}

fn change_slot(change: Option<i32>) -> Div {
    div()
        .w(px(56.))
        .flex_shrink_0()
        .text_sm()
        .font_weight(FontWeight::BOLD)
        .children(change.filter(|change| *change != 0).map(change_text))
}

fn ago_slot(ago_s: i64, cx: &App) -> Div {
    div()
        .w(px(110.))
        .flex_shrink_0()
        .text_xs()
        .text_color(cx.theme().muted_foreground)
        .child(ago_text(ago_s))
}

/// A game, its result, rating change and when, with Review; the row opens it for analysis.
pub fn game_row(game: &PlayedGame, on_open: OnOpen, cx: &App) -> Stateful<Div> {
    let muted = cx.theme().muted_foreground;
    let picture = category_of(&game.category).and_then(speed_picture);
    let (open, review) = (Rc::clone(&on_open), on_open);
    let (open_id, review_id) = (game.id.clone(), game.id.clone());
    row_frame(cx)
        .id(SharedString::from(format!("history-game-{}", game.id)))
        .cursor_pointer()
        .on_click(move |_, window, cx| open(open_id.clone(), false, window, cx))
        .child(picture_slot(picture))
        .child(
            v_flex()
                .flex_1()
                .min_w_0()
                .child(
                    div()
                        .font_weight(FontWeight::SEMIBOLD)
                        .child(game.opponent.clone()),
                )
                .child(div().text_xs().text_color(muted).child(game_line(game))),
        )
        .child(result_chip(game.result, cx))
        .child(change_slot(game.rating_change))
        .child(ago_slot(game.ago_s, cx))
        .child(
            Button::new(SharedString::from(format!("history-review-{}", game.id)))
                .ghost()
                .small()
                .child(img(REVIEW_PICTURE).size_4())
                .label("Review")
                .on_click(move |_, window, cx| {
                    cx.stop_propagation();
                    review(review_id.clone(), true, window, cx);
                }),
        )
}

/// Won in green, Lost in red, Draw in grey.
fn result_chip(result: PlayerResult, cx: &App) -> Div {
    let colour: Hsla = match result {
        PlayerResult::Won => rgb(START_GREEN).into(),
        PlayerResult::Lost => rgb(FALL).into(),
        PlayerResult::Drawn => cx.theme().muted_foreground,
    };
    div()
        .w(px(52.))
        .flex_shrink_0()
        .text_sm()
        .font_weight(FontWeight::BOLD)
        .text_color(colour)
        .child(result_word(result))
}

/// "↑ 8" in green or "↓ 12" in red; nothing for a change of nothing.
fn change_text(change: i32) -> Div {
    let (icon, colour) = if change >= 0 {
        (IconName::ArrowUp, START_GREEN)
    } else {
        (IconName::ArrowDown, FALL)
    };
    h_flex()
        .gap_0p5()
        .text_color(rgb(colour))
        .child(Icon::from(icon).size_3())
        .child(change.abs().to_string())
}
