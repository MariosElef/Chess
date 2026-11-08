use serde::Serialize;

/// How a puzzle went, for the server: solved without a mistake or not, whether it counts for
/// the rating, and how long it took.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct AttemptReport {
    pub solved: bool,
    pub rated: bool,
    pub duration_ms: i32,
}
