//! Andreas: gambles on everything, checks his kouponoui between moves, and once the game turns
//! against him, tries to buy his way out with a deal: a draw offer, sweetened.
//! Greek speakers get him in Cypriot, with Cypriot football and food.
use chess_domain::engine::UciScore;

use super::{behind_by, line_for, pool};
use crate::enums::play::BotMoment;

/// How far behind, in hundredths of a pawn, he has to be before he starts making deals.
const DEALS_FROM: i32 = 100;
/// How often, once behind, he tries a deal on his move.
const DEAL_CHANCE: f64 = 0.35;
/// How often he bets on a capture or a check.
const BET_CHANCE: f64 = 0.6;
/// How often he talks about his kouponoui after any other move.
const KOUPONOUI_CHANCE: f64 = 0.2;

const GREETINGS: &[&str] = &[
    "5 euros says I win this. You in?",
    "Quick game, I need to fill my kouponoui before kick-off.",
    "Let's make it interesting. Loser buys halloumi.",
    "I have a system for chess. Same as my system for the kouponoui. It never fails. Usually.",
];

const LOST_PIECE: &[&str] = &[
    "I had 20 euros on that knight!",
    "No problem. I'll win it back. I always win it back.",
    "That's fine, that's fine... double or nothing on the next one?",
    "Ugh. Like Apoelli in the 90th minute.",
    "*SLAM* THAT WAS MY RENT ON THAT KNIGHT! ...oh no. My keyboard.",
    "gamw tin mana tou kosma",
];

const TOOK_PIECE: &[&str] = &[
    "CHA-CHING! 💰",
    "Jackpot! Put it on my tab.",
    "That's going straight on the kouponoui.",
    "See? The system works!",
    "Winner winner, halloumi dinner!",
];

const IN_CHECK: &[&str] = &[
    "Check? Let's not be hasty. Can we talk about this?",
    "Ok, ok, I'll give you 10 euros to take that back.",
    "Didn't see that on the odds sheet.",
    "My king is good for it, I promise!",
    "CHECK?! *BANG!* I hit the desk and my keyboard just broke. Worth it.",
    "gamw tin mana tou kosma",
];

const GAVE_CHECK: &[&str] = &[
    "Check! Who wants to bet on what happens next?",
    "Check! Odds just went up, my friend.",
    "Check! I called that one. Ask anyone.",
    "Check! Put another 5 on me.",
];

const KOUPONOUI: &[&str] = &[
    "Hold on, checking my kouponoui... Apoelli scored!",
    "Over 2.5 goals AND both teams to score. Easy money.",
    "One more match and the kouponoui pays out. Just one more.",
    "I bet my car on this game. Don't tell my wife.",
    "Want to bet on which pawn moves next?",
    "If Apoellin draw tonight, I'm buying everyone souvla.",
    "My kouponoui needs 12 out of 12. I've got 11. It's happening.",
    "Psst. I'll give you a tip for the weekend. 1X2, trust me.",
];

const DEALS: &[&str] = &[
    "Listen, listen... draw, and I fill your kouponoui for the weekend. Deal?",
    "Let's call it a draw and split the pot. Fair, no?",
    "Draw now and I'll owe you 50 euros. I'm good for it!",
    "Draw, and I'll tell you my sure thing for Saturday.",
    "Tell you what: draw, and the halloumi is on me.",
    "Draw? I'll throw in my lucky pen. It's very lucky.",
];

const TAKEN: &[&str] = &[
    "Pleasure doing business! 🤝",
    "Smart move. The house always wins.",
    "Deal! The money is... coming. Next week.",
    "Haha! You'll get your kouponoui. Probably.",
];

const REFUSED: &[&str] = &[
    "Your loss! Double or nothing next game?",
    "Fine, fine. Offer stays open. Price goes up.",
    "Nobody turns down a deal from Andreas! ...ok, you did.",
    "Ok, but don't come crying when my kouponoui wins.",
    "*punches the desk, the keyboard snaps in half* FINE! The offer stays open!",
];

const GREETINGS_CY: &[&str] = &[
    "5 ευρώ λαλώ κερδίζω εγιώ. Μπαίνεις;",
    "Γρήγορη παρτίδα, να παίξω το κουπόνι μου πριν ξεκινήσει το ματς.",
    "Να το κάμουμεν ενδιαφέρον. Ο χαμένος κερνά χαλλούμι.",
    "Έσιω σύστημα τζαι για το σκάκι. Το ίδιο με του κουπονιού. Εν αποτυχαίνει ποτέ. Σχεδόν.",
];

const LOST_PIECE_CY: &[&str] = &[
    "Είσια 20 ευρώ πάνω σε τούτο το άλογο!",
    "Κανένα πρόβλημα. Θα τα ξαναπάρω. Πάντα τα παίρνω πίσω.",
    "Εντάξει, εντάξει... διπλό ή τίποτε στο επόμενο;",
    "Άχ. Σαν τον ΑΠΟΕΛ στο 90ο λεπτό.",
    "*ΜΠΑΜ* ΤΟ ΕΝΟΙΚΙΟ ΜΟΥ ΗΤΑΝ ΠΑΝΩ ΣΕ ΤΟΥΤΟ ΤΟ ΑΛΟΓΟ! ...α, το πληκτρολόγιο μου.",
    "Γαμώ την μάνα του Κοσμά",
];

const TOOK_PIECE_CY: &[&str] = &[
    "ΤΣΙΝΓΚ-ΤΣΙΝΓΚ! 💰",
    "Τζάκποτ! Γράψε το στο καρνέ μου.",
    "Τούτο πάει κατευθείαν στο κουπόνι.",
    "Είδες; Το σύστημα δουλεύκει!",
    "Κέρδισα! Σιεφταλιές για ούλλους!",
];

const IN_CHECK_CY: &[&str] = &[
    "Σαχ; Μεν βιαζόμαστεν. Να το συζητήσουμεν;",
    "Εντάξει, εντάξει, σου δίνω 10 ευρώ να το πάρεις πίσω.",
    "Τούτο εν ήταν στο δελτίο με τες αποδόσεις.",
    "Ο βασιλιάς μου εν εγγυητής, στο υπόσχουμαι!",
    "ΣΑΧ;! *ΜΠΑΜ!* Κοπάνησα το γραφείο τζαι έσπασε το πληκτρολόγιο μου. Άξιζε.",
    "Γαμώ την μάνα του Κοσμά",
];

const GAVE_CHECK_CY: &[&str] = &[
    "Σαχ! Ποιος στοιχηματίζει ίντα θα γίνει τώρα;",
    "Σαχ! Μόλις ανέβηκε η απόδοση, φίλε μου.",
    "Σαχ! Το είπα εγιώ. Ρώτα όποιον θέλεις.",
    "Σαχ! Βάλε άλλα 5 πάνω μου.",
];

const KOUPONOUI_CY: &[&str] = &[
    "Μισό λεπτό, θωρώ το κουπόνι μου... Έβαλεν ο ΑΠΟΕΛ γκολ!",
    "Over 2,5 γκολ ΚΑΙ γκολ τζαι οι δυο. Εύκολα λεφτά.",
    "Ένα ματς ακόμα τζαι το κουπόνι πληρώνει. Μόνο ένα.",
    "Στοίχημα το αμάξι μου σε τούτη την παρτίδα. Μεν το πεις της γυναίκας μου.",
    "Στοιχηματίζεις ποιο πιόνι θα κουνηθεί μετά;",
    "Αν φέρει ισοπαλία ο ΑΠΟΕΛ απόψε, κερνώ σιεφταλιές ούλλους.",
    "Το κουπόνι θέλει 12 στα 12. Έσιω 11. Θα γίνει.",
    "Ψιτ. Σου δίνω ένα tip για το Σαββατοκύριακον. 1Χ2, πίστεψέ με.",
];

const DEALS_CY: &[&str] = &[
    "Άκου, άκου... ισοπαλία τζαι σου γεμίζω το κουπόνι για το Σαββατοκύριακον. Deal;",
    "Ισοπαλία τζαι μοιραζόμαστεν το ποτ. Δίκαιο, ε;",
    "Ισοπαλία τώρα τζαι σου χρωστώ 50 ευρώ. Εγιώ εν καλός για τα λεφτά!",
    "Ισοπαλία, τζαι σου λαλώ το σίγουρο για το Σάββατο.",
    "Ξέρεις ίντα; Ισοπαλία τζαι ο καφές εν δικός μου.",
    "Ισοπαλία; Σου βάλλω τζαι το γούρικο στυλό μου. Πολλά γούρικο.",
];

const TAKEN_CY: &[&str] = &[
    "Χαρά μου που κάμαμε δουλειά! 🤝",
    "Έξυπνη κίνηση. Το μαγαζί πάντα κερδίζει.",
    "Deal! Τα λεφτά... έρκουνται. Την άλλη βδομάδα.",
    "Χαχά! Θα πάρεις το κουπόνι σου. Μάλλον.",
];

const REFUSED_CY: &[&str] = &[
    "Ζημιά σου! Διπλό ή τίποτε στην επόμενη παρτίδα;",
    "Εντάξει, εντάξει. Η προσφορά μένει. Η τιμή ανεβαίνει.",
    "Κανένας εν αρνείται deal του Ανδρέα! ...καλά, εσού.",
    "Εντάξει, αμμά μεν κλαίεις όταν κερδίσει το κουπόνι μου.",
    "*μπουνιά στο γραφείο, το πληκτρολόγιο σπά στα δύο* ΕΝΤΑΞΕΙ! Η προσφορά μένει!",
];

pub(super) fn greeting(roll: f64) -> &'static str {
    line_for(pool(GREETINGS, GREETINGS_CY), roll)
}

/// Bets on captures and checks; between them, his kouponoui.
pub(super) fn move_line(
    moment: BotMoment,
    (talk_roll, pick_roll): (f64, f64),
) -> Option<&'static str> {
    let (chance, lines): (f64, &[&'static str]) = match moment {
        BotMoment::LostPiece => (BET_CHANCE, pool(LOST_PIECE, LOST_PIECE_CY)),
        BotMoment::TookPiece => (BET_CHANCE, pool(TOOK_PIECE, TOOK_PIECE_CY)),
        BotMoment::InCheck => (BET_CHANCE, pool(IN_CHECK, IN_CHECK_CY)),
        BotMoment::GaveCheck => (BET_CHANCE, pool(GAVE_CHECK, GAVE_CHECK_CY)),
        BotMoment::Quiet => (KOUPONOUI_CHANCE, pool(KOUPONOUI, KOUPONOUI_CY)),
    };
    (talk_roll < chance).then(|| line_for(lines, pick_roll))
}

/// Once a pawn behind, he tries a deal now and then.
pub(super) fn offers_deal(score: UciScore, roll: f64) -> bool {
    behind_by(score) >= DEALS_FROM && roll < DEAL_CHANCE
}

pub(super) fn deal_line(roll: f64) -> &'static str {
    line_for(pool(DEALS, DEALS_CY), roll)
}

pub(super) fn deal_answer(taken: bool, roll: f64) -> &'static str {
    if taken {
        line_for(pool(TAKEN, TAKEN_CY), roll)
    } else {
        line_for(pool(REFUSED, REFUSED_CY), roll)
    }
}

#[cfg(test)]
mod tests {
    use crate::{enums::settings::Language, i18n::set_current};
    use chess_domain::engine::UciScore;

    use super::{REFUSED, TAKEN, deal_answer, move_line, offers_deal};
    use crate::enums::play::BotMoment;

    #[test]
    fn he_deals_only_once_he_is_behind_and_then_now_and_then() {
        assert!(!offers_deal(UciScore::Cp(0), 0.0), "not while level");
        assert!(!offers_deal(UciScore::Cp(-99), 0.0));
        assert!(offers_deal(UciScore::Cp(-100), 0.3));
        assert!(!offers_deal(UciScore::Cp(-100), 0.5), "not every move");
        assert!(offers_deal(UciScore::Mate(-2), 0.0));
    }

    #[test]
    fn he_answers_a_taken_or_refused_deal() {
        assert_eq!(deal_answer(true, 0.0), TAKEN[0]);
        assert_eq!(deal_answer(false, 0.0), REFUSED[0]);
    }

    #[test]
    fn he_bets_on_captures_and_talks_kouponoui_in_between() {
        assert!(move_line(BotMoment::TookPiece, (0.5, 0.0)).is_some());
        assert_eq!(move_line(BotMoment::TookPiece, (0.7, 0.0)), None);
        assert!(move_line(BotMoment::Quiet, (0.1, 0.0)).is_some());
        assert_eq!(move_line(BotMoment::Quiet, (0.3, 0.0)), None);
    }

    #[test]
    fn both_languages_have_lines_for_every_moment_and_they_differ() {
        let pools: [(&[&str], &[&str]); 9] = [
            (super::GREETINGS, super::GREETINGS_CY),
            (super::LOST_PIECE, super::LOST_PIECE_CY),
            (super::TOOK_PIECE, super::TOOK_PIECE_CY),
            (super::IN_CHECK, super::IN_CHECK_CY),
            (super::GAVE_CHECK, super::GAVE_CHECK_CY),
            (super::KOUPONOUI, super::KOUPONOUI_CY),
            (super::DEALS, super::DEALS_CY),
            (super::TAKEN, super::TAKEN_CY),
            (super::REFUSED, super::REFUSED_CY),
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
