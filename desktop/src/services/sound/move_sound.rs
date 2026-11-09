use chess_domain::{Move, Position};

use crate::{
    enums::sound::Sound,
    services::board::{is_capture, is_castle},
};

/// The sound `mv` makes: a check above all, then a promotion, a castle, a capture, else `plain`.
pub fn move_sound(before: &Position, after: &Position, mv: Move, plain: Sound) -> Sound {
    if after.is_check() {
        Sound::Check
    } else if mv.promotion.is_some() {
        Sound::Promote
    } else if is_castle(before, mv) {
        Sound::Castle
    } else if is_capture(before, mv) {
        Sound::Capture
    } else {
        plain
    }
}

#[cfg(test)]
mod tests {
    use chess_domain::{Move, PieceKind, Position};

    use super::move_sound;
    use crate::enums::sound::Sound;

    fn sound(fen: &str, uci: &str) -> Sound {
        let before: Position = fen.parse().expect("valid");
        let mv: Move = uci.parse().expect("a move");
        let after = before.play(mv).expect("legal");
        move_sound(&before, &after, mv, Sound::MoveOpponent)
    }

    #[test]
    fn a_quiet_move_makes_the_plain_sound() {
        let start = "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1";
        assert_eq!(sound(start, "e2e4"), Sound::MoveOpponent);
    }

    #[test]
    fn captures_castles_and_promotions_have_their_own() {
        assert_eq!(
            sound("4k3/8/8/3p4/4P3/8/8/4K3 w - - 0 1", "e4d5"),
            Sound::Capture
        );
        assert_eq!(
            sound("4k3/8/8/3pP3/8/8/8/4K3 w - d6 0 1", "e5d6"),
            Sound::Capture,
            "en passant takes a pawn that is not on the target"
        );
        assert_eq!(
            sound("4k3/8/8/8/8/8/8/4K2R w K - 0 1", "e1g1"),
            Sound::Castle
        );
        let promotion = Move::promoting(
            "b7".parse().expect("a square"),
            "b8".parse().expect("a square"),
            PieceKind::Knight,
        );
        let before: Position = "7k/1P6/8/8/8/8/8/K7 w - - 0 1".parse().expect("valid");
        let after = before.play(promotion).expect("legal");
        assert_eq!(
            move_sound(&before, &after, promotion, Sound::MoveSelf),
            Sound::Promote
        );
    }

    #[test]
    fn a_check_sounds_over_everything_else() {
        assert_eq!(
            sound("4k3/8/8/8/8/8/3q4/4K3 b - - 0 1", "d2d1"),
            Sound::Check
        );
        assert_eq!(
            sound("4k3/8/8/8/8/8/8/R3K3 w Q - 0 1", "e1c1"),
            Sound::Castle,
            "this castle gives no check: the rook lands on d1"
        );
    }
}
