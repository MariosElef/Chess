# Chess

A chess app for two people on one computer or online, a Stockfish analysis board, and puzzles: a GPUI
desktop app (`desktop/`), an axum server (`backend/`) and the chess rules both share
(`crates/chess-domain/`).

## Running it

```sh
docker compose up -d                       # Postgres on localhost:5433
DATABASE_URL=postgres://chess:change-me@localhost:5433/chess cargo run -p chess-backend
cargo run --release -p chess-desktop       # in another terminal
```

Play with `--release`; `cargo run -p chess-desktop` is a development build, slower to draw.
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

Puzzles take premoves: while the opponent's answer is on its way, the player can queue their
next move, played and judged the moment the answer lands.

## Online play

Play → Online finds an opponent who asked for the same time control. The game runs on the
server over a WebSocket (`/api/ws`): it checks every move, keeps both clocks, ends the game
and, for a rated game between two accounts, updates both ratings. Guests play unrated. If the
app drops its connection it reconnects on its own, and the game comes back where it was.
Before both sides have moved, either player can abort; a side that does not make its first
move within 30 seconds has the game aborted for it. An aborted game is never rated. Every move
costs at least a second, so a premove is never free: 5:02 becomes 5:01. Local games charge a
premove the same second. Games left unfinished when the server stops
are marked aborted when it starts again.

A guest is free again once no app signed in as it is connected, so Continue as guest always
gives the lowest-numbered guest nobody is using.

## Bots

Play → Bots: pick Vasilis (400), Danis (800), Stefanos (1200), Andreas (1600) or Sporos (2000),
or a custom bot from 400 to 2800, the colour to play, and a time control or none. Each bot
plays on a Stockfish of its own in the app, so it works offline. From 1400 up, Stockfish's own
strength limit plays it; below, it looks only a few moves ahead, chooses among its best few,
and now and then plays any move, so a low bot plays like a beginner rather than a slowed-down
master. It pauses between half a second and two seconds before moving, less when its clock is
low. Bot games are unrated; a bot makes and takes no draw offers.

## Game review

On the Analysis board, paste a FEN for one position or a PGN for a whole game (as chess.com or
Lichess give it), then Load. **Review** (shown once the game has a move) searches every position
at depth 16 with a few engines of one thread each, a quarter of the cores at most (one to
three), so it stays light on any computer; positions the opening table covers are not searched.
It labels each move the way chess.com's Game Review does:
Brilliant, Great, Best, Excellent, Good, Book, Forced, Inaccuracy, Mistake, Miss or Blunder.
While it runs, the board plays through the game with each move's slide and sound; when it is
ready, it goes back to the start. Each move's icon shows beside it and on the board, where the
move's squares take its colour; for any move but the best, a green arrow and "Best was …" show
what the engine preferred. The panel gives each side's accuracy, counts of each kind of move,
and a game rating: the rating that accuracy is typical of (an estimate, given to the nearest
fifty, with at least five moves of a side's own).

A move is graded by the expected points it loses against the engine's best move: Excellent up
to 0.02, Good to 0.05, Inaccuracy to 0.10, Mistake to 0.20, Blunder beyond (chess.com's
published bands). Brilliant is a sound piece sacrifice from a position that was not already
won; Great is the only good move; Miss lets the opponent's error go; Book follows the opening
table in `desktop/assets/review/openings.txt`; Forced is the only legal move. Chess.com's own
evaluation curve and accuracy formula are not published, so this uses Lichess's: the numbers
will be close to theirs, not equal.

## The board

Every board shares the same behaviour. Stepping through a game (arrow keys, the ◀ ▶ buttons or
the move list) plays each move's sound and slides the piece; a jump does not slide. Holding a key
or button keeps stepping, and after two seconds goes fast, several moves a step, still with sound.
Pressing a piece lifts it to the cursor at once; dragging it outlines the square under it, and a
right-click puts it back. Your own moves do not slide: you already put the piece there.

## Social

The Social page lists your friends, with who is online, and the friend requests each way.
Find players by name to add them; a friend who is online can be challenged to a game with the
time control and rating you pick. A challenge waits a minute for an answer, and accepting it
opens the game on Play for both. Challenges and requests waiting for you show as a count on
Social in the rail. Guests have friends too; a guest's friends stay with that guest account,
so the next person to get it has them.

## Stats

The Account page shows your rating in bullet, blitz, rapid, classical and puzzles, each with its
best and how your games went (won, lost, drawn) or how many puzzles you solved, plus your Puzzle
Rush bests. A `?` after a rating means too few results for it to be sure yet.

## Chat

Beside your friends on the Social page are your chats. Message a friend to start one, or make a
group of friends with a name; anyone in a group can add their own friends to it, and leave it.
Messages arrive at once while you are online, and unread ones count on Social in the rail.
During an online game the two players have a chat of their own beside the moves.

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
