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
        board::{BoardColumn, PLAYER_BAR, board_page, copy_fen},
        history::{MoveList, NavigationBar},
        panel::{ChoiceTile, PanelHeader, SidePanel, action_bar},
        play::{
            CapturedRow, ClockFace, GameOverActions, GameOverCard, GameSetup, OnlineOptions,
            PlayPanel, WaitingBar, with_card,
        },
        shell::PLAY_PICTURE,
    },
    enums::{
        play::{LiveStatus, PlayMode},
        sound::Sound,
    },
    models::{
        account::ModeStats,
        board::{MoveMade, ShownPosition},
        play::{AnalyzeGame, ClientMessage, GameSnapshot, LiveEvent, ServerMessage},
    },
    services::{
        history::GameHistory,
        play::{
            ClockWarning, DEFAULT_TIME_CONTROL, can_abort, clock_left, my_side, outcome_of,
            outcome_reason, rating_line, replay, result_line, result_title,
        },
    },
    views::{
        board::{BoardView, flip_button},
        chat::ChatView,
        history::{
            HistoryNavigation, Holding, hold_step, select_move, step_history, with_history_keys,
        },
        live::{LiveConnection, SessionSource},
        sound::{play_move_sound, play_sound},
        ticker::{CLOCK_TICK, start_ticker},
    },
};

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
    problem: Option<String>,
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
            cx.new(|cx| ChatView::new(source, (chat_id, "Chat".into()), window, cx))
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
            LiveEvent::Disconnected => {
                // The server forgets a seek when its player goes; the game waits for them.
                self.seeking = false;
                self.awaiting = false;
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
                self.problem = Some(message);
            }
            ServerMessage::Error { .. }
            | ServerMessage::FriendsChanged
            | ServerMessage::Challenges { .. }
            | ServerMessage::ChatMessage { .. }
            | ServerMessage::ConversationsChanged
            | ServerMessage::BattleSeeking
            | ServerMessage::BattleSeekCancelled
            | ServerMessage::Battle { .. }
            | ServerMessage::BattleChallenges { .. } => {}
        }
    }

    /// The server's latest word on a game: board, sounds and clock follow, and a premove goes.
    fn on_snapshot(&mut self, snapshot: GameSnapshot, cx: &mut Context<Self>) {
        let Some(game) = replay(&snapshot) else {
            self.problem = Some("The game could not be read".to_owned());
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
            .then(|| "Connection lost, reconnecting…".to_owned())
    }

    fn game_id(&self) -> Option<String> {
        self.live.as_ref().map(|live| live.snapshot.id.clone())
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

    /// Hands the game to the analysis board, at the position being looked at.
    fn analyze(&mut self, review: bool, cx: &mut Context<Self>) {
        if let Some(live) = &self.live {
            cx.emit(AnalyzeGame {
                game: live.history.game().clone(),
                viewing: live.history.viewing(),
                review,
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
        let outcome = outcome_of(&live.snapshot).filter(|_| !self.card_closed)?;
        let title = result_title(outcome.winner(), live.me, None);
        let detail = live.me.and_then(|me| rating_line(&live.snapshot, me));
        let close = cx.entity().downgrade();
        let card = GameOverCard::new(
            (title, outcome_reason(outcome).to_owned()),
            self.game_over_actions(live, cx),
            move |_, cx| {
                let _ = close.update(cx, |view, cx| {
                    view.card_closed = true;
                    cx.notify();
                });
            },
        )
        .detail(detail);
        Some(card)
    }

    /// Game Review, a look without a review, back to the lobby, or a rematch at the same time.
    fn game_over_actions(&self, live: &LiveBoard, cx: &mut Context<Self>) -> GameOverActions {
        let control = TimeControl {
            initial: Duration::from_secs(live.snapshot.initial_s.into()),
            increment: Duration::from_secs(live.snapshot.increment_s.into()),
        };
        let page = cx.entity().downgrade();
        let (review, fresh, again) = (page.clone(), page.clone(), page);
        let look = review.clone();
        GameOverActions::new(move |_, cx| {
            let _ = review.update(cx, |view, cx| view.analyze(true, cx));
        })
        .choice("Analyze", IconName::Microscope, move |_, cx| {
            let _ = look.update(cx, |view, cx| view.analyze(false, cx));
        })
        .choice("New game", IconName::Plus, move |_, cx| {
            let _ = fresh.update(cx, |view, cx| view.leave_game(cx));
        })
        .when(live.me.is_some(), |actions| {
            // Another opponent at the same time control, rated as this game was.
            let rated = live.snapshot.rated;
            actions.choice(control.label(), IconName::RotateCcw, move |_, cx| {
                let _ = again.update(cx, |view, cx| {
                    view.leave_game(cx);
                    view.choice = control;
                    view.rated = rated;
                    view.seek(cx);
                });
            })
        })
    }

    /// From a finished game back to the lobby.
    fn leave_game(&mut self, cx: &mut Context<Self>) {
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
                initial_s: seconds(self.choice.initial),
                increment_s: seconds(self.choice.increment),
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
        let problem = self.problem.clone().or_else(|| {
            self.connection
                .read(cx)
                .is_reconnecting()
                .then(|| "Connecting to the server…".to_owned())
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
        let me = (self.session)(cx).map_or_else(String::new, |session| session.user.username);
        let me = match self.my_rating() {
            Some(rating) => format!("{me} ({rating})"),
            None => me,
        };
        let clock = Some(self.choice.initial);
        let column = BoardColumn::new(self.board.clone())
            .top(WaitingBar::new("Opponent").clock(clock))
            .bottom(WaitingBar::new(me).clock(clock));
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
            .top(self.player_bar(live, bottom.opponent()))
            .bottom(self.player_bar(live, bottom))
            .tool(flip_button("online-flip-board", cx, OnlineView::flip))
            .tool(copy_fen(live.history.shown()));
        let side = SidePanel::new()
            .header(PanelHeader::new(PLAY_PICTURE, "Play Online"))
            .child(self.header(live, cx))
            .child(MoveList::new(
                live.history.sans().to_vec(),
                live.history.viewing(),
                select_move(page.clone()),
            ))
            .children(
                self.chat
                    .clone()
                    .map(|chat| div().h(px(240.)).flex_shrink_0().child(chat)),
            )
            .child(
                NavigationBar::new(
                    live.history.can_go_back(),
                    live.history.can_go_forward(),
                    step_history(page.clone()),
                )
                .on_hold(hold_step(page)),
            )
            .map(|this| {
                // An aborted game has no result to review: it keeps the plain buttons.
                if outcome_of(&live.snapshot).is_some() {
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
                "Your move".to_owned()
            } else {
                "Waiting for your opponent".to_owned()
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
                self.problem.clone().or_else(|| self.connection_line(cx)),
                |this, problem| {
                    this.child(div().text_sm().text_color(cx.theme().danger).child(problem))
                },
            )
    }

    /// The game buttons: Abort, Draw and Resign, an offer's answers, or what to do once over.
    fn actions(&self, live: &LiveBoard, cx: &mut Context<Self>) -> Vec<ChoiceTile> {
        let snapshot = &live.snapshot;
        if snapshot.status != LiveStatus::Ongoing {
            return vec![
                game_tile(("online-analyze", "Analyze"), false, cx, |view, cx| {
                    view.analyze(false, cx);
                })
                .icon(IconName::Microscope),
                game_tile(
                    ("online-new-game", "New game"),
                    false,
                    cx,
                    OnlineView::leave_game,
                )
                .icon(IconName::Plus),
            ];
        }
        if can_abort(snapshot) {
            return vec![
                game_tile(("online-abort", "Abort"), false, cx, OnlineView::abort)
                    .icon(IconName::Close),
            ];
        }
        let offer = snapshot.draw_offer.map(Side::from);
        if offer.is_some() && offer != live.me {
            return vec![
                game_tile(
                    ("online-accept-draw", "Accept draw"),
                    true,
                    cx,
                    |view, cx| {
                        view.answer_draw(true, cx);
                    },
                )
                .icon(IconName::Check),
                game_tile(("online-decline-draw", "Decline"), false, cx, |view, cx| {
                    view.answer_draw(false, cx);
                })
                .icon(IconName::Close),
            ];
        }
        vec![
            game_tile(
                ("online-offer-draw", "Draw"),
                false,
                cx,
                OnlineView::offer_draw,
            )
            .glyph("½")
            .disabled(offer.is_some()),
            game_tile(("online-resign", "Resign"), false, cx, OnlineView::resign)
                .icon(IconName::Flag),
        ]
    }

    /// A player's name and rating, what they have taken, and their clock.
    fn player_bar(&self, live: &LiveBoard, side: Side) -> Div {
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

fn seconds(duration: Duration) -> u32 {
    u32::try_from(duration.as_secs()).unwrap_or(u32::MAX)
}

#[cfg(test)]
mod tests {
    use std::rc::Rc;

    use axum::{
        Router,
        extract::ws::{Message, WebSocket, WebSocketUpgrade},
        routing::get,
    };
    use gpui_kit::{AppContext as _, Entity, TestAppContext, VisualTestContext};
    use serde_json::{Value, json};

    use chess_domain::TimeControl;

    use super::OnlineView;
    use crate::{
        clients::server::ServerClient,
        enums::sound::Sound,
        models::account::{Session, User},
        models::play::AnalyzeGame,
        testing::{click, wait_until},
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
        let session = Session {
            token: "abc".to_owned(),
            user: User {
                id: "ann".to_owned(),
                email: "ann@example.com".to_owned(),
                username: "Ann".to_owned(),
                guest: false,
            },
        };
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
        view.update(cx, |view, cx| view.analyze(false, cx));
        assert!(analyzed.get(), "the game went to the analysis board");
    }
}
