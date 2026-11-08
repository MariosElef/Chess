use serde::Deserialize;

/// The server's answer to a finished rush: the best of its kind, and whether this one set it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
pub struct RushRecord {
    pub best: i32,
    pub new_best: bool,
}
