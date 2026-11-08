use std::env;

/// The backend on this computer, as `cargo run -p chess-backend` serves it.
const DEFAULT_SERVER_URL: &str = "http://127.0.0.1:8080";

/// Where the chess server is: `CHESS_SERVER_URL`, or the local one.
pub fn server_url() -> String {
    env::var("CHESS_SERVER_URL").unwrap_or_else(|_| DEFAULT_SERVER_URL.to_owned())
}
