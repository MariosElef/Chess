//! A whole game: its positions and moves, and every way it can end, including those a single
//! position cannot see (repetition, resignation, a draw agreed between the players).
use crate::{GameStatus, IllegalMove, Move, Position, Side};

/// Three-fold repetition: the same position for the third time.
const REPETITIONS_FOR_DRAW: usize = 3;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Outcome {
    Checkmate { winner: Side },
    Resignation { winner: Side },
    Timeout { winner: Side },
    Stalemate,
    DrawAgreement,
    InsufficientMaterial,
    Repetition,
    FiftyMoves,
}

impl Outcome {
    /// `None` for every kind of draw.
    pub fn winner(self) -> Option<Side> {
        match self {
            Self::Checkmate { winner }
            | Self::Resignation { winner }
            | Self::Timeout { winner } => Some(winner),
            Self::Stalemate
            | Self::DrawAgreement
            | Self::InsufficientMaterial
            | Self::Repetition
            | Self::FiftyMoves => None,
        }
    }
}

#[must_use]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GameError {
    Illegal(IllegalMove),
    Over,
    NoDrawOffer,
}

impl From<IllegalMove> for GameError {
    fn from(illegal: IllegalMove) -> Self {
        Self::Illegal(illegal)
    }
}

impl std::fmt::Display for GameError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Illegal(illegal) => illegal.fmt(f),
            Self::Over => f.write_str("the game is over"),
            Self::NoDrawOffer => f.write_str("there is no draw offer to answer"),
        }
    }
}

impl std::error::Error for GameError {}

/// `positions[0]` is where the game began; `moves[i]` and `sans[i]` led to `positions[i + 1]`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Game {
    positions: Vec<Position>,
    moves: Vec<Move>,
    sans: Vec<String>,
    /// An ending the players chose: a resignation or an agreed draw.
    chosen_end: Option<Outcome>,
    draw_offer: Option<Side>,
}

impl Game {
    pub fn new() -> Self {
        Self::from_position(Position::starting())
    }

    /// A game that starts from any position, such as a puzzle's.
    pub fn from_position(start: Position) -> Self {
        Self {
            positions: vec![start],
            moves: Vec::new(),
            sans: Vec::new(),
            chosen_end: None,
            draw_offer: None,
        }
    }

    /// The position the game is at now.
    pub fn live(&self) -> &Position {
        self.positions.last().expect("a game always has its start")
    }

    pub fn positions(&self) -> &[Position] {
        &self.positions
    }

    pub fn moves(&self) -> &[Move] {
        &self.moves
    }

    /// The moves in Standard Algebraic Notation.
    pub fn sans(&self) -> &[String] {
        &self.sans
    }

    /// The game cut after its first `plies` moves: a fresh line from an earlier point, so a
    /// resignation, agreed draw or open offer does not carry over.
    pub fn truncated(&self, plies: usize) -> Self {
        let plies = plies.min(self.moves.len());
        Self {
            positions: self.positions[..=plies].to_vec(),
            moves: self.moves[..plies].to_vec(),
            sans: self.sans[..plies].to_vec(),
            chosen_end: None,
            draw_offer: None,
        }
    }

    /// The side whose draw offer is waiting for an answer.
    pub fn draw_offer(&self) -> Option<Side> {
        self.draw_offer
    }

    pub fn outcome(&self) -> Option<Outcome> {
        self.chosen_end.or_else(|| self.automatic_outcome())
    }

    pub fn is_over(&self) -> bool {
        self.outcome().is_some()
    }

    /// Plays a move for the side to move; an open offer from the other side lapses.
    pub fn play(&mut self, mv: Move) -> Result<(), GameError> {
        self.ensure_ongoing()?;
        let live = self.live();
        let mover = live.side_to_move();
        let san = live.san(mv)?;
        let next = live.play(mv)?;
        self.positions.push(next);
        self.moves.push(mv);
        self.sans.push(san);
        self.lapse_offer_against(mover);
        Ok(())
    }

    pub fn resign(&mut self, side: Side) -> Result<(), GameError> {
        self.ensure_ongoing()?;
        self.chosen_end = Some(Outcome::Resignation {
            winner: side.opponent(),
        });
        Ok(())
    }

    /// `side` ran out of time; the opponent wins.
    pub fn flag(&mut self, side: Side) -> Result<(), GameError> {
        self.ensure_ongoing()?;
        self.chosen_end = Some(Outcome::Timeout {
            winner: side.opponent(),
        });
        Ok(())
    }

    pub fn offer_draw(&mut self, side: Side) -> Result<(), GameError> {
        self.ensure_ongoing()?;
        self.draw_offer = Some(side);
        Ok(())
    }

    /// Only the side the offer was made to can accept it.
    pub fn accept_draw(&mut self, side: Side) -> Result<(), GameError> {
        self.ensure_ongoing()?;
        self.ensure_offered_to(side)?;
        self.draw_offer = None;
        self.chosen_end = Some(Outcome::DrawAgreement);
        Ok(())
    }

    pub fn decline_draw(&mut self, side: Side) -> Result<(), GameError> {
        self.ensure_offered_to(side)?;
        self.draw_offer = None;
        Ok(())
    }

    /// Endings the rules decide on their own, checkmate first so a mate on the last allowed move counts.
    fn automatic_outcome(&self) -> Option<Outcome> {
        let live = self.live();
        match live.status() {
            GameStatus::Checkmate { winner } => Some(Outcome::Checkmate { winner }),
            GameStatus::Stalemate => Some(Outcome::Stalemate),
            GameStatus::Ongoing => self.automatic_draw(live),
        }
    }

    fn automatic_draw(&self, live: &Position) -> Option<Outcome> {
        if live.board().has_insufficient_material() {
            Some(Outcome::InsufficientMaterial)
        } else if self.repetitions_of_live() >= REPETITIONS_FOR_DRAW {
            Some(Outcome::Repetition)
        } else if live.is_fifty_move_draw() {
            Some(Outcome::FiftyMoves)
        } else {
            None
        }
    }

    fn repetitions_of_live(&self) -> usize {
        let live = self.live();
        self.positions
            .iter()
            .filter(|position| position.is_same_position(live))
            .count()
    }

    /// A move by the side an offer was made to answers it with a no.
    fn lapse_offer_against(&mut self, mover: Side) {
        if self.draw_offer == Some(mover.opponent()) {
            self.draw_offer = None;
        }
    }

    fn ensure_ongoing(&self) -> Result<(), GameError> {
        if self.is_over() {
            Err(GameError::Over)
        } else {
            Ok(())
        }
    }

    fn ensure_offered_to(&self, side: Side) -> Result<(), GameError> {
        if self.draw_offer == Some(side.opponent()) {
            Ok(())
        } else {
            Err(GameError::NoDrawOffer)
        }
    }
}

impl Default for Game {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::{Game, GameError, Outcome};
    use crate::{Move, Position, Side};

    fn mv(from: &str, to: &str) -> Move {
        Move::new(
            from.parse().expect("a square"),
            to.parse().expect("a square"),
        )
    }

    fn play_all(game: &mut Game, moves: &[(&str, &str)]) {
        for (from, to) in moves {
            game.play(mv(from, to)).expect("legal");
        }
    }

    fn from_fen(fen: &str) -> Game {
        Game::from_position(fen.parse::<Position>().expect("a valid FEN"))
    }

    const KNIGHT_SHUFFLE: [(&str, &str); 4] =
        [("g1", "f3"), ("g8", "f6"), ("f3", "g1"), ("f6", "g8")];

    #[test]
    fn the_third_repetition_is_a_draw() {
        let mut game = Game::new();
        play_all(&mut game, &KNIGHT_SHUFFLE);
        assert_eq!(game.outcome(), None);
        play_all(&mut game, &KNIGHT_SHUFFLE);
        assert_eq!(game.outcome(), Some(Outcome::Repetition));
        assert_eq!(game.play(mv("e2", "e4")), Err(GameError::Over));
    }

    #[test]
    fn bare_kings_are_a_draw() {
        let mut game = from_fen("8/8/8/4k3/8/8/3q4/4K3 w - - 0 1");
        game.play(mv("e1", "d2")).expect("the king takes the queen");
        assert_eq!(game.outcome(), Some(Outcome::InsufficientMaterial));
    }

    #[test]
    fn a_hundred_quiet_half_moves_is_a_draw() {
        let mut game = from_fen("8/8/8/4k3/8/8/8/R3K3 w - - 99 70");
        game.play(mv("a1", "a2")).expect("legal");
        assert_eq!(game.outcome(), Some(Outcome::FiftyMoves));
    }

    #[test]
    fn checkmate_names_the_winner() {
        let mut game = Game::new();
        play_all(
            &mut game,
            &[("f2", "f3"), ("e7", "e5"), ("g2", "g4"), ("d8", "h4")],
        );
        let outcome = game.outcome().expect("over");
        assert_eq!(
            outcome,
            Outcome::Checkmate {
                winner: Side::Black
            }
        );
        assert_eq!(outcome.winner(), Some(Side::Black));
        assert_eq!(game.sans().last().map(String::as_str), Some("Qh4#"));
    }

    #[test]
    fn resigning_gives_the_game_to_the_opponent() {
        let mut game = Game::new();
        game.resign(Side::White).expect("ongoing");
        assert_eq!(
            game.outcome(),
            Some(Outcome::Resignation {
                winner: Side::Black
            })
        );
    }

    #[test]
    fn running_out_of_time_loses() {
        let mut game = Game::new();
        game.flag(Side::Black).expect("ongoing");
        assert_eq!(
            game.outcome(),
            Some(Outcome::Timeout {
                winner: Side::White
            })
        );
    }

    #[test]
    fn a_draw_offer_can_be_accepted_by_the_other_side_only() {
        let mut game = Game::new();
        game.offer_draw(Side::White).expect("ongoing");
        assert_eq!(game.accept_draw(Side::White), Err(GameError::NoDrawOffer));
        game.accept_draw(Side::Black).expect("offered to Black");
        assert_eq!(game.outcome(), Some(Outcome::DrawAgreement));
        assert_eq!(game.outcome().and_then(Outcome::winner), None);
    }

    #[test]
    fn moving_instead_of_answering_declines_the_offer() {
        let mut game = Game::new();
        game.play(mv("e2", "e4")).expect("legal");
        game.offer_draw(Side::White).expect("ongoing");
        game.play(mv("e7", "e5")).expect("legal");
        assert_eq!(game.draw_offer(), None);
        assert!(!game.is_over());
    }

    #[test]
    fn truncating_keeps_the_first_moves_and_drops_a_resignation() {
        let mut game = Game::new();
        play_all(&mut game, &[("e2", "e4"), ("e7", "e5"), ("g1", "f3")]);
        game.resign(Side::Black).expect("ongoing");
        let earlier = game.truncated(2);
        assert_eq!(earlier.sans(), ["e4", "e5"]);
        assert_eq!(earlier.outcome(), None);
        assert_eq!(game.truncated(99).moves().len(), 3);
    }

    #[test]
    fn a_declined_offer_is_gone() {
        let mut game = Game::new();
        game.offer_draw(Side::White).expect("ongoing");
        game.decline_draw(Side::Black).expect("offered to Black");
        assert_eq!(game.draw_offer(), None);
    }
}
