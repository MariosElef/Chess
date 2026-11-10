//! Portable Game Notation, how sites give a whole game: headers in brackets, then the moves in
//! SAN with their numbers, perhaps with comments, variations and a result.
use crate::{Game, Move, Outcome, PgnError, Position, Side};

impl Game {
    /// The game a PGN records, from its `[FEN "…"]` header if it has one, else the start.
    pub fn from_pgn(pgn: &str) -> Result<Self, PgnError> {
        let start = match header(pgn, "FEN") {
            Some(fen) => fen.parse().map_err(PgnError::StartingPosition)?,
            None => Position::starting(),
        };
        let mut game = Self::from_position(start);
        let written: Vec<String> = move_words(&movetext(pgn));
        if written.is_empty() {
            return Err(PgnError::NoMoves);
        }
        for (index, word) in written.into_iter().enumerate() {
            let unreadable = || PgnError::Move {
                ply: index + 1,
                written: word.clone(),
            };
            let mv = move_written(game.live(), &word).ok_or_else(unreadable)?;
            game.play(mv).map_err(|_| unreadable())?;
        }
        Ok(game)
    }

    /// The game as a PGN: a `[FEN "…"]` header when it did not begin at the start, the moves
    /// with their numbers, and the result, `*` while it is still going.
    pub fn to_pgn(&self) -> String {
        let start = self.positions()[0];
        let headers = if start == Position::starting() {
            String::new()
        } else {
            format!("[SetUp \"1\"]\n[FEN \"{start}\"]\n\n")
        };
        let mut words = Vec::new();
        for (index, (before, san)) in self.positions().iter().zip(self.sans()).enumerate() {
            let number = before.counters().fullmove_number;
            match before.side_to_move() {
                Side::White => words.push(format!("{number}.")),
                Side::Black if index == 0 => words.push(format!("{number}...")),
                Side::Black => {}
            }
            words.push(san.clone());
        }
        words.push(result(self.outcome()).to_owned());
        headers + &words.join(" ")
    }
}

/// How a PGN ends: who won, a draw, or `*` for a game not over.
fn result(outcome: Option<Outcome>) -> &'static str {
    match outcome.map(Outcome::winner) {
        Some(Some(Side::White)) => "1-0",
        Some(Some(Side::Black)) => "0-1",
        Some(None) => "1/2-1/2",
        None => "*",
    }
}

/// The value of a `[Name "value"]` header, wherever it is: a pasted PGN may lose line breaks.
fn header<'a>(pgn: &'a str, name: &str) -> Option<&'a str> {
    let opening = format!("[{name} \"");
    let start = pgn.find(&opening)? + opening.len();
    let length = pgn[start..].find('"')?;
    Some(&pgn[start..start + length])
}

/// The moves part: `[…]` headers, `{…}` and `;` comments and `(…)` variations taken out.
fn movetext(pgn: &str) -> String {
    let without_line_comments: String = pgn
        .lines()
        .map(|line| line.split(';').next().unwrap_or_default())
        .collect::<Vec<_>>()
        .join(" ");
    let mut kept = String::new();
    let (mut in_comment, mut in_header, mut variations) = (false, false, 0_usize);
    for c in without_line_comments.chars() {
        match c {
            '{' if !in_header => in_comment = true,
            '}' if !in_header => in_comment = false,
            '[' if !in_comment => in_header = true,
            ']' if !in_comment => in_header = false,
            '(' if !in_comment && !in_header => variations += 1,
            ')' if !in_comment && !in_header => variations = variations.saturating_sub(1),
            _ if in_comment || in_header || variations > 0 => {}
            _ => kept.push(c),
        }
    }
    kept
}

/// The moves as written, without numbers, codes, marks or the result.
fn move_words(movetext: &str) -> Vec<String> {
    movetext
        .split_whitespace()
        .map(without_move_number)
        .map(|word| word.trim_end_matches(['!', '?']))
        .filter(|word| !word.is_empty() && !word.starts_with('$'))
        .filter(|word| !matches!(*word, "1-0" | "0-1" | "1/2-1/2" | "*"))
        .map(str::to_owned)
        .collect()
}

/// `12.e4` as `e4`, and `12.` or `12...` as nothing; a word without a dot stays as it is.
fn without_move_number(word: &str) -> &str {
    match word.rfind('.') {
        Some(dot) if word[..dot].chars().all(|c| c.is_ascii_digit() || c == '.') => {
            &word[dot + 1..]
        }
        Some(_) | None => word,
    }
}

/// The legal move written `word`, check marks aside; castling with zeros too.
fn move_written(position: &Position, word: &str) -> Option<Move> {
    let wanted = word.replace('0', "O");
    let wanted = wanted.trim_end_matches(['+', '#']);
    position.legal_moves().into_iter().find(|&mv| {
        position
            .san(mv)
            .is_ok_and(|san| san.trim_end_matches(['+', '#']) == wanted)
    })
}

#[cfg(test)]
mod tests {
    use crate::{Game, PgnError, Side};

    #[test]
    fn a_site_pgn_reads_with_its_headers_comments_and_result() {
        let pgn = r#"[Event "Live Chess"]
[White "ann"]
[Black "bob"]
[Result "0-1"]

1. f3 {[%clk 0:02:59]} e5 2. g4?? (2. e4 Nf6) Qh4# $4 0-1"#;
        let game = Game::from_pgn(pgn).expect("a game");
        assert_eq!(game.sans(), ["f3", "e5", "g4", "Qh4#"]);
        assert!(game.is_over());
    }

    #[test]
    fn castling_with_zeros_and_a_set_up_start() {
        let game = Game::from_pgn("[FEN \"r3k2r/8/8/8/8/8/8/R3K2R w KQkq - 0 1\"]\n1. 0-0 0-0-0")
            .expect("a game");
        assert_eq!(game.sans(), ["O-O", "O-O-O"]);
    }

    #[test]
    fn a_wrong_move_says_where() {
        let error = Game::from_pgn("1. e4 e5 2. Ke3").expect_err("not legal");
        assert_eq!(
            error,
            PgnError::Move {
                ply: 3,
                written: "Ke3".to_owned()
            }
        );
        assert_eq!(error.to_string(), "2. Ke3 is not a legal move there");
        assert_eq!(Game::from_pgn("[Event \"x\"]"), Err(PgnError::NoMoves));
    }

    #[test]
    fn a_game_written_as_pgn_reads_back_as_the_same_game() {
        let mut game = Game::from_pgn("1. e4 e5 2. Nf3 Nc6 3. Bc4").expect("a game");
        assert_eq!(game.to_pgn(), "1. e4 e5 2. Nf3 Nc6 3. Bc4 *");
        game.resign(Side::Black).expect("a game still going");
        assert_eq!(game.to_pgn(), "1. e4 e5 2. Nf3 Nc6 3. Bc4 1-0");
        let read = Game::from_pgn(&game.to_pgn()).expect("its own pgn");
        assert_eq!(read.sans(), game.sans());
    }

    #[test]
    fn a_game_from_a_set_up_position_keeps_its_start_and_its_numbers() {
        let fen = "r3k2r/8/8/8/8/8/8/R3K2R b KQkq - 0 7";
        let game = Game::from_pgn(&format!("[FEN \"{fen}\"]\n7... O-O 8. O-O-O")).expect("a game");
        let pgn = game.to_pgn();
        assert_eq!(
            pgn,
            format!("[SetUp \"1\"]\n[FEN \"{fen}\"]\n\n7... O-O 8. O-O-O *")
        );
        assert_eq!(Game::from_pgn(&pgn), Ok(game));
    }

    #[test]
    fn a_pgn_pasted_onto_one_line_still_reads() {
        let one_line = r#"[Event "Live Chess"] [Result "0-1"] 1. f3 e5 2. g4 Qh4# 0-1"#;
        let game = Game::from_pgn(one_line).expect("a game");
        assert_eq!(game.sans().len(), 4);
    }
}
