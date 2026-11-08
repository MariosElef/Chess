use std::time::Duration;

use gpui_kit::*;

/// How often a running game clock is redrawn and checked, on Play and online.
pub const CLOCK_TICK: Duration = Duration::from_millis(100);

/// Wakes the page every `every` for as long as `tick` says to go on, or until the page is gone.
pub fn start_ticker<T: 'static>(
    every: Duration,
    cx: &mut Context<T>,
    tick: fn(&mut T, &mut Context<T>) -> bool,
) -> Task<()> {
    cx.spawn(async move |page, cx| {
        loop {
            cx.background_executor().timer(every).await;
            let keep_going = page.update(cx, tick).unwrap_or(false);
            if !keep_going {
                break;
            }
        }
    })
}
