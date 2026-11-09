# Chess

A chess app, free to use and share for non-commercial purposes: play online, against bots or a
friend at the same computer, solve puzzles, review your games with Stockfish, and keep up with
friends. It is a GPUI desktop app (`desktop/`), an axum server (`backend/`), and the chess rules
both share (`crates/chess-domain/`), all in Rust. A few of its pictures and sounds are other
people's and for personal use only: see [CREDITS.md](CREDITS.md).

Its look and feel follow [chess.com](https://www.chess.com), and its puzzles and much of its
know-how come from [Lichess](https://lichess.org): see [Thanks](#thanks).

This is a community, non-commercial project: anyone is welcome to use it, learn from it and
contribute by opening a pull request (see [Contributing](#contributing)), for any purpose except
a commercial one. See [License](#license).

## What it does

- **Play**: online games against other players, with ratings, premoves and reconnecting; bots
  from 400 to 2800 on Stockfish; or two people at one computer.
- **Puzzles**: Practice at the difficulty you pick, Puzzle Rush (3 minutes, 5 minutes or
  Survival) and Puzzle Battle against another player, each with its own rating.
- **Analysis**: paste a FEN or PGN, explore with Stockfish, and get a Game Review that labels
  every move the way chess.com does, with accuracy and an evaluation graph.
- **Social**: friends with who is online and when they last were, a friends' leaderboard and a
  leaderboard of everyone, challenges, and chats with friends and groups.
- **Account**: your rating in every mode, each with its graph, your history of games (to look at
  again or review), battles and puzzles, your profile picture, the devices signed in, and
  account deletion.
- **Settings**: light or dark, four board themes, four piece sets (Classic, Modern, Staunty and
  California), two sound sets (Standard and Lisp), and how hard Stockfish works.

## Screenshots

<table>
  <tr><td width="50%"><img src="docs/screenshots/play.png" alt="Play: a game just won, with Game Review on the card"><br><sub>Play: a game just won, with Game Review on the card</sub></td><td width="50%"><img src="docs/screenshots/game-review.png" alt="Game Review: every move labelled, with accuracy"><br><sub>Game Review: every move labelled, with accuracy</sub></td></tr>
  <tr><td width="50%"><img src="docs/screenshots/reviewing.png" alt="Reviewing: the board plays through the game"><br><sub>Reviewing: the board plays through the game</sub></td><td width="50%"><img src="docs/screenshots/game-review-2.png" alt="Game Review: the evaluation graph and move counts"><br><sub>Game Review: the evaluation graph and move counts</sub></td></tr>
  <tr><td width="50%"><img src="docs/screenshots/analysis.png" alt="Analysis: Stockfish lines and arrows"><br><sub>Analysis: Stockfish lines and arrows</sub></td><td width="50%"><img src="docs/screenshots/analysis-2.png" alt="Analysis: a loaded game"><br><sub>Analysis: a loaded game</sub></td></tr>
  <tr><td width="50%"><img src="docs/screenshots/puzzles.png" alt="Puzzles: Practice, Puzzle Rush and Puzzle Battle"><br><sub>Puzzles: Practice, Puzzle Rush and Puzzle Battle</sub></td><td width="50%"><img src="docs/screenshots/friends.png" alt="Friends, with the friends' leaderboard"><br><sub>Friends, with the friends' leaderboard</sub></td></tr>
  <tr><td width="50%"><img src="docs/screenshots/messages.png" alt="Messages: chats with friends and groups"><br><sub>Messages: chats with friends and groups</sub></td><td width="50%"><img src="docs/screenshots/settings.png" alt="Settings: themes, pieces, sounds and Stockfish"><br><sub>Settings: themes, pieces, sounds and Stockfish</sub></td></tr>
</table>

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

A wrong move plays on the board with the miss icon on its square for a second, as on chess.com;
in Practice it then goes back so you can try again. Puzzle Rush (3 minutes, 5 minutes or
Survival) counts down 3, 2, 1 with a beep each, then starts easy and gets harder with each
puzzle; three mistakes end it. Its panel shows your best score of the kind picked, ever and
today.

## Online play

Play → Online finds an opponent who asked for the same time control. The game runs on the server
over a WebSocket (`/api/ws`): it checks every move, keeps both clocks, ends the game and, for a
rated game between two accounts, updates both ratings. Guests play unrated. If the app drops its
connection it reconnects on its own, and the game comes back where it was. Before both sides
have moved, either player can abort; a side that does not make its first move within 30 seconds
has the game aborted for it. An aborted game is never rated; its card still comes up, saying it
was aborted, with New game and Rematch. Every move costs at least a second, so a premove is
never free: 5:02 becomes 5:01. Local games charge a premove the same second. Games left
unfinished when the server stops are marked aborted when it starts again.

When a game ends, Rematch on its card asks your opponent to play again at the same time control,
rated as before, with the colours swapped. Your opponent hears it and sees a toast in the corner
of whatever page they are on, with Accept, Decline and a bar running down the minute it waits;
the X only puts the toast away, leaving the rematch to their card or to run out. Their card for
that game also says Accept (then Accepting… until the game starts), so either answers it; an
accept refused from the toast shows on it. If both of you press Rematch, the game starts at
once. Your card says Asking… and then Cancel while it waits, and No rematch this time if it ends
unplayed: declined, run out, your opponent gone, or replaced by a rematch of another game you
asked for. If they cannot take one at all it says so at once: playing a game, or simply not
taking a rematch when offline or blocked. Asking for a rematch of another recent game replaces
your earlier one. Signed in on several devices, the device you asked from can cancel and the
others show Asking…; asking from a second device too keeps the first rematch and says so.
Leaving the card for a new game declines their rematch for all your devices, and withdraws yours
if you asked from that device. A rematch can be asked for up to ten minutes after the game, an
aborted one too, between any two players but those who blocked each other (unfriending leaves a
waiting rematch alone, and a friend's challenge replaces only your earlier friend's challenge),
while both accounts are still signed in somewhere from before it ended: if every sign-in since
is newer, as a shared guest account handed to someone new, it means starting a new game instead.

A guest is free again once no app signed in as it is connected, so Continue as guest always
gives the lowest-numbered guest nobody is using.

## Bots

Play → Bots: pick Vasilis (400), Danis (800), Stefanos (1200), Andreas (1600) or Sporos (2000),
or a custom bot from 400 to 2800, the colour to play, and a time control or none. Each bot
plays on a Stockfish of its own in the app, so it works offline. From 1400 up, Stockfish's own
strength limit plays it; below, it looks only a few moves ahead, chooses among its best few,
and now and then plays any move, so a low bot plays like a beginner rather than a slowed-down
master. It pauses between half a second and two seconds before moving, less when its clock is
low. Bot games are unrated; a bot takes no draw offers, and only Andreas makes them.

Each named bot has a character of its own: it talks in a bubble at the top of the game's panel,
reacting to captures, checks and now and then a quiet move, and some act on the game too.
Vasilis is bored and thinks only of food: he moves almost at once and shrugs off what happens
on the board. Danis is angry from his first words and resigns on his turn once his engine sees
him two pawns down or being mated. Stefanos is a metalhead who screams at nearly every capture
and check, and talks about goth girls in between. Andreas gambles on everything and checks his
kouponoui between moves; once a pawn down he now and then offers a deal, a draw you can accept
or decline. Sporos, the strongest, is sure nobody likes him and that he gave up his happiness
for games, and sighs about it while he beats you.

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

The Social page is a Friends page, as chess.com's: your friends with who is online now and when
the others last were, sorted by the most recently online or by name, and a Friends Leaderboard
beside them: the best three among your friends and you in bullet, blitz, rapid, classical and
puzzles, with your own place under them when you are lower. Search by name to find players and
add them; Friend Requests opens the requests each way. A friend who is online can be challenged:
Challenge opens a window with their picture, where you pick the time control and whether it is
rated, as in the Play lobby. A challenge waits a minute for an answer, and accepting it opens
the game on Play for both. On any other page a challenge shows as a toast in the corner, with
Accept, Decline, the minute it has left, and an X that only puts it away; Social keeps it in its
list. Challenges and requests waiting for you show as a count on Social in the rail. Guests have
friends too; a guest's friends stay with that guest account, so the next person to get it has
them.

Block a player from a friend's "…" menu or from the search: the friendship, any request and any
challenge or battle invitation between you end, and they can no longer ask to be friends (their
request is quietly dropped rather than refused) or message you directly. No one can put the two
of you in a new group, though a group you already share keeps going. The people you blocked are
listed under Friend Requests, where Unblock lifts your block; a block they made on you stays
theirs. Random pairing does not look at blocks, so a blocked player can still be paired with you
in a seek.

The Leaderboard, under Social in the rail, ranks everyone by rating, a board each for bullet,
blitz, rapid, classical, puzzles and Puzzle Battle: fifty places at a time, best first, with
Load more for the next fifty, in the middle of the page. Players with the same rating as shown
share a place. Each place shows the player's picture, name, rating and how many rated games,
puzzles or battles they have played there; a rating still provisional carries a "?", and the
first three places are in gold, silver and bronze. Your own place is pinned above the list. Only
accounts with a rated result on a board are on it: guests and closed accounts never are. View
profile on any place opens that player's profile: their picture, name and when they joined,
their ratings with their graphs and record in every mode and in puzzles, and Add friend, Accept
request, or where you already stand with them. Their email stays private.

## Puzzle Battle

Puzzles → Puzzle Battle races another player through the same puzzles for three minutes,
starting easy and getting harder; three mistakes end your run, the most solved wins, and a tie
goes to whoever reached that score first. Play a random opponent, or pick a friend who is online
to invite them; their invitation shows on your panel to accept or decline, and as a toast on any
other page. Both scores update live over the WebSocket, and either player can resign. Rated
battles between two signed-up players move a Puzzle Battle rating of its own; turn Rated off for
one that does not count, and a guest's battles never do. The panel shows the rating with your
wins this month. Invitations waiting for you show as a count on Puzzles in the rail. Battles
need the puzzle database imported.

## Stats

The Account page shows your rating in bullet, blitz, rapid, classical and puzzles, each on a
card with how your last result moved it and a graph of your last 30 ratings, as chess.com's
stats do, with its best and how your games went (won, lost, drawn) or how many puzzles you
solved, plus your Puzzle Rush bests and Puzzle Battle rating. A `?` after a rating means too few
results for it to be sure yet.

Account → Delete account closes the account once its password is typed again: every app signed
in as it is signed out and disconnected, its searches and invitations are withdrawn and any game
or battle it was playing is resigned, and its friends, chats, ratings and puzzle scores are
deleted. Its games and battles stay for the other players, under a removed name (`deleted_…`),
and its email and username are free to sign up with again. A guest account is shared, so it
cannot be deleted.

Change picture on the Account card picks a JPEG, PNG, WebP or GIF of up to 5 MB, and Remove goes
back to your initial. The server cuts the middle square out of it, scales it to 256 by 256, lays
any see-through parts on white and keeps only that, as a JPEG of about 20 KB; the original is
never stored. Your picture shows in the header's menu, your friends' lists, search, the
leaderboard, chats and groups, the challenge window, the bars beside the board in online games,
against a bot and in Puzzle Battle, everywhere your initial did. Each picture's address carries
a hash of its bytes, so the app loads it once per run and a new picture always has a new
address. Guests cannot set one, since their accounts are shared, and closing an account deletes
its picture.

Under the stats, History has three tabs, each newest first, ten at a time with Load more. Games
lists your finished online games: the speed, your opponent, the time control and length, whether
you won, lost or drew, the rating change, and when; click one to open it on the analysis board
at its final position, or Review to open it straight into Game Review. Battles lists your Puzzle
Battles with both scores, and Puzzles your puzzle attempts with the puzzle's rating, how long
you took, and the rating change. Guests have no history, since their accounts are shared.

Under History, Devices lists what is signed in to your account: this device first, then the
others with when each signed in and was last used (the app names itself to the server as "Chess
<version> (<system>)", shown as "Chess app on macOS"). Sign out everywhere else ends every other
session at once and closes their live connections. Signing out closes that session's live
connection too, so an app still using the same session, as a second copy sharing the saved
sign-in, goes back to the sign-in form saying you were signed out, instead of failing on every
page.

## Chat

Your chats have a page of their own, Messages: hover Social in the rail and pick Friends,
Messages or Leaderboard. Messages has the chats on the left and the one open on the right. A
friend's Message button opens theirs, or make a group of friends with a name; a group's members
button opens it: whoever made the group owns it and can rename it and remove members, anyone in
it can add their own friends and leave, and when the owner leaves, or closes their account, the
longest-standing member owns it. A direct chat only takes messages while the two are friends.
Messages arrive at once while you are online, and unread ones count on Social in the rail and on
Messages in its menu. During an online game the two players have a chat of their own beside the
moves. Each message shows its sender's picture, and a chat keeps to its newest message as
messages come; scroll up to read older ones.

## Known limits

The server takes the app's word on whether a puzzle was solved: Practice, Puzzle Rush and
Puzzle Battle all report the result rather than each move, so a modified app could claim
puzzles it never solved and move its puzzle or battle rating. Online games are different: the
server checks every move. Fine among friends; checking puzzle answers on the server would mean
sending each move and judging it against the solution there.

Some of what the server stores is written but not read back yet: the live mirror of each game
(its position, clocks, last move time and any draw offer), the clock after each move, and the
record of every challenge and rematch (with the game a rematch replays and the colours it fixed;
when two rematches cross, the second is recorded as cancelled and the first as accepted).
Nothing in the app shows them today, and a game still going when the server restarts is aborted
rather than resumed.

## Contributing

This is a community project, and contributions of any size are welcome, from a typo to a new
feature. To contribute:

1. Fork the repository and make a branch for your change.
2. For anything bigger than a small fix, open an issue first, so the idea can be talked over
   before you spend time on it.
3. Keep a pull request to one change, with a title that says what it does.
4. Run `./ci.sh` (below) and make sure it passes; add tests for what you change.
5. Update this README if your change is something a player would notice.
6. Open a pull request against `main`.

The code keeps to a few house rules, which the checks enforce where they can: the layers go
views → components → services → clients → models → enums, and services, models and enums never
use GPUI; logic lives in small named functions, with shared helpers in a named module; every
query has its own `.sql` file under `repositories/sql/`, with no table aliases and times from
the database's `now()`; no `_ =>` arms on the project's enums; `expect("why")` rather than
`unwrap`; and every `///` doc is one line.

### Checks

`./ci.sh` runs rustfmt, clippy with warnings as errors, every test, `cargo deny`, the layer
rules (ast-grep, in `desktop/lint/` and `backend/lint/`) and the check that every source file
is part of the build. The backend's tests need the compose Postgres running; each makes its own
throwaway database.

Queries are checked at compile time against the committed `.sqlx/` cache. After changing a
query or a migration:

```sh
DATABASE_URL=postgres://chess:change-me@localhost:5433/chess cargo sqlx prepare --workspace -- --all-targets
```

## Thanks

- **[chess.com](https://www.chess.com)**: the app's look and much of how it plays are modelled
  on chess.com: the Play lobby and its bots, Game Review's move labels and grading bands, Puzzle
  Rush and Puzzle Battle, the Friends page and its leaderboard, and the stats cards. The
  Standard sounds and the move-icon, page, speed and end-of-game pictures are chess.com's: they
  belong to chess.com and are here for personal use only, to be replaced before the app is
  shared (see [CREDITS.md](CREDITS.md)).
- **[Lichess](https://lichess.org)**: the puzzles come from the
  [Lichess puzzle database](https://database.lichess.org/#puzzles) (CC0) and its API; the Staunty
  and California pieces (by sadsnake1 and Jerry S.) and the Lisp sounds (EdinburghCollective's)
  are from [lila](https://github.com/lichess-org/lila), under CC BY-NC-SA 4.0, and the
  Classic pieces are Colin M.L. Burnett's set from lila too, under GPLv2+, which is why they,
  like the Modern pieces of unknown source, are to be replaced before the app is shared; and the
  evaluation curve and accuracy formula Game Review uses are Lichess's, which publishes them.
- **[Stockfish](https://stockfishchess.org)**: the engine behind analysis, Game Review and the
  bots.

## License

The app is licensed under
[Creative Commons Attribution-NonCommercial-ShareAlike 4.0](https://creativecommons.org/licenses/by-nc-sa/4.0/)
(CC BY-NC-SA 4.0); the full text is in [LICENSE](LICENSE). In short:

- **Share and adapt it**: copy it, change it and build on it, freely.
- **Credit it**: say where it came from, link to this licence, and say what you changed.
- **Not commercially**: no use for commercial advantage or money.
- **Share alike**: anything you make from it is shared under this same licence.

Because it forbids commercial use, it is source-available rather than open source in the Open
Source Initiative's sense. Everything in the repository made by someone else, and its own
licence, is listed in [CREDITS.md](CREDITS.md); some of those files are here for personal use
only and must be replaced before the app is shared.