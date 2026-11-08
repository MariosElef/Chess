use std::rc::Rc;

use gpui_kit::{
    component::slider::{SliderEvent, SliderState, SliderValue},
    *,
};

use super::{
    battle_view::{BattleBegan, BattleEnded, BattleView},
    practice_view::PracticeView,
    puzzle_request::TokenSource,
    rush_view::{RushSaved, RushView},
};
use crate::{
    clients::server::ServerClient,
    components::{
        board::{BoardColumn, board_page},
        panel::SidePanel,
        puzzles::{PuzzleMenu, RushMenu},
    },
    enums::puzzles::RushKind,
    models::{
        board::ShownPosition,
        puzzles::{PuzzleSummary, RatingRange},
    },
    services::puzzles::rating_line,
    views::{
        board::BoardView,
        live::{LiveConnection, SessionSource},
    },
};

/// The difficulty slider moves in steps of this many rating points.
const RANGE_STEP: f32 = 50.;

/// The page shows the menu until a mode is picked.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Stage {
    Menu,
    Practice,
    /// Puzzle Rush's panel: its scores and kinds, before a rush starts.
    RushMenu,
    Rush,
    /// Puzzle Battle's panel, or a battle being played.
    Battle,
}

/// The Puzzles page: a menu with the puzzle rating, then Practice at a chosen difficulty,
/// Puzzle Rush or Puzzle Battle.
pub struct PuzzlesView {
    stage: Stage,
    client: ServerClient,
    token: TokenSource,
    summary: PuzzleSummary,
    /// The difficulty, chosen in the menu and still adjustable while practising.
    range: RatingRange,
    slider: Entity<SliderState>,
    practice: Entity<PracticeView>,
    rush: Entity<RushView>,
    _slider_changes: Subscription,
    /// The board beside the menu, waiting on the starting position.
    board: Entity<BoardView>,
    /// The kind of Puzzle Rush picked in its panel.
    rush_pick: RushKind,
    /// Puzzle Battle: its panel, and the battle itself.
    battle: Entity<BattleView>,
    _battles: Subscription,
    /// Its invitations change the rail's count, so the page redraws when their number does.
    _battle_changes: Subscription,
    invitations_seen: usize,
    /// A battle over moves the wins and rating its panel shows.
    _battle_ends: Subscription,
    /// A rush's score saved moves the bests its panel shows.
    _rush_saves: Subscription,
}

impl PuzzlesView {
    pub fn new(
        client: ServerClient,
        token: impl Fn(&App) -> Option<String> + 'static,
        (live, session): (Entity<LiveConnection>, SessionSource),
        cx: &mut Context<Self>,
    ) -> Self {
        let battle = cx.new(|cx| BattleView::new(live, session, cx));
        let _battles = cx.subscribe(&battle, |view, _, BattleBegan, cx| {
            // A rush going on elsewhere on the page ends, so its score is the one it had, and a
            // practice puzzle stops redrawing its timer.
            view.rush.update(cx, |rush, cx| rush.leave(cx));
            view.practice.update(cx, |practice, cx| practice.leave(cx));
            view.stage = Stage::Battle;
            cx.notify();
        });
        // The battle redraws itself often while it runs; the page only for its invitations.
        let _battle_changes = cx.observe(&battle, |view: &mut Self, battle, cx| {
            let invitations = battle.read(cx).invitations();
            if invitations != view.invitations_seen {
                view.invitations_seen = invitations;
                cx.notify();
            }
        });
        let _battle_ends = cx.subscribe(&battle, |view, _, BattleEnded, cx| {
            view.refresh_summary(cx);
        });
        let from_battle = cx.weak_entity();
        battle.update(cx, |battle, _| {
            battle.on_back(move |_, cx| {
                let _ = from_battle.update(cx, |view, cx| view.back_to_menu(cx));
            });
        });
        let token: TokenSource = Rc::new(token);
        let range = RatingRange::DEFAULT;
        let slider = cx.new(|_| new_slider(range));
        let _slider_changes = cx.subscribe(&slider, |view, _, event: &SliderEvent, cx| {
            view.slider_moved(event, cx);
        });
        let practice = cx.new(|cx| {
            PracticeView::new(
                client.clone(),
                Rc::clone(&token),
                (slider.clone(), range),
                cx,
            )
        });
        let rush = cx.new(|cx| RushView::new(client.clone(), Rc::clone(&token), cx));
        let _rush_saves = cx.subscribe(&rush, |view, _, RushSaved, cx| view.refresh_summary(cx));
        let board = cx.new(|cx| BoardView::new(|_| Some(ShownPosition::waiting()), cx));
        let (from_practice, from_rush) = (cx.weak_entity(), cx.weak_entity());
        practice.update(cx, |practice, _| {
            practice.on_back(move |_, cx| {
                let _ = from_practice.update(cx, |view, cx| view.back_to_menu(cx));
            });
        });
        rush.update(cx, |rush, _| {
            rush.on_back(move |_, cx| {
                let _ = from_rush.update(cx, |view, cx| view.back_to_rush_menu(cx));
            });
        });
        Self {
            stage: Stage::Menu,
            client,
            token,
            summary: PuzzleSummary::default(),
            range,
            slider,
            practice,
            rush,
            _slider_changes,
            board,
            rush_pick: RushKind::ThreeMinutes,
            battle,
            _battles,
            _battle_changes,
            invitations_seen: 0,
            _battle_ends,
            _rush_saves,
        }
    }

    /// The page came on screen: the menu's rating and best scores are brought up to date.
    pub fn opened(&mut self, cx: &mut Context<Self>) {
        match self.stage {
            Stage::Menu | Stage::RushMenu => self.refresh_summary(cx),
            Stage::Battle => {
                self.refresh_summary(cx);
                self.battle.update(cx, |battle, cx| battle.opened(cx));
            }
            Stage::Practice | Stage::Rush => {}
        }
    }

    /// Battle invitations waiting for the player, for the rail's count.
    pub fn invitations(&self, cx: &App) -> usize {
        self.battle.read(cx).invitations()
    }

    fn refresh_summary(&mut self, cx: &mut Context<Self>) {
        let Some(token) = (self.token)(cx) else {
            return;
        };
        let request = self.client.puzzle_summary(&token);
        cx.spawn(async move |view, cx| {
            if let Ok(summary) = request.await {
                // The page outlives the request unless the window is closing.
                let _ = view.update(cx, |view, cx| {
                    view.summary = summary;
                    view.battle
                        .update(cx, |battle, cx| battle.set_summary(summary.battle, cx));
                    cx.notify();
                });
            }
        })
        .detach();
    }

    fn open_practice(&mut self, cx: &mut Context<Self>) {
        self.stage = Stage::Practice;
        let (range, rating) = (self.range, self.summary.rating);
        self.practice
            .update(cx, |practice, cx| practice.start(range, rating, cx));
        cx.notify();
    }

    fn open_rush(&mut self, kind: RushKind, cx: &mut Context<Self>) {
        self.stage = Stage::Rush;
        self.rush.update(cx, |rush, cx| rush.start(kind, cx));
        cx.notify();
    }

    /// From a rush back to its panel, with its scores fetched again; a rush going on ends.
    fn back_to_rush_menu(&mut self, cx: &mut Context<Self>) {
        self.rush.update(cx, |rush, cx| rush.leave(cx));
        self.stage = Stage::RushMenu;
        self.refresh_summary(cx);
        cx.notify();
    }

    /// Back to the menu: a rush still going ends and its score counts, Practice's clock stops,
    /// and a search for a battle opponent is called off.
    fn back_to_menu(&mut self, cx: &mut Context<Self>) {
        self.rush.update(cx, |rush, cx| rush.leave(cx));
        self.practice.update(cx, |practice, cx| practice.leave(cx));
        self.battle.update(cx, |battle, cx| battle.closing(cx));
        self.stage = Stage::Menu;
        self.refresh_summary(cx);
        cx.notify();
    }

    fn slider_moved(&mut self, event: &SliderEvent, cx: &mut Context<Self>) {
        let (SliderEvent::Change(value) | SliderEvent::Release(value)) = event;
        self.range = range_of(*value);
        let released = matches!(event, SliderEvent::Release(_));
        if self.stage == Stage::Practice {
            let range = self.range;
            self.practice.update(cx, |practice, cx| {
                practice.range_changed(range, released, cx);
            });
        }
        cx.notify();
    }

    fn menu(&self, cx: &mut Context<Self>) -> PuzzleMenu {
        let page = cx.entity().downgrade();
        let (practice_view, rush_view, battle_view) = (page.clone(), page.clone(), page);
        PuzzleMenu::new(
            rating_line(&self.summary),
            (self.slider.clone(), self.range),
        )
        .on_practice(move |_, cx| {
            let _ = practice_view.update(cx, |view, cx| view.open_practice(cx));
        })
        .on_rush(move |_, cx| {
            let _ = rush_view.update(cx, |view, cx| {
                view.stage = Stage::RushMenu;
                cx.notify();
            });
        })
        .on_battle(move |_, cx| {
            let _ = battle_view.update(cx, |view, cx| view.open_battle(cx));
        })
    }

    fn open_battle(&mut self, cx: &mut Context<Self>) {
        self.stage = Stage::Battle;
        self.battle.update(cx, |battle, cx| battle.opened(cx));
        cx.notify();
    }

    fn rush_menu(&self, cx: &mut Context<Self>) -> RushMenu {
        let page = cx.entity().downgrade();
        let (back, pick, play) = (page.clone(), page.clone(), page);
        RushMenu::new(
            self.rush_pick,
            (self.summary.best_rush, self.summary.best_today),
            move |_, cx| {
                let _ = back.update(cx, |view, cx| view.back_to_menu(cx));
            },
            move |kind, _, cx| {
                let _ = pick.update(cx, |view, cx| {
                    view.rush_pick = kind;
                    cx.notify();
                });
            },
            move |_, cx| {
                let _ = play.update(cx, |view, cx| view.open_rush(view.rush_pick, cx));
            },
        )
    }

    fn content(&self, cx: &mut Context<Self>) -> AnyElement {
        match self.stage {
            Stage::Menu => {
                let board = BoardColumn::new(self.board.clone());
                board_page(board, SidePanel::new().child(self.menu(cx))).into_any_element()
            }
            Stage::RushMenu => {
                let board = BoardColumn::new(self.board.clone());
                board_page(board, SidePanel::new().child(self.rush_menu(cx))).into_any_element()
            }
            Stage::Practice => self.practice.clone().into_any_element(),
            Stage::Rush => self.rush.clone().into_any_element(),
            Stage::Battle => self.battle.clone().into_any_element(),
        }
    }
}

impl Render for PuzzlesView {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div().size_full().child(self.content(cx))
    }
}

/// The two-handled difficulty slider, from the lowest to the highest rating served.
fn new_slider(range: RatingRange) -> SliderState {
    // The maximum first: a minimum above the default maximum of 100 panics.
    SliderState::new()
        .max(slider_point(RatingRange::HIGHEST))
        .min(slider_point(RatingRange::LOWEST))
        .step(RANGE_STEP)
        .default_value((slider_point(range.min), slider_point(range.max)))
}

/// A rating as a point on the slider.
fn slider_point(rating: i32) -> f32 {
    f32::from(i16::try_from(rating).unwrap_or(i16::MAX))
}

/// The slider's two handles as a rating range.
fn range_of(value: SliderValue) -> RatingRange {
    #[expect(
        clippy::cast_possible_truncation,
        reason = "the slider's points are whole ratings between 400 and 3200"
    )]
    let rating = |point: f32| point.round() as i32;
    RatingRange {
        min: rating(value.start()),
        max: rating(value.end()),
    }
}

#[cfg(test)]
mod tests {
    use std::rc::Rc;

    use gpui_kit::{AppContext as _, TestAppContext, VisualTestContext};

    use super::{PuzzlesView, Stage};
    use crate::{
        enums::puzzles::RushKind,
        testing::{click, puzzle_server, wait_until},
        views::live::{LiveConnection, SessionSource},
    };

    fn stage(view: &gpui_kit::Entity<PuzzlesView>, cx: &mut VisualTestContext) -> Stage {
        view.update(cx, |view, _| view.stage)
    }

    #[gpui_kit::test]
    fn the_menu_shows_the_summary_and_leads_to_practice_and_back(cx: &mut TestAppContext) {
        cx.executor().allow_parking();
        cx.update(gpui_kit::init);
        let (view, cx) = cx.add_window_view(|_, cx| {
            let client = puzzle_server();
            let session: SessionSource = Rc::new(|_| None);
            let live = cx.new(|_| LiveConnection::new(client.clone(), Rc::clone(&session)));
            PuzzlesView::new(client, |_| Some("abc".to_owned()), (live, session), cx)
        });
        view.update(cx, |view, cx| view.opened(cx));
        wait_until(cx, |cx| {
            view.update(cx, |view, _| view.summary.rating.is_some())
        });
        let best = view.update(cx, |view, _| view.summary.best_rush.three_minutes);
        assert_eq!(best, Some(9));
        assert_eq!(stage(&view, cx), Stage::Menu);

        click(cx, "start-practice");
        assert_eq!(stage(&view, cx), Stage::Practice);
        let shown = view.update(cx, |view, cx| view.practice.read(cx).player_rating());
        assert_eq!(
            shown.map(|rating| rating.rating),
            Some(1500),
            "the menu's rating shows before the first attempt"
        );
        view.update(cx, |view, cx| view.back_to_menu(cx));
        assert_eq!(stage(&view, cx), Stage::Menu);

        click(cx, "open-rush");
        assert_eq!(stage(&view, cx), Stage::RushMenu);
        click(cx, "rush-survival");
        assert_eq!(
            view.update(cx, |view, _| view.rush_pick),
            RushKind::Survival
        );
        click(cx, "rush-play");
        assert_eq!(stage(&view, cx), Stage::Rush);
    }
}
