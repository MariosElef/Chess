use crate::{Board, Move, Piece, PieceKind, Position, Side, Square};

const PROMOTIONS: [PieceKind; 4] = [
    PieceKind::Queen,
    PieceKind::Rook,
    PieceKind::Bishop,
    PieceKind::Knight,
];

/// Forward onto empty squares, diagonal captures with en passant, and promotion on the last rank.
pub(super) fn moves(position: &Position, from: Square, moves: &mut Vec<Move>) {
    pushes(position, from, moves);
    captures(position, from, moves);
}

/// A pawn of `by` on either diagonal behind `square` (from `by`'s point of view) attacks it.
pub(super) fn attacks(board: &Board, square: Square, by: Side) -> bool {
    let attacker = Some(Piece::new(by, PieceKind::Pawn));
    [-1, 1].into_iter().any(|files| {
        square
            .offset(files, -by.forward())
            .is_some_and(|from| board.piece_at(from) == attacker)
    })
}

/// One step onto an empty square, and a second from the starting rank if that is empty too.
fn pushes(position: &Position, from: Square, moves: &mut Vec<Move>) {
    let side = position.side_to_move();
    let Some(one) = empty_ahead(position, from) else {
        return;
    };
    add(side, from, one, moves);
    if is_on_start_rank(side, from)
        && let Some(two) = empty_ahead(position, one)
    {
        moves.push(Move::new(from, two));
    }
}

/// One step diagonally forward onto an enemy piece or the en passant square.
fn captures(position: &Position, from: Square, moves: &mut Vec<Move>) {
    let side = position.side_to_move();
    for files in [-1, 1] {
        if let Some(to) = from.offset(files, side.forward())
            && can_capture_on(position, to)
        {
            add(side, from, to, moves);
        }
    }
}

fn empty_ahead(position: &Position, from: Square) -> Option<Square> {
    from.offset(0, position.side_to_move().forward())
        .filter(|&to| position.board().piece_at(to).is_none())
}

fn is_on_start_rank(side: Side, square: Square) -> bool {
    square.rank() == side.back_rank().abs_diff(1)
}

fn can_capture_on(position: &Position, square: Square) -> bool {
    let holds_enemy = position
        .board()
        .piece_at(square)
        .is_some_and(|piece| piece.side != position.side_to_move());
    holds_enemy || position.en_passant() == Some(square)
}

/// Adds the move, or one move per promotion choice when it reaches the last rank.
fn add(side: Side, from: Square, to: Square, moves: &mut Vec<Move>) {
    if to.rank() == side.opponent().back_rank() {
        moves.extend(PROMOTIONS.map(|kind| Move::promoting(from, to, kind)));
    } else {
        moves.push(Move::new(from, to));
    }
}
