use std::{future::Future, rc::Rc};

use gpui_kit::App;

use crate::{
    clients::server::ServerClient,
    models::{
        puzzles::{AttemptReport, Puzzle, PuzzleRating, RatingRange},
        server::ServerError,
    },
};

/// Reads the signed-in session's token, for asking the server for puzzles.
pub type TokenSource = Rc<dyn Fn(&App) -> Option<String>>;

/// What the page says when nobody is signed in.
pub const SIGN_IN_FIRST: &str = "Sign in to solve puzzles.";

/// A request for a puzzle rated within `range`, or why none can be made.
pub fn request_puzzle(
    client: &ServerClient,
    token: &TokenSource,
    range: RatingRange,
    cx: &App,
) -> Result<impl Future<Output = Result<Puzzle, ServerError>> + use<>, &'static str> {
    let token = token(cx).ok_or(SIGN_IN_FIRST)?;
    Ok(client.next_puzzle(&token, range))
}

/// Tells the server how a puzzle went; nothing is sent while signed out.
pub fn report_attempt(
    client: &ServerClient,
    token: &TokenSource,
    puzzle_id: &str,
    report: AttemptReport,
    cx: &App,
) -> Option<impl Future<Output = Result<Option<PuzzleRating>, ServerError>> + use<>> {
    let token = token(cx)?;
    Some(client.record_attempt(&token, puzzle_id, report))
}
