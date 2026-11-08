//! The chess server: accounts and sessions now; friends, chat and live games to come.
mod config;
mod errors;
mod extract;
mod models;
mod repositories;
mod routes;
mod services;
mod shutdown;
mod state;
pub mod telemetry;

pub use config::AppConfig;
pub use repositories::database::Db;
pub use routes::build_router;
pub use shutdown::shutdown_signal;
pub use state::AppState;
