use std::{
    collections::HashMap,
    sync::{
        Arc, Mutex, PoisonError,
        atomic::{AtomicU64, Ordering},
    },
    time::Duration,
};

mod challenges;
mod presence;

use chess_domain::{Position, Rating, TimeControl};
use tokio::sync::mpsc;
use uuid::Uuid;

use self::challenges::PendingChallenge;
use super::{
    live_game::{GameCommand, GameStart, spawn_game},
    pairing::{Seek, cancel, queue, take_match},
};
use crate::{
    models::play::{
        game::NewGame,
        messages::{ClientMessage, ServerMessage},
        player::{Player, PlayerModel},
    },
    repositories::{database::Db, play::game, rating},
    services::rating::{from_row, whole},
};

/// Messages queued for one connection; a connection this far behind is dropped.
const OUTBOX_SIZE: usize = 64;
/// The longest starting time and increment a seek may ask for.
const LONGEST_START: u32 = 3 * 60 * 60;
const LONGEST_INCREMENT: u32 = 180;

/// Live play: who is connected, who is waiting for an opponent, and the games being played.
/// Each game runs on a task of its own; the hub only routes messages to and from it. Cloning
/// shares it.
#[derive(Clone)]
pub struct PlayHub {
    inner: Arc<HubInner>,
}

struct HubInner {
    db: Db,
    connections: Mutex<HashMap<Uuid, Vec<Outbox>>>,
    next_connection: AtomicU64,
    games: Mutex<HashMap<Uuid, LiveEntry>>,
    seeks: Mutex<Vec<Seek>>,
    challenges: Mutex<Vec<PendingChallenge>>,
}

/// One open socket's queue of messages to send.
struct Outbox {
    id: u64,
    sender: mpsc::Sender<ServerMessage>,
}

/// A game being played: its players, and where its commands go.
struct LiveEntry {
    players: [Uuid; 2],
    commands: mpsc::Sender<GameCommand>,
}

/// A connected socket: its id, for leaving, and the messages to send it.
pub struct Connection {
    pub id: u64,
    pub messages: mpsc::Receiver<ServerMessage>,
}

impl PlayHub {
    pub fn new(db: Db) -> Self {
        Self {
            inner: Arc::new(HubInner {
                db,
                connections: Mutex::new(HashMap::new()),
                next_connection: AtomicU64::new(0),
                games: Mutex::new(HashMap::new()),
                seeks: Mutex::new(Vec::new()),
                challenges: Mutex::new(Vec::new()),
            }),
        }
    }

    /// A new socket for `player`; their games in progress and challenges are sent to it
    /// straight away, so reconnecting puts them back where they were. Their friends hear they
    /// came online.
    pub fn connect(&self, player: &Player) -> Connection {
        let (sender, messages) = mpsc::channel(OUTBOX_SIZE);
        let id = self.inner.next_connection.fetch_add(1, Ordering::Relaxed);
        let first = {
            let mut connections = lock(&self.inner.connections);
            let outboxes = connections.entry(player.id).or_default();
            outboxes.push(Outbox { id, sender });
            outboxes.len() == 1
        };
        for commands in self.games_of(player.id) {
            // A full or finished game has its own way to reach the player.
            let _ = commands.try_send(GameCommand::Snapshot { player: player.id });
        }
        self.send_challenges(player.id);
        if first {
            self.tell_friends(player);
        }
        Connection { id, messages }
    }

    /// Whether the player has the app open: at least one socket connected.
    pub fn is_online(&self, player: Uuid) -> bool {
        lock(&self.inner.connections).contains_key(&player)
    }

    /// Everyone the live server is holding a place for: connected now, or at a board waiting
    /// for them to come back.
    pub fn players_present(&self) -> Vec<Uuid> {
        let mut present: Vec<Uuid> = lock(&self.inner.connections).keys().copied().collect();
        let at_boards = lock(&self.inner.games)
            .values()
            .flat_map(|game| game.players)
            .collect::<Vec<_>>();
        present.extend(at_boards);
        present.sort_unstable();
        present.dedup();
        present
    }

    /// A socket closed; with the player's last one gone, their seek and challenges go too, and
    /// their friends hear they left.
    pub fn disconnect(&self, player: &Player, connection_id: u64) {
        let gone = {
            let mut connections = lock(&self.inner.connections);
            let outboxes = connections.entry(player.id).or_default();
            outboxes.retain(|outbox| outbox.id != connection_id);
            let gone = outboxes.is_empty();
            if gone {
                connections.remove(&player.id);
            }
            gone
        };
        if gone {
            cancel(&mut lock(&self.inner.seeks), player);
            self.drop_challenges_of(player.id);
            self.tell_friends(player);
        }
    }

    pub async fn handle(&self, player: &Player, message: ClientMessage) {
        match message {
            ClientMessage::Seek {
                initial_s,
                increment_s,
                rated,
            } => self.seek(player, (initial_s, increment_s), rated).await,
            ClientMessage::CancelSeek => {
                cancel(&mut lock(&self.inner.seeks), player);
                self.send(player.id, ServerMessage::SeekCancelled);
            }
            ClientMessage::Move { game_id, uci } => {
                let command = GameCommand::Move {
                    player: player.id,
                    uci,
                };
                self.command(player, game_id, command).await;
            }
            ClientMessage::Resign { game_id } => {
                let command = GameCommand::Resign { player: player.id };
                self.command(player, game_id, command).await;
            }
            ClientMessage::Abort { game_id } => {
                let command = GameCommand::Abort { player: player.id };
                self.command(player, game_id, command).await;
            }
            ClientMessage::OfferDraw { game_id } => {
                let command = GameCommand::OfferDraw { player: player.id };
                self.command(player, game_id, command).await;
            }
            ClientMessage::AcceptDraw { game_id } => {
                let command = GameCommand::AcceptDraw { player: player.id };
                self.command(player, game_id, command).await;
            }
            ClientMessage::DeclineDraw { game_id } => {
                let command = GameCommand::DeclineDraw { player: player.id };
                self.command(player, game_id, command).await;
            }
            ClientMessage::Challenge {
                user_id,
                initial_s,
                increment_s,
                rated,
            } => {
                self.challenge(player, user_id, (initial_s, increment_s), rated)
                    .await;
            }
            ClientMessage::CancelChallenge { challenge_id } => {
                self.cancel_challenge(player, challenge_id);
            }
            ClientMessage::AcceptChallenge { challenge_id } => {
                self.accept_challenge(player, challenge_id).await;
            }
            ClientMessage::DeclineChallenge { challenge_id } => {
                self.decline_challenge(player, challenge_id);
            }
        }
    }

    /// Sends `message` to every socket the player has open; one too far behind is dropped.
    pub fn send(&self, player: Uuid, message: ServerMessage) {
        if let Some(outboxes) = lock(&self.inner.connections).get_mut(&player) {
            outboxes.retain(|outbox| outbox.sender.try_send(message.clone()).is_ok());
        }
    }

    /// The game is over: it takes no more commands.
    pub fn game_over(&self, game_id: Uuid) {
        lock(&self.inner.games).remove(&game_id);
    }

    /// Pairs the player with someone waiting for the same game, or queues them.
    async fn seek(&self, player: &Player, (initial_s, increment_s): (u32, u32), rated: bool) {
        if let Err(reason) = seek_refusal(player, (initial_s, increment_s), rated) {
            return self.refuse(player.id, reason);
        }
        if !self.games_of(player.id).is_empty() {
            return self.refuse(player.id, "Finish your game first");
        }
        let new = Seek {
            player: player.clone(),
            control: TimeControl {
                initial: Duration::from_secs(initial_s.into()),
                increment: Duration::from_secs(increment_s.into()),
            },
            rated,
        };
        let opponent = {
            let mut seeks = lock(&self.inner.seeks);
            let opponent = take_match(&mut seeks, &new);
            if opponent.is_none() {
                queue(&mut seeks, new.clone());
            }
            opponent
        };
        match opponent {
            Some(opponent) => {
                self.start_game((&opponent.player, &new.player), new.control, rated)
                    .await;
            }
            None => self.send(
                player.id,
                ServerMessage::Seeking {
                    initial_s,
                    increment_s,
                    rated,
                },
            ),
        }
    }

    /// Two players meet, from seeks or a challenge: colours are drawn, any seeks and
    /// challenges of theirs go, the game is stored and its task starts. The game's id, unless it could not start.
    async fn start_game(
        &self,
        (first, second): (&Player, &Player),
        control: TimeControl,
        rated: bool,
    ) -> Option<Uuid> {
        let (white, black) = if rand::random() {
            (first, second)
        } else {
            (second, first)
        };
        {
            let mut seeks = lock(&self.inner.seeks);
            cancel(&mut seeks, white);
            cancel(&mut seeks, black);
        }
        let category = control.category().name().to_lowercase();
        let start = GameStart {
            id: Uuid::now_v7(),
            white: self.at_board(white, &category).await,
            black: self.at_board(black, &category).await,
            rated,
            control,
            category,
        };
        let new_game = NewGame {
            id: start.id,
            white_id: white.id,
            black_id: black.id,
            category: start.category.clone(),
            initial_s: i32::try_from(control.initial.as_secs()).unwrap_or(i32::MAX),
            increment_s: i32::try_from(control.increment.as_secs()).unwrap_or(i32::MAX),
            rated,
            fen: Position::starting().to_string(),
        };
        if let Err(error) = game::insert(&self.inner.db, &new_game).await {
            tracing::error!(?error, "starting a game failed");
            for player in [white.id, black.id] {
                self.refuse(player, "The game could not start: try again");
            }
            return None;
        }
        let players = [white.id, black.id];
        let commands = spawn_game(start, self.clone(), self.inner.db.clone());
        lock(&self.inner.games).insert(new_game.id, LiveEntry { players, commands });
        // Both are playing now, so nothing else they had waiting can start.
        self.drop_challenges_of(white.id);
        self.drop_challenges_of(black.id);
        Some(new_game.id)
    }

    /// The player as their opponent sees them: with their rating in the category, or none for
    /// a guest. A player without one yet shows the starting rating.
    async fn at_board(&self, player: &Player, category: &str) -> PlayerModel {
        let rating = if player.guest {
            None
        } else {
            let stored = rating::get(&self.inner.db, player.id, category).await;
            let current = stored.ok().flatten().map_or(Rating::NEW, from_row);
            Some(whole(current.rating))
        };
        PlayerModel {
            id: player.id,
            username: player.username.clone(),
            rating,
        }
    }

    /// Hands the command to the game, if the player is in it.
    async fn command(&self, player: &Player, game_id: Uuid, command: GameCommand) {
        let commands = lock(&self.inner.games)
            .get(&game_id)
            .filter(|entry| entry.players.contains(&player.id))
            .map(|entry| entry.commands.clone());
        match commands {
            Some(commands) => {
                // A game that just ended takes nothing more, and says so in its last message.
                let _ = commands.send(command).await;
            }
            None => self.refuse(player.id, "That game is not being played"),
        }
    }

    fn games_of(&self, player: Uuid) -> Vec<mpsc::Sender<GameCommand>> {
        lock(&self.inner.games)
            .values()
            .filter(|entry| entry.players.contains(&player))
            .map(|entry| entry.commands.clone())
            .collect()
    }

    fn refuse(&self, player: Uuid, message: &str) {
        self.send(
            player,
            ServerMessage::Error {
                message: message.to_owned(),
            },
        );
    }
}

/// Why a seek cannot be taken, if it cannot.
fn seek_refusal(
    player: &Player,
    (initial_s, increment_s): (u32, u32),
    rated: bool,
) -> Result<(), &'static str> {
    if rated && player.guest {
        return Err("Guests play unrated games: sign up to play rated");
    }
    if !(1..=LONGEST_START).contains(&initial_s) || increment_s > LONGEST_INCREMENT {
        return Err("That time control is not played here");
    }
    Ok(())
}

/// Each lock guards entries that do not depend on each other, so one poisoned by a panic is
/// still safe to use.
fn lock<T>(mutex: &Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(PoisonError::into_inner)
}
