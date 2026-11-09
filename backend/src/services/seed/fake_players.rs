//! Creates the made-up players, gives most of them a drawn avatar, and has them play each other
//! over the past weeks, oldest game first, so their ratings and rating graphs grow as real ones do.
#![expect(
    clippy::float_arithmetic,
    reason = "made-up players are chance and proportions"
)]

use std::{collections::HashMap, time::Duration as Pause};

use chess_domain::{RatedResult, Rating, Score};
use chrono::{DateTime, Duration, Utc};
use uuid::Uuid;

use super::{
    fake_games::{CATEGORY_WEIGHTS, Pairing, Terms, index_for, play_game},
    fake_names::fake_username,
};
use crate::{
    clients::dicebear::DiceBearClient,
    enums::play::Color,
    models::{
        rating::RatingSource,
        seed::{FakeGame, FakePlayer},
    },
    repositories::{account::avatar, database::Db, rating, seed},
    services::{
        account::{hash_password, picture_hash, square_jpeg},
        rating::to_row,
    },
};

/// How many games each player plays, on average.
const GAMES_PER_PLAYER: usize = 70;
/// The oldest a made-up account is, in days.
const OLDEST_DAYS: i64 = 120;
/// Strengths are spread around this, by about this much.
const STRENGTH: (f64, f64) = (1450.0, 380.0);
/// The weakest and strongest a made-up player is.
const STRENGTH_RANGE: (f64, f64) = (450.0, 2650.0);
/// Opponents are looked for within this much of a player's strength.
const PAIRING_REACH: f64 = 300.0;
/// How many in a hundred get a picture; the rest keep their initial, as many real players do.
const WITH_PICTURE: f64 = 0.8;
/// DiceBear's CC0 styles: drawn faces, people, pixel characters and figures.
const AVATAR_STYLES: [&str; 5] = ["lorelei", "notionists", "open-peeps", "pixel-art", "thumbs"];
/// DiceBear allows ten PNGs a second; one every 150 ms stays well under.
const AVATAR_PAUSE: Pause = Pause::from_millis(150);
/// A player's first rating in a speed: near their real strength, already fairly sure of it, as
/// for someone who played elsewhere before joining.
const STARTING_DEVIATION: f64 = 160.0;
/// How many in a hundred have also played Puzzle Battle.
const BATTLE_PLAYERS: f64 = 0.4;

/// A made-up player while seeding: who they are, how strong they really are, and when they joined.
struct Seeded {
    id: Uuid,
    strength: f64,
    joined: DateTime<Utc>,
}

/// What a seeding made, for the log.
#[derive(Debug, Default)]
pub struct SeedReport {
    pub players: usize,
    pub pictures: usize,
    pub games: usize,
}

/// Creates `count` made-up players, their pictures, games, ratings and puzzle ratings.
pub async fn seed_fake_players(db: &Db, count: usize) -> anyhow::Result<SeedReport> {
    let mut report = SeedReport::default();
    let password_hash = hash_password(random_password()).await?;
    let players = create_players(db, count, &password_hash).await?;
    report.players = players.len();
    report.pictures = give_pictures(db, &players).await?;
    report.games = play_games(db, &players).await?;
    set_puzzle_ratings(db, &players).await?;
    Ok(report)
}

/// A long password nobody is told: the accounts exist, but nobody signs in as them.
fn random_password() -> String {
    (0..48)
        .map(|_| char::from(b'a' + rand::random_range(0..26u8)))
        .collect()
}

async fn create_players(db: &Db, count: usize, password_hash: &str) -> anyhow::Result<Vec<Seeded>> {
    let mut players = Vec::with_capacity(count);
    // A taken name is skipped; a few times the count of tries is plenty.
    for _ in 0..count * 5 {
        if players.len() == count {
            break;
        }
        let player = new_player(password_hash);
        if seed::insert_player(db, &player).await? {
            players.push(Seeded {
                id: player.id,
                strength: strength(),
                joined: player.joined,
            });
        }
    }
    Ok(players)
}

fn new_player(password_hash: &str) -> FakePlayer {
    let rolls = (rand::random(), rand::random(), rand::random());
    let days_ago = rand::random_range(7..OLDEST_DAYS);
    FakePlayer {
        id: Uuid::now_v7(),
        username: fake_username(rolls),
        joined: Utc::now()
            - Duration::days(days_ago)
            - Duration::minutes(rand::random_range(0..1440)),
        password_hash: password_hash.to_owned(),
    }
}

/// A strength around the middle, rarely very weak or very strong: three rolls averaged.
fn strength() -> f64 {
    let spread: f64 = (0..3).map(|_| rand::random::<f64>()).sum::<f64>() / 3.0 - 0.5;
    (STRENGTH.0 + spread * 2.0 * STRENGTH.1 * 1.7).clamp(STRENGTH_RANGE.0, STRENGTH_RANGE.1)
}

/// Most players get a drawn avatar, each in a style of their own and seeded by their id, stored
/// as an uploaded picture is; one DiceBear cannot draw just goes without.
async fn give_pictures(db: &Db, players: &[Seeded]) -> anyhow::Result<usize> {
    let dicebear = DiceBearClient::new()?;
    let mut given = 0;
    for player in players
        .iter()
        .filter(|_| rand::random::<f64>() < WITH_PICTURE)
    {
        let style = AVATAR_STYLES[index_for(rand::random(), AVATAR_STYLES.len())];
        let Ok(png) = dicebear.avatar_png(style, player.id).await else {
            continue;
        };
        tokio::time::sleep(AVATAR_PAUSE).await;
        let Ok(jpeg) = square_jpeg(&png) else {
            continue;
        };
        let hash = picture_hash(&jpeg);
        avatar::save(db, player.id, (&hash, &jpeg)).await?;
        avatar::set_hash(db, player.id, Some(&hash)).await?;
        given += 1;
    }
    Ok(given)
}

/// Every game, played and stored oldest first, each moving both players' ratings.
async fn play_games(db: &Db, players: &[Seeded]) -> anyhow::Result<usize> {
    let mut schedule = schedule(players);
    schedule.sort_by_key(|(_, _, started_at)| *started_at);
    let mut ratings: HashMap<(Uuid, &'static str), Rating> = HashMap::new();
    let mut roll = || rand::random::<f64>();
    for (pairing, terms, started_at) in &schedule {
        let game = play_game(pairing, *terms, *started_at, &mut roll);
        store_game(db, (&game, pairing), &mut ratings).await?;
    }
    Ok(schedule.len())
}

/// Who plays whom, at what speed and when: opponents of about the same strength, after both joined.
fn schedule(players: &[Seeded]) -> Vec<(Pairing, Terms, DateTime<Utc>)> {
    let games = players.len() * GAMES_PER_PLAYER / 2;
    (0..games)
        .filter_map(|_| {
            let first = &players[index_for(rand::random(), players.len())];
            let second = opponent_for(first, players)?;
            let pairing = coloured(first, second);
            Some((pairing, terms(), game_time(first, second)))
        })
        .collect()
}

/// Someone near `player`'s strength, or anyone after a few tries.
fn opponent_for<'a>(player: &Seeded, players: &'a [Seeded]) -> Option<&'a Seeded> {
    let others = || players.iter().filter(|other| other.id != player.id);
    let near: Vec<&Seeded> = others()
        .filter(|other| (other.strength - player.strength).abs() < PAIRING_REACH)
        .collect();
    let pool: Vec<&Seeded> = if near.is_empty() {
        others().collect()
    } else {
        near
    };
    pool.get(index_for(rand::random(), pool.len())).copied()
}

fn coloured(first: &Seeded, second: &Seeded) -> Pairing {
    let (white, black) = if rand::random::<bool>() {
        (first, second)
    } else {
        (second, first)
    };
    Pairing {
        white: (white.id, white.strength),
        black: (black.id, black.strength),
    }
}

/// A speed picked by how often each is played.
fn terms() -> Terms {
    let mut roll = rand::random::<f64>();
    for (terms, weight) in CATEGORY_WEIGHTS {
        if roll < weight {
            return terms;
        }
        roll -= weight;
    }
    CATEGORY_WEIGHTS[0].0
}

/// A moment between the later of the two joining and an hour ago.
fn game_time(first: &Seeded, second: &Seeded) -> DateTime<Utc> {
    let from = first.joined.max(second.joined);
    let until = Utc::now() - Duration::hours(1);
    let span = (until - from).num_seconds().max(1);
    from + Duration::seconds(rand::random_range(0..span))
}

/// The game, its moves, and both players' new ratings, all at once.
async fn store_game(
    db: &Db,
    (game, pairing): (&FakeGame, &Pairing),
    ratings: &mut HashMap<(Uuid, &'static str), Rating>,
) -> anyhow::Result<()> {
    let white = current(ratings, pairing.white, game.category);
    let black = current(ratings, pairing.black, game.category);
    let (white_after, black_after) = rated(game, white, black);
    let mut tx = db.begin().await?;
    seed::insert_game(&mut *tx, game).await?;
    seed::insert_moves(&mut *tx, game).await?;
    let source = RatingSource::Game(game.id);
    rating::record_result(
        &mut tx,
        game.white_id,
        game.category,
        source,
        to_row(white_after),
    )
    .await?;
    rating::record_result(
        &mut tx,
        game.black_id,
        game.category,
        source,
        to_row(black_after),
    )
    .await?;
    tx.commit().await?;
    ratings.insert((game.white_id, game.category), white_after);
    ratings.insert((game.black_id, game.category), black_after);
    Ok(())
}

/// The player's rating in `category` so far, or their first one near their real strength.
fn current(
    ratings: &HashMap<(Uuid, &'static str), Rating>,
    (player, strength): (Uuid, f64),
    category: &'static str,
) -> Rating {
    ratings
        .get(&(player, category))
        .copied()
        .unwrap_or_else(|| starting(strength))
}

fn starting(strength: f64) -> Rating {
    Rating {
        rating: strength + rand::random_range(-120.0..120.0),
        deviation: STARTING_DEVIATION,
        volatility: Rating::NEW.volatility,
    }
}

/// Both players' ratings after the game, by Glicko-2.
fn rated(game: &FakeGame, white: Rating, black: Rating) -> (Rating, Rating) {
    let white_score = score_for(game, Color::White);
    let black_score = score_for(game, Color::Black);
    let white_after = white.after(&[RatedResult {
        opponent: black,
        score: white_score,
    }]);
    let black_after = black.after(&[RatedResult {
        opponent: white,
        score: black_score,
    }]);
    (white_after, black_after)
}

fn score_for(game: &FakeGame, side: Color) -> Score {
    match game.winner {
        None => Score::Draw,
        Some(winner) if winner == side => Score::Win,
        Some(_) => Score::Loss,
    }
}

/// Puzzle ratings near each player's strength, and a battle rating for some: set outright, as a
/// fresh server may have no puzzles for attempts to point at.
async fn set_puzzle_ratings(db: &Db, players: &[Seeded]) -> anyhow::Result<()> {
    for player in players {
        let puzzles = settled(player.strength + rand::random_range(-150.0..250.0));
        seed::set_rating(
            db,
            (player.id, "puzzle"),
            to_row(puzzles),
            rand::random_range(25..600),
        )
        .await?;
        if rand::random::<f64>() < BATTLE_PLAYERS {
            let battles = settled(player.strength + rand::random_range(-200.0..200.0));
            seed::set_rating(
                db,
                (player.id, "puzzle_battle"),
                to_row(battles),
                rand::random_range(3..80),
            )
            .await?;
        }
    }
    Ok(())
}

/// A rating sure enough not to show as provisional.
fn settled(rating: f64) -> Rating {
    Rating {
        rating: rating.clamp(STRENGTH_RANGE.0, STRENGTH_RANGE.1 + 300.0),
        deviation: rand::random_range(55.0..90.0),
        volatility: 0.06,
    }
}

#[cfg(test)]
mod tests {
    use chess_domain::Score;

    use super::score_for;
    use crate::{
        enums::play::{Color, GameResult},
        models::seed::FakeGame,
    };

    #[test]
    fn the_winner_scores_a_win_and_a_draw_is_a_draw() {
        let mut game = FakeGame {
            id: uuid::Uuid::now_v7(),
            white_id: uuid::Uuid::now_v7(),
            black_id: uuid::Uuid::now_v7(),
            category: "blitz",
            initial_s: 180,
            increment_s: 2,
            moves: Vec::new(),
            fen: String::new(),
            result: GameResult::Resignation,
            winner: Some(Color::Black),
            white_ms: 1,
            black_ms: 1,
            started_at: chrono::Utc::now(),
            finished_at: chrono::Utc::now(),
        };
        assert_eq!(score_for(&game, Color::Black), Score::Win);
        assert_eq!(score_for(&game, Color::White), Score::Loss);
        game.winner = None;
        assert_eq!(score_for(&game, Color::White), Score::Draw);
    }
}
