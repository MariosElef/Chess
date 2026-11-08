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
}
