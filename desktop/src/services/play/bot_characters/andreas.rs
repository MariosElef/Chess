//! Andreas: gambles on everything, checks his kouponoui between moves, and once the game turns
//! against him, tries to buy his way out with a deal: a draw offer, sweetened.
use chess_domain::engine::UciScore;

use super::{behind_by, line_for};
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
];

pub(super) fn greeting(roll: f64) -> &'static str {
    line_for(GREETINGS, roll)
}

/// Bets on captures and checks; between them, his kouponoui.
pub(super) fn move_line(
    moment: BotMoment,
    (talk_roll, pick_roll): (f64, f64),
) -> Option<&'static str> {
    let (chance, lines): (f64, &[&'static str]) = match moment {
        BotMoment::LostPiece => (BET_CHANCE, LOST_PIECE),
        BotMoment::TookPiece => (BET_CHANCE, TOOK_PIECE),
        BotMoment::InCheck => (BET_CHANCE, IN_CHECK),
        BotMoment::GaveCheck => (BET_CHANCE, GAVE_CHECK),
        BotMoment::Quiet => (KOUPONOUI_CHANCE, KOUPONOUI),
    };
    (talk_roll < chance).then(|| line_for(lines, pick_roll))
}

/// Once a pawn behind, he tries a deal now and then.
pub(super) fn offers_deal(score: UciScore, roll: f64) -> bool {
    behind_by(score) >= DEALS_FROM && roll < DEAL_CHANCE
}

pub(super) fn deal_line(roll: f64) -> &'static str {
    line_for(DEALS, roll)
}

pub(super) fn deal_answer(taken: bool, roll: f64) -> &'static str {
    if taken {
        line_for(TAKEN, roll)
    } else {
        line_for(REFUSED, roll)
    }
}

#[cfg(test)]
mod tests {
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
}
