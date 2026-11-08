//! What the analysis engine is doing, and what a game review calls each move.

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

/// What a game review calls a move, after chess.com's Game Review; see
/// `services::analysis::review` for the rules.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MoveClass {
    Brilliant,
    Great,
    Best,
    Excellent,
    Good,
    Book,
    Forced,
    Inaccuracy,
    Mistake,
    Miss,
    Blunder,
}

impl MoveClass {
    /// Best first, as the review's counts list them.
    pub const ALL: [Self; 11] = [
        Self::Brilliant,
        Self::Great,
        Self::Best,
        Self::Excellent,
        Self::Good,
        Self::Book,
        Self::Forced,
        Self::Inaccuracy,
        Self::Mistake,
        Self::Miss,
        Self::Blunder,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Self::Brilliant => "Brilliant",
            Self::Great => "Great",
            Self::Best => "Best",
            Self::Excellent => "Excellent",
            Self::Good => "Good",
            Self::Book => "Book",
            Self::Forced => "Forced",
            Self::Inaccuracy => "Inaccuracy",
            Self::Mistake => "Mistake",
            Self::Miss => "Miss",
            Self::Blunder => "Blunder",
        }
    }

    /// What the move is, after "Nf3 is": "a blunder", "the best move", "brilliant".
    pub fn verdict(self) -> &'static str {
        match self {
            Self::Brilliant => "brilliant",
            Self::Great => "a great move",
            Self::Best => "the best move",
            Self::Excellent => "excellent",
            Self::Good => "good",
            Self::Book => "a book move",
            Self::Forced => "forced",
            Self::Inaccuracy => "an inaccuracy",
            Self::Mistake => "a mistake",
            Self::Miss => "a miss",
            Self::Blunder => "a blunder",
        }
    }

    /// The icon's colour, `0xRRGGBB`, which the move's squares take on the board.
    pub fn color(self) -> u32 {
        match self {
            Self::Brilliant => 0x26_c2_a3,
            Self::Great => 0x74_9b_bf,
            Self::Best | Self::Excellent | Self::Forced => 0x81_b6_4c,
            Self::Good => 0x95_b7_76,
            Self::Book => 0xd5_a4_7d,
            Self::Inaccuracy => 0xf7_c6_31,
            Self::Mistake => 0xff_a4_59,
            Self::Miss => 0xff_77_69,
            Self::Blunder => 0xfa_41_2d,
        }
    }

    /// The icon's asset path.
    pub fn icon_path(self) -> &'static str {
        match self {
            Self::Brilliant => "review/brilliant.png",
            Self::Great => "review/great.png",
            Self::Best => "review/best.png",
            Self::Excellent => "review/excellent.png",
            Self::Good => "review/good.png",
            Self::Book => "review/book.png",
            Self::Forced => "review/forced.png",
            Self::Inaccuracy => "review/inaccuracy.png",
            Self::Mistake => "review/mistake.png",
            Self::Miss => "review/miss.png",
            Self::Blunder => "review/blunder.png",
        }
    }
}
