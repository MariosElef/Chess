use std::rc::Rc;

use gpui_kit::{
    component::{
        button::{Button, ButtonVariants},
        h_flex,
        label::Label,
        slider::{SliderEvent, SliderState, SliderValue},
        v_flex,
    },
    *,
};

use super::{practice_view::PracticeView, puzzle_request::TokenSource, rush_view::RushView};
use crate::{
    clients::server::ServerClient,
    components::puzzles::PuzzleMenu,
    enums::puzzles::RushKind,
    models::puzzles::{PuzzleSummary, RatingRange},
    services::puzzles::rating_line,
};

/// The difficulty slider moves in steps of this many rating points.
const RANGE_STEP: f32 = 50.;

/// The page shows the menu until a mode is picked.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Stage {
    Menu,
    Practice,
    Rush,
}

/// The Puzzles page: a menu with the puzzle rating, then Practice at a chosen difficulty or
/// Puzzle Rush.
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
}

impl PuzzlesView {
    pub fn new(
        client: ServerClient,
        token: impl Fn(&App) -> Option<String> + 'static,
        cx: &mut Context<Self>,
    ) -> Self {
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
        }
    }

    /// The page came on screen: the menu's rating and best scores are brought up to date.
    pub fn opened(&mut self, cx: &mut Context<Self>) {
        if self.stage == Stage::Menu {
            self.refresh_summary(cx);
        }
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

    /// Back to the menu; a rush still going ends there and its score counts.
    fn back_to_menu(&mut self, cx: &mut Context<Self>) {
        self.rush.update(cx, |rush, cx| rush.leave(cx));
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
        let (practice_view, rush_view) = (cx.entity().downgrade(), cx.entity().downgrade());
        PuzzleMenu::new(
            rating_line(&self.summary),
            (self.slider.clone(), self.range),
            self.summary.best_rush,
        )
        .on_practice(move |_, cx| {
            let _ = practice_view.update(cx, |view, cx| view.open_practice(cx));
        })
        .on_rush(move |kind, _, cx| {
            let _ = rush_view.update(cx, |view, cx| view.open_rush(kind, cx));
        })
    }

    fn content(&self, cx: &mut Context<Self>) -> AnyElement {
        match self.stage {
            Stage::Menu => div()
                .flex()
                .justify_center()
                .child(self.menu(cx))
                .into_any_element(),
            Stage::Practice => self.practice.clone().into_any_element(),
            Stage::Rush => self.rush.clone().into_any_element(),
        }
    }

    fn header(&self, cx: &mut Context<Self>) -> Div {
        let back = (self.stage != Stage::Menu).then(|| {
            Button::new("puzzles-menu")
                .ghost()
                .label("← Puzzles")
                .on_click(cx.listener(|view, _, _, cx| view.back_to_menu(cx)))
        });
        h_flex()
            .gap_4()
            .items_center()
            .child(Label::new("Puzzles").text_2xl())
            .children(back)
    }
}

impl Render for PuzzlesView {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .size_full()
            .p_6()
            .gap_4()
            .child(self.header(cx))
            .child(div().flex_1().min_h_0().child(self.content(cx)))
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
    use gpui_kit::{TestAppContext, VisualTestContext};

    use super::{PuzzlesView, Stage};
    use crate::testing::{puzzle_server, wait_until};

    fn stage(view: &gpui_kit::Entity<PuzzlesView>, cx: &mut VisualTestContext) -> Stage {
        view.update(cx, |view, _| view.stage)
    }

    #[gpui_kit::test]
    fn the_menu_shows_the_summary_and_leads_to_practice_and_back(cx: &mut TestAppContext) {
        cx.executor().allow_parking();
        cx.update(gpui_kit::init);
        let (view, cx) = cx.add_window_view(|_, cx| {
            PuzzlesView::new(puzzle_server(), |_| Some("abc".to_owned()), cx)
        });
        view.update(cx, |view, cx| view.opened(cx));
        wait_until(cx, |cx| {
            view.update(cx, |view, _| view.summary.rating.is_some())
        });
        let best = view.update(cx, |view, _| view.summary.best_rush.three_minutes);
        assert_eq!(best, Some(9));
        assert_eq!(stage(&view, cx), Stage::Menu);

        view.update(cx, |view, cx| view.open_practice(cx));
        assert_eq!(stage(&view, cx), Stage::Practice);
        let shown = view.update(cx, |view, cx| view.practice.read(cx).player_rating());
        assert_eq!(
            shown.map(|rating| rating.rating),
            Some(1500),
            "the menu's rating shows before the first attempt"
        );
        view.update(cx, |view, cx| view.back_to_menu(cx));
        assert_eq!(stage(&view, cx), Stage::Menu);
    }
}
