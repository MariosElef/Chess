use gpui_kit::*;

use std::time::{Duration, Instant};

use chess_domain::GameStatus;

use super::keys::{CONTEXT, FirstPosition, LivePosition, NextPosition, PreviousPosition};
use crate::{
    enums::{
        history::{NavStep, StepPace},
        sound::Sound,
    },
    services::history::{GameHistory, StepStreak},
    views::sound::play_move_sound,
};

/// How long a navigation button is held before it starts repeating, and how often it repeats:
/// as a held key does.
const REPEAT_AFTER: Duration = Duration::from_millis(400);
const REPEAT_EVERY: Duration = Duration::from_millis(60);
/// Moves taken per step once holding goes fast.
const FAST_MOVES: usize = 3;

/// A page's holding of a key or navigation button: the steps so far, and the repeat a held
/// button keeps going.
#[derive(Default)]
pub struct Holding {
    streak: StepStreak,
    repeat: Option<Task<()>>,
}

/// A page that walks through a game's history. The keys, buttons and move list all come here:
/// the history moves, the move that led to the position reached makes its sound, and the page
/// then does what reaching a position means for it. Held for two seconds, a key or button goes
/// fast: several moves a step, without sounds or slides.
pub trait HistoryNavigation: Sized + 'static {
    /// The history being walked; `None` while the page has none.
    fn history_mut(&mut self) -> Option<&mut GameHistory>;

    fn holding(&mut self) -> &mut Holding;

    /// The board shows another position.
    fn after_history_move(&mut self, cx: &mut Context<Self>);

    fn navigate(&mut self, step: NavStep, cx: &mut Context<Self>) {
        let pace = self.holding().streak.step(Instant::now());
        let moves = match pace {
            StepPace::Fast if step.is_single() => FAST_MOVES,
            StepPace::Fast | StepPace::Normal => 1,
        };
        let with_sound = pace == StepPace::Normal;
        self.walk(
            |history| (0..moves).for_each(|_| history.step(step)),
            with_sound,
            cx,
        );
    }

    /// Shows the position after the move at `index`.
    fn select_move(&mut self, index: usize, cx: &mut Context<Self>) {
        self.walk(|history| history.jump_to_move(index), true, cx);
    }

    fn walk(
        &mut self,
        change: impl FnOnce(&mut GameHistory),
        with_sound: bool,
        cx: &mut Context<Self>,
    ) {
        let Some(history) = self.history_mut() else {
            return;
        };
        let before = history.viewing();
        change(history);
        if with_sound && history.viewing() != before {
            sound_of_shown_move(history, cx);
        }
        self.after_history_move(cx);
    }

    /// A navigation button pressed: after a moment it repeats until let go.
    fn start_holding(&mut self, step: NavStep, cx: &mut Context<Self>) {
        let repeat = cx.spawn(async move |page, cx| {
            cx.background_executor().timer(REPEAT_AFTER).await;
            loop {
                if page.update(cx, |page, cx| page.navigate(step, cx)).is_err() {
                    break;
                }
                cx.background_executor().timer(REPEAT_EVERY).await;
            }
        });
        self.holding().repeat = Some(repeat);
    }

    fn stop_holding(&mut self) {
        self.holding().repeat = None;
    }
}

/// The sound of the move that led to the shown position, as if it were played now: a capture,
/// a castle, a check, and the end when it ended the game. The start makes none.
fn sound_of_shown_move(history: &GameHistory, cx: &App) {
    let Some(index) = history.viewing().checked_sub(1) else {
        return;
    };
    let game = history.game();
    let (Some(&mv), Some(before), Some(after)) = (
        game.moves().get(index),
        game.positions().get(index),
        game.positions().get(index + 1),
    ) else {
        return;
    };
    let ends_game = after.status() != GameStatus::Ongoing;
    play_move_sound(before, after, mv, Sound::MoveSelf, ends_game, cx);
}

/// The arrow keys move through the history while `element` or anything in it has focus.
pub fn with_history_keys<T: HistoryNavigation>(element: Div, cx: &mut Context<T>) -> Div {
    element
        .key_context(CONTEXT)
        .on_action(cx.listener(|page: &mut T, _: &PreviousPosition, _, cx| {
            page.navigate(NavStep::Back, cx);
        }))
        .on_action(cx.listener(|page: &mut T, _: &NextPosition, _, cx| {
            page.navigate(NavStep::Forward, cx);
        }))
        .on_action(cx.listener(|page: &mut T, _: &FirstPosition, _, cx| {
            page.navigate(NavStep::Start, cx);
        }))
        .on_action(cx.listener(|page: &mut T, _: &LivePosition, _, cx| {
            page.navigate(NavStep::Live, cx);
        }))
}

// The page outlives what it renders; a failed update means the window is closing.

/// For the navigation buttons.
pub fn step_history<T: HistoryNavigation>(
    page: WeakEntity<T>,
) -> impl Fn(NavStep, &mut Window, &mut App) + 'static {
    move |step, _, cx| {
        let _ = page.update(cx, |page, cx| page.navigate(step, cx));
    }
}

/// For holding a navigation button: `Some` when pressed, `None` when let go.
pub fn hold_step<T: HistoryNavigation>(
    page: WeakEntity<T>,
) -> impl Fn(Option<NavStep>, &mut Window, &mut App) + 'static {
    move |held, _, cx| {
        let _ = page.update(cx, |page, cx| match held {
            Some(step) => page.start_holding(step, cx),
            None => page.stop_holding(),
        });
    }
}

/// For the move list.
pub fn select_move<T: HistoryNavigation>(
    page: WeakEntity<T>,
) -> impl Fn(usize, &mut Window, &mut App) + 'static {
    move |index, _, cx| {
        let _ = page.update(cx, |page, cx| page.select_move(index, cx));
    }
}
