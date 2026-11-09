/// How one puzzle went in a run or a session: solved or missed, and how hard it was.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PuzzleOutcome {
    pub solved: bool,
    pub rating: u32,
}
