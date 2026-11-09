//! Stefanos: a metalhead who screams at every capture and check, and between moves cannot stop
//! talking about goth girls.
use super::{line_for, speaks};
use crate::enums::play::BotMoment;

/// How often he screams when a piece is taken or a king checked: nearly always.
const SCREAM_CHANCE: f64 = 0.85;
/// How often he rambles after any other move.
const RAMBLE_CHANCE: f64 = 0.25;

const GREETINGS: &[&str] = &[
    "ARE YOU READY TO ROOOOCK?! 🤘",
    "Welcome to the PIT. The chess pit. 🤘",
    "Black pieces. Black clothes. Black soul. Let's go.",
    "I only play to the sound of double bass drums. BRRRRRRRR",
    "Hope you like your chess HEAVY.",
];

const LOST_PIECE: &[&str] = &[
    "NOOOOOOOOOOOOOO!!! 🤘💀",
    "AAAAARGHHHHHH! MY SOUL BURNS!",
    "BLEGHHHHH!!!",
    "THE DARKNESS TAKES ANOTHER ONE!!!",
    "RAAAAAAAH! That hurt more than a breakup with a goth girl.",
];

const TOOK_PIECE: &[&str] = &[
    "YEAAAAAAAAHHHHHHHH!!! 🤘🤘",
    "INTO THE ABYSS WITH YOU!!!",
    "HEADBANG TIME!!! 🤘",
    "BLEGH! BLEGH! BLEGH!",
    "SLAAAAAAYYYYYYYYYY!!!",
];

const IN_CHECK: &[&str] = &[
    "WHOAAAAAAAAAAAA!!!",
    "MY KING IS IN THE MOSH PIT!!!",
    "GRAAAAAAAAH, GET OFF THE STAGE!",
    "THE WALL OF DEATH IS COMING FOR MY KING!!!",
];

const GAVE_CHECK: &[&str] = &[
    "CHEEEEEEEEEEEEEECK!!! 🤘",
    "FEEL THE BREAKDOWN!!!",
    "YOUR KING CAN'T ESCAPE THE RIFF!!!",
    "CHECK! *guitar solo intensifies*",
];

const RAMBLING: &[&str] = &[
    "Met a goth girl at a concert once. She had a bat-shaped purse. Perfect.",
    "Goth girls play the Black side. Just saying.",
    "The black queen is my type. Pale, mysterious, deadly.",
    "If a goth girl asked me to resign, I would. Instantly.",
    "Do goth girls like chess? Asking for me.",
    "This position needs more eyeliner.",
    "My dream date: a cemetery at midnight and a chess board.",
    "*air guitar*",
];

pub(super) fn greeting(roll: f64) -> &'static str {
    line_for(GREETINGS, roll)
}

/// Screams at almost every capture and check; between them, goth girls.
pub(super) fn move_line(moment: BotMoment, rolls: (f64, f64)) -> Option<&'static str> {
    let (chance, lines): (f64, &[&'static str]) = match moment {
        BotMoment::LostPiece => (SCREAM_CHANCE, LOST_PIECE),
        BotMoment::TookPiece => (SCREAM_CHANCE, TOOK_PIECE),
        BotMoment::InCheck => (SCREAM_CHANCE, IN_CHECK),
        BotMoment::GaveCheck => (SCREAM_CHANCE, GAVE_CHECK),
        BotMoment::Quiet => (RAMBLE_CHANCE, RAMBLING),
    };
    speaks(chance, lines, rolls)
}

#[cfg(test)]
mod tests {
    use super::{RAMBLING, TOOK_PIECE, greeting, move_line};
    use crate::enums::play::BotMoment;

    #[test]
    fn he_screams_at_nearly_every_capture_and_rambles_in_between() {
        assert!(!greeting(0.0).is_empty());
        let scream = move_line(BotMoment::TookPiece, (0.8, 0.0));
        assert_eq!(scream, TOOK_PIECE.first().copied());
        assert_eq!(move_line(BotMoment::TookPiece, (0.9, 0.0)), None);
        let ramble = move_line(BotMoment::Quiet, (0.2, 0.999));
        assert_eq!(ramble, RAMBLING.last().copied());
        assert_eq!(move_line(BotMoment::Quiet, (0.3, 0.0)), None);
    }
}
