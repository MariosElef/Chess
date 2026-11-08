use chess_domain::{IllegalMove, Move, Position};

/// Every position of the game in order, the moves between them, and which position the board shows.
/// `positions[0]` is the start; `sans[i]` is the move that led to `positions[i + 1]`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GameHistory {
    positions: Vec<Position>,
    sans: Vec<String>,
    viewing: usize,
}

impl GameHistory {
    pub fn new() -> Self {
        Self {
            positions: vec![Position::starting()],
            sans: Vec::new(),
            viewing: 0,
        }
    }

    /// The position the game is actually at; moves are played from here.
    pub fn live(&self) -> &Position {
        self.positions
            .last()
            .expect("the history always holds the start")
    }

    /// The position on the board right now, possibly an earlier one.
    pub fn shown(&self) -> &Position {
        &self.positions[self.viewing]
    }

    pub fn is_live(&self) -> bool {
        self.viewing == self.last_index()
    }

    /// The moves played so far, written the way people read them.
    pub fn sans(&self) -> &[String] {
        &self.sans
    }

    /// Which position is on the board: 0 is the start, `n` is the one after the n-th move.
    pub fn viewing(&self) -> usize {
        self.viewing
    }

    /// Plays `mv` from the live position, writes it down, and jumps the board to the result.
    pub fn play(&mut self, mv: Move) -> Result<(), IllegalMove> {
        let live = self.live();
        let san = live.san(mv)?;
        let next = live.play(mv)?;
        self.sans.push(san);
        self.positions.push(next);
        self.viewing = self.last_index();
        Ok(())
    }

    /// Shows the position after the move at `index` in `sans`.
    pub fn jump_to_move(&mut self, index: usize) {
        self.viewing = (index + 1).min(self.last_index());
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

    fn last_index(&self) -> usize {
        self.positions.len() - 1
    }
}

#[cfg(test)]
mod tests {
    use chess_domain::{Move, Position};

    use super::GameHistory;

    fn played(history: &mut GameHistory, from: &str, to: &str) {
        let mv = Move::new(
            from.parse().expect("a square"),
            to.parse().expect("a square"),
        );
        history.play(mv).expect("legal");
    }

    fn two_moves() -> GameHistory {
        let mut history = GameHistory::new();
        played(&mut history, "e2", "e4");
        played(&mut history, "e7", "e5");
        history
    }

    #[test]
    fn a_new_game_shows_the_start_and_cannot_move_through_time() {
        let history = GameHistory::new();
        assert_eq!(history.shown(), &Position::starting());
        assert!(!history.can_go_back());
        assert!(!history.can_go_forward());
    }

    #[test]
    fn back_shows_earlier_positions_without_changing_the_game() {
        let mut history = two_moves();
        let live = *history.live();
        history.back();
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
        history.forward();
        history.jump_to_live();
        assert!(history.is_live());
    }

    #[test]
    fn moves_are_written_down_in_order() {
        let history = two_moves();
        assert_eq!(history.sans(), ["e4", "e5"]);
    }

    #[test]
    fn jumping_to_a_move_shows_the_position_after_it() {
        let mut history = two_moves();
        history.jump_to_move(0);
        assert_eq!(history.viewing(), 1);
        assert_eq!(history.shown().side_to_move(), chess_domain::Side::Black);
        history.jump_to_move(99);
        assert!(history.is_live());
    }

    #[test]
    fn a_move_jumps_back_to_the_live_position() {
        let mut history = two_moves();
        history.jump_to_start();
        played(&mut history, "g1", "f3");
        assert!(history.is_live());
        assert_eq!(history.shown(), history.live());
    }
}
