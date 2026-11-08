use chess_domain::{Move, PieceKind, Position, Side, Square};

use super::{
    BoardGrid,
    annotations::Annotations,
    selection::{ClickOutcome, click_outcome, targets},
};

/// What the player is doing on a board, for any page that shows one: the picked-up piece, the
/// pawn waiting for its promotion choice, the drawings, and which way up the board is.
/// Methods return the move to play, if any; the page decides what playing it means.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct BoardInteraction {
    selected: Option<Square>,
    /// Black at the bottom.
    flipped: bool,
    /// A pawn on the last rank while the player picks what it becomes.
    pending_promotion: Option<(Square, Square)>,
    /// Red squares and arrows drawn with the right button.
    annotations: Annotations,
    /// Where the right button went down, until it comes back up.
    right_press: Option<Square>,
}

impl BoardInteraction {
    pub fn flipped(&self) -> bool {
        self.flipped
    }

    pub fn is_promoting(&self) -> bool {
        self.pending_promotion.is_some()
    }

    /// Whether a move can be started: the page allows it and no promotion choice is open.
    pub fn accepts_moves(&self, page_allows: bool) -> bool {
        page_allows && !self.is_promoting()
    }

    /// A left-click wipes the drawings, as on chess.com, then selects or makes a move.
    pub fn click(
        &mut self,
        position: &Position,
        square: Square,
        page_allows: bool,
    ) -> Option<Move> {
        self.annotations.clear();
        if !self.accepts_moves(page_allows) {
            return None;
        }
        self.apply(click_outcome(position, self.selected, square))
    }

    /// A drag ending on `to`; a square the piece cannot reach puts it back.
    pub fn drop(
        &mut self,
        position: &Position,
        from: Square,
        to: Square,
        page_allows: bool,
    ) -> Option<Move> {
        self.annotations.clear();
        if !self.accepts_moves(page_allows) {
            return None;
        }
        match click_outcome(position, Some(from), to) {
            ClickOutcome::Select(_) => {
                self.selected = None;
                None
            }
            outcome @ (ClickOutcome::Play(_) | ClickOutcome::Promote { .. }) => self.apply(outcome),
        }
    }

    /// The piece the player picked for the waiting pawn.
    pub fn promote(&mut self, kind: PieceKind) -> Option<Move> {
        let (from, to) = self.pending_promotion.take()?;
        self.put_down();
        Some(Move::promoting(from, to, kind))
    }

    pub fn cancel_promotion(&mut self) {
        self.pending_promotion = None;
        self.selected = None;
    }

    pub fn right_down(&mut self, square: Square) {
        self.right_press = Some(square);
    }

    /// Releasing where it was pressed marks the square; elsewhere, it draws an arrow.
    /// Returns whether the drawings changed.
    pub fn right_up(&mut self, square: Square) -> bool {
        let Some(start) = self.right_press.take() else {
            return false;
        };
        self.annotations.release(start, square);
        true
    }

    pub fn flip(&mut self) {
        self.flipped = !self.flipped;
    }

    /// Puts down a picked-up piece, closes the promotion picker and wipes the drawings.
    pub fn clear(&mut self) {
        self.selected = None;
        self.pending_promotion = None;
        self.annotations.clear();
        self.right_press = None;
    }

    /// The board for `position` with everything this interaction draws; the page adds its
    /// promotion prompt and callbacks.
    pub fn grid(
        &self,
        position: &Position,
        last_move: Option<Move>,
        draggable: Option<Side>,
    ) -> BoardGrid {
        BoardGrid::new(*position.board())
            .flipped(self.flipped)
            .selected(self.selected)
            .targets(targets(position, self.selected))
            .last_move(last_move)
            .check(check_square(position))
            .draggable_side(draggable)
            .marks(self.annotations.marks().to_vec())
            .arrows(self.annotations.arrows().to_vec())
    }

    fn apply(&mut self, outcome: ClickOutcome) -> Option<Move> {
        match outcome {
            ClickOutcome::Select(selected) => {
                self.selected = selected;
                None
            }
            ClickOutcome::Play(mv) => {
                self.put_down();
                Some(mv)
            }
            ClickOutcome::Promote { from, to } => {
                self.selected = Some(from);
                self.pending_promotion = Some((from, to));
                None
            }
        }
    }

    /// After a move: nothing is held and the drawings belong to the old position.
    fn put_down(&mut self) {
        self.selected = None;
        self.annotations.clear();
    }
}

/// The king's square while the side to move is in check.
fn check_square(position: &Position) -> Option<Square> {
    position
        .is_check()
        .then(|| position.board().king_of(position.side_to_move()))
        .flatten()
}

#[cfg(test)]
mod tests {
    use chess_domain::{Move, PieceKind, Position, Square};

    use super::BoardInteraction;

    fn sq(name: &str) -> Square {
        name.parse().expect("a square")
    }

    #[test]
    fn a_click_selects_then_a_second_click_plays() {
        let start = Position::starting();
        let mut interaction = BoardInteraction::default();
        assert_eq!(interaction.click(&start, sq("e2"), true), None);
        assert_eq!(
            interaction.click(&start, sq("e4"), true),
            Some(Move::new(sq("e2"), sq("e4")))
        );
        assert_eq!(interaction, BoardInteraction::default());
    }

    #[test]
    fn a_promotion_waits_for_the_choice() {
        let position: Position = "8/4P3/8/8/8/8/8/k6K w - - 0 1".parse().expect("valid");
        let mut interaction = BoardInteraction::default();
        interaction.click(&position, sq("e7"), true);
        assert_eq!(interaction.click(&position, sq("e8"), true), None);
        assert!(interaction.is_promoting());
        assert!(!interaction.accepts_moves(true));
        assert_eq!(
            interaction.promote(PieceKind::Knight),
            Some(Move::promoting(sq("e7"), sq("e8"), PieceKind::Knight))
        );
        assert!(!interaction.is_promoting());
    }

    #[test]
    fn a_drop_on_an_unreachable_square_puts_the_piece_back() {
        let start = Position::starting();
        let mut interaction = BoardInteraction::default();
        assert_eq!(interaction.drop(&start, sq("e2"), sq("e5"), true), None);
        assert_eq!(
            interaction.drop(&start, sq("g1"), sq("f3"), true),
            Some(Move::new(sq("g1"), sq("f3")))
        );
    }

    #[test]
    fn when_the_page_refuses_moves_a_click_only_wipes_the_drawings() {
        let start = Position::starting();
        let mut interaction = BoardInteraction::default();
        interaction.right_down(sq("e4"));
        assert!(interaction.right_up(sq("e4")));
        assert_eq!(interaction.click(&start, sq("e2"), false), None);
        assert_eq!(interaction, BoardInteraction::default());
    }

    #[test]
    fn a_right_release_without_a_press_changes_nothing() {
        let mut interaction = BoardInteraction::default();
        assert!(!interaction.right_up(sq("e4")));
    }
}
