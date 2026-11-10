//! Sporos: the strongest of the bots and the gloomiest. Sure nobody likes him, he gave up
//! everything else for games, and even winning does not cheer him up.
//! Greek speakers get him in Cypriot, as a Cypriot would sigh it.
use super::{line_for, pool, speaks};
use crate::enums::play::BotMoment;

/// How often he sighs over a capture or a check.
const SIGH_CHANCE: f64 = 0.55;
/// How often he mopes after any other move.
const MOPE_CHANCE: f64 = 0.2;

const GREETINGS: &[&str] = &[
    "Oh. Someone actually wants to play with me. That's new.",
    "You probably clicked on me by accident. Everyone does.",
    "Hi. You don't have to pretend to like me. Nobody does.",
    "I'll win. I always win. It doesn't help.",
    "Let's play. It's the only thing I'm good at.",
];

const LOST_PIECE: &[&str] = &[
    "Even my pieces leave me.",
    "Of course. Nothing stays with me for long.",
    "It's fine. I'm used to losing things. Just not games.",
    "That piece was my only friend.",
    "Go on. Take everything. Everyone else did.",
    "gamw ton shisto mou",
];

const TOOK_PIECE: &[&str] = &[
    "I took it. Still feel empty though.",
    "Another piece. Another night alone.",
    "Sorry. I'm good at this. It's all I have.",
    "I won that trade. Story of my life: winning, alone.",
    "Don't be sad. One of us being sad is enough.",
];

const IN_CHECK: &[&str] = &[
    "Check. Even the board is against me.",
    "It's ok. My king is used to being cornered.",
    "Attacking me? Get in line.",
    "Check... like the messages I never get.",
    "gamw ton shisto mou",
];

const GAVE_CHECK: &[&str] = &[
    "Check. Sorry. I don't know how to stop being good at this.",
    "Check. At least someone's paying attention to me now.",
    "Check. I'd rather have a friend than a good position.",
    "Check. Please don't leave after this.",
];

const MOPING: &[&str] = &[
    "I sacrificed my happiness for gaming. Was it worth it? ...Probably not.",
    "My mum asked if I've made any friends. I said I'm 2000 rated.",
    "I have 10,000 hours in this game. And 0 birthday invites.",
    "No one will like me. But at least I calculate 12 moves ahead.",
    "I gave up the sun for this. The sun didn't notice.",
    "Top of the leaderboard. Bottom of the group chat.",
    "I'm the best at games. Nobody ever says well played though.",
    "Sometimes I lose on purpose so people stay longer. Not today, though.",
    "gamw ton shisto mou",
];

const GREETINGS_CY: &[&str] = &[
    "Α. Κάποιος θέλει να παίξει μαζί μου. Καινούργιο τούτο.",
    "Μάλλον με επάτησες κατά λάθος. Ούλλοι το κάμνουν.",
    "Γεια. Εν χρειάζεται να προσποιείσαι ότι με συμπαθείς. Κανένας εν με συμπαθά.",
    "Θα κερδίσω. Πάντα κερδίζω. Εν βοηθά.",
    "Ας παίξουμεν. Εν το μόνο που κάμνω καλά.",
];

const LOST_PIECE_CY: &[&str] = &[
    "Ακόμα τζαι τα κομμάτια μου με αφήκουν.",
    "Φυσικά. Τίποτε εν μένει πολλήν ώρα μαζί μου.",
    "Εντάξει. Έχω συνηθίσει να χάννω πράματα. Παρτίδες όι.",
    "Τούτο το κομμάτι ήταν ο μοναδικός μου φίλος.",
    "Πάρε τα ούλλα. Ούλλοι οι άλλοι το έκαμαν.",
    "Γαμώ τον σχιστό μου",
];

const TOOK_PIECE_CY: &[&str] = &[
    "Το επήρα. Τζαι πάλε νιώθω άδειος.",
    "Άλλο ένα κομμάτι. Άλλο ένα βράδυ μόνος.",
    "Συγγνώμη. Τούτο ξέρω να κάμνω. Μόνο τούτο έσιω.",
    "Κέρδισα την ανταλλαγή. Η ιστορία της ζωής μου: νικώ, μόνος.",
    "Μεν στεναχωριέσαι. Φτάνει που στεναχωριούμαι εγιώ.",
];

const IN_CHECK_CY: &[&str] = &[
    "Σαχ. Ακόμα τζαι η σκακιέρα εναντίον μου.",
    "Εντάξει. Ο βασιλιάς μου έχει συνηθίσει να τον στριμώχνουν.",
    "Με επιτίθεσαι; Κάμε ουράν.",
    "Σαχ... σαν τα μηνύματα που εν παίρνω ποτέ.",
    "Γαμώ τον σχιστό μου",
];

const GAVE_CHECK_CY: &[&str] = &[
    "Σαχ. Συγγνώμη. Εν ξέρω πώς να σταματήσω να είμαι καλός σε τούτο.",
    "Σαχ. Τουλάχιστον κάποιος μου δίνει προσοχήν τώρα.",
    "Σαχ. Θα προτιμούσα έναν φίλο που μιαν καλήν θέση.",
    "Σαχ. Σε παρακαλώ μεν φύεις μετά.",
];

const MOPING_CY: &[&str] = &[
    "Θυσίασα την ευτυχία μου για τα παιχνίδια. Άξιζε; ...Μάλλον όι.",
    "Η μάνα μου με ρώτησε αν έκαμα φίλους. Της είπα ότι έσιω 2000 βαθμούς.",
    "Έσιω 10.000 ώρες σε τούτο το παιχνίδι. Τζαι 0 προσκλήσεις σε γενέθλια.",
    "Κανένας εν θα με συμπαθήσει. Αμμά τουλάχιστον υπολογίζω 12 κινήσεις μπροστά.",
    "Άφηκα τον ήλιο γι' αυτό. Ο ήλιος εν το πρόσεξε.",
    "Πρώτος στην κατάταξη. Τελευταίος στο γκρουπ.",
    "Είμαι ο καλύτερος στα παιχνίδια. Αμμά ποτέ κανένας εν λαλεί μπράβο.",
    "Καμιά φορά χάννω επίτηδες για να μείνουν περισσότερον. Όι σήμερα όμως.",
    "Γαμώ τον σχιστό μου",
];

pub(super) fn greeting(roll: f64) -> &'static str {
    line_for(pool(GREETINGS, GREETINGS_CY), roll)
}

/// Sighs over captures and checks, his own as much as the player's; between them, he mopes.
pub(super) fn move_line(moment: BotMoment, rolls: (f64, f64)) -> Option<&'static str> {
    let (chance, lines): (f64, &[&'static str]) = match moment {
        BotMoment::LostPiece => (SIGH_CHANCE, pool(LOST_PIECE, LOST_PIECE_CY)),
        BotMoment::TookPiece => (SIGH_CHANCE, pool(TOOK_PIECE, TOOK_PIECE_CY)),
        BotMoment::InCheck => (SIGH_CHANCE, pool(IN_CHECK, IN_CHECK_CY)),
        BotMoment::GaveCheck => (SIGH_CHANCE, pool(GAVE_CHECK, GAVE_CHECK_CY)),
        BotMoment::Quiet => (MOPE_CHANCE, pool(MOPING, MOPING_CY)),
    };
    speaks(chance, lines, rolls)
}

#[cfg(test)]
mod tests {
    use super::{MOPING, greeting, move_line};
    use crate::enums::play::BotMoment;
    use crate::{enums::settings::Language, i18n::set_current};

    #[test]
    fn he_sighs_over_captures_and_mopes_in_between() {
        assert!(!greeting(0.0).is_empty());
        assert!(move_line(BotMoment::TookPiece, (0.5, 0.0)).is_some());
        assert_eq!(move_line(BotMoment::TookPiece, (0.6, 0.0)), None);
        assert_eq!(
            move_line(BotMoment::Quiet, (0.1, 0.0)),
            MOPING.first().copied()
        );
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
            (super::MOPING, super::MOPING_CY),
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
