use chess_domain::{Game, GameError, Move, Position, Side};

use super::NavStep;

/// The game being played, and which of its positions the board is showing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GameHistory {
    game: Game,
    /// 0 is the start; `n` is the position after the n-th move.
    viewing: usize,
}

impl GameHistory {
    pub fn new() -> Self {
        Self {
            game: Game::new(),
            viewing: 0,
        }
    }

    /// A game shown at position `viewing` (0 is its start), clamped to its last position.
    pub fn from_game(game: Game, viewing: usize) -> Self {
        let last = game.positions().len() - 1;
        Self {
            game,
            viewing: viewing.min(last),
        }
    }

    /// A history that starts from any position, such as a pasted FEN.
    pub fn from_position(position: Position) -> Self {
        Self::from_game(Game::from_position(position), 0)
    }

    pub fn game(&self) -> &Game {
        &self.game
    }

    /// The position the game is actually at; moves are played from here.
    pub fn live(&self) -> &Position {
        self.game.live()
    }

    /// The position on the board right now, possibly an earlier one.
    pub fn shown(&self) -> &Position {
        &self.game.positions()[self.viewing]
    }

    /// The move that led to the shown position, for the last-move highlight.
    pub fn shown_last_move(&self) -> Option<Move> {
        self.viewing
            .checked_sub(1)
            .map(|index| self.game.moves()[index])
    }

    pub fn sans(&self) -> &[String] {
        self.game.sans()
    }

    pub fn viewing(&self) -> usize {
        self.viewing
    }

    pub fn is_live(&self) -> bool {
        self.viewing == self.last_index()
    }

    /// Plays `mv` from the live position and jumps the board to the result.
    pub fn play(&mut self, mv: Move) -> Result<(), GameError> {
        self.game.play(mv)?;
        self.jump_to_live();
        Ok(())
    }

    /// Plays `mv` from the position on the board; from an earlier one, the later moves are
    /// dropped first, as on an analysis board.
    pub fn play_from_shown(&mut self, mv: Move) -> Result<(), GameError> {
        if !self.is_live() {
            self.game = self.game.truncated(self.viewing);
        }
        self.play(mv)
    }

    pub fn step(&mut self, step: NavStep) {
        match step {
            NavStep::Start => self.jump_to_start(),
            NavStep::Back => self.back(),
            NavStep::Forward => self.forward(),
            NavStep::Live => self.jump_to_live(),
        }
    }

    pub fn resign(&mut self, side: Side) -> Result<(), GameError> {
        self.game.resign(side)
    }

    pub fn flag(&mut self, side: Side) -> Result<(), GameError> {
        self.game.flag(side)
    }

    pub fn offer_draw(&mut self, side: Side) -> Result<(), GameError> {
        self.game.offer_draw(side)
    }

    pub fn accept_draw(&mut self, side: Side) -> Result<(), GameError> {
        self.game.accept_draw(side)
    }

    pub fn decline_draw(&mut self, side: Side) -> Result<(), GameError> {
        self.game.decline_draw(side)
    }

    pub fn can_go_back(&self) -> bool {
        self.viewing > 0
    }

    pub fn can_go_forward(&self) -> bool {
        !self.is_live()
    }

    pub fn back(&mut self) {
        self.viewing = self.viewing.saturating_sub(1);
    }

    pub fn forward(&mut self) {
        self.viewing = (self.viewing + 1).min(self.last_index());
    }

    pub fn jump_to_start(&mut self) {
        self.viewing = 0;
    }

    pub fn jump_to_live(&mut self) {
        self.viewing = self.last_index();
    }

    /// Shows the position after the move at `index` in `sans`.
    pub fn jump_to_move(&mut self, index: usize) {
        self.viewing = (index + 1).min(self.last_index());
    }

    fn last_index(&self) -> usize {
        self.game.positions().len() - 1
    }
}

#[cfg(test)]
mod tests {
    use chess_domain::{Move, Position, Side};

    use super::GameHistory;

    fn mv(from: &str, to: &str) -> Move {
        Move::new(
            from.parse().expect("a square"),
            to.parse().expect("a square"),
        )
    }

    fn two_moves() -> GameHistory {
        let mut history = GameHistory::new();
        history.play(mv("e2", "e4")).expect("legal");
        history.play(mv("e7", "e5")).expect("legal");
        history
    }

    #[test]
    fn a_new_game_shows_the_start_and_cannot_move_through_time() {
        let history = GameHistory::new();
        assert_eq!(history.shown(), &Position::starting());
        assert!(!history.can_go_back());
        assert!(!history.can_go_forward());
        assert_eq!(history.shown_last_move(), None);
    }

    #[test]
    fn back_shows_earlier_positions_without_changing_the_game() {
        let mut history = two_moves();
        let live = *history.live();
        history.back();
        assert_eq!(history.shown_last_move(), Some(mv("e2", "e4")));
        history.back();
        assert_eq!(history.shown(), &Position::starting());
        assert_eq!(history.live(), &live);
        assert!(!history.is_live());
    }

    #[test]
    fn back_and_forward_stop_at_the_ends() {
        let mut history = two_moves();
        history.forward();
        assert!(history.is_live());
        history.jump_to_start();
        history.back();
        assert!(!history.can_go_back());
        history.jump_to_live();
        assert!(history.is_live());
    }

    #[test]
    fn moves_are_written_down_in_order() {
        assert_eq!(two_moves().sans(), ["e4", "e5"]);
    }

    #[test]
    fn jumping_to_a_move_shows_the_position_after_it() {
        let mut history = two_moves();
        history.jump_to_move(0);
        assert_eq!(history.viewing(), 1);
        assert_eq!(history.shown().side_to_move(), Side::Black);
        history.jump_to_move(99);
        assert!(history.is_live());
    }

    #[test]
    fn playing_from_an_earlier_position_drops_the_later_moves() {
        let mut history = two_moves();
        history.step(super::NavStep::Back);
        history.play_from_shown(mv("c7", "c5")).expect("legal");
        assert_eq!(history.sans(), ["e4", "c5"]);
        assert!(history.is_live());
    }

    #[test]
    fn playing_from_the_live_position_just_adds_the_move() {
        let mut history = two_moves();
        history.play_from_shown(mv("g1", "f3")).expect("legal");
        assert_eq!(history.sans(), ["e4", "e5", "Nf3"]);
    }

    #[test]
    fn a_game_can_be_opened_at_any_position() {
        let history = GameHistory::from_game(two_moves().game().clone(), 1);
        assert_eq!(history.viewing(), 1);
        assert_eq!(
            GameHistory::from_game(two_moves().game().clone(), 9).viewing(),
            2
        );
    }

    #[test]
    fn a_move_jumps_back_to_the_live_position() {
        let mut history = two_moves();
        history.jump_to_start();
        history.play(mv("g1", "f3")).expect("legal");
        assert!(history.is_live());
        assert_eq!(history.shown(), history.live());
    }
}
