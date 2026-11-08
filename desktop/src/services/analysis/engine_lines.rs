use chess_domain::{
    Position, Side,
    engine::{Eval, SearchInfo},
};

use crate::models::analysis::LineView;

/// How many moves of each line are written out.
const LINE_LENGTH: usize = 12;

/// One engine line in the panel: its evaluation for White, then its moves numbered.
pub fn line_view(info: &SearchInfo, analysed: &Position) -> LineView {
    let moves = &info.pv[..info.pv.len().min(LINE_LENGTH)];
    let sans = analysed.san_line(moves);
    LineView {
        eval: Eval::from_uci(info.score, analysed.side_to_move()),
        moves: numbered_line(
            analysed.counters().fullmove_number,
            analysed.side_to_move(),
            &sans,
        ),
    }
}

/// Whether the evaluation favours Black: below zero, or a mate Black delivers.
pub fn favours_black(eval: Eval) -> bool {
    match eval {
        Eval::Centipawns(centipawns) => centipawns < 0,
        Eval::Mate { winner, .. } => winner == Side::Black,
    }
}

/// Nodes per second, short: `850 kn/s`, `1.2 Mn/s`.
pub fn format_nps(nps: u64) -> String {
    if nps >= 1_000_000 {
        format!("{}.{} Mn/s", nps / 1_000_000, nps % 1_000_000 / 100_000)
    } else {
        format!("{} kn/s", nps / 1_000)
    }
}

/// Moves with their numbers from the position's move and side: `12. e4 e5 13. Nf3`.
pub fn numbered_line(fullmove: u32, side_to_move: Side, sans: &[String]) -> String {
    let mut number = fullmove;
    let mut side = side_to_move;
    let mut words = Vec::new();
    for (index, san) in sans.iter().enumerate() {
        match (side, index) {
            (Side::White, _) => words.push(format!("{number}. {san}")),
            (Side::Black, 0) => words.push(format!("{number}… {san}")),
            (Side::Black, _) => words.push(san.clone()),
        }
        if side == Side::Black {
            number += 1;
        }
        side = side.opponent();
    }
    words.join(" ")
}

#[cfg(test)]
mod tests {
    use chess_domain::Side;

    use super::{format_nps, numbered_line};

    fn sans(text: &str) -> Vec<String> {
        text.split_whitespace().map(str::to_owned).collect()
    }

    #[test]
    fn white_lines_number_every_pair() {
        assert_eq!(
            numbered_line(1, Side::White, &sans("e4 e5 Nf3")),
            "1. e4 e5 2. Nf3"
        );
    }

    #[test]
    fn black_lines_start_with_an_ellipsis() {
        assert_eq!(
            numbered_line(12, Side::Black, &sans("Nf6 Bd3 O-O")),
            "12… Nf6 13. Bd3 O-O"
        );
    }

    #[test]
    fn speeds_read_in_thousands_or_millions() {
        assert_eq!(format_nps(625_500), "625 kn/s");
        assert_eq!(format_nps(1_250_000), "1.2 Mn/s");
    }
}
