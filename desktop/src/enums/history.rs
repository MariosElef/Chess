//! Steps through a game's history.

/// One move through a game's history.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum NavStep {
    Start,
    Back,
    Forward,
    Live,
}
