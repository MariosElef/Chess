//! The bots: who they are, how each strength plays, which move they choose, and how long they
//! seem to think. Randomness comes in as plain numbers from 0 to 1, so every rule here is the
//! same each time for the same rolls.
#![expect(
    clippy::float_arithmetic,
    reason = "bots choose by chance, and think for a share of their time"
)]

use std::time::Duration;

use chess_domain::{Move, Side};

use crate::{
    enums::play::{BotCharacter, ColourChoice},
    models::play::Bot,
};

/// The named bots, weakest first.
pub const BOTS: [Bot; 6] = [
    Bot {
        name: "Vasilis",
        rating: 400,
        picture: Some("bots/vasilis.jpg"),
        character: Some(BotCharacter::Vasilis),
    },
    Bot {
        name: "Danis",
        rating: 800,
        picture: Some("bots/danis.jpg"),
        character: Some(BotCharacter::Danis),
    },
    Bot {
        name: "Stefanos",
        rating: 1200,
        picture: Some("bots/stefanos.jpg"),
        character: Some(BotCharacter::Stefanos),
    },
    Bot {
        name: "Andreas",
        rating: 1600,
        picture: Some("bots/andreas.jpg"),
        character: Some(BotCharacter::Andreas),
    },
    Bot {
        name: "Sporos",
        rating: 2000,
        picture: Some("bots/sporos.jpg"),
        character: Some(BotCharacter::Sporos),
    },
    Bot {
        name: "Ferris",
        rating: 2400,
        picture: Some("bots/ferris.svg"),
        character: Some(BotCharacter::Ferris),
    },
];

/// The custom bot's strength: from, to, in steps of, and where it starts.
pub const CUSTOM_STRENGTH: (u32, u32, u32, u32) = (400, 2800, 100, 1500);

/// From this rating up, Stockfish's own strength limit plays the bot; below, a weaker search does.
const OWN_LIMIT_FROM: u32 = 1400;
/// The lowest and highest ratings Stockfish's limit takes.
const LIMIT_RANGE: (u32, u32) = (1320, 3190);
/// How long a bot on Stockfish's limit searches each move.
const LIMITED_SEARCH_MS: u64 = 500;
/// Weak bots by starting rating: how deep they look, how many best moves, and how often random.
const WEAK: [(u32, u32, usize, f64); 4] = [
    (400, 1, 4, 0.25),
    (800, 2, 3, 0.12),
    (1100, 3, 2, 0.06),
    (1300, 5, 2, 0.03),
];

/// How a bot of some strength plays.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BotStyle {
    /// Stockfish's own strength limit, for the stronger bots.
    pub elo_limit: Option<u32>,
    /// How deep the weaker bots look; the stronger ones search for a fixed time.
    pub depth: Option<u32>,
    pub search_ms: Option<u64>,
    /// How many of the engine's best moves the bot chooses among.
    pub choices: usize,
    /// How often the bot plays any legal move instead.
    pub wild_chance: f64,
}

pub fn custom_bot(rating: u32) -> Bot {
    Bot {
        name: "Custom",
        rating,
        picture: None,
        character: None,
    }
}

pub fn bot_style(rating: u32) -> BotStyle {
    if rating >= OWN_LIMIT_FROM {
        return BotStyle {
            elo_limit: Some(rating.clamp(LIMIT_RANGE.0, LIMIT_RANGE.1)),
            depth: None,
            search_ms: Some(LIMITED_SEARCH_MS),
            choices: 1,
            wild_chance: 0.0,
        };
    }
    let (_, depth, choices, wild_chance) = WEAK
        .iter()
        .rev()
        .find(|(from, ..)| rating >= *from)
        .copied()
        .unwrap_or(WEAK[0]);
    BotStyle {
        elo_limit: None,
        depth: Some(depth),
        search_ms: None,
        choices,
        wild_chance,
    }
}

/// The engine options that make Stockfish play this way.
pub fn bot_options(style: &BotStyle) -> Vec<(&'static str, String)> {
    let mut options = vec![("MultiPV", style.choices.to_string())];
    match style.elo_limit {
        Some(elo) => {
            options.push(("UCI_LimitStrength", "true".to_owned()));
            options.push(("UCI_Elo", elo.to_string()));
        }
        None => options.push(("Skill Level", "0".to_owned())),
    }
    options
}

/// The bot's move: now and then any legal move, otherwise one of the engine's best few.
pub fn pick_bot_move(
    style: &BotStyle,
    (best, legal): (&[Move], &[Move]),
    (wild_roll, pick_roll): (f64, f64),
) -> Option<Move> {
    let pool = if wild_roll < style.wild_chance || best.is_empty() {
        legal
    } else {
        &best[..style.choices.min(best.len())]
    };
    pool.get(index_for(pick_roll, pool.len())).copied()
}

/// A roll from 0 to 1 as an index into `len` things.
pub(super) fn index_for(roll: f64, len: usize) -> usize {
    let len_f = u32::try_from(len).map_or(f64::from(u32::MAX), f64::from);
    // The roll is below 1, so this is below `len`.
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "0..len"
    )]
    let index = (roll.clamp(0.0, 0.999_999) * len_f) as usize;
    index.min(len.saturating_sub(1))
}

/// How long the bot seems to think: half a second to two, less when its clock is low.
pub fn think_time(clock_left: Option<Duration>, roll: f64) -> Duration {
    let natural = Duration::from_secs_f64(0.5 + 1.5 * roll.clamp(0.0, 1.0));
    match clock_left {
        Some(left) => natural.min(left / 30).max(Duration::from_millis(100)),
        None => natural,
    }
}

/// The player's side for their choice; `coin` decides Random.
pub fn player_side(choice: ColourChoice, coin: bool) -> Side {
    match choice {
        ColourChoice::White => Side::White,
        ColourChoice::Black => Side::Black,
        ColourChoice::Random if coin => Side::White,
        ColourChoice::Random => Side::Black,
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use chess_domain::{Move, Side};

    use super::{BOTS, bot_options, bot_style, pick_bot_move, player_side, think_time};
    use crate::enums::play::ColourChoice;

    fn mv(uci: &str) -> Move {
        uci.parse().expect("a move")
    }

    #[test]
    fn stronger_bots_use_stockfishs_limit_and_weaker_ones_look_less_far() {
        let strong = bot_style(1600);
        assert_eq!(strong.elo_limit, Some(1600));
        assert!(bot_options(&strong).contains(&("UCI_Elo", "1600".to_owned())));
        assert_eq!(bot_style(3000).elo_limit, Some(3000));
        let beginner = bot_style(400);
        assert_eq!((beginner.elo_limit, beginner.depth), (None, Some(1)));
        assert!(bot_style(400).wild_chance > bot_style(1200).wild_chance);
        assert!(bot_style(1200).depth < bot_style(1300).depth);
        assert!(BOTS.windows(2).all(|pair| pair[0].rating < pair[1].rating));
    }

    #[test]
    fn a_bot_chooses_among_its_best_moves_or_now_and_then_anything() {
        let style = bot_style(800);
        let best = [mv("e2e4"), mv("d2d4"), mv("g1f3"), mv("c2c4")];
        let legal = [mv("a2a3"), mv("h2h3")];
        assert_eq!(
            pick_bot_move(&style, (&best, &legal), (0.9, 0.0)),
            Some(mv("e2e4"))
        );
        assert_eq!(
            pick_bot_move(&style, (&best, &legal), (0.9, 0.99)),
            Some(mv("g1f3")),
            "an 800 bot chooses among the best three"
        );
        assert_eq!(
            pick_bot_move(&style, (&best, &legal), (0.01, 0.99)),
            Some(mv("h2h3")),
            "and sometimes plays anything"
        );
        let master = bot_style(2000);
        assert_eq!(
            pick_bot_move(&master, (&best, &legal), (0.0, 0.99)),
            Some(mv("e2e4")),
            "a strong bot always plays the engine's move"
        );
    }

    #[test]
    fn the_bot_thinks_less_when_its_clock_is_low() {
        assert_eq!(think_time(None, 0.0), Duration::from_millis(500));
        assert_eq!(think_time(None, 1.0), Duration::from_secs(2));
        assert_eq!(
            think_time(Some(Duration::from_secs(6)), 1.0),
            Duration::from_millis(200)
        );
    }

    #[test]
    fn random_colour_follows_the_coin() {
        assert_eq!(player_side(ColourChoice::Random, true), Side::White);
        assert_eq!(player_side(ColourChoice::Random, false), Side::Black);
        assert_eq!(player_side(ColourChoice::Black, true), Side::Black);
    }
}
