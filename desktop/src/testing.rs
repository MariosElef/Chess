//! Real mouse input for the page tests: clicks and drags on squares found by their
//! `square-e4` selectors.
use gpui_kit::{Modifiers, MouseButton, Pixels, Point, VisualTestContext, point, px};

/// Far enough to start a drag, so a press is not taken for a click.
const DRAG_NUDGE: Pixels = px(6.);

/// The middle of the element drawn with `selector`, once the window has settled.
pub fn centre_of(cx: &mut VisualTestContext, selector: &'static str) -> Point<Pixels> {
    cx.run_until_parked();
    cx.debug_bounds(selector)
        .expect("the square is drawn")
        .center()
}

pub fn click(cx: &mut VisualTestContext, selector: &'static str) {
    let at = centre_of(cx, selector);
    cx.simulate_click(at, Modifiers::none());
}

/// Presses on `selector` and moves past the drag threshold, leaving the button down.
pub fn lift(cx: &mut VisualTestContext, selector: &'static str) -> Point<Pixels> {
    let start = centre_of(cx, selector);
    let left = MouseButton::Left;
    cx.simulate_mouse_down(start, left, Modifiers::none());
    cx.simulate_mouse_move(
        start + point(DRAG_NUDGE, DRAG_NUDGE),
        Some(left),
        Modifiers::none(),
    );
    cx.run_until_parked();
    start
}

/// Lets go of a lifted piece at `at`.
pub fn release(cx: &mut VisualTestContext, at: Point<Pixels>) {
    cx.simulate_mouse_up(at, MouseButton::Left, Modifiers::none());
    cx.run_until_parked();
}

/// Press on one square, move to the other, release.
pub fn drag(cx: &mut VisualTestContext, from: &'static str, to: &'static str) {
    lift(cx, from);
    let end = centre_of(cx, to);
    cx.simulate_mouse_move(end, Some(MouseButton::Left), Modifiers::none());
    release(cx, end);
}

/// The opponent's scripted answer comes within this.
pub const ANSWER_WAIT: std::time::Duration = std::time::Duration::from_secs(1);

/// A server for the puzzle pages: always Lichess's 5laTS, a mate in three for White.
pub fn puzzle_server() -> crate::clients::server::ServerClient {
    use axum::{
        Json, Router,
        routing::{get, post},
    };
    use serde_json::json;
    crate::clients::server::ServerClient::serving(
        Router::new()
            .route(
                "/api/puzzles/next",
                get(|| async {
                    Json(json!({
                        "id": "5laTS", "rating": 1593, "plays": 2015,
                        "themes": ["mateIn3"],
                        "fen": "7k/pp2r1p1/2pb3r/3p1B1Q/3N4/8/P2q2PP/4R2K w - - 1 1",
                        "last_move": "h7h8",
                        "solution": ["h5e8", "e7e8", "e1e8", "d6f8", "e8f8"]
                    }))
                }),
            )
            .route(
                "/api/puzzles/{id}/attempts",
                post(|| async {
                    Json(json!({ "rating": {
                        "rating": 1512, "deviation": 80, "provisional": false, "change": 12
                    }}))
                }),
            )
            .route(
                "/api/puzzles/rush",
                post(|| async { Json(json!({ "best": 1, "new_best": true })) }),
            )
            .route(
                "/api/puzzles/summary",
                get(|| async {
                    Json(json!({
                        "guest": false,
                        "rating": { "rating": 1500, "deviation": 300, "provisional": true },
                        "best_rush": { "three_minutes": 9, "five_minutes": null, "survival": null },
                        "best_today": {
                            "three_minutes": 4, "five_minutes": null, "survival": null
                        },
                        "battle": { "rating": null, "wins_this_month": 2 }
                    }))
                }),
            ),
    )
}

/// Answers from the server arrive on another thread: lets the app run until `done`.
pub fn wait_until(cx: &mut VisualTestContext, done: impl Fn(&mut VisualTestContext) -> bool) {
    for _ in 0..300 {
        cx.run_until_parked();
        if done(cx) {
            return;
        }
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
    panic!("the server's answer never arrived");
}

/// One of the player's moves by clicking, then the opponent's answer after its pause.
pub fn play_and_wait(cx: &mut VisualTestContext, from: &'static str, to: &'static str) {
    click(cx, from);
    click(cx, to);
    cx.executor().advance_clock(ANSWER_WAIT);
    cx.run_until_parked();
}

/// What an app asked a fake server, in order: each socket message as sent, or a line of its own.
pub type Heard = std::sync::Arc<std::sync::Mutex<Vec<String>>>;

/// A fake server's socket: sends `first` if given, then writes down each message the app sends.
pub async fn record_socket(
    mut socket: axum::extract::ws::WebSocket,
    first: Option<serde_json::Value>,
    heard: Heard,
) {
    use axum::extract::ws::Message;
    if let Some(first) = first
        && socket
            .send(Message::Text(first.to_string().into()))
            .await
            .is_err()
    {
        return;
    }
    while let Some(Ok(Message::Text(text))) = socket.recv().await {
        heard.lock().expect("the list").push(text.to_string());
    }
}

/// Whether the fake server heard `line`: a socket message as "type id", or a line as written.
pub fn was_heard(heard: &Heard, line: &str) -> bool {
    heard
        .lock()
        .expect("the list")
        .iter()
        .any(|said| summary(said) == line)
}

/// The first socket message of type `kind` the fake server heard, whole.
pub fn heard_message(heard: &Heard, kind: &str) -> Option<serde_json::Value> {
    heard
        .lock()
        .expect("the list")
        .iter()
        .filter_map(|said| serde_json::from_str::<serde_json::Value>(said).ok())
        .find(|message| message["type"] == kind)
}

/// A socket message as "type id", by the challenge or game it is about; any other line as is.
fn summary(said: &str) -> String {
    let Ok(message) = serde_json::from_str::<serde_json::Value>(said) else {
        return said.to_owned();
    };
    let kind = message["type"].as_str().unwrap_or_default();
    let id = message["challenge_id"]
        .as_str()
        .or_else(|| message["game_id"].as_str())
        .unwrap_or_default();
    format!("{kind} {id}")
}

/// Ann's session, as an account or as a guest.
pub fn ann(guest: bool) -> crate::models::account::Session {
    use crate::models::account::{Session, User};
    Session {
        token: "abc".to_owned(),
        user: User {
            id: "ann".to_owned(),
            email: "ann@example.com".to_owned(),
            username: "ann".to_owned(),
            guest,
            avatar: None,
        },
    }
}

/// A challenge to Ann from `from`, waiting its full minute: a rematch of `rematch_of` if given.
pub fn challenge_json(id: &str, from: &str, rematch_of: Option<&str>) -> serde_json::Value {
    serde_json::json!({
        "id": id, "from": { "id": from, "username": from },
        "to": { "id": "ann", "username": "ann" },
        "initial_s": 180, "increment_s": 2, "rated": true, "rematch_of": rematch_of,
        "wait_s": 60, "seconds_left": 60
    })
}
