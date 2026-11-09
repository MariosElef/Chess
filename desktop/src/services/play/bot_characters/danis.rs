//! Danis: angry from his first words, loud whenever a piece falls or a king is checked, and gone
//! the moment he is two pawns down.
use chess_domain::engine::UciScore;

use super::{behind_by, line_for, speaks};
use crate::enums::play::BotMoment;

/// How far behind, in hundredths of a pawn, Danis can stand before he quits.
const QUITS_AT: i32 = 200;
/// How often he speaks up when a piece is taken or a king checked.
const LOUD_CHANCE: f64 = 0.7;
/// How often he grumbles after any other move.
const QUIET_CHANCE: f64 = 0.12;

const GREETINGS: &[&str] = &[
    "What are you looking at? Just move.",
    "I hate this game. Let's get it over with.",
    "You again?! Ugh.",
    "Don't even think about winning. I'm already annoyed.",
    "Hurry up, I don't have all day!",
];

const QUIT_LINES: &[&str] = &[
    "This board is rigged! I'm out!",
    "Lucky! Pure luck! I'm not playing this garbage!",
    "My mouse slipped! Whatever, I quit!",
    "This game is stupid and so are you! BYE!",
    "I wasn't even trying! I QUIT!",
];

const LOST_PIECE: &[&str] = &[
    "Are you serious?! That was MY piece!",
    "Cheap trick. Really cheap.",
    "Ugh! I saw that coming, I just didn't care.",
    "Fine, take it. See if I care. (I care.)",
    "Who taught you to play like that?!",
];

const TOOK_PIECE: &[&str] = &[
    "Ha! Finally something goes right.",
    "Mine now. Stop crying.",
    "That's what you get. Still mad though.",
    "About time this board gave me something.",
    "Yeah, I'll take that. Don't look at me.",
];

const IN_CHECK: &[&str] = &[
    "Check?! Don't you dare check me!",
    "Get away from my king!",
    "Oh, real brave. Check. Wow.",
    "Stop poking my king, it's annoying!",
];

const GAVE_CHECK: &[&str] = &[
    "Check! Run, you coward!",
    "CHECK. Feel that?",
    "Where are you going, huh? Check!",
    "Check! And I'm still angry.",
];

const QUIET: &[&str] = &[
    "Move faster!",
    "This is so boring.",
    "Why is this taking so long?!",
    "I hate this opening.",
    "Stop staring at the board and MOVE.",
    "Hmph.",
];

pub(super) fn greeting(roll: f64) -> &'static str {
    line_for(GREETINGS, roll)
}

/// Loud when pieces fall or kings are checked, grumbling now and then otherwise.
pub(super) fn move_line(moment: BotMoment, rolls: (f64, f64)) -> Option<&'static str> {
    let (chance, lines): (f64, &[&'static str]) = match moment {
        BotMoment::LostPiece => (LOUD_CHANCE, LOST_PIECE),
        BotMoment::TookPiece => (LOUD_CHANCE, TOOK_PIECE),
        BotMoment::InCheck => (LOUD_CHANCE, IN_CHECK),
        BotMoment::GaveCheck => (LOUD_CHANCE, GAVE_CHECK),
        BotMoment::Quiet => (QUIET_CHANCE, QUIET),
    };
    speaks(chance, lines, rolls)
}

/// Two pawns down, or being mated, is enough.
pub(super) fn rage_quits(score: UciScore) -> bool {
    behind_by(score) >= QUITS_AT
}

pub(super) fn quit_line(roll: f64) -> &'static str {
    line_for(QUIT_LINES, roll)
}

#[cfg(test)]
mod tests {
    use chess_domain::engine::UciScore;

    use super::{LOST_PIECE, QUIT_LINES, greeting, move_line, quit_line, rage_quits};
    use crate::enums::play::BotMoment;

    #[test]
    fn he_quits_once_he_is_two_pawns_down_or_being_mated() {
        assert!(!rage_quits(UciScore::Cp(-199)));
        assert!(rage_quits(UciScore::Cp(-200)));
        assert!(rage_quits(UciScore::Mate(-3)));
        assert!(!rage_quits(UciScore::Mate(2)), "not while he mates");
        assert!(!rage_quits(UciScore::Cp(500)));
    }

    #[test]
    fn he_shouts_often_about_lost_pieces_and_grumbles_now_and_then() {
        let lost = move_line(BotMoment::LostPiece, (0.5, 0.0));
        assert_eq!(lost, LOST_PIECE.first().copied());
        assert_eq!(move_line(BotMoment::LostPiece, (0.9, 0.0)), None);
        assert!(move_line(BotMoment::Quiet, (0.05, 0.0)).is_some());
        assert_eq!(move_line(BotMoment::Quiet, (0.5, 0.0)), None);
    }

    #[test]
    fn he_greets_and_quits_with_one_of_his_lines() {
        assert!(!greeting(0.0).is_empty());
        assert_eq!(quit_line(0.999), QUIT_LINES[QUIT_LINES.len() - 1]);
    }
}
