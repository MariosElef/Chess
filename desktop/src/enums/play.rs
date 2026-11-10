//! How an online game stands, as the server says it.
use chess_domain::Side;
use serde::Deserialize;

use crate::i18n::tr;

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
    pub fn by(self) -> String {
        match self {
            Self::Checkmate => tr!("play.ending.checkmate"),
            Self::Resignation => tr!("play.ending.resignation"),
            Self::Timeout => tr!("play.ending.timeout"),
            Self::Stalemate => tr!("play.ending.stalemate"),
            Self::DrawAgreement => tr!("play.ending.draw_agreement"),
            Self::InsufficientMaterial => tr!("play.ending.insufficient_material"),
            Self::Repetition => tr!("play.ending.repetition"),
            Self::FiftyMove => tr!("play.ending.fifty_move"),
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

    pub fn label(self) -> String {
        match self {
            Self::Local => tr!("play.mode.local"),
            Self::Online => tr!("play.mode.online"),
            Self::Bots => tr!("play.mode.bots"),
        }
    }

    /// A name that never changes with the language, for element ids.
    pub fn id(self) -> &'static str {
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

    pub fn label(self) -> String {
        match self {
            Self::White => tr!("play.colour.white"),
            Self::Random => tr!("play.colour.random"),
            Self::Black => tr!("play.colour.black"),
        }
    }

    /// A name that never changes with the language, for element ids.
    pub fn id(self) -> &'static str {
        match self {
            Self::White => "White",
            Self::Random => "Random",
            Self::Black => "Black",
        }
    }
}

/// Why a game against a bot could not start or went on: kept as a reason and said when drawn,
/// so it follows a change of language.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BotProblem {
    StockfishMissing,
    StockfishFailed,
    EngineStopped,
}

impl BotProblem {
    pub fn message(self) -> String {
        match self {
            Self::StockfishMissing => tr!("play.problem.stockfish_missing"),
            Self::StockfishFailed => tr!("play.problem.stockfish_failed"),
            Self::EngineStopped => tr!("play.problem.bot_engine_stopped"),
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
