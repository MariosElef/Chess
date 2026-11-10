//! What each bot says and does at the board, beyond choosing its moves: one file per bot, and
//! here each question sent to the bot that answers it.
//! Randomness comes in as plain numbers from 0 to 1, as for the bots' moves.
mod andreas;
mod danis;
mod sporos;
mod stefanos;
mod vasilis;

use std::time::Duration;

use chess_domain::{Move, Position, Side, engine::UciScore};

use super::bots::index_for;
use crate::{
    enums::{
        play::{BotCharacter, BotMoment},
        settings::Language,
    },
    i18n,
    services::board::is_capture,
};

/// What the bot says as the game starts, if it says anything.
pub fn greeting(character: Option<BotCharacter>, roll: f64) -> Option<&'static str> {
    match character? {
        BotCharacter::Danis => Some(danis::greeting(roll)),
        BotCharacter::Vasilis => Some(vasilis::greeting(roll)),
        BotCharacter::Stefanos => Some(stefanos::greeting(roll)),
        BotCharacter::Andreas => Some(andreas::greeting(roll)),
        BotCharacter::Sporos => Some(sporos::greeting(roll)),
    }
}

/// What the bot says about `moment`, if it speaks up: the first roll decides whether, the second
/// which line.
pub fn move_line(
    character: Option<BotCharacter>,
    moment: BotMoment,
    rolls: (f64, f64),
) -> Option<&'static str> {
    match character? {
        BotCharacter::Danis => danis::move_line(moment, rolls),
        BotCharacter::Vasilis => vasilis::move_line(moment, rolls),
        BotCharacter::Stefanos => stefanos::move_line(moment, rolls),
        BotCharacter::Andreas => andreas::move_line(moment, rolls),
        BotCharacter::Sporos => sporos::move_line(moment, rolls),
    }
}

/// Whether the bot walks out of the game, given its engine's score from its own side.
pub fn rage_quits(character: Option<BotCharacter>, score: UciScore) -> bool {
    match character {
        Some(BotCharacter::Danis) => danis::rage_quits(score),
        Some(
            BotCharacter::Vasilis
            | BotCharacter::Stefanos
            | BotCharacter::Andreas
            | BotCharacter::Sporos,
        )
        | None => false,
    }
}

/// What the bot shouts as it walks out.
pub fn quit_line(character: Option<BotCharacter>, roll: f64) -> Option<&'static str> {
    match character? {
        BotCharacter::Danis => Some(danis::quit_line(roll)),
        BotCharacter::Vasilis
        | BotCharacter::Stefanos
        | BotCharacter::Andreas
        | BotCharacter::Sporos => None,
    }
}

/// The deal the bot offers with its move, if it makes one: a draw offer, with what it says.
/// `score` is its engine's, from its own side; the first roll decides whether, the second
/// which line.
pub fn deal_offer(
    character: Option<BotCharacter>,
    score: UciScore,
    (deal_roll, pick_roll): (f64, f64),
) -> Option<&'static str> {
    match character? {
        BotCharacter::Andreas => {
            andreas::offers_deal(score, deal_roll).then(|| andreas::deal_line(pick_roll))
        }
        BotCharacter::Vasilis
        | BotCharacter::Danis
        | BotCharacter::Stefanos
        | BotCharacter::Sporos => None,
    }
}

/// What the bot says when the player takes its deal or turns it down.
pub fn deal_answer(
    character: Option<BotCharacter>,
    taken: bool,
    roll: f64,
) -> Option<&'static str> {
    match character? {
        BotCharacter::Andreas => Some(andreas::deal_answer(taken, roll)),
        BotCharacter::Vasilis
        | BotCharacter::Danis
        | BotCharacter::Stefanos
        | BotCharacter::Sporos => None,
    }
}

/// How long the bot takes over a move, given how long any bot would naturally think.
pub fn think_time_for(character: Option<BotCharacter>, natural: Duration) -> Duration {
    match character {
        Some(BotCharacter::Vasilis) => vasilis::think_time(natural),
        Some(
            BotCharacter::Danis
            | BotCharacter::Stefanos
            | BotCharacter::Andreas
            | BotCharacter::Sporos,
        )
        | None => natural,
    }
}

/// What `mv`, just played from `before` to `after`, means to the bot playing `bot_side`.
pub fn moment_of(before: &Position, after: &Position, mv: Move, bot_side: Side) -> BotMoment {
    let bot_moved = before.side_to_move() == bot_side;
    match (after.is_check(), is_capture(before, mv), bot_moved) {
        (true, _, true) => BotMoment::GaveCheck,
        (true, _, false) => BotMoment::InCheck,
        (false, true, true) => BotMoment::TookPiece,
        (false, true, false) => BotMoment::LostPiece,
        (false, false, _) => BotMoment::Quiet,
    }
}

/// How far behind the side to move stands, in hundredths of a pawn; being mated is furthest.
fn behind_by(score: UciScore) -> i32 {
    match score {
        UciScore::Cp(centipawns) => -centipawns,
        UciScore::Mate(moves) if moves <= 0 => i32::MAX,
        UciScore::Mate(_) => i32::MIN,
    }
}

/// The pool of lines the bot speaks from in the language the app is showing: the Greek setting
/// gets the character written again in Cypriot Greek, as a Cypriot would say it, not a translation
/// of the English. The app has no separate Cypriot language, so `Language::Greek` selects it.
fn pool(
    english: &'static [&'static str],
    cypriot: &'static [&'static str],
) -> &'static [&'static str] {
    match i18n::current() {
        Language::English => english,
        Language::Greek => cypriot,
    }
}

/// One of `lines` if the bot speaks up, which it does `chance` of the time: the first roll decides
/// whether, the second which line.
fn speaks(
    chance: f64,
    lines: &[&'static str],
    (talk_roll, pick_roll): (f64, f64),
) -> Option<&'static str> {
    (talk_roll < chance).then(|| line_for(lines, pick_roll))
}

/// One of `lines`, by a roll from 0 to 1.
fn line_for(lines: &[&'static str], roll: f64) -> &'static str {
    lines[index_for(roll, lines.len())]
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use chess_domain::{Move, Position, Side, engine::UciScore};

    use super::{
        deal_answer, deal_offer, greeting, moment_of, move_line, quit_line, rage_quits,
        think_time_for,
    };
    use crate::{
        enums::{
            play::{BotCharacter, BotMoment},
            settings::Language,
        },
        i18n::set_current,
    };

    /// The moment `uci` makes from `fen`, for a bot playing `bot_side`.
    fn moment(fen: &str, uci: &str, bot_side: Side) -> BotMoment {
        let before: Position = fen.parse().expect("valid");
        let mv: Move = uci.parse().expect("a move");
        let after = before.play(mv).expect("legal");
        moment_of(&before, &after, mv, bot_side)
    }

    #[test]
    fn a_move_is_seen_from_the_bots_side() {
        let capture = "rnbqkbnr/ppp1pppp/8/3p4/4P3/8/PPPP1PPP/RNBQKBNR w KQkq - 0 2";
        assert_eq!(moment(capture, "e4d5", Side::Black), BotMoment::LostPiece);
        assert_eq!(moment(capture, "e4d5", Side::White), BotMoment::TookPiece);
        let check = "rnbqkbnr/ppp2ppp/8/3pp3/4P3/8/PPPP1PPP/RNBQKBNR w KQkq - 0 3";
        assert_eq!(moment(check, "f1b5", Side::Black), BotMoment::InCheck);
        assert_eq!(moment(check, "f1b5", Side::White), BotMoment::GaveCheck);
        assert_eq!(moment(check, "g1f3", Side::Black), BotMoment::Quiet);
    }

    #[test]
    fn only_danis_quits_and_the_custom_bot_says_nothing() {
        let danis = Some(BotCharacter::Danis);
        assert!(greeting(danis, 0.0).is_some());
        assert!(quit_line(danis, 0.0).is_some());
        assert!(rage_quits(danis, UciScore::Cp(-300)));
        let sporos = Some(BotCharacter::Sporos);
        assert!(!rage_quits(sporos, UciScore::Cp(-900)));
        assert!(!rage_quits(None, UciScore::Mate(-1)));
        assert_eq!(quit_line(None, 0.5), None);
        assert_eq!(greeting(None, 0.5), None);
        assert_eq!(move_line(None, BotMoment::TookPiece, (0.0, 0.0)), None);
    }

    #[test]
    fn vasilis_hurries_and_never_quits_and_the_others_take_their_time() {
        let vasilis = Some(BotCharacter::Vasilis);
        let natural = Duration::from_secs(1);
        assert!(think_time_for(vasilis, natural) < natural);
        assert!(!rage_quits(vasilis, UciScore::Cp(-900)));
        assert!(greeting(vasilis, 0.0).is_some());
        assert_eq!(think_time_for(Some(BotCharacter::Danis), natural), natural);
        assert_eq!(think_time_for(None, natural), natural);
    }

    #[test]
    fn only_andreas_makes_deals() {
        let andreas = Some(BotCharacter::Andreas);
        let losing = UciScore::Cp(-300);
        assert!(deal_offer(andreas, losing, (0.0, 0.0)).is_some());
        assert!(deal_answer(andreas, false, 0.0).is_some());
        assert_eq!(
            deal_offer(Some(BotCharacter::Danis), losing, (0.0, 0.0)),
            None
        );
        assert_eq!(deal_answer(None, true, 0.0), None);
    }

    /// Danis's English catchphrases, which stay in English inside his Cypriot lines.
    const CATCHPHRASES: [&str; 4] = [
        "WWWWWOOOOOOWWW RELAX",
        "WHY ARE YOU FUCKING USING A CHRONUS?",
        "DID YOU BUY YOUR ACCOUNT?",
        "OOOHH MY GOOOODDD",
    ];

    /// Rolls close enough together to land on every line of any pool, the longest having 15.
    const ROLLS: [f64; 20] = [
        0.0, 0.05, 0.1, 0.15, 0.2, 0.25, 0.3, 0.35, 0.4, 0.45, 0.5, 0.55, 0.6, 0.65, 0.7, 0.75,
        0.8, 0.85, 0.9, 0.95,
    ];

    /// Whether `letter` is one of the Greek letters, capital or small.
    fn is_greek_letter(letter: char) -> bool {
        ('Α'..='ω').contains(&letter)
    }

    /// Every line `character` can say in the current language, found by sweeping the rolls.
    fn every_line(character: BotCharacter) -> Vec<&'static str> {
        let moments = [
            BotMoment::Quiet,
            BotMoment::GaveCheck,
            BotMoment::InCheck,
            BotMoment::TookPiece,
            BotMoment::LostPiece,
        ];
        let losing = UciScore::Cp(-300);
        let mut lines = Vec::new();
        for roll in ROLLS {
            let who = Some(character);
            lines.extend(greeting(who, roll));
            lines.extend(quit_line(who, roll));
            lines.extend(deal_offer(who, losing, (0.0, roll)));
            lines.extend(deal_answer(who, true, roll));
            lines.extend(deal_answer(who, false, roll));
            for moment in moments {
                lines.extend(move_line(who, moment, (0.0, roll)));
            }
        }
        lines
    }

    #[test]
    fn no_cypriot_line_is_left_in_english_by_mistake() {
        set_current(Language::Greek);
        let characters = [
            BotCharacter::Vasilis,
            BotCharacter::Danis,
            BotCharacter::Stefanos,
            BotCharacter::Andreas,
            BotCharacter::Sporos,
        ];
        for character in characters {
            let lines = every_line(character);
            assert!(!lines.is_empty());
            for line in lines {
                let greek = line.chars().any(is_greek_letter);
                let catchphrase = CATCHPHRASES.iter().any(|phrase| line.contains(phrase));
                assert!(
                    greek || catchphrase,
                    "{character:?} says {line:?} in English"
                );
            }
        }
        set_current(Language::English);
    }
}
