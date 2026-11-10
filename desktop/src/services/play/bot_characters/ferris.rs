//! Ferris: the Rust crab, the strongest of the bots, who sees the board as a program. He speaks
//! in borrow checker errors and ownership rules, clicks his claws, and moves blazingly fast.
//! Greek speakers get him in Cypriot, his Rust kept as Rust.
use std::time::Duration;

use super::{line_for, pool, speaks};
use crate::enums::play::BotMoment;

/// Blazingly fast: half the thought any other bot would give the move.
const SPEED_DIVISOR: u32 = 2;
/// How often he speaks up when a piece is taken or a king checked.
const REACT_CHANCE: f64 = 0.5;
/// How often he muses after any other move.
const MUSE_CHANCE: f64 = 0.2;

const GREETINGS: &[&str] = &[
    "Hello, fellow Rustacean! *clicks claws* Let's play.",
    "I've been compiled with --release. Good luck.",
    "Ownership rules apply: every piece has exactly one owner. Me.",
    "Fearless concurrency. Fearless chess.",
    "*clicks claws* Memory-safe chess only, please.",
];

const LOST_PIECE: &[&str] = &[
    "error[E0382]: use of moved value: my knight",
    "That piece went out of scope. Dropped. Freed.",
    "thread 'main' panicked at 'lost a bishop'",
    "Huh. Even the borrow checker didn't see that coming.",
    "Fine. It's not a memory leak if you took it.",
];

const TOOK_PIECE: &[&str] = &[
    "Your piece has been moved. You can no longer use it.",
    "Ownership transferred. To me.",
    "Zero-cost abstraction: it cost you a piece.",
    "*snip snip*",
    "Dropped, like a value going out of scope.",
];

const IN_CHECK: &[&str] = &[
    "Check? My king is behind a Mutex. Good luck locking it.",
    "I'll just .unwrap() my way out of this.",
    "This is unsafe { } territory.",
    "warning: unused king safety. Noted.",
];

const GAVE_CHECK: &[&str] = &[
    "Check! Your king does not implement Escape.",
    "Check. The compiler is never wrong.",
    "*clicks claws* Check!",
    "cannot borrow `king` as mutable: it is in check",
];

const MUSING: &[&str] = &[
    "Have you considered rewriting your opening in Rust?",
    "cargo check... all clear.",
    "Blazingly fast. The move, I mean.",
    "I only walk sideways. My bishops get it.",
    "No garbage collector here. Just good moves.",
    "Crabs evolved five separate times. That's how good crabs are.",
    "unsafe { play_your_move() }",
    "My moves have a 'static lifetime.",
];

const GREETINGS_CY: &[&str] = &[
    "Γεια σου, Rustacean! *κλικ κλακ* Πάμεν να παίξουμεν.",
    "Εκάμαν με compile με --release. Καλή τύχη σου.",
    "Κανόνες ownership: κάθε κομμάτι έσιει έναν ιδιοκτήτη. Εμέναν.",
    "Fearless concurrency, άφοβο σκάκι.",
    "*κλικ κλακ* Σκάκι μόνο με ασφάλεια μνήμης, παρακαλώ.",
];

const LOST_PIECE_CY: &[&str] = &[
    "error[E0382]: use of moved value: το άλογο μου",
    "Τούτο το κομμάτι εβγήκε που το scope. Ελευθερώθηκε.",
    "thread 'main' panicked: έχασα αξιωματικό",
    "Ούτε ο borrow checker εν το είδε τούτο.",
    "Εντάξει. Εν είναι memory leak αφού το πήρες εσού.",
];

const TOOK_PIECE_CY: &[&str] = &[
    "Το κομμάτι σου έκαμε move. Εν μπορείς να το χρησιμοποιήσεις άλλο.",
    "Ownership transferred. Σε μένα.",
    "Zero-cost abstraction: εκόστισε σου ένα κομμάτι.",
    "*σνιπ σνιπ*",
    "Dropped, όπως μια τιμή που βγαίνει που το scope.",
];

const IN_CHECK_CY: &[&str] = &[
    "Σαχ; Ο βασιλιάς μου εν πίσω που Mutex. Καλή τύχη.",
    "Θα κάμω .unwrap() τζαι θα φύω.",
    "Τούτο εν unsafe { } έδαφος.",
    "warning: unused ασφάλεια βασιλιά. Εσημείωσα το.",
];

const GAVE_CHECK_CY: &[&str] = &[
    "Σαχ! Ο βασιλιάς σου εν κάμνει implement Escape.",
    "Σαχ. Ο compiler εν κάμνει ποττέ λάθος.",
    "*κλικ κλακ* Σαχ!",
    "cannot borrow `king` as mutable: εν σε σαχ",
];

const MUSING_CY: &[&str] = &[
    "Εσκέφτηκες να ξαναγράψεις το άνοιγμα σου σε Rust;",
    "cargo check... ούλλα καλά.",
    "Blazingly fast. Η κίνηση, εννοώ.",
    "Περπατώ μόνο πλάγια. Οι αξιωματικοί μου καταλάβουν.",
    "Εν έχουμε garbage collector δαμέ. Μόνο καλές κινήσεις.",
    "Τα καβούρια εξελίχθηκαν πέντε φορές. Τόσο καλά είμαστεν.",
    "unsafe { παίξε_την_κίνηση_σου() }",
    "Οι κινήσεις μου έχουν lifetime 'static.",
];

pub(super) fn greeting(roll: f64) -> &'static str {
    line_for(pool(GREETINGS, GREETINGS_CY), roll)
}

/// Compiler errors when pieces fall or kings are checked, and Rust musings now and then.
pub(super) fn move_line(moment: BotMoment, rolls: (f64, f64)) -> Option<&'static str> {
    let (chance, lines): (f64, &[&'static str]) = match moment {
        BotMoment::LostPiece => (REACT_CHANCE, pool(LOST_PIECE, LOST_PIECE_CY)),
        BotMoment::TookPiece => (REACT_CHANCE, pool(TOOK_PIECE, TOOK_PIECE_CY)),
        BotMoment::InCheck => (REACT_CHANCE, pool(IN_CHECK, IN_CHECK_CY)),
        BotMoment::GaveCheck => (REACT_CHANCE, pool(GAVE_CHECK, GAVE_CHECK_CY)),
        BotMoment::Quiet => (MUSE_CHANCE, pool(MUSING, MUSING_CY)),
    };
    speaks(chance, lines, rolls)
}

/// Blazingly fast: half the thought any other bot would give the move.
pub(super) fn think_time(natural: Duration) -> Duration {
    natural / SPEED_DIVISOR
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::{MUSING, greeting, move_line, think_time};
    use crate::{
        enums::{play::BotMoment, settings::Language},
        i18n::set_current,
    };

    #[test]
    fn he_moves_blazingly_fast() {
        assert_eq!(think_time(Duration::from_secs(2)), Duration::from_secs(1));
    }

    #[test]
    fn he_reacts_to_half_the_captures_and_checks_and_muses_now_and_then() {
        assert!(!greeting(0.0).is_empty());
        assert!(move_line(BotMoment::LostPiece, (0.4, 0.0)).is_some());
        assert_eq!(move_line(BotMoment::GaveCheck, (0.6, 0.0)), None);
        let musing = move_line(BotMoment::Quiet, (0.1, 0.999));
        assert_eq!(musing, MUSING.last().copied());
        assert_eq!(move_line(BotMoment::Quiet, (0.3, 0.0)), None);
    }

    #[test]
    fn both_languages_have_lines_for_every_moment_and_they_differ() {
        let pools: [(&[&str], &[&str]); 6] = [
            (super::GREETINGS, super::GREETINGS_CY),
            (super::LOST_PIECE, super::LOST_PIECE_CY),
            (super::TOOK_PIECE, super::TOOK_PIECE_CY),
            (super::IN_CHECK, super::IN_CHECK_CY),
            (super::GAVE_CHECK, super::GAVE_CHECK_CY),
            (super::MUSING, super::MUSING_CY),
        ];
        for (english, cypriot) in pools {
            assert!(!english.is_empty() && !cypriot.is_empty());
            assert_ne!(english, cypriot);
        }
    }

    #[test]
    fn the_cypriot_pool_is_used_when_the_app_is_in_greek() {
        set_current(Language::Greek);
        let line = greeting(0.0);
        set_current(Language::English);
        assert!(super::GREETINGS_CY.contains(&line));
    }
}
