use gpui_kit::{
    component::{ActiveTheme as _, h_flex, v_flex},
    prelude::FluentBuilder as _,
    *,
};

use crate::{
    components::play::speed_label,
    components::shell::PUZZLES_PICTURE,
    models::account::{ModeStats, PuzzleStats, Stats},
    services::account::{category_of, rating_text, solved_share},
};

/// The player's ratings in every game mode and in puzzles, each on a tile with its best and
/// how the games or puzzles went.
#[derive(IntoElement)]
pub struct StatsPanel {
    stats: Stats,
}

impl StatsPanel {
    pub fn new(stats: Stats) -> Self {
        Self { stats }
    }
}

impl RenderOnce for StatsPanel {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let modes: Vec<Div> = self
            .stats
            .modes
            .iter()
            .map(|mode| mode_tile(mode, cx))
            .collect();
        v_flex()
            .w_full()
            .max_w(px(720.))
            .gap_3()
            .child(
                div()
                    .text_sm()
                    .text_color(cx.theme().muted_foreground)
                    .child("Stats"),
            )
            .child(h_flex().flex_wrap().gap_3().children(modes))
            .child(puzzle_tile(self.stats.puzzles, cx))
    }
}

fn mode_tile(mode: &ModeStats, cx: &App) -> Div {
    let name = category_of(&mode.category)
        .map_or_else(|| div().text_sm().child(mode.category.clone()), speed_label);
    let record = format!(
        "{} games · {} won · {} lost · {} drawn",
        mode.games, mode.wins, mode.losses, mode.draws
    );
    tile(cx)
        .w(px(168.))
        .child(name)
        .child(big_number(rating_text(mode.rating, mode.provisional)))
        .when_some(mode.best, |this, best| {
            this.child(small_line(format!("Best {best}"), cx))
        })
        .child(small_line(record, cx))
}

fn puzzle_tile(puzzles: PuzzleStats, cx: &App) -> Div {
    let solved = solved_share(puzzles.solved, puzzles.attempted).map_or_else(
        || "No puzzles yet".to_owned(),
        |share| {
            format!(
                "{} of {} solved · {share}%",
                puzzles.solved, puzzles.attempted
            )
        },
    );
    let rush = puzzles.best_rush;
    let best_rush = [
        ("3 min", rush.three_minutes),
        ("5 min", rush.five_minutes),
        ("Survival", rush.survival),
    ]
    .map(|(name, best)| {
        format!(
            "{name} {}",
            best.map_or("—".to_owned(), |best| best.to_string())
        )
    })
    .join(" · ");
    tile(cx)
        .w_full()
        .child(
            h_flex()
                .gap_1p5()
                .text_sm()
                .child(img(PUZZLES_PICTURE).size_4().flex_shrink_0())
                .child("Puzzles"),
        )
        .child(big_number(rating_text(puzzles.rating, puzzles.provisional)))
        .when_some(puzzles.best, |this, best| {
            this.child(small_line(format!("Best {best}"), cx))
        })
        .child(small_line(solved, cx))
        .child(small_line(format!("Puzzle Rush best: {best_rush}"), cx))
        .child(small_line(
            format!(
                "Puzzle Battle rating: {}",
                rating_text(puzzles.battle_rating, puzzles.battle_provisional)
            ),
            cx,
        ))
}

fn tile(cx: &App) -> Div {
    v_flex()
        .gap_1()
        .p_3()
        .rounded(cx.theme().radius)
        .border_1()
        .border_color(cx.theme().border)
}

fn big_number(text: String) -> Div {
    div()
        .text_2xl()
        .font_weight(FontWeight::SEMIBOLD)
        .child(text)
}

fn small_line(text: String, cx: &App) -> Div {
    div()
        .text_xs()
        .text_color(cx.theme().muted_foreground)
        .child(text)
}
