use gpui_kit::*;

use super::keys::{CONTEXT, FirstPosition, LivePosition, NextPosition, PreviousPosition};
use crate::enums::history::NavStep;

/// A page that walks through a game's history; each page decides what reaching a position
/// means for it, and the keys, buttons and move list all come here.
pub trait HistoryNavigation: Sized + 'static {
    fn navigate(&mut self, step: NavStep, cx: &mut Context<Self>);

    /// Shows the position after the move at `index`.
    fn select_move(&mut self, index: usize, cx: &mut Context<Self>);
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

/// For the move list.
pub fn select_move<T: HistoryNavigation>(
    page: WeakEntity<T>,
) -> impl Fn(usize, &mut Window, &mut App) + 'static {
    move |index, _, cx| {
        let _ = page.update(cx, |page, cx| page.select_move(index, cx));
    }
}
