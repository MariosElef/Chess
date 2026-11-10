//! Vasilis: bored out of his mind and thinking only of food. He does the bare minimum: he barely
//! thinks before moving, and whatever happens on the board, he does not much care.
//! Greek speakers get him in Cypriot, as a Cypriot would say it.
use std::time::Duration;

use super::{line_for, pool, speaks};
use crate::enums::play::BotMoment;

/// He spends this share of a bot's usual thinking time: a fifth.
const EFFORT_DIVISOR: u32 = 5;
/// How often he can be bothered to react when a piece is taken or a king checked.
const REACT_CHANCE: f64 = 0.4;
/// How often his mind drifts to food after any other move.
const DRIFT_CHANCE: f64 = 0.2;

const GREETINGS: &[&str] = &[
    "oh. chess. ok. is there food after this?",
    "let's make this quick, i left a souvlaki on the table",
    "*yawn* ...you go first. or me. whatever.",
    "i'm only here because the kitchen was closed",
    "can we play with pizza slices instead of pieces?",
    "TI KAMNEI REEEE",
];

const LOST_PIECE: &[&str] = &[
    "eh, keep it. i wasn't using it.",
    "that one looked like a meatball anyway",
    "ok. less pieces, less work for me.",
    "whatever. is it lunch yet?",
    "you can have it. i'd trade it for a gyro.",
];

const TOOK_PIECE: &[&str] = &[
    "oh. i took something. cool i guess.",
    "nom.",
    "didn't mean to, it was just there",
    "that's a snack",
    "yeah ok mine now. anyway, about lunch...",
];

const IN_CHECK: &[&str] = &[
    "check? ugh, now i have to move the king. effort.",
    "can't my king just take a nap",
    "fine. moving. slowly.",
    "you're making me work. not cool.",
    "TI KAMNEI REEEE",
];

const GAVE_CHECK: &[&str] = &[
    "check i think. or whatever it's called.",
    "oops, check. didn't try.",
    "check. can we finish now? i'm starving",
    "check. now i'm hungry from all this.",
];

const DRIFTING: &[&str] = &[
    "i could really go for a burger right now",
    "do you think the queen eats moussaka?",
    "*stomach growls*",
    "so bored...",
    "i'm thinking about pasta. don't mind me.",
    "is it snack time? feels like snack time",
    "this board looks like a waffle",
    "zzz... huh? oh, my move? ok.",
    "TI KAMNEI REEEE",
];

const GREETINGS_CY: &[&str] = &[
    "α. σκάκι. καλά. θα φάμεν μετά;",
    "κάμε το γρήγορα, άφηκα ένα σουβλάκι πάνω στο τραπέζι",
    "*χασμουρητό* ...ξεκίνα εσού. ή εγιώ. ό,τι.",
    "ήρτα μόνο που εκλείσαν τον φούρνον",
    "να παίξουμεν με κομμάτια πίτσα αντί πιόνια;",
    "ΤΙ ΚΑΜΝΕΙ ΡΕΕΕΕ",
];

const LOST_PIECE_CY: &[&str] = &[
    "άντε, κράτα το. εν το χρησιμοποιούσα.",
    "τζαι τούτο σαν κεφτές έμοιαζε",
    "μα εντάξει. λλιότερα κομμάτια, λλιότερη δουλειά.",
    "ό,τι. εν ώρα για φαί;",
    "πάρ' το. θα το άλλαζα με μιαν σιεφταλιά.",
];

const TOOK_PIECE_CY: &[&str] = &[
    "α. έκαμα κάτι. ωραία νομίζω.",
    "νάμ.",
    "εν το ήθελα, ήταν τζεί",
    "έτοιμο σνακ",
    "ναι εντάξει δικό μου. τέλος πάντων, για το μεσημέρι...",
];

const IN_CHECK_CY: &[&str] = &[
    "σαχ; ωχ, τζαι τώρα θα κουνήσω τον βασιλιά. κόπος.",
    "εν μπορεί ο βασιλιάς μου να πάρει έναν υπνάκον",
    "εντάξει. κουνιέμαι. σιγά σιγά.",
    "με βάλλεις να δουλέψω. εν ωραίο τούτο.",
    "ΤΙ ΚΑΜΝΕΙ ΡΕΕΕΕ",
];

const GAVE_CHECK_CY: &[&str] = &[
    "σαχ νομίζω. ή όπως αλλιώς το λαλούν.",
    "ωπ, σαχ. εν το ήθελα.",
    "σαχ. τελειώνουμεν; πεινώ",
    "σαχ. τζαι τώρα επείνασα με ούλλα τούτα.",
];

const DRIFTING_CY: &[&str] = &[
    "θα έτρωα ένα σουβλάκι τώρα",
    "η βασίλισσα τρώει μουσακά, λαλείς;",
    "*γουργουρίζει η κοιλιά μου*",
    "βαρκούμαι τόσο...",
    "σκέφτουμαι έναν φρέντο. μεν με ακούεις.",
    "ώρα για φλαούνες; μου φαίνεται ώρα για φλαούνες",
    "η σκακιέρα μοιάζει με βάφλα",
    "zzz... ε; α, η σειρά μου; εντάξει.",
    "ΤΙ ΚΑΜΝΕΙ ΡΕΕΕΕ",
];

pub(super) fn greeting(roll: f64) -> &'static str {
    line_for(pool(GREETINGS, GREETINGS_CY), roll)
}

/// Shrugs when pieces fall, otherwise now and then drifts off to food.
pub(super) fn move_line(moment: BotMoment, rolls: (f64, f64)) -> Option<&'static str> {
    let (chance, lines): (f64, &[&'static str]) = match moment {
        BotMoment::LostPiece => (REACT_CHANCE, pool(LOST_PIECE, LOST_PIECE_CY)),
        BotMoment::TookPiece => (REACT_CHANCE, pool(TOOK_PIECE, TOOK_PIECE_CY)),
        BotMoment::InCheck => (REACT_CHANCE, pool(IN_CHECK, IN_CHECK_CY)),
        BotMoment::GaveCheck => (REACT_CHANCE, pool(GAVE_CHECK, GAVE_CHECK_CY)),
        BotMoment::Quiet => (DRIFT_CHANCE, pool(DRIFTING, DRIFTING_CY)),
    };
    speaks(chance, lines, rolls)
}

/// The bare minimum: a fifth of the thought any other bot would give the move.
pub(super) fn think_time(natural: Duration) -> Duration {
    natural / EFFORT_DIVISOR
}

#[cfg(test)]
mod tests {
    use crate::{enums::settings::Language, i18n::set_current};
    use std::time::Duration;

    use super::{DRIFTING, greeting, move_line, think_time};
    use crate::enums::play::BotMoment;

    #[test]
    fn he_barely_thinks() {
        assert_eq!(
            think_time(Duration::from_secs(2)),
            Duration::from_millis(400)
        );
    }

    #[test]
    fn he_shrugs_now_and_then_and_drifts_off_to_food() {
        assert!(!greeting(0.0).is_empty());
        assert!(move_line(BotMoment::LostPiece, (0.3, 0.0)).is_some());
        assert_eq!(move_line(BotMoment::LostPiece, (0.5, 0.0)), None);
        let drift = move_line(BotMoment::Quiet, (0.1, 0.999));
        assert_eq!(drift, DRIFTING.last().copied());
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
            (super::DRIFTING, super::DRIFTING_CY),
        ];
        for (english, cypriot) in pools {
            assert!(!english.is_empty() && !cypriot.is_empty());
            assert_ne!(english, cypriot);
        }
    }

    #[test]
    fn the_cypriot_pool_is_used_when_the_app_is_in_greek() {
        set_current(Language::Greek);
        let line = super::greeting(0.0);
        set_current(Language::English);
        assert!(super::GREETINGS_CY.contains(&line));
    }
}
