//! The game's sounds, played from the set Settings picks: Standard or Lichess's Lisp.

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Sound {
    /// A plain move by the player, or by White on the Play board.
    MoveSelf,
    /// A plain move by the opponent, or by Black on the Play board.
    MoveOpponent,
    Capture,
    Castle,
    Check,
    Promote,
    /// A premove was queued.
    Premove,
    /// A piece was dropped where it cannot go.
    Illegal,
    GameStart,
    GameEnd,
    /// A draw was offered.
    Notify,
    /// A clock is down to the last 5% of its starting time.
    LowTime,
    /// A puzzle was solved.
    PuzzleCorrect,
}

impl Sound {
    /// Every sound, in the order Settings plays them to show a set off.
    pub const ALL: [Self; 13] = [
        Self::GameStart,
        Self::MoveSelf,
        Self::MoveOpponent,
        Self::Capture,
        Self::Castle,
        Self::Check,
        Self::Promote,
        Self::Premove,
        Self::Illegal,
        Self::Notify,
        Self::LowTime,
        Self::PuzzleCorrect,
        Self::GameEnd,
    ];
}
