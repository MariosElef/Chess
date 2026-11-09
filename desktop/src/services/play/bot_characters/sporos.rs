//! Sporos: the strongest of the bots and the gloomiest. Sure nobody likes him, he gave up
//! everything else for games, and even winning does not cheer him up.
use super::{line_for, speaks};
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
];

pub(super) fn greeting(roll: f64) -> &'static str {
    line_for(GREETINGS, roll)
}

/// Sighs over captures and checks, his own as much as the player's; between them, he mopes.
pub(super) fn move_line(moment: BotMoment, rolls: (f64, f64)) -> Option<&'static str> {
    let (chance, lines): (f64, &[&'static str]) = match moment {
        BotMoment::LostPiece => (SIGH_CHANCE, LOST_PIECE),
        BotMoment::TookPiece => (SIGH_CHANCE, TOOK_PIECE),
        BotMoment::InCheck => (SIGH_CHANCE, IN_CHECK),
        BotMoment::GaveCheck => (SIGH_CHANCE, GAVE_CHECK),
        BotMoment::Quiet => (MOPE_CHANCE, MOPING),
    };
    speaks(chance, lines, rolls)
}

#[cfg(test)]
mod tests {
    use super::{MOPING, greeting, move_line};
    use crate::enums::play::BotMoment;

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
}
