# Chess

A chess app for two people on one computer or online, a Stockfish analysis board, and puzzles: a GPUI
desktop app (`desktop/`), an axum server (`backend/`) and the chess rules both share
(`crates/chess-domain/`).

## Running it

```sh
docker compose up -d                       # Postgres on localhost:5433
DATABASE_URL=postgres://chess:change-me@localhost:5433/chess cargo run -p chess-backend
cargo run -p chess-desktop                 # in another terminal
```

The server applies its migrations when it starts. The app opens on sign-in: create an account,
sign in, or continue as one of the ten guests (Guest, Guest2 … Guest10). The analysis board runs
Stockfish (`brew install stockfish`), found at `STOCKFISH_PATH`, else in Homebrew's folders,
else on the `PATH`.

### Settings

| where | variable | default |
|---|---|---|
| server | `DATABASE_URL` | required |
| server | `BIND_ADDR` | `127.0.0.1:8080` |
| server | `LICHESS_URL` | `https://lichess.org` |
| server | `RUST_LOG` | `chess_backend=info,tower_http=info` |
| app | `CHESS_SERVER_URL` | `http://127.0.0.1:8080` |
| app | `STOCKFISH_PATH` | Homebrew's, then the `PATH` |

The app keeps its own settings (appearance, board, Stockfish, the signed-in session) in
`~/Library/Application Support/Chess/`.

## Puzzles

Puzzles come from the [Lichess puzzle database](https://database.lichess.org/#puzzles) (CC0,
about 6 million puzzles). Until it is imported, the server asks Lichess's API for each puzzle.
To import it (a 310 MB download, about 2 GB in Postgres; a newer file updates the puzzles in
place):

```sh
curl -O https://database.lichess.org/lichess_db_puzzle.csv.zst
DATABASE_URL=postgres://chess:change-me@localhost:5433/chess \
  cargo run --release -p chess-backend -- import-puzzles lichess_db_puzzle.csv.zst
```

## Online play

Play → Online finds an opponent who asked for the same time control. The game runs on the
server over a WebSocket (`/api/ws`): it checks every move, keeps both clocks, ends the game
and, for a rated game between two accounts, updates both ratings. Guests play unrated. If the
app drops its connection it reconnects on its own, and the game comes back where it was.
Before both sides have moved, either player can abort; a side that does not make its first
move within 30 seconds has the game aborted for it. An aborted game is never rated. Every move
costs at least 0.01 s, so a premove is never free. Games left unfinished when the server stops
are marked aborted when it starts again.

A guest is free again once no app signed in as it is connected, so Continue as guest always
gives the lowest-numbered guest nobody is using.

## Checks

`./ci.sh` runs rustfmt, clippy with warnings as errors, every test, `cargo deny`, the layer
rules (ast-grep, in `desktop/lint/` and `backend/lint/`) and the check that every source file
is part of the build. The backend's tests need the compose Postgres running; each makes its own
throwaway database.

Queries are checked at compile time against the committed `.sqlx/` cache. After changing a
query or a migration:

```sh
DATABASE_URL=postgres://chess:change-me@localhost:5433/chess cargo sqlx prepare --workspace -- --all-targets
```
