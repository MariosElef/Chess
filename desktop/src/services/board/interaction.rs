use chess_domain::{Board, Move, PieceKind, Position, Side, Square};

use super::{
    annotations::Annotations,
    premove::{premove_outcome, premove_targets},
    selection::{click_outcome, targets},
};
use crate::enums::board::ClickOutcome;

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

    pub fn selected(&self) -> Option<Square> {
        self.selected
    }

    /// The squares marked red with the right button.
    pub fn marks(&self) -> &[Square] {
        self.annotations.marks()
    }

    /// The arrows drawn with the right button.
    pub fn arrows(&self) -> &[(Square, Square)] {
        self.annotations.arrows()
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

    /// A left-click by the side waiting to move: selects one of its pieces on `board` (the
    /// position as the queued premoves leave it) or returns the premove to queue.
    pub fn premove_click(&mut self, board: &Board, side: Side, square: Square) -> Option<Move> {
        self.annotations.clear();
        if self.is_promoting() {
            return None;
        }
        self.apply(premove_outcome(board, side, self.selected, square))
    }

    /// A drag by the side waiting to move; a square its piece could never reach puts it back.
    pub fn premove_drop(
        &mut self,
        board: &Board,
        side: Side,
        from: Square,
        to: Square,
    ) -> Option<Move> {
        self.annotations.clear();
        if self.is_promoting() {
            return None;
        }
        match premove_outcome(board, side, Some(from), to) {
            ClickOutcome::Select(_) => {
                self.selected = None;
                None
            }
            outcome @ (ClickOutcome::Play(_) | ClickOutcome::Promote { .. }) => self.apply(outcome),
        }
    }

    /// Where the selected piece may legally move in `position`.
    pub fn legal_targets(&self, position: &Position) -> Vec<Square> {
        targets(position, self.selected)
    }

    /// Where the selected piece could premove to on `board`.
    pub fn premove_targets(&self, board: &Board) -> Vec<Square> {
        premove_targets(board, self.selected)
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

#[cfg(test)]
mod tests {
    use chess_domain::{Board, Move, PieceKind, Position, Side, Square};

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
    fn a_knight_can_be_premoved_twice_through_the_preview() {
        let start = Board::starting();
        let mut interaction = BoardInteraction::default();
        assert_eq!(
            interaction.premove_click(&start, Side::Black, sq("g8")),
            None
        );
        let first = interaction
            .premove_click(&start, Side::Black, sq("f6"))
            .expect("a premove");
        assert_eq!(first, Move::new(sq("g8"), sq("f6")));
        let preview = start.after_premove(first);
        assert_eq!(
            interaction.premove_click(&preview, Side::Black, sq("f6")),
            None
        );
        assert_eq!(
            interaction.premove_click(&preview, Side::Black, sq("g4")),
            Some(Move::new(sq("f6"), sq("g4")))
        );
    }

    #[test]
    fn a_promotion_premove_waits_for_its_piece() {
        let board = *"8/8/8/8/8/8/4p3/k6K w - - 0 1"
            .parse::<Position>()
            .expect("valid")
            .board();
        let mut interaction = BoardInteraction::default();
        assert_eq!(
            interaction.premove_drop(&board, Side::Black, sq("e2"), sq("e1")),
            None
        );
        assert!(interaction.is_promoting());
        assert_eq!(
            interaction.promote(PieceKind::Queen),
            Some(Move::promoting(sq("e2"), sq("e1"), PieceKind::Queen))
        );
    }

    #[test]
    fn a_right_release_without_a_press_changes_nothing() {
        let mut interaction = BoardInteraction::default();
        assert!(!interaction.right_up(sq("e4")));
    }
}
