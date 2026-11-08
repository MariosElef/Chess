use chess_domain::{Board, Move, Position, Side, Square};

use crate::enums::board::ClickMode;

/// A click continues the selection; else the side waiting starts a premove, or it is a move.
pub fn click_mode(
    preview: &Board,
    waiting: Side,
    selected: Option<Square>,
    clicked: Square,
) -> ClickMode {
    match preview.piece_at(selected.unwrap_or(clicked)) {
        Some(piece) if piece.side == waiting => ClickMode::Premove,
        Some(_) | None => ClickMode::Move,
    }
}

/// The moves each side has queued to play as soon as it is their turn, in order.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PremoveQueue {
    white: Vec<Move>,
    black: Vec<Move>,
}

impl PremoveQueue {
    pub fn queue(&mut self, side: Side, mv: Move) {
        self.side_mut(side).push(mv);
    }

    /// The premove that plays next for `side`, once its turn comes.
    pub fn first(&self, side: Side) -> Option<Move> {
        self.side(side).first().copied()
    }

    pub fn pop(&mut self, side: Side) -> Option<Move> {
        let moves = self.side_mut(side);
        (!moves.is_empty()).then(|| moves.remove(0))
    }

    /// The side to move's next premove if legal in `position`; if not, its whole queue is dropped.
    pub fn next_playable(&mut self, position: &Position) -> Option<Move> {
        let side = position.side_to_move();
        let mv = self.first(side)?;
        if position.legal_moves().contains(&mv) {
            self.pop(side)
        } else {
            self.clear(side);
            None
        }
    }

    /// Drops everything `side` queued, as when one of its premoves turns out illegal.
    pub fn clear(&mut self, side: Side) {
        self.side_mut(side).clear();
    }

    pub fn clear_all(&mut self) {
        self.white.clear();
        self.black.clear();
    }

    pub fn is_empty(&self) -> bool {
        self.white.is_empty() && self.black.is_empty()
    }

    /// Every queued move's squares, for the board to colour.
    pub fn pairs(&self) -> Vec<(Square, Square)> {
        self.white
            .iter()
            .chain(&self.black)
            .map(|mv| (mv.from, mv.to))
            .collect()
    }

    /// `live` with every queued move already made, so the next premove starts from there.
    pub fn preview(&self, live: &Board, side_to_move: Side) -> Board {
        let waiting = side_to_move.opponent();
        self.side(waiting)
            .iter()
            .chain(self.side(side_to_move))
            .fold(*live, |board, &mv| board.after_premove(mv))
    }

    fn side(&self, side: Side) -> &[Move] {
        match side {
            Side::White => &self.white,
            Side::Black => &self.black,
        }
    }

    fn side_mut(&mut self, side: Side) -> &mut Vec<Move> {
        match side {
            Side::White => &mut self.white,
            Side::Black => &mut self.black,
        }
    }
}

#[cfg(test)]
mod tests {
    use chess_domain::{Board, Move, Piece, PieceKind, Position, Side, Square};

    use super::{PremoveQueue, click_mode};
    use crate::enums::board::ClickMode;

    fn mv(from: &str, to: &str) -> Move {
        Move::new(
            from.parse().expect("a square"),
            to.parse().expect("a square"),
        )
    }

    fn sq(name: &str) -> Square {
        name.parse().expect("a square")
    }

    #[test]
    fn premoves_play_in_the_order_they_were_queued() {
        let mut queue = PremoveQueue::default();
        queue.queue(Side::Black, mv("g8", "f6"));
        queue.queue(Side::Black, mv("f6", "g4"));
        assert_eq!(queue.first(Side::Black), Some(mv("g8", "f6")));
        assert_eq!(queue.pop(Side::Black), Some(mv("g8", "f6")));
        assert_eq!(queue.first(Side::Black), Some(mv("f6", "g4")));
        assert_eq!(queue.first(Side::White), None);
    }

    #[test]
    fn the_preview_chains_the_queued_moves() {
        let mut queue = PremoveQueue::default();
        queue.queue(Side::Black, mv("g8", "f6"));
        queue.queue(Side::Black, mv("f6", "g4"));
        let preview = queue.preview(&Board::starting(), Side::White);
        assert_eq!(
            preview.piece_at(sq("g4")),
            Some(Piece::new(Side::Black, PieceKind::Knight))
        );
        assert_eq!(preview.piece_at(sq("g8")), None);
        assert_eq!(queue.pairs(), [(sq("g8"), sq("f6")), (sq("f6"), sq("g4"))]);
    }

    #[test]
    fn a_click_on_the_waiting_sides_piece_is_a_premove() {
        let start = Board::starting();
        assert_eq!(
            click_mode(&start, Side::Black, None, sq("e7")),
            ClickMode::Premove
        );
        assert_eq!(
            click_mode(&start, Side::Black, None, sq("e2")),
            ClickMode::Move
        );
        assert_eq!(
            click_mode(&start, Side::Black, None, sq("e4")),
            ClickMode::Move
        );
        assert_eq!(
            click_mode(&start, Side::Black, Some(sq("g8")), sq("f6")),
            ClickMode::Premove,
            "the selection decides where the click lands"
        );
    }

    #[test]
    fn the_next_premove_plays_if_legal_and_otherwise_drops_the_queue() {
        let mut queue = PremoveQueue::default();
        queue.queue(Side::Black, mv("e7", "e5"));
        queue.queue(Side::Black, mv("e5", "e4"));
        let after_e4 = Position::starting().play(mv("e2", "e4")).expect("legal");
        assert_eq!(queue.next_playable(&after_e4), Some(mv("e7", "e5")));
        let after_e5 = after_e4.play(mv("e7", "e5")).expect("legal");
        let after_nf3 = after_e5.play(mv("g1", "f3")).expect("legal");
        assert_eq!(
            queue.next_playable(&after_nf3),
            None,
            "e4 is blocked by White's pawn"
        );
        assert!(queue.is_empty());
    }

    #[test]
    fn clearing_one_side_leaves_the_other() {
        let mut queue = PremoveQueue::default();
        queue.queue(Side::Black, mv("e7", "e5"));
        queue.queue(Side::White, mv("e2", "e4"));
        queue.clear(Side::Black);
        assert_eq!(queue.first(Side::Black), None);
        assert_eq!(queue.first(Side::White), Some(mv("e2", "e4")));
        queue.clear_all();
        assert!(queue.is_empty());
    }
}
