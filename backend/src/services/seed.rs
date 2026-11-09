//! Made-up players, so an empty server's leaderboards, profiles and stats have something in them:
//! `chess-backend seed-fake-players <count>`. They play real games against each other, and their
//! ratings move by Glicko-2 as a real player's would.
mod fake_games;
mod fake_names;
mod fake_players;

pub use fake_players::seed_fake_players;
