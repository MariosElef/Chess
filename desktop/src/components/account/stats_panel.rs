use gpui_kit::{
    component::{ActiveTheme as _, h_flex, v_flex},
    *,
};

use super::rating_card::RatingCard;
use crate::{
    components::{play::speed_picture, shell::PUZZLES_PICTURE},
    models::account::{ModeStats, PuzzleStats, Stats},
    services::account::{category_of, rating_text, solved_share},
};

/// The player's ratings in every mode and in puzzles, each on a card with its graph.
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
        let speeds: Vec<RatingCard> = self.stats.modes.into_iter().map(mode_card).collect();
        v_flex()
            .w_full()
            .max_w(px(1072.))
            .gap_3()
            .child(
                div()
                    .text_lg()
                    .font_weight(FontWeight::BOLD)
                    .text_color(cx.theme().foreground)
                    .child("Stats"),
            )
            // One even row of the speeds, every card as tall as the tallest, then puzzles across.
            .child(h_flex().items_stretch().gap_3().children(speeds))
            .child(puzzle_card(self.stats.puzzles).wide())
    }
}

fn mode_card(mode: ModeStats) -> RatingCard {
    let category = category_of(&mode.category);
    let name = category.map_or_else(
        || mode.category.clone(),
        |category| category.name().to_owned(),
    );
    let picture = category.and_then(speed_picture);
    let record = format!(
        "{} games · {} won · {} lost · {} drawn",
        mode.games, mode.wins, mode.losses, mode.draws
    );
    let card = RatingCard::new(
        (picture, name),
        rating_text(mode.rating, mode.provisional),
        mode.history,
    );
    best_line(card, mode.best).line(record)
}

fn puzzle_card(puzzles: PuzzleStats) -> RatingCard {
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
    let card = RatingCard::new(
        (Some(PUZZLES_PICTURE), "Puzzles"),
        rating_text(puzzles.rating, puzzles.provisional),
        puzzles.history,
    );
    best_line(card, puzzles.best)
        .line(solved)
        .line(format!("Rush: {best_rush}"))
        .line(format!(
            "Battle rating {}",
            rating_text(puzzles.battle_rating, puzzles.battle_provisional)
        ))
}

/// "Best 1630", once there is one.
fn best_line(card: RatingCard, best: Option<i32>) -> RatingCard {
    match best {
        Some(best) => card.line(format!("Best {best}")),
        None => card,
    }
}
