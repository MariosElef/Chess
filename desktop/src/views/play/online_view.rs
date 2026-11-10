use std::{
    rc::Rc,
    time::{Duration, Instant},
};

use chess_domain::{Move, Side, TimeControl};
use gpui_kit::{
    assets::IconName,
    component::{ActiveTheme as _, h_flex, label::Label, v_flex},
    prelude::FluentBuilder as _,
    *,
};

use super::game_controls::game_tile;
use crate::{
    components::{
        board::{BoardColumn, PLAYER_BAR, board_page, copy_buttons},
        history::{MoveList, NavigationBar},
        panel::{ChoiceTile, PanelHeader, SidePanel, action_bar},
        play::{
            CapturedRow, ClockFace, GameOverActions, GameOverCard, GameSetup, OnlineOptions,
            PlayPanel, WaitingBar, with_card,
        },
        shell::PLAY_PICTURE,
        social::strip_avatar,
    },
    enums::{
        play::{LiveStatus, PlayMode},
        sound::Sound,
    },
    i18n::tr,
    models::{
        account::ModeStats,
        board::{MoveMade, ShownPosition},
        play::{AnalyzeGame, Challenge, ClientMessage, GameSnapshot, LiveEvent, ServerMessage},
    },
    services::{
        history::GameHistory,
        play::{
            ClockWarning, DEFAULT_TIME_CONTROL, can_abort, clock_left, has_ended, my_side,
            outcome_of, outcome_reason, rating_line, replay, result_line, result_title,
            whole_seconds,
        },
        social::face_of,
    },
    views::{
        board::{BoardView, flip_button},
        chat::ChatView,
        history::{
            HistoryNavigation, Holding, hold_step, select_move, step_history, with_history_keys,
        },
        live::{LiveConnection, SessionSource, my_face},
        sound::{play_move_sound, play_sound},
        ticker::{CLOCK_TICK, start_ticker},
    },
};

/// How long after a rematch goes a new game may take to arrive, before "No rematch" shows.
const REMATCH_SETTLES: Duration = Duration::from_millis(1500);

/// Switches the Play page to another way to play.
type OnMode = Rc<dyn Fn(PlayMode, &mut Window, &mut App)>;

/// An online game on the board: the server's last word, the history, and an unconfirmed move.
struct LiveBoard {
    snapshot: GameSnapshot,
    received_at: Instant,
    history: GameHistory,
    me: Option<Side>,
    pending: Option<Move>,
    /// Whether the player has heard their clock is low this game.
    clock_warning: ClockWarning,
}

/// Play against someone online: a lobby to ask for a game, then the game over the socket.
pub struct OnlineView {
    connection: Entity<LiveConnection>,
    _live: Subscription,
    session: SessionSource,
    /// A seek was sent and its answer, or refusal, is still to come.
    awaiting: bool,
    choice: TimeControl,
    rated: bool,
    seeking: bool,
    live: Option<LiveBoard>,
    problem: Option<LiveProblem>,
    board: Entity<BoardView>,
    _board_moves: Subscription,
    /// A key or navigation button held down.
    holding: Holding,
    _ticker: Option<Task<()>>,
    focus_handle: FocusHandle,
    /// The two players' chat, beside the moves; made when the game is first drawn.
    chat: Option<Entity<ChatView>>,
    /// The end-of-game card was put away, to look at the board.
    card_closed: bool,
    /// The player's rating in each speed, for their bar under the board.
    ratings: Vec<ModeStats>,
    /// The lobby's tabs: another way to play, which the Play page shows.
    on_mode: Option<OnMode>,
    /// A rematch of the game on the board: asked for by the player, or by their opponent.
    rematch: Rematch,
    /// A rematch sent has gone: unless a new game follows soon, it was declined or ran out.
    _rematch_gone: Option<Task<()>>,
    /// The game this app last asked a rematch of; another device's rematches are left alone.
    asked_rematch_of: Option<String>,
    /// The player pressed Accept on the opponent's rematch; its refusal is this app's to show.
    accepting_rematch: bool,
    /// An ask was in flight as the connection dropped: the next lists say whether it was posted.
    ask_unsure: bool,
}

/// Why the last request went wrong: the server's own sentence, or something this app found.
/// The app's own are kept as reasons and said when drawn, so they follow the language.
#[derive(Debug, Clone, PartialEq, Eq)]
enum LiveProblem {
    Server(String),
    NoRematch,
    GameUnreadable,
}

impl LiveProblem {
    fn message(&self) -> String {
        match self {
            Self::Server(message) => message.clone(),
            Self::NoRematch => tr!("play.problem.no_rematch"),
            Self::GameUnreadable => tr!("play.problem.game_unreadable"),
        }
    }
}

/// Where a rematch of the finished game on the board stands.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
enum Rematch {
    #[default]
    None,
    /// The player pressed Rematch; the server has yet to post it.
    Asking,
    /// The player asked; the opponent has yet to answer.
    Sent,
    /// The opponent asked; the player can accept.
    Received(String),
}

impl OnlineView {
    pub fn new(
        connection: Entity<LiveConnection>,
        session: SessionSource,
        cx: &mut Context<Self>,
    ) -> Self {
        let _live = cx.subscribe(&connection, |view, _, event: &LiveEvent, cx| {
            view.on_event(event.clone(), cx);
        });
        let view = cx.weak_entity();
        let board = cx.new(|board| {
            BoardView::new(
                move |cx| view.upgrade().map(|view| view.read(cx).shown_position()),
                board,
            )
        });
        let _board_moves = cx.subscribe(&board, |view, _, MoveMade(mv), cx| {
            view.move_made(*mv, cx);
        });
        Self {
            connection,
            _live,
            session,
            awaiting: false,
            choice: DEFAULT_TIME_CONTROL.unwrap_or(TimeControl::new(3, 2)),
            rated: true,
            seeking: false,
            live: None,
            problem: None,
            board,
            _board_moves,
            holding: Holding::default(),
            _ticker: None,
            focus_handle: cx.focus_handle(),
            chat: None,
            rematch: Rematch::default(),
            _rematch_gone: None,
            asked_rematch_of: None,
            accepting_rematch: false,
            ask_unsure: false,
            card_closed: false,
            ratings: Vec::new(),
            on_mode: None,
        }
    }

    /// The chat follows the game on the board: one for each game the player is in.
    fn keep_chat(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let wanted = self
            .live
            .as_ref()
            .filter(|live| live.me.is_some())
            .map(|live| live.snapshot.chat_id.clone());
        let current = self
            .chat
            .as_ref()
            .map(|chat| chat.read(cx).conversation_id().to_owned());
        if wanted == current {
            return;
        }
        self.chat = wanted.map(|chat_id| {
            let source = (&self.connection, self.session.clone());
            cx.new(|cx| ChatView::new(source, (chat_id, tr!("play.panel.chat").into()), window, cx))
        });
    }

    pub fn focus_handle(&self) -> FocusHandle {
        self.focus_handle.clone()
    }

    /// An online game is on the board: the page shows it instead of its own menus.
    pub fn in_game(&self) -> bool {
        self.live.is_some()
    }

    /// Opens the live connection, unless it is open; a game the player is in then arrives.
    pub fn connect(&mut self, cx: &mut Context<Self>) {
        self.connection
            .update(cx, |connection, cx| connection.connect(cx));
        self.fetch_ratings(cx);
    }

    /// What the lobby's tabs do: the Play page switches to the way to play picked.
    pub fn on_mode(&mut self, on_mode: impl Fn(PlayMode, &mut Window, &mut App) + 'static) {
        self.on_mode = Some(Rc::new(on_mode));
    }

    /// The player's ratings, for the lobby's bar; they move with each rated game.
    fn fetch_ratings(&self, cx: &mut Context<Self>) {
        let Some(token) = (self.session)(cx).map(|session| session.token) else {
            return;
        };
        let request = self.connection.read(cx).client().stats(&token);
        cx.spawn(async move |view, cx| {
            // Without them the bar shows the name alone.
            if let Ok(stats) = request.await {
                let _ = view.update(cx, |view, cx| {
                    view.ratings = stats.modes;
                    cx.notify();
                });
            }
        })
        .detach();
    }

    /// The player's rating in the chosen speed, once they have one.
    fn my_rating(&self) -> Option<i32> {
        let key = self.choice.category().name().to_lowercase();
        self.ratings
            .iter()
            .find(|mode| mode.category == key)
            .and_then(|mode| mode.rating)
    }

    fn send(&self, message: ClientMessage, cx: &App) {
        self.connection.read(cx).send(message);
    }

    fn on_event(&mut self, event: LiveEvent, cx: &mut Context<Self>) {
        match event {
            LiveEvent::Connected => self.problem = None,
            LiveEvent::Disconnected | LiveEvent::SignedOut => {
                // Gone with the connection; the server keeps them while another app is on.
                self.seeking = false;
                self.awaiting = false;
                self.forget_rematch_on_disconnect();
            }
            LiveEvent::Message(message) => self.on_message(*message, cx),
        }
        cx.notify();
    }

    fn on_message(&mut self, message: ServerMessage, cx: &mut Context<Self>) {
        match message {
            ServerMessage::Seeking { .. } => {
                self.seeking = true;
                self.awaiting = false;
            }
            ServerMessage::SeekCancelled => self.seeking = false,
            ServerMessage::Game { game } => {
                self.awaiting = false;
                self.on_snapshot(game, cx);
            }
            // Only a refusal of this view's own seek or game is its to show.
            ServerMessage::Error { message } if self.awaiting || self.live.is_some() => {
                self.awaiting = false;
                if let Some(live) = self.live.as_mut() {
                    live.pending = None;
                }
                self.problem = Some(LiveProblem::Server(message));
            }
            ServerMessage::Challenges { sent, received } => {
                self.follow_rematch(&sent, &received, cx);
            }
            ServerMessage::RematchRefused { game_id, message } => {
                self.rematch_refused(&game_id, message);
            }
            ServerMessage::Error { .. }
            | ServerMessage::FriendsChanged
            | ServerMessage::ChatMessage { .. }
            | ServerMessage::ConversationsChanged
            | ServerMessage::ProfileChanged
            | ServerMessage::BattleSeeking
            | ServerMessage::BattleSeekCancelled
            | ServerMessage::Battle { .. }
            | ServerMessage::BattleChallenges { .. }
            | ServerMessage::InviteRefused { .. } => {}
        }
    }

    /// The rematch of the game on the board as the server has it; theirs first, to accept.
    fn follow_rematch(
        &mut self,
        sent: &[Challenge],
        received: &[Challenge],
        cx: &mut Context<Self>,
    ) {
        let unsure = std::mem::take(&mut self.ask_unsure);
        let Some(game_id) = self.game_id() else {
            self.rematch = Rematch::None;
            return;
        };
        let of_this_game =
            |challenge: &&Challenge| challenge.rematch_of.as_deref() == Some(game_id.as_str());
        let theirs = received.iter().find(of_this_game);
        if theirs.is_none() {
            // Theirs is gone: whatever this app's Accept became, it no longer waits on it.
            self.accepting_rematch = false;
        }
        let now = if let Some(theirs) = theirs {
            Rematch::Received(theirs.id.clone())
        } else if sent.iter().any(|mine| of_this_game(&mine)) {
            Rematch::Sent
        } else if self.rematch == Rematch::Asking && !unsure {
            // Not posted yet: this list was sent for something else.
            Rematch::Asking
        } else {
            Rematch::None
        };
        // Only the app that asked says how it went; the player's other devices stay quiet.
        if matches!(self.rematch, Rematch::Sent) && now == Rematch::None && self.asked_here() {
            self.asked_rematch_of = None;
            self._rematch_gone = Some(self.say_if_no_rematch(game_id, cx));
        }
        self.rematch = now;
    }

    /// A rematch refused: this app's own ask or accept says why; another device's is left be.
    fn rematch_refused(&mut self, game_id: &str, message: String) {
        let asked_here = self.asked_rematch_of.as_deref() == Some(game_id);
        if asked_here {
            self.asked_rematch_of = None;
        }
        if self.board_game() != Some(game_id) || !(asked_here || self.accepting_rematch) {
            return;
        }
        // An ask can be made again; an accept refused leaves their rematch waiting, as it was.
        if self.rematch == Rematch::Asking {
            self.rematch = Rematch::None;
        }
        self.accepting_rematch = false;
        self.problem = Some(LiveProblem::Server(message));
    }

    /// The connection dropped: an ask may be lost, so the lists sent again say if it was posted.
    fn forget_rematch_on_disconnect(&mut self) {
        self.ask_unsure = self.rematch == Rematch::Asking;
        self._rematch_gone = None;
        self.accepting_rematch = false;
    }

    /// Whether this app asked for the rematch of the game on the board.
    fn asked_here(&self) -> bool {
        self.asked_rematch_of.is_some() && self.asked_rematch_of.as_deref() == self.board_game()
    }

    /// Why the last request went wrong, as it reads now.
    fn problem_text(&self) -> Option<String> {
        self.problem.as_ref().map(LiveProblem::message)
    }

    /// The id of the game on the board, if any.
    fn board_game(&self) -> Option<&str> {
        self.live.as_ref().map(|live| live.snapshot.id.as_str())
    }

    /// The id of the game on the board, owned, to send or keep.
    fn game_id(&self) -> Option<String> {
        self.board_game().map(str::to_owned)
    }

    /// A rematch sent went: a new game, if it was accepted, comes within a moment; else it says so.
    fn say_if_no_rematch(&self, game_id: String, cx: &mut Context<Self>) -> Task<()> {
        cx.spawn(async move |view, cx| {
            cx.background_executor().timer(REMATCH_SETTLES).await;
            // The view outlives the wait unless the window is closing.
            let _ = view.update(cx, |view, cx| {
                if view.board_game() == Some(game_id.as_str()) && view.rematch == Rematch::None {
                    view.problem = Some(LiveProblem::NoRematch);
                    cx.notify();
                }
            });
        })
    }

    /// Asks the opponent to play again, withdraws the ask, or accepts theirs; once each.
    fn rematch_pressed(&mut self, cx: &mut Context<Self>) {
        let Some(game_id) = self.game_id() else {
            return;
        };
        let message = match &self.rematch {
            Rematch::None => {
                self.asked_rematch_of = Some(game_id.clone());
                self.rematch = Rematch::Asking;
                ClientMessage::OfferRematch { game_id }
            }
            // Waiting on the server, or on an ask another of the player's devices made.
            Rematch::Asking => return,
            Rematch::Sent if !self.asked_here() => return,
            Rematch::Sent => {
                self.asked_rematch_of = None;
                self.rematch = Rematch::None;
                ClientMessage::CancelRematch { game_id }
            }
            Rematch::Received(_) if self.accepting_rematch => return,
            // Accepted by game, so a refusal comes as this game's, and one gone is not re-asked.
            Rematch::Received(_) => {
                self.accepting_rematch = true;
                ClientMessage::AcceptRematch { game_id }
            }
        };
        self._rematch_gone = None;
        self.problem = None;
        self.send(message, cx);
        cx.notify();
    }

    /// The server's latest word on a game: board, sounds and clock follow, and a premove goes.
    fn on_snapshot(&mut self, snapshot: GameSnapshot, cx: &mut Context<Self>) {
        let Some(game) = replay(&snapshot) else {
            self.problem = Some(LiveProblem::GameUnreadable);
            return;
        };
        let me = (self.session)(cx).and_then(|session| my_side(&snapshot, &session.user.id));
        let previous = self
            .live
            .as_ref()
            .filter(|live| live.snapshot.id == snapshot.id);
        let was_live = previous.is_none_or(|live| live.history.is_live());
        let viewing = if was_live {
            game.moves().len()
        } else {
            previous.map_or(0, |live| live.history.viewing())
        };
        self.sound_for(previous.map(|live| &live.snapshot), &snapshot, me, cx);
        let new_game = previous.is_none();
        if new_game {
            self.card_closed = false;
            self.rematch = Rematch::None;
            self._rematch_gone = None;
            self.asked_rematch_of = None;
            self.accepting_rematch = false;
        }
        let clock_warning = previous.map_or_else(ClockWarning::default, |live| live.clock_warning);
        let finished = snapshot.status != LiveStatus::Ongoing;
        self.live = Some(LiveBoard {
            history: GameHistory::from_game(game, viewing),
            snapshot,
            received_at: Instant::now(),
            me,
            pending: None,
            clock_warning,
        });
        self.seeking = false;
        self.problem = None;
        if new_game {
            self.board.update(cx, |board, cx| {
                board.reset(cx);
                board.face(me.unwrap_or(Side::White), cx);
            });
        }
        if finished {
            self._ticker = None;
            self.board.update(cx, |board, _| board.drop_all_premoves());
        } else if self._ticker.is_none() {
            self._ticker = Some(start_ticker(CLOCK_TICK, cx, OnlineView::tick));
        }
        self.play_due_premove(cx);
    }

    /// The sound of what changed since the last snapshot: a start, a move, or the game ending.
    fn sound_for(
        &self,
        before: Option<&GameSnapshot>,
        after: &GameSnapshot,
        me: Option<Side>,
        cx: &App,
    ) {
        let finished = after.status != LiveStatus::Ongoing;
        let moved = after.moves.len() > before.map_or(0, |before| before.moves.len());
        if before.is_none() && after.moves.is_empty() && !finished {
            play_sound(Sound::GameStart, cx);
        }
        if moved
            && let Some(game) = replay(after)
            && let [.., before_move, after_move] = game.positions()
            && let Some(&mv) = game.moves().last()
        {
            let mine = me == Some(before_move.side_to_move());
            let plain = if mine {
                Sound::MoveSelf
            } else {
                Sound::MoveOpponent
            };
            let already_heard =
                mine && self.live.as_ref().and_then(|live| live.pending) == Some(mv);
            if !already_heard || finished {
                play_move_sound(before_move, after_move, mv, plain, finished, cx);
            }
        } else if finished && before.is_some_and(|before| before.status == LiveStatus::Ongoing) {
            play_sound(Sound::GameEnd, cx);
        }
        let offered_to_me = after.draw_offer.map(Side::from) == me.map(Side::opponent);
        if offered_to_me && before.is_some_and(|before| before.draw_offer.is_none()) {
            play_sound(Sound::Notify, cx);
        }
    }

    /// On the player's turn, their first queued premove is played if it is legal.
    fn play_due_premove(&mut self, cx: &mut Context<Self>) {
        let Some(live) = &self.live else {
            return;
        };
        let position = *live.history.live();
        let my_turn = live.me == Some(position.side_to_move());
        if live.snapshot.status != LiveStatus::Ongoing || !my_turn {
            return;
        }
        if let Some(mv) = self
            .board
            .update(cx, |board, _| board.next_premove(&position))
        {
            self.move_made(mv, cx);
        }
    }

    /// The player's move: shown at once and sent; the server confirms it or takes it back.
    fn move_made(&mut self, mv: Move, cx: &mut Context<Self>) {
        let Some(live) = self.live.as_mut() else {
            return;
        };
        let before = *live.history.live();
        let Ok(after) = before.play(mv) else {
            return;
        };
        live.pending = Some(mv);
        play_move_sound(&before, &after, mv, Sound::MoveSelf, false, cx);
        let message = ClientMessage::Move {
            game_id: live.snapshot.id.clone(),
            uci: mv.to_string(),
        };
        self.send(message, cx);
        cx.notify();
    }

    /// What the board shows: the live position with the pending move, or an earlier one.
    fn shown_position(&self) -> ShownPosition {
        let Some(live) = &self.live else {
            return ShownPosition::waiting();
        };
        let ongoing = live.snapshot.status == LiveStatus::Ongoing;
        let looking_live = live.history.is_live();
        let pending = live
            .pending
            .filter(|_| looking_live)
            .and_then(|mv| Some((mv, live.history.live().play(mv).ok()?)));
        match pending {
            Some((mv, after)) => ShownPosition {
                position: after,
                last_move: Some(mv),
                can_move: ongoing,
                premoves: true,
                player: live.me,
                engine_arrow: None,
                badge: None,
                best_arrow: None,
                outcome: None,
            },
            None => ShownPosition {
                position: *live.history.shown(),
                last_move: live.history.shown_last_move(),
                can_move: ongoing && looking_live,
                premoves: true,
                player: live.me,
                engine_arrow: None,
                badge: None,
                best_arrow: None,
                outcome: looking_live.then(|| outcome_of(&live.snapshot)).flatten(),
            },
        }
    }

    /// While the connection is down, that it is being tried again.
    fn connection_line(&self, cx: &App) -> Option<String> {
        self.connection
            .read(cx)
            .is_reconnecting()
            .then(|| tr!("play.problem.reconnecting"))
    }

    fn resign(&mut self, cx: &mut Context<Self>) {
        if let Some(game_id) = self.game_id() {
            self.send(ClientMessage::Resign { game_id }, cx);
        }
    }

    fn abort(&mut self, cx: &mut Context<Self>) {
        if let Some(game_id) = self.game_id() {
            self.send(ClientMessage::Abort { game_id }, cx);
        }
    }

    /// Hands the game to Game Review, at the position being looked at.
    fn review(&mut self, cx: &mut Context<Self>) {
        if let Some(live) = &self.live {
            cx.emit(AnalyzeGame {
                game: live.history.game().clone(),
                viewing: live.history.viewing(),
                review: true,
                outcome: outcome_of(&live.snapshot),
            });
        }
    }

    /// Turns the board round; the players' bars follow it.
    fn flip(&mut self, cx: &mut Context<Self>) {
        self.board.update(cx, |board, cx| board.flip(cx));
        cx.notify();
    }

    fn offer_draw(&mut self, cx: &mut Context<Self>) {
        if let Some(game_id) = self.game_id() {
            self.send(ClientMessage::OfferDraw { game_id }, cx);
        }
    }

    fn answer_draw(&mut self, accept: bool, cx: &mut Context<Self>) {
        if let Some(game_id) = self.game_id() {
            let answer = if accept {
                ClientMessage::AcceptDraw { game_id }
            } else {
                ClientMessage::DeclineDraw { game_id }
            };
            self.send(answer, cx);
        }
    }

    /// The end-of-game card: who won and how, the rating change, and what to do next.
    fn game_over_card(&self, live: &LiveBoard, cx: &mut Context<Self>) -> Option<GameOverCard> {
        if self.card_closed || !has_ended(&live.snapshot) {
            return None;
        }
        let words = match outcome_of(&live.snapshot) {
            Some(outcome) => (
                result_title(outcome.winner(), live.me, None),
                outcome_reason(outcome),
            ),
            None => (tr!("play.title.aborted"), tr!("play.reason.aborted")),
        };
        let detail = live.me.and_then(|me| rating_line(&live.snapshot, me));
        let close = cx.entity().downgrade();
        let card = GameOverCard::new(words, self.game_over_actions(live, cx), move |_, cx| {
            let _ = close.update(cx, |view, cx| {
                view.card_closed = true;
                cx.notify();
            });
        })
        .detail(detail);
        Some(card)
    }

    /// Game Review, then the lobby or a rematch; an aborted game has no review.
    fn game_over_actions(&self, live: &LiveBoard, cx: &mut Context<Self>) -> GameOverActions {
        let page = cx.entity().downgrade();
        let (review, fresh, rematch) = (page.clone(), page.clone(), page);
        let aborted = outcome_of(&live.snapshot).is_none();
        GameOverActions::new(move |_, cx| {
            let _ = review.update(cx, |view, cx| view.review(cx));
        })
        .when(aborted, GameOverActions::without_review)
        .choice(
            ("new-game", tr!("play.actions.new_game")),
            IconName::Plus,
            move |_, cx| {
                let _ = fresh.update(cx, |view, cx| view.leave_game(cx));
            },
        )
        .when(live.me.is_some(), |actions| {
            actions.choice(
                (
                    "rematch",
                    rematch_label(&self.rematch, (self.asked_here(), self.accepting_rematch)),
                ),
                IconName::RotateCcw,
                move |_, cx| {
                    let _ = rematch.update(cx, |view, cx| view.rematch_pressed(cx));
                },
            )
        })
    }

    /// From a finished game back to the lobby.
    fn leave_game(&mut self, cx: &mut Context<Self>) {
        // The player went on to something else: their ask is withdrawn, the opponent's declined.
        if let Some(game_id) = self.game_id()
            && self.asked_here()
        {
            self.send(ClientMessage::CancelRematch { game_id }, cx);
        }
        if let Rematch::Received(challenge_id) = std::mem::take(&mut self.rematch) {
            self.send(ClientMessage::DeclineChallenge { challenge_id }, cx);
        }
        self.asked_rematch_of = None;
        self.accepting_rematch = false;
        self.ask_unsure = false;
        self.live = None;
        self._ticker = None;
        // The lobby's waiting board faces White, whichever side the game had the player on.
        self.board.update(cx, |board, cx| {
            board.reset(cx);
            board.face(Side::White, cx);
        });
        self.fetch_ratings(cx);
        cx.notify();
    }

    fn seek(&mut self, cx: &mut Context<Self>) {
        self.connect(cx);
        let can_rate = (self.session)(cx).is_some_and(|session| !session.user.guest);
        self.problem = None;
        self.awaiting = true;
        self.send(
            ClientMessage::Seek {
                initial_s: whole_seconds(self.choice.initial),
                increment_s: whole_seconds(self.choice.increment),
                rated: self.rated && can_rate,
            },
            cx,
        );
        cx.notify();
    }

    fn cancel_seek(&mut self, cx: &mut Context<Self>) {
        self.send(ClientMessage::CancelSeek, cx);
        cx.notify();
    }

    fn lobby(&self, cx: &mut Context<Self>) -> Div {
        let can_rate = (self.session)(cx).is_some_and(|session| !session.user.guest);
        let view = cx.entity().downgrade();
        let (choose, start, rated, cancel) = (view.clone(), view.clone(), view.clone(), view);
        let problem = self.problem_text().or_else(|| {
            self.connection
                .read(cx)
                .is_reconnecting()
                .then(|| tr!("play.problem.connecting"))
        });
        let setup = GameSetup::new(
            Some(self.choice),
            move |choice, _, cx| {
                if let Some(choice) = choice {
                    let _ = choose.update(cx, |view, cx| {
                        view.choice = choice;
                        cx.notify();
                    });
                }
            },
            move |_, cx| {
                let _ = start.update(cx, |view, cx| view.seek(cx));
            },
        )
        .online(OnlineOptions {
            rated: self.rated,
            can_rate,
            seeking: self.seeking,
            problem,
            on_rated: Rc::new(move |on, _, cx| {
                let _ = rated.update(cx, |view, cx| {
                    view.rated = on;
                    cx.notify();
                });
            }),
            on_cancel: Rc::new(move |_, cx| {
                let _ = cancel.update(cx, |view, cx| view.cancel_seek(cx));
            }),
        });
        let face = my_face(&self.session, cx);
        let me_name = face
            .as_ref()
            .map_or_else(String::new, |face| face.name.clone());
        let me = match self.my_rating() {
            Some(rating) => format!("{me_name} ({rating})"),
            None => me_name,
        };
        let clock = Some(self.choice.initial);
        let column = BoardColumn::new(self.board.clone())
            .top(WaitingBar::new(tr!("play.bar.opponent")).clock(clock))
            .bottom(WaitingBar::new(me).person(face).clock(clock));
        let on_mode = self.on_mode.clone();
        let panel = PlayPanel::new(PlayMode::Online, move |mode, window, cx| {
            if let Some(on_mode) = &on_mode {
                on_mode(mode, window, cx);
            }
        })
        .child(setup);
        board_page(column, panel)
    }

    fn game_view(&self, live: &LiveBoard, cx: &mut Context<Self>) -> Div {
        let page = cx.entity().downgrade();
        let bottom = self.board.read(cx).bottom_side();
        let card = self.game_over_card(live, cx);
        let column = BoardColumn::new(with_card(self.board.clone(), card))
            .top(self.player_bar(live, bottom.opponent(), cx))
            .bottom(self.player_bar(live, bottom, cx))
            .tool(flip_button("online-flip-board", cx, OnlineView::flip))
            .tool(copy_buttons(live.history.shown(), live.history.game()));
        let side = SidePanel::new()
            .header(PanelHeader::new(
                PLAY_PICTURE,
                tr!("play.panel.play_online"),
            ))
            .child(self.header(live, cx))
            .child(MoveList::new(
                live.history.sans().to_vec(),
                live.history.viewing(),
                select_move(page.clone()),
            ))
            .children(self.chat.clone().map(|chat| {
                div()
                    .h(px(240.))
                    .flex_shrink_0()
                    .p_3()
                    .rounded(px(8.))
                    .bg(cx.theme().muted)
                    .child(chat)
            }))
            .child(
                NavigationBar::new(
                    live.history.can_go_back(),
                    live.history.can_go_forward(),
                    step_history(page.clone()),
                )
                .on_hold(hold_step(page)),
            )
            .map(|this| {
                if has_ended(&live.snapshot) {
                    this.child(self.game_over_actions(live, cx))
                } else {
                    this.child(action_bar(self.actions(live, cx)))
                }
            });
        board_page(column, side)
    }

    /// How the game stands, at the top of the panel.
    fn header(&self, live: &LiveBoard, cx: &mut Context<Self>) -> Div {
        let snapshot = &live.snapshot;
        let status = result_line(snapshot, live.me).unwrap_or_else(|| {
            let mover = live.history.live().side_to_move();
            if Some(mover) == live.me {
                tr!("play.status.your_move")
            } else {
                tr!("play.status.waiting")
            }
        });
        let rating = live.me.and_then(|me| rating_line(snapshot, me));
        v_flex()
            .gap_1()
            .child(Label::new(status).text_lg())
            .when_some(rating, |this, rating| {
                this.child(
                    div()
                        .text_sm()
                        .text_color(cx.theme().muted_foreground)
                        .child(rating),
                )
            })
            .when_some(
                self.problem_text().or_else(|| self.connection_line(cx)),
                |this, problem| {
                    this.child(div().text_sm().text_color(cx.theme().danger).child(problem))
                },
            )
    }

    /// The game buttons while it is on: Abort, Draw and Resign, or an offer's answers.
    fn actions(&self, live: &LiveBoard, cx: &mut Context<Self>) -> Vec<ChoiceTile> {
        let snapshot = &live.snapshot;
        if can_abort(snapshot) {
            return vec![
                game_tile(
                    ("online-abort", tr!("play.actions.abort")),
                    false,
                    cx,
                    OnlineView::abort,
                )
                .icon(IconName::Close),
            ];
        }
        let offer = snapshot.draw_offer.map(Side::from);
        if offer.is_some() && offer != live.me {
            return vec![
                game_tile(
                    ("online-accept-draw", tr!("play.actions.accept_draw")),
                    true,
                    cx,
                    |view, cx| {
                        view.answer_draw(true, cx);
                    },
                )
                .icon(IconName::Check),
                game_tile(
                    ("online-decline-draw", tr!("play.actions.decline")),
                    false,
                    cx,
                    |view, cx| {
                        view.answer_draw(false, cx);
                    },
                )
                .icon(IconName::Close),
            ];
        }
        vec![
            game_tile(
                ("online-offer-draw", tr!("play.actions.draw")),
                false,
                cx,
                OnlineView::offer_draw,
            )
            .glyph("½")
            .disabled(offer.is_some()),
            game_tile(
                ("online-resign", tr!("play.actions.resign")),
                false,
                cx,
                OnlineView::resign,
            )
            .icon(IconName::Flag),
        ]
    }

    /// A player's name and rating, what they have taken, and their clock.
    fn player_bar(&self, live: &LiveBoard, side: Side, cx: &App) -> Div {
        let snapshot = &live.snapshot;
        let player = match side {
            Side::White => &snapshot.white,
            Side::Black => &snapshot.black,
        };
        let name = match player.rating {
            Some(rating) => format!("{} ({rating})", player.username),
            None => player.username.clone(),
        };
        let left = clock_left(snapshot, side, live.received_at.elapsed());
        let running = snapshot.running.map(Side::from) == Some(side);
        h_flex()
            .w_full()
            .h(PLAYER_BAR)
            .flex_shrink_0()
            .justify_between()
            .gap_4()
            .child(
                h_flex()
                    .gap_3()
                    .child(strip_avatar(
                        &face_of(&player.id, &player.username, player.avatar.as_deref()),
                        cx,
                    ))
                    .child(Label::new(name))
                    .child(CapturedRow::for_side(live.history.shown().board(), side)),
            )
            .child(ClockFace::new(left, running))
    }

    /// Redraws the running clock and warns once time is low; whether to keep ticking.
    fn tick(&mut self, cx: &mut Context<Self>) -> bool {
        let Some(live) = self
            .live
            .as_mut()
            .filter(|live| live.snapshot.status == LiveStatus::Ongoing)
        else {
            return false;
        };
        if let Some(me) = live.me {
            let left = clock_left(&live.snapshot, me, live.received_at.elapsed());
            let initial = Duration::from_secs(u64::from(live.snapshot.initial_s));
            if live.clock_warning.is_due(me, left, initial) {
                play_sound(Sound::LowTime, cx);
            }
        }
        cx.notify();
        true
    }
}

impl EventEmitter<AnalyzeGame> for OnlineView {}

impl HistoryNavigation for OnlineView {
    fn history_mut(&mut self) -> Option<&mut GameHistory> {
        self.live.as_mut().map(|live| &mut live.history)
    }

    fn holding(&mut self) -> &mut Holding {
        &mut self.holding
    }

    fn after_history_move(&mut self, cx: &mut Context<Self>) {
        self.board.update(cx, |board, cx| board.put_down(cx));
        cx.notify();
    }
}

impl Render for OnlineView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.keep_chat(window, cx);
        let content = match self.live.take() {
            Some(live) => {
                let view = self.game_view(&live, cx);
                self.live = Some(live);
                view
            }
            None => self.lobby(cx),
        };
        with_history_keys(v_flex(), cx)
            .track_focus(&self.focus_handle)
            .size_full()
            .child(content)
    }
}

/// The rematch button's word, short to fit the card: Rematch, Asking…, Cancel or Accept.
fn rematch_label(rematch: &Rematch, (asked_here, accepting): (bool, bool)) -> String {
    match rematch {
        Rematch::None => tr!("play.rematch.ask"),
        Rematch::Asking => tr!("play.rematch.asking"),
        // Only the device that asked can cancel; another shows it waiting.
        Rematch::Sent if !asked_here => tr!("play.rematch.asking"),
        Rematch::Sent => tr!("play.rematch.cancel"),
        Rematch::Received(_) if accepting => tr!("play.rematch.accepting"),
        Rematch::Received(_) => tr!("play.rematch.accept"),
    }
}

#[cfg(test)]
mod tests {
    use std::{cell::Cell, rc::Rc, sync::Arc};

    use axum::{
        Router,
        extract::ws::{Message, WebSocket, WebSocketUpgrade},
        routing::get,
    };
    use gpui_kit::{AppContext as _, Entity, TestAppContext, VisualTestContext};
    use serde_json::{Value, json};

    use chess_domain::TimeControl;

    use super::{OnlineView, REMATCH_SETTLES, Rematch, rematch_label};
    use crate::{
        clients::server::ServerClient,
        enums::sound::Sound,
        models::play::{AnalyzeGame, LiveEvent, ServerMessage},
        testing::{Heard, ann, click, record_socket, wait_until, was_heard},
        views::live::{LiveConnection, SessionSource},
        views::sound::{played_sounds, record_sounds},
    };

    /// The game as the fake server sends it: Ann (the player) is white against Bob.
    fn game(moves: &[&str], finished: Option<(&str, &str)>) -> String {
        let (status, result, winner) = match finished {
            Some((result, winner)) => ("finished", json!(result), json!(winner)),
            None => ("ongoing", Value::Null, Value::Null),
        };
        let running = if finished.is_some() || moves.is_empty() {
            Value::Null
        } else if moves.len() % 2 == 1 {
            json!("black")
        } else {
            json!("white")
        };
        json!({ "type": "game", "game": {
            "id": "g1",
            "white": { "id": "ann", "username": "Ann", "rating": 1500 },
            "black": { "id": "bob", "username": "Bob", "rating": 1500 },
            "rated": true, "initial_s": 180, "increment_s": 2,
            "moves": moves, "white_ms": 180_000, "black_ms": 180_000,
            "running": running, "status": status, "result": result, "winner": winner,
            "draw_offer": null,
            "rating_changes": finished.map(|_| json!({ "white": -8, "black": 8 })),
            "chat_id": "c1",
        }})
        .to_string()
    }

    /// The game called off before anyone moved.
    fn aborted() -> String {
        let mut message: Value = serde_json::from_str(&game(&[], None)).expect("json");
        message["game"]["status"] = json!("aborted");
        message.to_string()
    }

    /// Pairs the player at once with Bob, who answers e2e4 with e7e5; a resignation ends it.
    async fn bob(mut socket: WebSocket) {
        while let Some(Ok(Message::Text(text))) = socket.recv().await {
            let message: Value = serde_json::from_str(text.as_str()).expect("json");
            let answers = match message["type"].as_str() {
                Some("seek") => vec![game(&[], None)],
                Some("move") => vec![game(&["e2e4"], None), game(&["e2e4", "e7e5"], None)],
                Some("resign") => vec![game(&["e2e4", "e7e5"], Some(("resignation", "black")))],
                Some("abort") => vec![aborted()],
                _ => Vec::new(),
            };
            for answer in answers {
                if socket.send(Message::Text(answer.into())).await.is_err() {
                    return;
                }
            }
        }
    }

    fn online(cx: &mut TestAppContext) -> (Entity<OnlineView>, &mut VisualTestContext) {
        cx.executor().allow_parking();
        cx.update(|cx| {
            gpui_kit::init(cx);
            record_sounds(cx);
        });
        let app = Router::new().route(
            "/api/ws",
            get(|upgrade: WebSocketUpgrade| async move { upgrade.on_upgrade(bob) }),
        );
        let client = ServerClient::serving(app);
        let session = ann(false);
        cx.add_window_view(|_, cx| {
            let session: SessionSource = Rc::new(move |_| Some(session.clone()));
            let connection = cx.new(|_| LiveConnection::new(client, Rc::clone(&session)));
            OnlineView::new(connection, session, cx)
        })
    }

    #[gpui_kit::test]
    fn a_time_is_picked_from_the_speeds(cx: &mut TestAppContext) {
        let (view, cx) = online(cx);
        click(cx, "time-3 | 2");
        let choice = view.read_with(cx, |view, _| view.choice);
        assert_eq!(choice, TimeControl::new(3, 2));
    }

    #[gpui_kit::test]
    fn a_found_game_is_played_and_resigned(cx: &mut TestAppContext) {
        let (view, cx) = online(cx);
        view.update(cx, |view, cx| view.seek(cx));
        wait_until(cx, |cx| view.read_with(cx, |view, _| view.in_game()));
        assert!(
            cx.update(|_, cx| played_sounds(cx))
                .contains(&Sound::GameStart)
        );

        click(cx, "square-e2");
        click(cx, "square-e4");
        wait_until(cx, |cx| {
            view.read_with(cx, |view, _| {
                view.live
                    .as_ref()
                    .is_some_and(|live| live.snapshot.moves.len() == 2)
            })
        });
        assert!(
            cx.update(|_, cx| played_sounds(cx))
                .contains(&Sound::MoveOpponent)
        );

        view.update(cx, |view, cx| view.resign(cx));
        wait_until(cx, |cx| {
            view.read_with(cx, |view, _| {
                view.live
                    .as_ref()
                    .is_some_and(|live| live.snapshot.rating_changes.is_some())
            })
        });
        view.read_with(cx, |view, _| {
            let live = view.live.as_ref().expect("the game");
            assert_eq!(
                super::result_line(&live.snapshot, live.me).as_deref(),
                Some("You lost by resignation")
            );
            assert_eq!(
                super::rating_line(&live.snapshot, live.me.expect("a side")).as_deref(),
                Some("Rating 1492 (-8)")
            );
        });
        assert_eq!(
            cx.update(|_, cx| played_sounds(cx)).last(),
            Some(&Sound::GameEnd)
        );

        // The panel's Game Review, once the game is over, sends the game to be reviewed.
        let reviewed = Rc::new(std::cell::Cell::new(false));
        let heard = Rc::clone(&reviewed);
        cx.update(|_, cx| {
            cx.subscribe(&view, move |_, event: &AnalyzeGame, _| {
                heard.set(event.review)
            })
            .detach();
        });
        click(cx, "game-review");
        assert!(reviewed.get(), "Game Review asks for a review");
        click(cx, "close-game-over");
        cx.run_until_parked();
        assert!(
            cx.debug_bounds("game-over").is_none(),
            "the cross puts it away"
        );
    }

    #[gpui_kit::test]
    fn only_my_own_pieces_move(cx: &mut TestAppContext) {
        let (view, cx) = online(cx);
        view.update(cx, |view, cx| view.seek(cx));
        wait_until(cx, |cx| view.read_with(cx, |view, _| view.in_game()));
        click(cx, "square-e7");
        click(cx, "square-e5");
        cx.run_until_parked();
        view.read_with(cx, |view, _| {
            let live = view.live.as_ref().expect("the game");
            assert_eq!(live.pending, None, "black's pawn is Bob's");
            assert!(live.snapshot.moves.is_empty());
        });
    }

    #[gpui_kit::test]
    fn a_game_called_off_says_so_and_can_be_analyzed(cx: &mut TestAppContext) {
        let (view, cx) = online(cx);
        view.update(cx, |view, cx| view.seek(cx));
        wait_until(cx, |cx| view.read_with(cx, |view, _| view.in_game()));
        view.read_with(cx, |view, _| {
            let live = view.live.as_ref().expect("the game");
            assert!(
                super::can_abort(&live.snapshot),
                "Abort before anyone moved"
            );
        });
        view.update(cx, |view, cx| view.abort(cx));
        wait_until(cx, |cx| {
            view.read_with(cx, |view, _| {
                view.live
                    .as_ref()
                    .and_then(|live| super::result_line(&live.snapshot, live.me))
                    .is_some()
            })
        });
        view.read_with(cx, |view, _| {
            let live = view.live.as_ref().expect("the game");
            assert_eq!(
                super::result_line(&live.snapshot, live.me).as_deref(),
                Some("Game aborted")
            );
        });
        let analyzed = Rc::new(std::cell::Cell::new(false));
        let heard = Rc::clone(&analyzed);
        cx.update(|_, cx| {
            cx.subscribe(&view, move |_, _: &AnalyzeGame, _| heard.set(true))
                .detach();
        });
        view.update(cx, |view, cx| view.review(cx));
        assert!(analyzed.get(), "the game went to Game Review");
    }

    #[test]
    fn the_rematch_button_says_where_the_rematch_stands() {
        let sent = Rematch::Sent;
        let received = Rematch::Received("r".to_owned());
        assert_eq!(rematch_label(&Rematch::None, (false, false)), "Rematch");
        assert_eq!(rematch_label(&Rematch::Asking, (true, false)), "Asking…");
        assert_eq!(
            rematch_label(&sent, (false, false)),
            "Asking…",
            "another device asked"
        );
        assert_eq!(rematch_label(&sent, (true, false)), "Cancel");
        assert_eq!(rematch_label(&received, (false, false)), "Accept");
        assert_eq!(rematch_label(&received, (false, true)), "Accepting…");
    }

    /// Ann's online view, connected to a server that only listens, with the game `g1` over.
    fn after_a_game(
        cx: &mut TestAppContext,
    ) -> (Entity<OnlineView>, Heard, &mut VisualTestContext) {
        cx.executor().allow_parking();
        cx.update(|cx| {
            gpui_kit::init(cx);
            record_sounds(cx);
        });
        let sent = Heard::default();
        let heard = Arc::clone(&sent);
        let app = Router::new().route(
            "/api/ws",
            get(move |upgrade: WebSocketUpgrade| async move {
                upgrade.on_upgrade(move |socket| record_socket(socket, None, heard))
            }),
        );
        let client = ServerClient::serving(app);
        let session = ann(false);
        let connected = Rc::new(Cell::new(false));
        let told = Rc::clone(&connected);
        let (view, cx) = cx.add_window_view(move |_, cx| {
            let session: SessionSource = Rc::new(move |_| Some(session.clone()));
            let connection = cx.new(|_| LiveConnection::new(client, Rc::clone(&session)));
            cx.subscribe(&connection, move |_, _, event: &LiveEvent, _| {
                if matches!(event, LiveEvent::Connected) {
                    told.set(true);
                }
            })
            .detach();
            connection.update(cx, |live, cx| live.connect(cx));
            OnlineView::new(connection, session, cx)
        });
        wait_until(cx, |_| connected.get());
        hear(&view, cx, &finished_g1());
        (view, sent, cx)
    }

    /// The game `g1`, which Ann resigned.
    fn finished_g1() -> String {
        game(&["e2e4", "e7e5"], Some(("resignation", "black")))
    }

    /// The view hears `message` from the server.
    fn hear(view: &Entity<OnlineView>, cx: &mut VisualTestContext, message: &str) {
        let message: ServerMessage = serde_json::from_str(message).expect("a server message");
        view.update(cx, |view, cx| view.on_message(message, cx));
    }

    /// The challenges message with `sent` and `received` holding rematches of `g1` by id.
    fn rematches(sent: &[&str], received: &[&str]) -> String {
        let rematch = |id: &&str| {
            json!({
                "id": id, "from": { "id": "ann", "username": "Ann" },
                "to": { "id": "bob", "username": "Bob" },
                "initial_s": 180, "increment_s": 2, "rated": true, "rematch_of": "g1",
                "wait_s": 60, "seconds_left": 60
            })
        };
        json!({
            "type": "challenges",
            "sent": sent.iter().map(rematch).collect::<Vec<_>>(),
            "received": received.iter().map(rematch).collect::<Vec<_>>(),
        })
        .to_string()
    }

    /// The server's refusal of the rematch of `game`.
    fn refused(game: &str, message: &str) -> String {
        json!({ "type": "rematch_refused", "game_id": game, "message": message }).to_string()
    }

    fn label(view: &Entity<OnlineView>, cx: &mut VisualTestContext) -> String {
        view.read_with(cx, |view, _| {
            rematch_label(&view.rematch, (view.asked_here(), view.accepting_rematch))
        })
    }

    #[gpui_kit::test]
    fn a_rematch_asked_here_is_withdrawn_from_here(cx: &mut TestAppContext) {
        let (view, sent, cx) = after_a_game(cx);
        view.update(cx, |view, cx| view.rematch_pressed(cx));
        assert_eq!(label(&view, cx), "Asking…");
        wait_until(cx, |_| was_heard(&sent, "offer_rematch g1"));
        hear(&view, cx, &rematches(&["r1"], &[]));
        assert_eq!(label(&view, cx), "Cancel");
        view.update(cx, |view, cx| view.rematch_pressed(cx));
        wait_until(cx, |_| was_heard(&sent, "cancel_rematch g1"));
        hear(&view, cx, &rematches(&[], &[]));
        assert!(
            !view.read_with(cx, |view, _| view._rematch_gone.is_some()),
            "withdrawn by the player: nothing to tell"
        );
    }

    #[gpui_kit::test]
    fn a_rematch_gone_unplayed_says_so_only_after_a_moment(cx: &mut TestAppContext) {
        let (view, _sent, cx) = after_a_game(cx);
        view.update(cx, |view, cx| view.rematch_pressed(cx));
        hear(&view, cx, &rematches(&["r1"], &[]));
        hear(&view, cx, &rematches(&[], &[]));
        assert_eq!(view.read_with(cx, |view, _| view.problem_text()), None);
        cx.executor().advance_clock(REMATCH_SETTLES);
        cx.run_until_parked();
        assert_eq!(
            view.read_with(cx, |view, _| view.problem_text()).as_deref(),
            Some("No rematch this time")
        );
    }

    #[gpui_kit::test]
    fn a_rematch_accepted_in_time_says_nothing(cx: &mut TestAppContext) {
        let (view, _sent, cx) = after_a_game(cx);
        view.update(cx, |view, cx| view.rematch_pressed(cx));
        hear(&view, cx, &rematches(&["r1"], &[]));
        hear(&view, cx, &rematches(&[], &[]));
        let mut next: Value = serde_json::from_str(&game(&[], None)).expect("json");
        next["game"]["id"] = json!("g2");
        hear(&view, cx, &next.to_string());
        cx.executor().advance_clock(REMATCH_SETTLES);
        cx.run_until_parked();
        assert_eq!(view.read_with(cx, |view, _| view.problem_text()), None);
    }

    #[gpui_kit::test]
    fn another_devices_rematch_waits_here_and_cannot_be_withdrawn(cx: &mut TestAppContext) {
        let (view, sent, cx) = after_a_game(cx);
        hear(&view, cx, &rematches(&["r1"], &[]));
        assert_eq!(label(&view, cx), "Asking…");
        view.update(cx, |view, cx| view.rematch_pressed(cx));
        hear(&view, cx, &rematches(&[], &[]));
        assert!(
            !view.read_with(cx, |view, _| view._rematch_gone.is_some()),
            "only the device that asked says no rematch"
        );
        assert!(!was_heard(&sent, "cancel_rematch g1"));
    }

    #[gpui_kit::test]
    fn the_cards_accept_goes_by_game_and_a_refusal_leaves_theirs_waiting(cx: &mut TestAppContext) {
        let (view, sent, cx) = after_a_game(cx);
        hear(&view, cx, &rematches(&[], &["r1"]));
        view.update(cx, |view, cx| view.rematch_pressed(cx));
        assert_eq!(label(&view, cx), "Accepting…");
        wait_until(cx, |_| was_heard(&sent, "accept_rematch g1"));
        hear(&view, cx, &refused("g1", "Bob is playing a game"));
        view.read_with(cx, |view, _| {
            assert_eq!(
                view.rematch,
                Rematch::Received("r1".to_owned()),
                "still theirs to accept"
            );
            assert_eq!(
                view.problem_text().as_deref(),
                Some("Bob is playing a game")
            );
        });
        assert_eq!(label(&view, cx), "Accept");
        // A game that failed to start takes theirs too: the lists after say so, the reason stays.
        hear(&view, cx, &rematches(&[], &[]));
        assert_eq!(
            view.read_with(cx, |view, _| view.problem_text()).as_deref(),
            Some("Bob is playing a game")
        );
    }

    #[gpui_kit::test]
    fn leaving_withdraws_this_apps_ask_and_declines_theirs(cx: &mut TestAppContext) {
        let (view, sent, cx) = after_a_game(cx);
        view.update(cx, |view, cx| view.rematch_pressed(cx));
        view.update(cx, |view, cx| view.leave_game(cx));
        wait_until(cx, |_| was_heard(&sent, "cancel_rematch g1"));

        hear(&view, cx, &finished_g1());
        hear(&view, cx, &rematches(&[], &["r2"]));
        view.update(cx, |view, cx| view.leave_game(cx));
        wait_until(cx, |_| was_heard(&sent, "decline_challenge r2"));
    }

    #[gpui_kit::test]
    fn a_refused_rematch_says_why_and_can_be_asked_again(cx: &mut TestAppContext) {
        let (view, _sent, cx) = after_a_game(cx);
        view.update(cx, |view, cx| view.rematch_pressed(cx));
        hear(&view, cx, &refused("g1", "Bob is not taking a rematch"));
        view.read_with(cx, |view, _| {
            assert_eq!(view.rematch, Rematch::None);
            assert_eq!(
                view.problem_text().as_deref(),
                Some("Bob is not taking a rematch")
            );
        });
        // A refusal of another game's rematch changes nothing here.
        view.update(cx, |view, cx| view.rematch_pressed(cx));
        hear(&view, cx, &refused("g9", "no"));
        assert_eq!(
            view.read_with(cx, |view, _| view.rematch.clone()),
            Rematch::Asking
        );
    }

    #[gpui_kit::test]
    fn an_accept_whose_rematch_goes_settles_back_to_rematch(cx: &mut TestAppContext) {
        let (view, _sent, cx) = after_a_game(cx);
        hear(&view, cx, &rematches(&[], &["r1"]));
        view.update(cx, |view, cx| view.rematch_pressed(cx));
        hear(&view, cx, &rematches(&[], &[]));
        view.read_with(cx, |view, _| {
            assert_eq!(view.rematch, Rematch::None);
            assert!(!view.accepting_rematch);
            assert!(
                view.asked_rematch_of.is_none(),
                "not turned into an ask of the player's own"
            );
        });
        assert_eq!(label(&view, cx), "Rematch");
    }

    #[gpui_kit::test]
    fn a_dropped_connection_keeps_a_rematch_the_server_still_holds(cx: &mut TestAppContext) {
        let (view, sent, cx) = after_a_game(cx);
        view.update(cx, |view, cx| view.rematch_pressed(cx));
        hear(&view, cx, &rematches(&["r1"], &[]));
        view.update(cx, |view, cx| view.on_event(LiveEvent::Disconnected, cx));
        // Another device kept the account online, so the lists sent again still hold it.
        hear(&view, cx, &rematches(&["r1"], &[]));
        assert_eq!(label(&view, cx), "Cancel", "still the device that asked");
        view.update(cx, |view, cx| view.leave_game(cx));
        wait_until(cx, |_| was_heard(&sent, "cancel_rematch g1"));
    }

    #[gpui_kit::test]
    fn an_ask_in_flight_as_the_connection_drops_is_settled_by_the_next_lists(
        cx: &mut TestAppContext,
    ) {
        let (view, _sent, cx) = after_a_game(cx);
        view.update(cx, |view, cx| view.rematch_pressed(cx));
        view.update(cx, |view, cx| view.on_event(LiveEvent::Disconnected, cx));
        hear(&view, cx, &rematches(&[], &[]));
        assert_eq!(
            label(&view, cx),
            "Rematch",
            "never posted: it can be asked again"
        );

        view.update(cx, |view, cx| view.rematch_pressed(cx));
        view.update(cx, |view, cx| view.on_event(LiveEvent::Disconnected, cx));
        hear(&view, cx, &rematches(&["r1"], &[]));
        assert_eq!(
            label(&view, cx),
            "Cancel",
            "posted, and still this device's to withdraw"
        );
    }

    #[gpui_kit::test]
    fn a_refusal_for_another_devices_rematch_is_not_shown_here(cx: &mut TestAppContext) {
        let (view, _sent, cx) = after_a_game(cx);
        hear(&view, cx, &rematches(&[], &["r1"]));
        hear(&view, cx, &refused("g1", "Bob is playing a game"));
        assert_eq!(view.read_with(cx, |view, _| view.problem_text()), None);
    }

    #[gpui_kit::test]
    fn asking_after_another_device_did_leaves_that_ask_to_it(cx: &mut TestAppContext) {
        let (view, _sent, cx) = after_a_game(cx);
        view.update(cx, |view, cx| view.rematch_pressed(cx));
        hear(
            &view,
            cx,
            &refused("g1", "You asked for this rematch on another device"),
        );
        hear(&view, cx, &rematches(&["r1"], &[]));
        assert_eq!(label(&view, cx), "Asking…", "that device's to withdraw");
    }

    #[gpui_kit::test]
    fn leaving_forgets_an_ask_lost_in_a_dropped_connection(cx: &mut TestAppContext) {
        let (view, _sent, cx) = after_a_game(cx);
        view.update(cx, |view, cx| view.rematch_pressed(cx));
        view.update(cx, |view, cx| view.on_event(LiveEvent::Disconnected, cx));
        view.update(cx, |view, cx| view.leave_game(cx));
        assert!(!view.read_with(cx, |view, _| view.ask_unsure));
    }

    #[gpui_kit::test]
    fn an_aborted_game_has_its_card_with_a_rematch_but_no_review(cx: &mut TestAppContext) {
        let (view, sent, cx) = after_a_game(cx);
        hear(&view, cx, &aborted());
        let words = view.update(cx, |view, cx| {
            let live = view.live.as_ref().expect("the game");
            view.game_over_card(live, cx).map(|card| card.words())
        });
        assert_eq!(
            words,
            Some((
                "Game aborted".to_owned(),
                "called off before both sides moved".to_owned()
            ))
        );
        view.update(cx, |view, cx| view.rematch_pressed(cx));
        wait_until(cx, |_| was_heard(&sent, "offer_rematch g1"));
    }
}
