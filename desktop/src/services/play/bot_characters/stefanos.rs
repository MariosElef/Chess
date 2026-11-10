//! Stefanos: a metalhead who screams at every capture and check, and between moves cannot stop
//! talking about goth girls.
//! Greek speakers get him in Cypriot, as a Cypriot would scream it.
use super::{line_for, pool, speaks};
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
    "O VILLOS M EN MIALOS",
];

const LOST_PIECE: &[&str] = &[
    "NOOOOOOOOOOOOOO!!! 🤘💀",
    "AAAAARGHHHHHH! MY SOUL BURNS!",
    "BLEGHHHHH!!!",
    "THE DARKNESS TAKES ANOTHER ONE!!!",
    "RAAAAAAAH! That hurt more than a breakup with a goth girl.",
    "VILLON EKSANAFAEEEES?",
];

const TOOK_PIECE: &[&str] = &[
    "YEAAAAAAAAHHHHHHHH!!! 🤘🤘",
    "INTO THE ABYSS WITH YOU!!!",
    "HEADBANG TIME!!! 🤘",
    "BLEGH! BLEGH! BLEGH!",
    "SLAAAAAAYYYYYYYYYY!!!",
    "VROMO PASHAAAA",
];

const IN_CHECK: &[&str] = &[
    "WHOAAAAAAAAAAAA!!!",
    "MY KING IS IN THE MOSH PIT!!!",
    "GRAAAAAAAAH, GET OFF THE STAGE!",
    "THE WALL OF DEATH IS COMING FOR MY KING!!!",
    "VILLON EKSANAFAEEEES?",
];

const GAVE_CHECK: &[&str] = &[
    "CHEEEEEEEEEEEEEECK!!! 🤘",
    "FEEL THE BREAKDOWN!!!",
    "YOUR KING CAN'T ESCAPE THE RIFF!!!",
    "CHECK! *guitar solo intensifies*",
    "VROMO PASHAAAA",
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
    "O VILLOS M EN MIALOS",
];

const GREETINGS_CY: &[&str] = &[
    "ΕΣΟΥ ΕΤΟΙΜΟΣ ΓΙΑ ΡΟΟΟΟΚ;! 🤘",
    "Καλώς ήρτες στο PIT. Το σκακιστικό PIT. 🤘",
    "Μαύρα κομμάτια. Μαύρα ρούχα. Μαύρη ψυχή. Πάμεν.",
    "Παίζω μόνο με διπλό μπάσο στο φόντο. ΜΠΡΡΡΡΡΡΡ",
    "Ελπίζω να σου αρέσει το σκάκι ΒΑΡΥ.",
    "Ο ΒΙΛΛΟΣ Μ ΕΝ ΜΙΑΛΟΣ",
];

const LOST_PIECE_CY: &[&str] = &[
    "ΟΟΟΟΟΟΟΟΟΟΟΟΟΧΙΙΙΙ!!! 🤘💀",
    "ΑΑΑΑΑΑΑΡΡΡΡΓΧ! Η ΨΥΧΗ ΜΟΥ ΚΑΙΕΙ!",
    "ΜΠΛΕΕΕΕΕΓΧΧΧ!!!",
    "ΤΟ ΣΚΟΤΑΔΙ ΠΑΙΡΝΕΙ ΤΖΑΙ ΑΛΛΟ ΕΝΑ!!!",
    "ΡΑΑΑΑΑΑΑΧ! Επόνεσε πιο πολλά που χωρισμό με γκοθού.",
    "ΒΙΛΛΟΝ ΕΞΑΝΑΦΑΕΕΕΕΣ;",
];

const TOOK_PIECE_CY: &[&str] = &[
    "ΝΑΑΑΑΑΑΑΙΙΙΙΙΙΙ!!! 🤘🤘",
    "ΣΤΗΝ ΑΒΥΣΣΟ ΜΕ ΣΕΝΑΝ!!!",
    "ΩΡΑ ΓΙΑ HEADBANG!!! 🤘",
    "ΜΠΛΕΓΧ! ΜΠΛΕΓΧ! ΜΠΛΕΓΧ!",
    "ΣΦΑΑΑΑΑΑΑΓΗΗΗΗΗ!!!",
    "ΒΡΩΜΟ ΠΑΣΑΑΑΑ",
];

const IN_CHECK_CY: &[&str] = &[
    "ΩΩΩΩΩΩΩΩΩΩΩΩΩ!!!",
    "Ο ΒΑΣΙΛΙΑΣ ΜΟΥ ΕΜΠΗΚΕ ΣΤΟ MOSH PIT!!!",
    "ΓΡΑΑΑΑΑΑΧ, ΚΑΤΕΒΑ ΠΟΥ ΤΗΝ ΣΚΗΝΗ!",
    "ΤΟ WALL OF DEATH ΕΡΚΕΤΑΙ ΓΙΑ ΤΟΝ ΒΑΣΙΛΙΑ ΜΟΥ!!!",
    "ΒΙΛΛΟΝ ΕΞΑΝΑΦΑΕΕΕΕΣ;",
];

const GAVE_CHECK_CY: &[&str] = &[
    "ΣΑΑΑΑΑΑΑΑΑΑΧ!!! 🤘",
    "ΝΙΩΣΕ ΤΟ BREAKDOWN!!!",
    "Ο ΒΑΣΙΛΙΑΣ ΣΟΥ ΕΝ ΜΠΟΡΕΙ ΝΑ ΞΕΦΥΓΕΙ ΠΟΥ ΤΟ ΡΙΦ!!!",
    "ΣΑΧ! *το σόλο της κιθάρας δυναμώνει*",
    "ΒΡΩΜΟ ΠΑΣΑΑΑΑ",
];

const RAMBLING_CY: &[&str] = &[
    "Εγνώρισα μιαν γκοθού σε συναυλία. Είσιε τσάντα σαν νυχτερίδα. Τέλεια.",
    "Οι γκοθούδες παίζουν με τα Μαύρα. Απλά το λαλώ.",
    "Η μαύρη βασίλισσα εν ο τύπος μου. Χλωμή, μυστηριώδης, θανατηφόρα.",
    "Αν μια γκοθού μου εζητούσε να παραιτηθώ, θα παραιτούμουν. Αμέσως.",
    "Αρέσκει το σκάκι στες γκοθούδες; Ρωτώ για έναν φίλο.",
    "Σε τούτην την θέση λείπει λλίο eyeliner.",
    "Το ιδανικό ραντεβού: νεκροταφείο τα μεσάνυκτα τζαι μιαν σκακιέρα.",
    "*αέρινη κιθάρα*",
    "Ο ΒΙΛΛΟΣ Μ ΕΝ ΜΙΑΛΟΣ",
];

pub(super) fn greeting(roll: f64) -> &'static str {
    line_for(pool(GREETINGS, GREETINGS_CY), roll)
}

/// Screams at almost every capture and check; between them, goth girls.
pub(super) fn move_line(moment: BotMoment, rolls: (f64, f64)) -> Option<&'static str> {
    let (chance, lines): (f64, &[&'static str]) = match moment {
        BotMoment::LostPiece => (SCREAM_CHANCE, pool(LOST_PIECE, LOST_PIECE_CY)),
        BotMoment::TookPiece => (SCREAM_CHANCE, pool(TOOK_PIECE, TOOK_PIECE_CY)),
        BotMoment::InCheck => (SCREAM_CHANCE, pool(IN_CHECK, IN_CHECK_CY)),
        BotMoment::GaveCheck => (SCREAM_CHANCE, pool(GAVE_CHECK, GAVE_CHECK_CY)),
        BotMoment::Quiet => (RAMBLE_CHANCE, pool(RAMBLING, RAMBLING_CY)),
    };
    speaks(chance, lines, rolls)
}

#[cfg(test)]
mod tests {
    use super::{RAMBLING, TOOK_PIECE, greeting, move_line};
    use crate::enums::play::BotMoment;
    use crate::{enums::settings::Language, i18n::set_current};

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

    #[test]
    fn both_languages_have_lines_for_every_moment_and_they_differ() {
        let pools: [(&[&str], &[&str]); 6] = [
            (super::GREETINGS, super::GREETINGS_CY),
            (super::LOST_PIECE, super::LOST_PIECE_CY),
            (super::TOOK_PIECE, super::TOOK_PIECE_CY),
            (super::IN_CHECK, super::IN_CHECK_CY),
            (super::GAVE_CHECK, super::GAVE_CHECK_CY),
            (super::RAMBLING, super::RAMBLING_CY),
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
