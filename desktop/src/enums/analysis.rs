//! What the analysis engine is doing.

/// What the engine is doing, as the panel's header says it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EngineStatus {
    Off,
    Starting,
    Running {
        depth: u32,
        nps: u64,
    },
    /// The position is over: nothing to search.
    Finished,
    NotFound,
    Stopped,
}
