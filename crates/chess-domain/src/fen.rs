use std::{fmt, str::FromStr};

use crate::{Board, CastlingRights, MoveCounters, Piece, PieceKind, Position, Side, Square};

#[must_use]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FenError {
    MissingField,
    Placement,
    Piece(char),
    SideToMove,
    Castling(char),
    EnPassant,
    MoveCounter,
}

impl std::fmt::Display for FenError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MissingField => {
                f.write_str("a FEN needs placement, side, castling and en passant")
            }
            Self::Placement => f.write_str("the placement must describe 8 ranks of 8 squares"),
            Self::Piece(c) => write!(f, "{c:?} is not a piece letter"),
            Self::SideToMove => f.write_str("the side to move must be w or b"),
            Self::Castling(c) => write!(f, "{c:?} is not a castling right"),
            Self::EnPassant => f.write_str("en passant must be - or a square"),
            Self::MoveCounter => f.write_str("the move counters must be whole numbers"),
        }
    }
}

impl std::error::Error for FenError {}

/// Reads Forsyth–Edwards Notation; the two move counters at the end may be left out.
impl FromStr for Position {
    type Err = FenError;

    fn from_str(fen: &str) -> Result<Self, Self::Err> {
        let mut fields = fen.split_whitespace();
        let mut next = || fields.next().ok_or(FenError::MissingField);
        let board = placement(next()?)?;
        let side = side_to_move(next()?)?;
        let castling = castling(next()?)?;
        let en_passant = en_passant(next()?)?;
        let counters = counters(next().ok(), next().ok())?;
        Ok(Self::new(board, side, castling, en_passant, counters))
    }
}

/// Writes the position as FEN, the inverse of parsing it.
impl fmt::Display for Position {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let counters = self.counters();
        write!(
            f,
            "{} {} {} {} {} {}",
            placement_text(self.board()),
            side_letter(self.side_to_move()),
            self.castling(),
            self.en_passant()
                .map_or_else(|| "-".to_owned(), |square| square.to_string()),
            counters.halfmove_clock,
            counters.fullmove_number,
        )
    }
}

fn side_to_move(field: &str) -> Result<Side, FenError> {
    match field {
        "w" => Ok(Side::White),
        "b" => Ok(Side::Black),
        _ => Err(FenError::SideToMove),
    }
}

fn en_passant(field: &str) -> Result<Option<Square>, FenError> {
    if field == "-" {
        return Ok(None);
    }
    field.parse().map(Some).map_err(|_| FenError::EnPassant)
}

fn counters(halfmove: Option<&str>, fullmove: Option<&str>) -> Result<MoveCounters, FenError> {
    Ok(MoveCounters {
        halfmove_clock: counter(halfmove, MoveCounters::START.halfmove_clock)?,
        fullmove_number: counter(fullmove, MoveCounters::START.fullmove_number)?,
    })
}

fn counter(field: Option<&str>, default: u32) -> Result<u32, FenError> {
    field.map_or(Ok(default), |text| {
        text.parse().map_err(|_| FenError::MoveCounter)
    })
}

/// Each rank from 8 down to 1; a run of empty squares is written as its length.
fn placement_text(board: &Board) -> String {
    (0..8)
        .rev()
        .map(|rank| rank_text(board, rank))
        .collect::<Vec<_>>()
        .join("/")
}

fn rank_text(board: &Board, rank: u8) -> String {
    let mut text = String::new();
    let mut empty = 0;
    for file in 0..8 {
        let square = Square::new(file, rank).expect("0..8 stays on the board");
        match board.piece_at(square) {
            Some(piece) => {
                push_empty_run(&mut text, &mut empty);
                text.push(piece_letter(piece));
            }
            None => empty += 1,
        }
    }
    push_empty_run(&mut text, &mut empty);
    text
}

fn push_empty_run(text: &mut String, empty: &mut u32) {
    if *empty > 0 {
        text.push_str(&empty.to_string());
        *empty = 0;
    }
}

fn piece_letter(piece: Piece) -> char {
    let letter = match piece.kind {
        PieceKind::King => 'k',
        PieceKind::Queen => 'q',
        PieceKind::Rook => 'r',
        PieceKind::Bishop => 'b',
        PieceKind::Knight => 'n',
        PieceKind::Pawn => 'p',
    };
    match piece.side {
        Side::White => letter.to_ascii_uppercase(),
        Side::Black => letter,
    }
}

fn side_letter(side: Side) -> char {
    match side {
        Side::White => 'w',
        Side::Black => 'b',
    }
}

fn placement(field: &str) -> Result<Board, FenError> {
    let ranks: Vec<&str> = field.split('/').collect();
    if ranks.len() != 8 {
        return Err(FenError::Placement);
    }
    let mut board = Board::empty();
    for (rank, row) in (0..8).rev().zip(ranks) {
        let mut file = 0;
        for c in row.chars() {
            if let Some(empty) = c.to_digit(10) {
                file += u8::try_from(empty).map_err(|_| FenError::Placement)?;
                continue;
            }
            let square = Square::new(file, rank).ok_or(FenError::Placement)?;
            board.set(square, Some(piece(c)?));
            file += 1;
        }
        if file != 8 {
            return Err(FenError::Placement);
        }
    }
    Ok(board)
}

fn piece(c: char) -> Result<Piece, FenError> {
    let side = if c.is_ascii_uppercase() {
        Side::White
    } else {
        Side::Black
    };
    let kind = match c.to_ascii_lowercase() {
        'k' => PieceKind::King,
        'q' => PieceKind::Queen,
        'r' => PieceKind::Rook,
        'b' => PieceKind::Bishop,
        'n' => PieceKind::Knight,
        'p' => PieceKind::Pawn,
        _ => return Err(FenError::Piece(c)),
    };
    Ok(Piece::new(side, kind))
}

fn castling(field: &str) -> Result<CastlingRights, FenError> {
    let mut rights = CastlingRights::default();
    if field == "-" {
        return Ok(rights);
    }
    for c in field.chars() {
        match c {
            'K' => rights.white_king_side = true,
            'Q' => rights.white_queen_side = true,
            'k' => rights.black_king_side = true,
            'q' => rights.black_queen_side = true,
            _ => return Err(FenError::Castling(c)),
        }
    }
    Ok(rights)
}

#[cfg(test)]
mod tests {
    use super::FenError;
    use crate::Position;

    #[test]
    fn the_starting_fen_is_the_starting_position() {
        let parsed: Position = "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1"
            .parse()
            .expect("valid");
        assert_eq!(parsed, Position::starting());
    }

    #[test]
    fn writing_a_fen_gives_back_what_was_read() {
        for fen in [
            "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1",
            "r3k2r/p1ppqpb1/bn2pnp1/3PN3/1p2P3/2N2Q1p/PPPBBPPP/R3K2R w KQkq - 0 1",
            "8/2p5/3p4/KP5r/1R3p1k/8/4P1P1/8 w - - 0 1",
            "r3k2r/Pppp1ppp/1b3nbN/nP6/BBP1P3/q4N2/Pp1P2PP/R2Q1RK1 w kq - 3 17",
        ] {
            let position: Position = fen.parse().expect("valid");
            assert_eq!(position.to_string(), fen);
        }
    }

    #[test]
    fn missing_counters_default_to_the_start() {
        let position: Position = "8/8/8/8/8/8/8/k6K w - -".parse().expect("valid");
        assert_eq!(position.to_string(), "8/8/8/8/8/8/8/k6K w - - 0 1");
    }

    #[test]
    fn broken_fens_say_what_is_wrong() {
        assert_eq!(
            "8/8/8/8/8/8/8/8 w - - x 1".parse::<Position>(),
            Err(FenError::MoveCounter)
        );
        assert_eq!("8/8 w - -".parse::<Position>(), Err(FenError::Placement));
        assert_eq!(
            "8/8/8/8/8/8/8/7x w - -".parse::<Position>(),
            Err(FenError::Piece('x'))
        );
        assert_eq!(
            "8/8/8/8/8/8/8/8 x - -".parse::<Position>(),
            Err(FenError::SideToMove)
        );
        assert_eq!(
            "8/8/8/8/8/8/8/8 w".parse::<Position>(),
            Err(FenError::MissingField)
        );
    }
}
