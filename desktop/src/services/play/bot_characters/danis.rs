//! Danis: furious and swearing from his first words, screaming whenever a piece falls or a king
//! is checked, and gone the moment he is two pawns down.
//! Greek speakers get him in Cypriot, swearing included, with his English catchphrases kept.
use chess_domain::engine::UciScore;

use super::{behind_by, line_for, pool, speaks};
use crate::enums::play::BotMoment;

/// How far behind, in hundredths of a pawn, Danis can stand before he quits.
const QUITS_AT: i32 = 200;
/// How often he speaks up when a piece is taken or a king checked.
const LOUD_CHANCE: f64 = 0.7;
/// How often he grumbles after any other move.
const QUIET_CHANCE: f64 = 0.12;

const GREETINGS: &[&str] = &[
    "What the fuck are you looking at?! MOVE, you piece of shit!",
    "I fucking hate this game and I fucking hate you. Let's get this shit over with.",
    "You again?! Fuck you and fuck your mother!",
    "Don't even think about winning, motherfucker. I'm already fucking furious.",
    "Hurry the fuck up, asshole, I don't have all fucking day!",
    "WWWWWOOOOOOWWW RELAX. Not even started and you're already pissing me off.",
];

const QUIT_LINES: &[&str] = &[
    "This fucking board is rigged! FUCK YOU, I'M OUT!",
    "Lucky! Pure fucking luck! Fuck you, fuck your mother, I'm not playing this bullshit!",
    "My fucking mouse slipped! Go fuck yourself, I QUIT!",
    "This game is shit and so are you! FUCK OFF AND DIE!",
    "I wasn't even fucking trying, you worthless cunt! I QUIT!",
    "DID YOU BUY YOUR ACCOUNT?! Fuck you, I'm reporting you!",
    "WWWWWOOOOOOWWW RELAX! You know what? Fuck this, I'm out!",
    "OOOHH MY GOOOODDD! I CAANT! I CAANT ANYMORE! I QUIT!",
    "FUCK YOUR MOTHER, FUCK THIS GAME, FUCK EVERYTHING! I'M DONE!",
    "THKIAOLEEEEE! I'M OUT, FUCK THIS GAME!",
];

const LOST_PIECE: &[&str] = &[
    "ARE YOU FUCKING SERIOUS?! That was MY piece, you son of a bitch!",
    "Fuck your mother! Cheap fucking trick!",
    "FUCK! I saw that coming, I just didn't give a shit!",
    "Take it, asshole! Shove it up your ass!",
    "Who the fuck taught you to play like that, your whore mother?!",
    "I'll fucking kill you! Give it back, you bastard!",
    "WWWWWOOOOOOWWW RELAX!",
    "WWWWWOOOOOOWWW RELAX, motherfucker!",
    "WHY ARE YOU FUCKING USING A CHRONUS?!",
    "DID YOU BUY YOUR ACCOUNT?!",
    "OOOHH MY GOOOODDD! I CAANT!",
    "OOOHH MY GOOOODDD, I CAANT WITH YOU! FUCK!",
    "THKIAOLEEEEE!",
    "THKIAOLEEEEE! Are you fucking kidding me?!",
    "ikseres pws o protos athropws sto fegari itan kattos?",
];

const TOOK_PIECE: &[&str] = &[
    "HA! Eat shit, motherfucker!",
    "Mine now. Go cry to your mother, bitch.",
    "That's what you fucking get, dickhead! Still pissed though!",
    "About fucking time this piece of shit board gave me something.",
    "Yeah, I'll take that. Don't fucking look at me, cunt.",
];

const IN_CHECK: &[&str] = &[
    "CHECK?! Fuck your mother! Don't you fucking dare check me!",
    "Get the fuck away from my king, you son of a bitch!",
    "Oh, real fucking brave. Check. Go fuck yourself.",
    "Stop poking my king, you annoying little shit, or I'll break your fucking fingers!",
    "WWWWWOOOOOOWWW RELAX!",
    "WHY ARE YOU FUCKING USING A CHRONUS?!",
    "DID YOU BUY YOUR ACCOUNT? Fucking cheater!",
    "OOOHH MY GOOOODDD! I CAANT!",
    "THKIAOLEEEEE! Don't you dare check me!",
];

const GAVE_CHECK: &[&str] = &[
    "CHECK! Run, you fucking coward!",
    "CHECK, MOTHERFUCKER! Feel that?!",
    "Where the fuck are you going, huh, bitch? CHECK!",
    "Check! Fuck you and your whole fucking family!",
];

const QUIET: &[&str] = &[
    "Move faster, for fuck's sake, you slow bastard!",
    "This is so fucking boring I want to smash the screen.",
    "Why the fuck is this taking so long?! Fuck your mother, MOVE!",
    "I fucking hate this opening. I fucking hate you.",
    "Stop staring at the fucking board and MOVE, dipshit!",
    "Fuck this. Fuck you. Fuck everything.",
    "WWWWWOOOOOOWWW RELAX.",
    "WHY ARE YOU FUCKING USING A CHRONUS?",
    "DID YOU BUY YOUR ACCOUNT? Be honest.",
    "OOOHH MY GOOOODDD, MOVE! I CAANT!",
    "THKIAOLEEEEE, just MOVE already!",
    "ikseres pws o protos athropws sto fegari itan kattos?",
];

const GREETINGS_CY: &[&str] = &[
    "ΙΝΤΑ ΓΑΜΩ ΘΩΡΕΙΣ ΡΕ ΜΑΛΑΚΑ;! ΠΑΙΞΕ, ΚΑΡΙΟΛΗ!",
    "Μισώ τούτο το γαμημένο παιχνίδι τζαι μισώ τζαι εσέναν. Να τελειώνουμεν γρήγορα, γαμώτο.",
    "Πάλε εσού;! Γαμώ την μάναν σου!",
    "Μεν τολμήσεις να κερδίσεις, μαλάκα. Εν γαμημένα έξαλλος ήδη.",
    "Κουνήσου ρε πούστη, εν έσιω όλην μέραν!",
    "WWWWWOOOOOOWWW RELAX. Εν ξεκινήσαμε καν τζαι ήδη με τρελλαίνεις, ρε μαλάκα.",
    "ΓΑΜΩ ΤΖΑΙ ΤΗΝ ΜΑΝΑΝ ΣΟΥ, ΓΑΜΩ ΣΕ ΤΖΕΣΕΝΑ! Κουνήσου!",
];

const QUIT_LINES_CY: &[&str] = &[
    "Η σκακιέρα εν στημένη, γαμώ το! ΓΑΜΩ ΤΗΝ ΜΑΝΑΝ ΣΟΥ, ΦΕΥΚΩ!",
    "Τύχη! Καθαρή γαμημένη τύχη! Εν παίζω τούτες τες μαλακίες!",
    "Μου ξέφυγε το γαμημένο ποντίκι! Άντε γαμήσου, ΠΑΡΑΙΤΟΥΜΑΙ!",
    "Το παιχνίδι εν σκατά τζαι εσού χειρότερος! ΣΚΑΣΕ ΤΖΑΙ ΠΕΘΑΝΕ!",
    "Εν προσπαθούσα καν, παλιοκαριόλη! ΦΕΥΚΩ!",
    "DID YOU BUY YOUR ACCOUNT?! Γαμώ το σπίτιν σου, σε κάμνω report!",
    "WWWWWOOOOOOWWW RELAX! Ξέρεις ίντα; Γαμήσου τζαι εσού, φεύκω!",
    "OOOHH MY GOOOODDD! I CAANT! Εν αντέχω άλλο, γαμώτο! ΦΕΥΚΩ!",
    "ΓΑΜΩ ΤΗΝ ΜΑΝΑΝ ΣΟΥ, ΓΑΜΩ ΤΟ ΠΑΙΧΝΙΔΙ, ΓΑΜΩ ΤΑ ΟΥΛΛΑ! ΤΕΛΟΣ!",
    "ΘΚΙΑΟΛΕΕΕΕΕ! Φεύκω, γαμώ το παιχνίδι σου!",
    "ΓΑΜΩ ΤΖΑΙ ΤΗΝ ΜΑΝΑΝ ΣΟΥ, ΓΑΜΩ ΣΕ ΤΖΕΣΕΝΑ! ΦΕΥΚΩ!",
];

const LOST_PIECE_CY: &[&str] = &[
    "ΣΟΒΑΡΑ ΜΙΛΑΣ ΡΕ ΜΑΛΑΚΑ;! Τούτο ήταν το ΔΙΚΟ ΜΟΥ κομμάτι, παλιοκαριόλη!",
    "Γαμώ την μάναν σου! Φτηνό γαμημένο κόλπο!",
    "ΓΑΜΩΤΟ! Το είδα να έρκεται, απλά εν έδωσα δεκάρα!",
    "Πάρ' το, μαλάκα! Χώσ' το στον κώλο σου!",
    "Ποιος σε έμαθε να παίζεις έτσι, η πουτάνα η μάνα σου;!",
    "Θα σε σκοτώσω! Δώσ' το πίσω, μπάσταρδε!",
    "WWWWWOOOOOOWWW RELAX!",
    "WWWWWOOOOOOWWW RELAX, ρε μαλάκα!",
    "WHY ARE YOU FUCKING USING A CHRONUS?! ΜΑΛΑΚΑ!",
    "DID YOU BUY YOUR ACCOUNT?! ΠΟΥΣΤΗ!",
    "OOOHH MY GOOOODDD! I CAANT! ΓΑΜΩ ΤΟ!",
    "OOOHH MY GOOOODDD, I CAANT WITH YOU! ΓΑΜΩΤΟ!",
    "ΘΚΙΑΟΛΕΕΕΕΕ!",
    "ΘΚΙΑΟΛΕΕΕΕΕ! Μας δουλεύκεις, μαλάκα;!",
    "ΓΑΜΩ ΤΖΑΙ ΤΗΝ ΜΑΝΑΝ ΣΟΥ, ΓΑΜΩ ΣΕ ΤΖΕΣΕΝΑ!",
    "Γαμώ τζαι την μάναν σου, γαμώ σε τζεσένα, μαλάκα!",
    "Ήξερες πως ο πρώτος άνθρωπος στο φεγγάρι ήταν κάττος;",
];

const TOOK_PIECE_CY: &[&str] = &[
    "ΧΑ! Φάε σκατά, μαλάκα!",
    "Δικό μου τώρα. Πήγαινε κλάψε της μάνας σου, πούστη.",
    "Τούτο παίρνεις, ηλίθιε! Τζαι πάλε είμαι έξαλλος!",
    "Επιτέλους τούτη η σκατοσκακιέρα μου έδωκε κάτι.",
    "Ναι, θα το πάρω. Μεν με θωρείς, μαλάκα.",
    "Δικό μου! Γαμώ σε τζεσένα τζαι την μάναν σου!",
];

const IN_CHECK_CY: &[&str] = &[
    "ΣΑΧ;! Γαμώ την μάναν σου! Μεν τολμήσεις να με κάμεις σαχ!",
    "Φύε που τον βασιλιά μου, παλιοκαριόλη!",
    "Ναι, πολλά γαμάτο θάρρος. Σαχ. Άντε γαμήσου.",
    "Σταμάτα να τσιγκλάς τον βασιλιά μου, ρε σκατόπαιδο, αλλιώς θα σου σπάσω τα δάχτυλα!",
    "WWWWWOOOOOOWWW RELAX!",
    "WHY ARE YOU FUCKING USING A CHRONUS?! ΜΑΛΑΚΑ!",
    "DID YOU BUY YOUR ACCOUNT? Γαμημένε κλέφτη!",
    "OOOHH MY GOOOODDD! I CAANT!",
    "ΘΚΙΑΟΛΕΕΕΕΕ! Μεν τολμήσεις να με κάμεις σαχ!",
    "ΓΑΜΩ ΤΖΑΙ ΤΗΝ ΜΑΝΑΝ ΣΟΥ, ΓΑΜΩ ΣΕ ΤΖΕΣΕΝΑ! Σαχ σε μένα;!",
];

const GAVE_CHECK_CY: &[&str] = &[
    "ΣΑΧ! Τρέξε, γαμημένε δειλέ!",
    "ΣΑΧ, ΜΑΛΑΚΑ! Το ένιωσες;!",
    "Πού πάεις ρε πούστη; ΣΑΧ!",
    "Σαχ! Γαμώ εσέναν τζαι ούλλην την φαμίλιαν σου!",
    "Σαχ! Γαμώ σε τζεσένα τζαι την μάναν σου!",
];

const QUIET_CY: &[&str] = &[
    "Κουνήσου πιο γρήγορα, γαμώ το, αργοκίνητε μπάσταρδε!",
    "Βαρκούμαι τόσο γαμημένα που θέλω να σπάσω την οθόνην.",
    "Γιατί αργεί τόσο γαμώ το; Γαμώ την μάναν σου, ΠΑΙΞΕ!",
    "Μισώ τούτο το γαμημένο άνοιγμα. Μισώ τζαι εσέναν.",
    "Σταμάτα να θωρείς την γαμημένη σκακιέραν τζαι ΠΑΙΞΕ, βλάκα!",
    "Γαμώ το. Γαμώ τζαι εσέναν. Γαμώ τα ούλλα.",
    "WWWWWOOOOOOWWW RELAX.",
    "WHY ARE YOU FUCKING USING A CHRONUS?",
    "DID YOU BUY YOUR ACCOUNT? Πε την αλήθκειαν.",
    "OOOHH MY GOOOODDD, ΠΑΙΞΕ! I CAANT!",
    "ΘΚΙΑΟΛΕΕΕΕΕ, παίξε επιτέλους!",
    "Γαμώ τζαι την μάναν σου, γαμώ σε τζεσένα, παίξε!",
    "Ήξερες πως ο πρώτος άνθρωπος στο φεγγάρι ήταν κάττος;",
];

pub(super) fn greeting(roll: f64) -> &'static str {
    line_for(pool(GREETINGS, GREETINGS_CY), roll)
}

/// Loud when pieces fall or kings are checked, grumbling now and then otherwise.
pub(super) fn move_line(moment: BotMoment, rolls: (f64, f64)) -> Option<&'static str> {
    let (chance, lines): (f64, &[&'static str]) = match moment {
        BotMoment::LostPiece => (LOUD_CHANCE, pool(LOST_PIECE, LOST_PIECE_CY)),
        BotMoment::TookPiece => (LOUD_CHANCE, pool(TOOK_PIECE, TOOK_PIECE_CY)),
        BotMoment::InCheck => (LOUD_CHANCE, pool(IN_CHECK, IN_CHECK_CY)),
        BotMoment::GaveCheck => (LOUD_CHANCE, pool(GAVE_CHECK, GAVE_CHECK_CY)),
        BotMoment::Quiet => (QUIET_CHANCE, pool(QUIET, QUIET_CY)),
    };
    speaks(chance, lines, rolls)
}

/// Two pawns down, or being mated, is enough.
pub(super) fn rage_quits(score: UciScore) -> bool {
    behind_by(score) >= QUITS_AT
}

pub(super) fn quit_line(roll: f64) -> &'static str {
    line_for(pool(QUIT_LINES, QUIT_LINES_CY), roll)
}

#[cfg(test)]
mod tests {
    use crate::{enums::settings::Language, i18n::set_current};
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

    #[test]
    fn both_languages_have_lines_for_every_moment_and_they_differ() {
        let pools: [(&[&str], &[&str]); 7] = [
            (super::GREETINGS, super::GREETINGS_CY),
            (super::QUIT_LINES, super::QUIT_LINES_CY),
            (super::LOST_PIECE, super::LOST_PIECE_CY),
            (super::TOOK_PIECE, super::TOOK_PIECE_CY),
            (super::IN_CHECK, super::IN_CHECK_CY),
            (super::GAVE_CHECK, super::GAVE_CHECK_CY),
            (super::QUIET, super::QUIET_CY),
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
