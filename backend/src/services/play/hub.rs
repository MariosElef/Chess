use std::{
    collections::{HashMap, HashSet},
    sync::{
        Arc, Mutex, PoisonError,
        atomic::{AtomicU64, Ordering},
    },
    time::Duration,
};

mod battles;
mod challenges;
mod presence;
mod rematches;

use chess_domain::{Position, Rating, TimeControl};
use tokio::sync::mpsc;
use uuid::Uuid;

use self::{
    battles::{BattleEntry, BattleSeek, PendingBattle},
    challenges::PendingChallenge,
};
use super::{
    live_game::{GameCommand, GameStart, spawn_game},
    pairing::{Seek, cancel, queue, take_match},
    whole_seconds::{whole_seconds, whole_seconds_up},
};
use crate::{
    enums::chat::ConversationKind,
    models::{
        chat::conversation::NewConversation,
        play::{
            game::NewGame,
            messages::{ClientMessage, ServerMessage},
            player::{Player, PlayerModel},
        },
        social::friend::UserRef,
    },
    repositories::{account::user, chat, database::Db, play::game, rating},
    services::rating::{from_row, whole},
};

/// Messages queued for one connection; a connection this far behind is dropped.
const OUTBOX_SIZE: usize = 64;
/// The longest starting time and increment a seek may ask for.
const LONGEST_START: u32 = 3 * 60 * 60;
const LONGEST_INCREMENT: u32 = 180;

/// Live play: who is connected, who is waiting for an opponent, and the games being played.
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
    battle_seeks: Mutex<Vec<BattleSeek>>,
    battle_challenges: Mutex<Vec<PendingBattle>>,
    battles: Mutex<HashMap<Uuid, BattleEntry>>,
    /// Players whose battle is being set up: its puzzles picked, its row stored.
    battles_starting: Mutex<HashSet<Uuid>>,
    /// Players whose game is being set up: a second can start for neither meanwhile.
    games_starting: Mutex<HashSet<Uuid>>,
    /// Closed accounts: anything still on its way from one of their sockets is ignored.
    closed: Mutex<HashSet<Uuid>>,
    /// Pictures changed since their players' sockets opened, which carry the old ones.
    pictures: Mutex<HashMap<Uuid, Option<String>>>,
}

/// The two players of a game about to start: either way round, or White then Black.
enum Players<'a> {
    EitherWay(&'a Player, &'a Player),
    Fixed(&'a Player, &'a Player),
}

/// One open socket's queue of messages to send, and the session that opened it.
struct Outbox {
    id: u64,
    session: Uuid,
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
                battle_seeks: Mutex::new(Vec::new()),
                battle_challenges: Mutex::new(Vec::new()),
                battles: Mutex::new(HashMap::new()),
                battles_starting: Mutex::new(HashSet::new()),
                games_starting: Mutex::new(HashSet::new()),
                closed: Mutex::new(HashSet::new()),
                pictures: Mutex::new(HashMap::new()),
            }),
        }
    }

    /// A socket for `player` on `session`, sent their games, battle, challenges and invitations.
    pub fn connect(&self, player: &Player, session: Uuid) -> Connection {
        let (sender, messages) = mpsc::channel(OUTBOX_SIZE);
        let id = self.inner.next_connection.fetch_add(1, Ordering::Relaxed);
        let first = {
            let mut connections = lock(&self.inner.connections);
            // Checked under the connections lock that closing takes too: a socket opening as its
            // account closes either is shut by it or finds it closed and has its outbox dropped.
            if self.is_closed(player.id) {
                return Connection { id, messages };
            }
            let outboxes = connections.entry(player.id).or_default();
            outboxes.push(Outbox {
                id,
                session,
                sender,
            });
            outboxes.len() == 1
        };
        for commands in self.games_of(player.id) {
            // A full or finished game has its own way to reach the player.
            let _ = commands.try_send(GameCommand::Snapshot { player: player.id });
        }
        self.send_challenges(player.id);
        self.resend_battle(player.id);
        self.send_battle_challenges(player.id);
        if first {
            self.remember_online_and_tell(player);
        }
        Connection { id, messages }
    }

    /// Whether the player has the app open: at least one socket connected.
    pub fn is_online(&self, player: Uuid) -> bool {
        lock(&self.inner.connections).contains_key(&player)
    }

    /// Everyone the live server holds a place for: connected now, or awaited back at a board.
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

    /// Closes the player's sockets opened by any session but `keep`: their sessions have ended.
    pub fn close_sockets_except(&self, player: Uuid, keep: Uuid) {
        // Dropping an outbox ends its socket, which then says it is gone through `disconnect`.
        if let Some(outboxes) = lock(&self.inner.connections).get_mut(&player) {
            outboxes.retain(|outbox| outbox.session == keep);
        }
    }

    /// A socket closed; with the player's last one gone, their seeks and invitations go too.
    pub fn disconnect(&self, player: &Player, connection_id: u64) {
        let gone = {
            let mut connections = lock(&self.inner.connections);
            // Already gone with the player's other sockets: their leaving was handled then.
            let Some(outboxes) = connections.get_mut(&player.id) else {
                return;
            };
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
            self.drop_battle_wishes_of(player.id);
            self.remember_online_and_tell(player);
        }
    }

    /// Stores now as when the player last had the app open, then tells their friends.
    fn remember_online_and_tell(&self, player: &Player) {
        let hub = self.clone();
        let player = player.clone();
        tokio::spawn(async move {
            if let Err(error) = user::mark_online(&hub.inner.db, player.id).await {
                tracing::error!(?error, "storing when a player was online failed");
            }
            hub.tell_friends(&player);
        });
    }

    /// Handles a message from one of the player's sockets; a refusal answers that socket alone.
    pub async fn handle(&self, player: &Player, socket: u64, message: ClientMessage) {
        ASKING_SOCKET
            .scope((player.id, socket), self.dispatch(player, message))
            .await;
    }

    async fn dispatch(&self, player: &Player, message: ClientMessage) {
        if self.is_closed(player.id) {
            return;
        }
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
            ClientMessage::OfferRematch { game_id } => {
                self.offer_rematch(player, game_id).await;
            }
            ClientMessage::AcceptRematch { game_id } => {
                self.accept_rematch(player, game_id).await;
            }
            ClientMessage::CancelRematch { game_id } => self.cancel_rematch(player, game_id),
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
            ClientMessage::SeekBattle { rated } => self.seek_battle(player, rated).await,
            ClientMessage::CancelBattleSeek => self.cancel_battle_seek(player.id),
            ClientMessage::BattleAnswer { battle_id, solved } => {
                self.battle_answer(player, battle_id, solved).await;
            }
            ClientMessage::ChallengeBattle { user_id, rated } => {
                self.challenge_battle(player, user_id, rated).await;
            }
            ClientMessage::AcceptBattle { challenge_id } => {
                self.accept_battle(player, challenge_id).await;
            }
            ClientMessage::DeclineBattle { challenge_id } => {
                self.decline_battle(player, challenge_id);
            }
            ClientMessage::ResignBattle { battle_id } => {
                self.resign_battle(player, battle_id).await;
            }
        }
    }

    /// Sends `message` to every socket the player has open; one too far behind is dropped.
    pub fn send(&self, player: Uuid, message: ServerMessage) {
        if let Some(outboxes) = lock(&self.inner.connections).get_mut(&player) {
            outboxes.retain(|outbox| outbox.sender.try_send(message.clone()).is_ok());
        }
    }

    /// A paired game did not start: the seeker waits again, hears why, or was seated elsewhere.
    fn seek_again_or_refuse(&self, seek: &Seek, refusal: StartRefusal) {
        let free = !self.is_playing(seek.player.id) && self.is_online(seek.player.id);
        match after_failed_start(refusal, free) {
            AfterFailedStart::Nothing => {}
            // Sought again from the start: another waiting seeker may take them now.
            AfterFailedStart::SeekAgain => {
                let (hub, seek) = (self.clone(), seek.clone());
                let control = (
                    whole_seconds(seek.control.initial),
                    whole_seconds(seek.control.increment),
                );
                tokio::spawn(async move { hub.seek(&seek.player, control, seek.rated).await });
            }
            AfterFailedStart::Refuse(reason) => self.refuse(seek.player.id, reason),
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
        if self.is_playing(player.id) {
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
                let players = Players::EitherWay(&opponent.player, &new.player);
                if let Err(refusal) = self.start_game(players, new.control, rated).await {
                    self.seek_again_or_refuse(&opponent, refusal);
                    self.seek_again_or_refuse(&new, refusal);
                }
            }
            None => self.send(player.id, seeking_message(&new)),
        }
    }

    /// Two players meet: colours drawn or fixed, the game is stored and starts; its id, or why not.
    async fn start_game(
        &self,
        players: Players<'_>,
        control: TimeControl,
        rated: bool,
    ) -> Result<Uuid, StartRefusal> {
        let (white, black) = match players {
            Players::EitherWay(first, second) if rand::random() => (first, second),
            Players::EitherWay(first, second) => (second, first),
            Players::Fixed(white, black) => (white, black),
        };
        // Two games accepted at once, as from two devices, cannot both seat the same player.
        if !self.take_seats([white.id, black.id]) {
            return Err(StartRefusal::SeatsTaken);
        }
        let started = self
            .seat_and_start((white, black), control, rated)
            .await
            .map_err(StartRefusal::Failed);
        let mut starting = lock(&self.inner.games_starting);
        starting.remove(&white.id);
        starting.remove(&black.id);
        started
    }

    /// Reserves both players for a game being set up, unless either is playing or being seated.
    fn take_seats(&self, players: [Uuid; 2]) -> bool {
        let in_game = |player: Uuid| !self.games_of(player).is_empty();
        reserve(&mut lock(&self.inner.games_starting), players, in_game)
    }

    /// Whether the player is in a game, or one is being set up for them.
    fn is_playing(&self, player: Uuid) -> bool {
        !self.games_of(player).is_empty() || lock(&self.inner.games_starting).contains(&player)
    }

    /// Stores the game and its chat, and starts it; its id, or why not.
    async fn seat_and_start(
        &self,
        (white, black): (&Player, &Player),
        control: TimeControl,
        rated: bool,
    ) -> Result<Uuid, &'static str> {
        {
            let mut seeks = lock(&self.inner.seeks);
            cancel(&mut seeks, white);
            cancel(&mut seeks, black);
        }
        if [white.id, black.id].iter().any(|id| self.is_closed(*id)) {
            return Err("The game could not start: your opponent has left");
        }
        let category = control.category().name().to_lowercase();
        let Some(chat_id) = self.start_game_chat([white.id, black.id]).await else {
            return Err("The game could not start: try again");
        };
        let (white_model, white_rating) = self.at_board(white, &category).await;
        let (black_model, black_rating) = self.at_board(black, &category).await;
        let start = GameStart {
            id: Uuid::now_v7(),
            white: white_model,
            black: black_model,
            ratings: (white_rating, black_rating),
            rated,
            control,
            category,
            chat_id,
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
            conversation_id: chat_id,
        };
        if let Err(error) = game::insert(&self.inner.db, &new_game).await {
            tracing::error!(?error, "starting a game failed");
            return Err("The game could not start: try again");
        }
        let players = [white.id, black.id];
        let commands = spawn_game(start, self.clone(), self.inner.db.clone());
        lock(&self.inner.games).insert(
            new_game.id,
            LiveEntry {
                players,
                commands: commands.clone(),
            },
        );
        // A player whose account closed while the game was being stored resigns it at once.
        for player in players.into_iter().filter(|id| self.is_closed(*id)) {
            let _ = commands.try_send(GameCommand::Resign { player });
        }
        // Both are playing now, so nothing else they had waiting can start.
        self.drop_challenges_of(white.id);
        self.drop_challenges_of(black.id);
        Ok(new_game.id)
    }

    /// The two players' chat for a game about to start; `None` if it could not be stored.
    async fn start_game_chat(&self, players: [Uuid; 2]) -> Option<Uuid> {
        let id = Uuid::now_v7();
        let new = NewConversation {
            id,
            kind: ConversationKind::Game,
            title: None,
            direct_key: None,
            created_by_id: None,
        };
        let stored = async {
            let mut tx = self.inner.db.begin().await?;
            chat::conversation::insert(&mut *tx, &new).await?;
            for player in players {
                chat::conversation::add_member(&mut *tx, id, player, "member").await?;
            }
            tx.commit().await
        };
        match stored.await {
            Ok(()) => Some(id),
            Err(error) => {
                tracing::error!(?error, "starting a game's chat failed");
                None
            }
        }
    }

    /// The player as their opponent sees them, with their rating in the category; none for a guest.
    async fn at_board(&self, player: &Player, category: &str) -> (PlayerModel, Rating) {
        let stored = rating::get(&self.inner.db, player.id, category).await;
        let current = stored.ok().flatten().map_or(Rating::NEW, from_row);
        let model = PlayerModel {
            id: player.id,
            username: player.username.clone(),
            avatar: self.picture_of(player),
            rating: (!player.guest).then(|| whole(current.rating)),
        };
        (model, current)
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

    /// The player's picture changed: challenges and invitations they send carry the new one.
    pub fn picture_changed(&self, player: Uuid, avatar: Option<String>) {
        lock(&self.inner.pictures).insert(player, avatar.clone());
        self.repicture_challenges(player, avatar.as_deref());
        self.repicture_battle_challenges(player, avatar.as_deref());
    }

    /// The player as a challenge or invitation names them, with their picture as it is now.
    fn user_ref(&self, player: &Player) -> UserRef {
        UserRef {
            id: player.id,
            username: player.username.clone(),
            avatar: self.picture_of(player),
        }
    }

    /// The player's picture as it is now: changed since their socket opened, or as it was then.
    fn picture_of(&self, player: &Player) -> Option<String> {
        let changed = lock(&self.inner.pictures).get(&player.id).cloned();
        changed.unwrap_or_else(|| player.avatar.clone())
    }

    /// A closed account's last word: its seeks go, its games are resigned and its sockets shut.
    pub fn forget_player(&self, player: Uuid) {
        {
            let mut connections = lock(&self.inner.connections);
            lock(&self.inner.closed).insert(player);
            connections.remove(&player);
        }
        lock(&self.inner.seeks).retain(|seek| seek.player.id != player);
        self.drop_challenges_of(player);
        self.drop_battle_wishes_of(player);
        for commands in self.games_of(player) {
            // A game already over has its result.
            let _ = commands.try_send(GameCommand::Resign { player });
        }
        self.resign_battles_of(player);
    }

    /// Whether the player's account has closed since the hub started.
    fn is_closed(&self, player: Uuid) -> bool {
        lock(&self.inner.closed).contains(&player)
    }

    fn games_of(&self, player: Uuid) -> Vec<mpsc::Sender<GameCommand>> {
        lock(&self.inner.games)
            .values()
            .filter(|entry| entry.players.contains(&player))
            .map(|entry| entry.commands.clone())
            .collect()
    }

    fn refuse(&self, player: Uuid, message: &str) {
        self.reply(
            player,
            ServerMessage::Error {
                message: message.to_owned(),
            },
        );
    }

    /// Sends to the asking socket when the player is who asked, else to all of the player's.
    fn reply(&self, player: Uuid, message: ServerMessage) {
        let asking = ASKING_SOCKET
            .try_with(|(asker, socket)| (*asker == player).then_some(*socket))
            .ok()
            .flatten();
        if let Some(outboxes) = lock(&self.inner.connections).get_mut(&player) {
            // The player's other devices did not ask: they hear nothing, even once it has closed.
            outboxes.retain(|outbox| {
                asking.is_some_and(|id| outbox.id != id)
                    || outbox.sender.try_send(message.clone()).is_ok()
            });
        }
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

/// Takes the lock even if a panic poisoned it, as its entries do not depend on each other.
fn lock<T>(mutex: &Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(PoisonError::into_inner)
}

tokio::task_local! {
    /// The player and socket whose message is being handled, so a refusal answers it alone.
    static ASKING_SOCKET: (Uuid, u64);
}

/// Why a game did not start.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum StartRefusal {
    /// One of the two is already playing, or being seated in another game.
    SeatsTaken,
    /// It failed for both, in a sentence.
    Failed(&'static str),
}

impl StartRefusal {
    /// The refusal in a sentence, as the players hear it.
    fn message(self) -> &'static str {
        match self {
            StartRefusal::SeatsTaken => "The game could not start: one of you is already playing",
            StartRefusal::Failed(message) => message,
        }
    }
}

/// What becomes of a seeker whose paired game did not start.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum AfterFailedStart {
    /// Seated in another game meanwhile, or gone: nothing more to do.
    Nothing,
    /// Free, and only the other player was taken: seeking again.
    SeekAgain,
    /// The game failed for both: told why.
    Refuse(&'static str),
}

/// What a seeker's failed game means for them, by why it failed and whether they are still free.
fn after_failed_start(refusal: StartRefusal, free: bool) -> AfterFailedStart {
    match (refusal, free) {
        (_, false) => AfterFailedStart::Nothing,
        (StartRefusal::SeatsTaken, true) => AfterFailedStart::SeekAgain,
        (StartRefusal::Failed(message), true) => AfterFailedStart::Refuse(message),
    }
}

/// The word that the player is waiting for a game like `seek`.
fn seeking_message(seek: &Seek) -> ServerMessage {
    ServerMessage::Seeking {
        initial_s: whole_seconds(seek.control.initial),
        increment_s: whole_seconds(seek.control.increment),
        rated: seek.rated,
    }
}

/// Reserves both players in `starting`, unless either is busy or already being seated.
fn reserve(starting: &mut HashSet<Uuid>, players: [Uuid; 2], busy: impl Fn(Uuid) -> bool) -> bool {
    let free = players
        .iter()
        .all(|player| !starting.contains(player) && !busy(*player));
    if free {
        starting.extend(players);
    }
    free
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use uuid::Uuid;

    use super::{AfterFailedStart, StartRefusal, after_failed_start, reserve};

    #[test]
    fn a_player_is_reserved_for_one_game_at_a_time() {
        let (ann, bob, cat) = (Uuid::from_u128(1), Uuid::from_u128(2), Uuid::from_u128(3));
        let mut starting = HashSet::new();
        assert!(reserve(&mut starting, [ann, bob], |_| false));
        assert!(
            !reserve(&mut starting, [ann, cat], |_| false),
            "Ann is being seated"
        );
        assert!(
            !reserve(&mut starting, [bob, cat], |_| false),
            "Bob is being seated"
        );
        assert!(
            !reserve(&mut HashSet::new(), [cat, ann], |player| player == cat),
            "in a game"
        );
        assert_eq!(starting.len(), 2, "a refused pair reserves nobody");
    }

    #[test]
    fn a_seeker_whose_game_failed_waits_again_only_if_the_other_was_taken() {
        let failed = StartRefusal::Failed("The game could not start: try again");
        let taken = StartRefusal::SeatsTaken;
        assert_eq!(after_failed_start(taken, true), AfterFailedStart::SeekAgain);
        assert_eq!(after_failed_start(taken, false), AfterFailedStart::Nothing);
        assert_eq!(
            after_failed_start(failed, true),
            AfterFailedStart::Refuse("The game could not start: try again")
        );
    }
}
