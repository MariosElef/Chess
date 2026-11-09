use std::rc::Rc;

use gpui_kit::{
    assets::IconName,
    component::{
        ActiveTheme as _,
        button::{Button, ButtonVariants},
        h_flex, v_flex,
    },
    prelude::FluentBuilder as _,
    *,
};

use super::{EmptyState, avatar};
use crate::{
    enums::social::RankingBoard,
    models::{account::Face, social::RankedPlayer},
    services::account::rating_text,
};

type OnBoard = Rc<dyn Fn(RankingBoard, &mut Window, &mut App)>;
type OnMore = Rc<dyn Fn(&mut Window, &mut App)>;

/// The gold, silver and bronze of the first three places.
const PODIUM_COLOURS: [u32; 3] = [0x00e3_aa24, 0x00b3_aca8, 0x00b3_7314];

/// A leaderboard: a tab per board, the player's own place, then everyone's, best first.
#[derive(IntoElement)]
pub struct RankingPanel {
    board: RankingBoard,
    rows: Vec<(RankedPlayer, Face)>,
    me: Option<(RankedPlayer, Face)>,
    /// More places come after these.
    more: bool,
    /// A page is on its way.
    loading: bool,
    problem: Option<String>,
    on_board: OnBoard,
    on_more: OnMore,
}

impl RankingPanel {
    pub fn new(
        board: RankingBoard,
        on_board: impl Fn(RankingBoard, &mut Window, &mut App) + 'static,
        on_more: impl Fn(&mut Window, &mut App) + 'static,
    ) -> Self {
        Self {
            board,
            rows: Vec::new(),
            me: None,
            more: false,
            loading: false,
            problem: None,
            on_board: Rc::new(on_board),
            on_more: Rc::new(on_more),
        }
    }

    /// The places shown, each with how its player looks.
    pub fn rows(mut self, rows: Vec<(RankedPlayer, Face)>) -> Self {
        self.rows = rows;
        self
    }

    /// The player's own place, pinned above the rest; none off the board, as a guest.
    pub fn me(mut self, me: Option<(RankedPlayer, Face)>) -> Self {
        self.me = me;
        self
    }

    pub fn more(mut self, more: bool) -> Self {
        self.more = more;
        self
    }

    pub fn loading(mut self, loading: bool) -> Self {
        self.loading = loading;
        self
    }

    pub fn problem(mut self, problem: Option<String>) -> Self {
        self.problem = problem;
        self
    }
}

impl RenderOnce for RankingPanel {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = cx.theme();
        let tabs: Vec<Stateful<Div>> = RankingBoard::ALL
            .into_iter()
            .map(|board| board_tab(board, (board == self.board, Rc::clone(&self.on_board)), cx))
            .collect();
        let my_id = self.me.as_ref().map(|(me, _)| me.id.clone());
        let noun = self.board.played_noun();
        let first_load = self.loading && self.rows.is_empty();
        let empty = !self.loading && self.rows.is_empty() && self.problem.is_none();
        let rows: Vec<Div> = self
            .rows
            .iter()
            .map(|(player, face)| {
                let mine = my_id.as_deref() == Some(player.id.as_str());
                place_row((player, face), (mine, noun), cx)
            })
            .collect();
        let on_more = Rc::clone(&self.on_more);
        v_flex()
            .w_full()
            .gap_4()
            .child(
                h_flex()
                    .gap_6()
                    .border_b_1()
                    .border_color(theme.border)
                    .children(tabs),
            )
            .when_some(self.me.as_ref(), |this, (me, face)| {
                this.child(
                    v_flex()
                        .gap_1()
                        .child(
                            div()
                                .text_sm()
                                .text_color(theme.muted_foreground)
                                .child("Your place"),
                        )
                        .child(place_row((me, face), (true, noun), cx)),
                )
            })
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
                    .when(first_load, |this| {
                        this.child(
                            div()
                                .p_3()
                                .text_sm()
                                .text_color(theme.muted_foreground)
                                .child("Loading…"),
                        )
                    })
                    .when(empty, |this| {
                        this.child(EmptyState::new(
                            IconName::Trophy,
                            "No one here yet",
                            "Players show up once they have a rated result on this board.",
                        ))
                    })
                    .children(rows)
                    .when(self.more, |this| {
                        this.child(
                            Button::new("leaderboard-more")
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

/// A board's name along the top, underlined when chosen.
fn board_tab(board: RankingBoard, (chosen, on_board): (bool, OnBoard), cx: &App) -> Stateful<Div> {
    let theme = cx.theme();
    let (bright, muted) = (theme.foreground, theme.muted_foreground);
    let id = format!("leaderboard-tab-{}", board.key());
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
        .child(board.label())
        .on_click(move |_, window, cx| on_board(board, window, cx))
}

/// One place: its number, the player's picture and name, their rating and how much they played.
fn place_row(
    (player, face): (&RankedPlayer, &Face),
    (mine, noun): (bool, &'static str),
    cx: &App,
) -> Div {
    let theme = cx.theme();
    h_flex()
        .w_full()
        .gap_3()
        .px_3()
        .py_2()
        .rounded(px(6.))
        .when(mine, |this| this.bg(theme.background))
        .child(
            div()
                .w(px(36.))
                .flex_shrink_0()
                .text_right()
                .font_weight(FontWeight::BOLD)
                .text_color(place_colour(player.place, theme.muted_foreground))
                .child(format!("#{}", player.place)),
        )
        .child(avatar(&face.name, face.picture.as_deref(), px(32.), cx))
        .child(
            div()
                .flex_1()
                .min_w_0()
                .font_weight(FontWeight::SEMIBOLD)
                .truncate()
                .child(player.username.clone()),
        )
        .child(
            div()
                .text_sm()
                .text_color(theme.muted_foreground)
                .child(format!("{} {noun}", player.played)),
        )
        .child(
            div()
                .w(px(64.))
                .flex_shrink_0()
                .text_right()
                .font_weight(FontWeight::BOLD)
                .child(rating_text(Some(player.rating), player.provisional)),
        )
}

/// The first three places in gold, silver and bronze; the rest muted.
fn place_colour(place: i64, muted: Hsla) -> Hsla {
    usize::try_from(place - 1)
        .ok()
        .and_then(|index| PODIUM_COLOURS.get(index))
        .map_or(muted, |colour| rgb(*colour).into())
}

#[cfg(test)]
mod tests {
    use gpui_kit::{Hsla, black};

    use super::{PODIUM_COLOURS, place_colour};

    #[test]
    fn the_first_three_places_are_gold_silver_and_bronze() {
        let muted: Hsla = black();
        for (place, colour) in (1..=3).zip(PODIUM_COLOURS) {
            assert_eq!(place_colour(place, muted), gpui_kit::rgb(colour).into());
        }
        assert_eq!(place_colour(4, muted), muted, "the rest are muted");
    }
}
