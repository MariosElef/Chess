use serde::Serialize;

/// How a puzzle went, for the server: solved cleanly or not, if it is rated, and how long.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct AttemptReport {
    pub solved: bool,
    pub rated: bool,
    pub duration_ms: i32,
}
