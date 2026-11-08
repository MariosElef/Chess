//! The chess server: accounts and sessions now; friends, chat and live games to come.
mod clients;
mod commands;
mod config;
mod enums;
mod errors;
mod extract;
mod models;
mod repositories;
mod routes;
mod services;
mod shutdown;
mod state;
pub mod telemetry;

pub use clients::lichess::LichessClient;
pub use commands::import_puzzles;
pub use config::AppConfig;
pub use repositories::database::Db;
pub use routes::build_router;
pub use services::puzzles::import::import_puzzles as import_puzzle_file;
pub use shutdown::shutdown_signal;
pub use state::AppState;
