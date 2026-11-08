//! Steps through a game's history.

/// One move through a game's history.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum NavStep {
    Start,
    Back,
    Forward,
    Live,
}

impl NavStep {
    /// A single move back or forward, which holding repeats; Start and Live go all the way.
    pub fn is_single(self) -> bool {
        match self {
            Self::Back | Self::Forward => true,
            Self::Start | Self::Live => false,
        }
    }
}

/// How fast a walk through the history goes: one move at a time, or several when held.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum StepPace {
    Normal,
    Fast,
}
