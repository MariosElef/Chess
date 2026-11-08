use std::env;

use anyhow::Context as _;

/// Where the server listens when `BIND_ADDR` is not set: this computer only.
const DEFAULT_BIND_ADDR: &str = "127.0.0.1:8080";

/// The server's settings, read from the environment once at startup; a missing one stops it
/// there. No `Debug`: the database URL carries a password.
pub struct AppConfig {
    pub database_url: String,
    pub bind_addr: String,
}

impl AppConfig {
    pub fn load() -> anyhow::Result<Self> {
        Ok(Self {
            database_url: env::var("DATABASE_URL").context("DATABASE_URL is not set")?,
            bind_addr: env::var("BIND_ADDR").unwrap_or_else(|_| DEFAULT_BIND_ADDR.to_owned()),
        })
    }
}
