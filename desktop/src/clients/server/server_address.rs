use std::env;

/// The server a build talks to: `CHESS_SERVER_URL` as it was when the app was built, as the
/// installers are, else the backend on this computer as `cargo run -p chess-backend` serves it.
const BUILT_FOR: &str = match option_env!("CHESS_SERVER_URL") {
    Some(url) => url,
    None => "http://127.0.0.1:8080",
};

/// Where the chess server is: `CHESS_SERVER_URL` when the app runs, else the one it was built for.
pub fn server_url() -> String {
    env::var("CHESS_SERVER_URL").unwrap_or_else(|_| BUILT_FOR.to_owned())
}
