//! How an online game stands, as the server says it.
use chess_domain::Side;
use serde::Deserialize;

/// A side, as it travels the wire.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Color {
    White,
    Black,
}

impl From<Color> for Side {
    fn from(color: Color) -> Self {
        match color {
            Color::White => Self::White,
            Color::Black => Self::Black,
        }
    }
}

/// Whether an online game is still being played.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LiveStatus {
    Ongoing,
    Finished,
    /// Called off before both sides had moved: no result, never rated.
    Aborted,
}

/// How a finished game went for one player.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PlayerResult {
    Won,
    Lost,
    Drawn,
}

/// How an online game ended.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GameResult {
    Checkmate,
    Resignation,
    Timeout,
    Stalemate,
    DrawAgreement,
    InsufficientMaterial,
    Repetition,
    FiftyMove,
}

impl GameResult {
    /// The words after "by": `checkmate`, `agreement`, `the fifty-move rule`…
    pub fn by(self) -> &'static str {
        match self {
            Self::Checkmate => "checkmate",
            Self::Resignation => "resignation",
            Self::Timeout => "time",
            Self::Stalemate => "stalemate",
            Self::DrawAgreement => "agreement",
            Self::InsufficientMaterial => "insufficient material",
            Self::Repetition => "threefold repetition",
            Self::FiftyMove => "the fifty-move rule",
        }
    }
}

/// Who the game is against: someone at this computer, someone online, or a bot.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum PlayMode {
    #[default]
    Online,
    Bots,
    Local,
}

impl PlayMode {
    /// In the order the tabs show them, the first being where Play opens.
    pub const ALL: [Self; 3] = [Self::Online, Self::Bots, Self::Local];

    pub fn label(self) -> &'static str {
        match self {
            Self::Local => "Local",
            Self::Online => "Online",
            Self::Bots => "Bots",
        }
    }
}

/// The colour the player asks for against a bot.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum ColourChoice {
    White,
    #[default]
    Random,
    Black,
}

impl ColourChoice {
    pub const ALL: [Self; 3] = [Self::White, Self::Random, Self::Black];

    pub fn label(self) -> &'static str {
        match self {
            Self::White => "White",
            Self::Random => "Random",
            Self::Black => "Black",
        }
    }
}

/// Who a named bot is: each has its own way of talking and acting at the board.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BotCharacter {
    /// Bored, hungry, and moves almost without thinking.
    Vasilis,
    /// Angry all the time, and quits any game he is losing.
    Danis,
    /// A metalhead who screams at captures and talks about goth girls.
    Stefanos,
    /// Gambles on everything, and offers deals once he is behind.
    Andreas,
    /// The strongest, and sure nobody likes him.
    Sporos,
}

/// What just happened on the board, as a bot sees it, for it to react to.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BotMoment {
    /// The player took one of the bot's pieces.
    LostPiece,
    /// The bot took one of the player's.
    TookPiece,
    /// The player put the bot in check.
    InCheck,
    /// The bot put the player in check.
    GaveCheck,
    /// Any other move.
    Quiet,
}
