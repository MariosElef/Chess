use std::time::Duration;

use chess_domain::{Clock, Game, Move, Rating, Side, TimeControl};
use tokio::{
    sync::mpsc,
    time::{Instant, sleep_until},
};
use uuid::Uuid;

use super::{game_rating::rate_game, hub::PlayHub};
use crate::{
    enums::play::{Color, GameResult, GameStatus},
    models::play::{
        game::{FinishedGame, GameSnapshot, PlayedMove, RatingChanges},
        messages::ServerMessage,
        player::PlayerModel,
    },
    repositories::{database::Db, play::game},
};

/// Commands waiting for a game; a player sending faster than this waits.
const COMMANDS_AHEAD: usize = 32;
/// The least a move costs on the clock, so a premove played the instant the turn comes takes
/// a second, as the clock shows it: 5:02 becomes 5:01.
const LEAST_MOVE_TIME: Duration = Duration::from_secs(1);
/// How long each side has for its first move before the game is called off.
pub const FIRST_MOVE_WAIT: Duration = Duration::from_secs(30);
/// Moves after which the game can no longer be aborted: one each.
const OPENING_PLIES: usize = 2;

/// What a player asks of a game they are in.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GameCommand {
    Move {
        player: Uuid,
        uci: String,
    },
    Resign {
        player: Uuid,
    },
    Abort {
        player: Uuid,
    },
    OfferDraw {
        player: Uuid,
    },
    AcceptDraw {
        player: Uuid,
    },
    DeclineDraw {
        player: Uuid,
    },
    /// Send the game as it stands to this player: they have just (re)connected.
    Snapshot {
        player: Uuid,
    },
}

/// A game as it starts: who plays which side, and how.
#[derive(Debug, Clone, PartialEq)]
pub struct GameStart {
    pub id: Uuid,
    pub white: PlayerModel,
    pub black: PlayerModel,
    pub rated: bool,
    pub control: TimeControl,
    /// The `play.rating_categories` id its ratings move in.
    pub category: String,
    /// The two players' chat.
    pub chat_id: Uuid,
    /// White's and Black's ratings as the game began, for rating it if one closes meanwhile.
    pub ratings: (Rating, Rating),
}

/// Runs the game on a task of its own, which alone holds its board and clock; the commands
/// arrive through the returned sender, one at a time, so no lock is ever needed.
pub fn spawn_game(start: GameStart, hub: PlayHub, db: Db) -> mpsc::Sender<GameCommand> {
    let (sender, commands) = mpsc::channel(COMMANDS_AHEAD);
    tokio::spawn(run(LiveGame::new(start, hub, db), commands));
    sender
}

/// Plays commands until the game is over, ending it on a flag when the side to move's time
/// runs out first, or calling it off when a first move does not come in time.
async fn run(mut live: LiveGame, mut commands: mpsc::Receiver<GameCommand>) {
    live.broadcast();
    while !live.game.is_over() && !live.aborted {
        let flag = live.flag_deadline();
        let first_move = live.first_move_deadline();
        tokio::select! {
            command = commands.recv() => match command {
                Some(command) => live.handle(command).await,
                None => return,
            },
            () = sleep_until(flag.unwrap_or_else(Instant::now)), if flag.is_some() => {
                live.flag();
            }
            () = sleep_until(first_move.unwrap_or_else(Instant::now)), if first_move.is_some() => {
                live.aborted = true;
            }
        }
    }
    if live.aborted {
        live.abort().await;
    } else {
        live.finish().await;
    }
}

struct LiveGame {
    start: GameStart,
    game: Game,
    clock: Clock,
    /// When the side to move's turn began; `None` before the first move and once it is over.
    turn_started: Option<Instant>,
    rating_changes: Option<RatingChanges>,
    /// When the side to move's wait for its first move began, while one is still owed.
    first_move_since: Instant,
    /// Called off before both sides moved.
    aborted: bool,
    hub: PlayHub,
    db: Db,
}

impl LiveGame {
    fn new(start: GameStart, hub: PlayHub, db: Db) -> Self {
        Self {
            clock: Clock::new(start.control),
            start,
            game: Game::new(),
            turn_started: None,
            rating_changes: None,
            first_move_since: Instant::now(),
            aborted: false,
            hub,
            db,
        }
    }

    async fn handle(&mut self, command: GameCommand) {
        match command {
            GameCommand::Move { player, uci } => self.play(player, &uci).await,
            GameCommand::Resign { player } => self.resign(player),
            GameCommand::Abort { player } => self.request_abort(player),
            GameCommand::OfferDraw { player } => self.offer_draw(player).await,
            GameCommand::AcceptDraw { player } => self.accept_draw(player),
            GameCommand::DeclineDraw { player } => self.decline_draw(player).await,
            GameCommand::Snapshot { player } => {
                self.hub.send(player, self.message());
            }
        }
    }

    /// The side to move's move: refused unless it is theirs and legal; their time is
    /// charged and the opponent's clock starts.
    async fn play(&mut self, player: Uuid, uci: &str) {
        let Some(side) = self.refuse_unless_playing(player) else {
            return;
        };
        if side != self.side_to_move() {
            return self.refuse(player, "It is not your turn");
        }
        let Ok(mv) = uci.parse::<Move>() else {
            return self.refuse(player, &format!("'{uci}' is not a move"));
        };
        // The flag goes by the time really taken; the least a move costs is charged after.
        let (elapsed, charged) = (self.elapsed(), self.move_time());
        if self.clock.is_flagged(side, elapsed) {
            self.flag();
            return;
        }
        if let Err(error) = self.game.play(mv) {
            return self.refuse(player, &error.to_string());
        }
        self.clock.finish_turn(side, charged);
        self.turn_started = (!self.game.is_over()).then(Instant::now);
        self.first_move_since = Instant::now();
        self.store_move(mv).await;
        // A move that ends the game is announced once, with its ratings, by `finish`.
        if !self.game.is_over() {
            self.broadcast();
        }
    }

    /// Calls the game off, if neither side has made a second move yet.
    fn request_abort(&mut self, player: Uuid) {
        if self.refuse_unless_playing(player).is_none() {
            return;
        }
        if !self.can_abort() {
            return self.refuse(player, "Both sides have moved; resign instead");
        }
        self.aborted = true;
    }

    fn can_abort(&self) -> bool {
        self.game.moves().len() < OPENING_PLIES
    }

    /// When the side to move must have made its first move, while one is still owed.
    fn first_move_deadline(&self) -> Option<Instant> {
        self.can_abort()
            .then(|| self.first_move_since + FIRST_MOVE_WAIT)
    }

    /// Stores the game as called off, tells both players, and lets the hub forget it.
    async fn abort(&mut self) {
        self.stop_clock();
        if let Err(error) = game::abort(&self.db, self.start.id).await {
            tracing::error!(?error, game = %self.start.id, "storing the abort failed");
        }
        self.broadcast();
        self.hub.game_over(self.start.id);
    }

    fn resign(&mut self, player: Uuid) {
        let Some(side) = self.refuse_unless_playing(player) else {
            return;
        };
        self.stop_clock();
        if let Err(error) = self.game.resign(side) {
            self.refuse(player, &error.to_string());
        }
    }

    async fn offer_draw(&mut self, player: Uuid) {
        let Some(side) = self.refuse_unless_playing(player) else {
            return;
        };
        if let Err(error) = self.game.offer_draw(side) {
            return self.refuse(player, &error.to_string());
        }
        self.store_draw_offer(Some(side.into())).await;
        self.broadcast();
    }

    fn accept_draw(&mut self, player: Uuid) {
        let Some(side) = self.refuse_unless_playing(player) else {
            return;
        };
        if let Err(error) = self.game.accept_draw(side) {
            return self.refuse(player, &error.to_string());
        }
        self.stop_clock();
    }

    async fn decline_draw(&mut self, player: Uuid) {
        let Some(side) = self.refuse_unless_playing(player) else {
            return;
        };
        if let Err(error) = self.game.decline_draw(side) {
            return self.refuse(player, &error.to_string());
        }
        self.store_draw_offer(None).await;
        self.broadcast();
    }

    /// The side to move ran out of time.
    fn flag(&mut self) {
        let side = self.side_to_move();
        self.stop_clock();
        // A game over by then has nothing to flag.
        let _ = self.game.flag(side);
    }

    /// Rates a rated game, stores how it ended, tells both players, and lets the hub forget it.
    async fn finish(&mut self) {
        self.stop_clock();
        let Some(outcome) = self.game.outcome() else {
            return;
        };
        if self.start.rated {
            let players = (
                (self.start.white.id, self.start.ratings.0),
                (self.start.black.id, self.start.ratings.1),
            );
            match rate_game(
                &self.db,
                self.start.id,
                players,
                &self.start.category,
                outcome.winner(),
            )
            .await
            {
                Ok(changes) => self.rating_changes = Some(changes),
                Err(error) => tracing::error!(?error, game = %self.start.id, "rating failed"),
            }
        }
        let finished = FinishedGame {
            game_id: self.start.id,
            result: GameResult::from(outcome),
            winner: outcome.winner().map(Color::from),
            white_ms: millis(self.remaining(Side::White)),
            black_ms: millis(self.remaining(Side::Black)),
        };
        if let Err(error) = game::finish(&self.db, &finished).await {
            tracing::error!(?error, game = %self.start.id, "storing the end failed");
        }
        self.broadcast();
        self.hub.game_over(self.start.id);
    }

    /// The player's side, or `None` and a refusal for someone not in this game.
    fn refuse_unless_playing(&self, player: Uuid) -> Option<Side> {
        let side = if player == self.start.white.id {
            Some(Side::White)
        } else if player == self.start.black.id {
            Some(Side::Black)
        } else {
            None
        };
        if side.is_none() {
            self.refuse(player, "You are not playing this game");
        }
        side
    }

    fn refuse(&self, player: Uuid, message: &str) {
        self.hub.send(
            player,
            ServerMessage::Error {
                message: message.to_owned(),
            },
        );
    }

    async fn store_move(&self, mv: Move) {
        let played = PlayedMove {
            game_id: self.start.id,
            ply: i32::try_from(self.game.moves().len()).unwrap_or(i32::MAX),
            uci: mv.to_string(),
            fen: self.game.live().to_string(),
            white_ms: millis(self.clock.remaining(Side::White, Duration::ZERO)),
            black_ms: millis(self.clock.remaining(Side::Black, Duration::ZERO)),
        };
        // The game goes on in memory; a lost row is logged, not fatal.
        if let Err(error) = game::record_move(&self.db, &played).await {
            tracing::error!(?error, game = %self.start.id, "storing a move failed");
        }
    }

    async fn store_draw_offer(&self, offered_by: Option<Color>) {
        if let Err(error) = game::set_draw_offer(&self.db, self.start.id, offered_by).await {
            tracing::error!(?error, game = %self.start.id, "storing a draw offer failed");
        }
    }

    fn side_to_move(&self) -> Side {
        self.game.live().side_to_move()
    }

    /// What the move being played costs: the time since the turn began, and at least
    /// `LEAST_MOVE_TIME` once the clock is running.
    fn move_time(&self) -> Duration {
        self.turn_started.map_or(Duration::ZERO, |started| {
            started.elapsed().max(LEAST_MOVE_TIME)
        })
    }

    fn elapsed(&self) -> Duration {
        self.turn_started
            .map_or(Duration::ZERO, |started| started.elapsed())
    }

    /// Takes the running turn's time off the side to move, and stops the clock.
    fn stop_clock(&mut self) {
        if self.turn_started.is_some() {
            self.clock.spend(self.side_to_move(), self.elapsed());
            self.turn_started = None;
        }
    }

    /// When the side to move's time runs out, if their clock is running.
    fn flag_deadline(&self) -> Option<Instant> {
        let left = self.clock.remaining(self.side_to_move(), Duration::ZERO);
        self.turn_started.map(|started| started + left)
    }

    /// `side`'s time left now, counting the running turn.
    fn remaining(&self, side: Side) -> Duration {
        let running = self.turn_started.is_some() && side == self.side_to_move();
        let elapsed = if running {
            self.elapsed()
        } else {
            Duration::ZERO
        };
        self.clock.remaining(side, elapsed)
    }

    fn snapshot(&self) -> GameSnapshot {
        let outcome = self.game.outcome();
        GameSnapshot {
            id: self.start.id,
            white: self.start.white.clone(),
            black: self.start.black.clone(),
            rated: self.start.rated,
            initial_s: seconds(self.start.control.initial),
            increment_s: seconds(self.start.control.increment),
            moves: self.game.moves().iter().map(ToString::to_string).collect(),
            white_ms: self
                .remaining(Side::White)
                .as_millis()
                .try_into()
                .unwrap_or(u64::MAX),
            black_ms: self
                .remaining(Side::Black)
                .as_millis()
                .try_into()
                .unwrap_or(u64::MAX),
            running: self.turn_started.map(|_| Color::from(self.side_to_move())),
            status: match (self.aborted, outcome) {
                (true, _) => GameStatus::Aborted,
                (false, Some(_)) => GameStatus::Finished,
                (false, None) => GameStatus::Ongoing,
            },
            result: outcome.map(GameResult::from),
            winner: outcome
                .and_then(|outcome| outcome.winner())
                .map(Color::from),
            draw_offer: self.game.draw_offer().map(Color::from),
            rating_changes: self.rating_changes,
            chat_id: self.start.chat_id,
        }
    }

    fn message(&self) -> ServerMessage {
        ServerMessage::Game {
            game: self.snapshot(),
        }
    }

    /// The game as it stands, to both players.
    fn broadcast(&self) {
        let message = self.message();
        self.hub.send(self.start.white.id, message.clone());
        self.hub.send(self.start.black.id, message);
    }
}

fn millis(duration: Duration) -> i32 {
    i32::try_from(duration.as_millis()).unwrap_or(i32::MAX)
}

fn seconds(duration: Duration) -> u32 {
    u32::try_from(duration.as_secs()).unwrap_or(u32::MAX)
}
