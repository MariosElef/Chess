use std::{future::Future, rc::Rc};

use gpui_kit::App;

use crate::{
    clients::server::ServerClient,
    models::{
        puzzles::{AttemptReport, Puzzle, PuzzleProblem, PuzzleRating, RatingRange},
        server::ServerError,
    },
};

/// Reads the signed-in session's token, for asking the server for puzzles.
pub type TokenSource = Rc<dyn Fn(&App) -> Option<String>>;

/// A request for a puzzle rated within `range`, or why none can be made.
pub fn request_puzzle(
    client: &ServerClient,
    token: &TokenSource,
    range: RatingRange,
    cx: &App,
) -> Result<impl Future<Output = Result<Puzzle, ServerError>> + use<>, PuzzleProblem> {
    let token = token(cx).ok_or(PuzzleProblem::SignInFirst)?;
    Ok(client.next_puzzle(&token, range))
}

/// A puzzle at the player's own rating, the server picking; none while signed out.
pub fn request_rated_puzzle(
    client: &ServerClient,
    token: &TokenSource,
    cx: &App,
) -> Result<impl Future<Output = Result<Puzzle, ServerError>> + use<>, PuzzleProblem> {
    let token = token(cx).ok_or(PuzzleProblem::SignInFirst)?;
    Ok(client.rated_puzzle(&token))
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
