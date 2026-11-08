use std::{
    rc::Rc,
    time::{Duration, Instant},
};

use gpui_kit::*;

use super::puzzle_solver::PuzzleSolver;
use crate::{
    clients::server::ServerClient,
    components::{
        board::{BoardColumn, board_page},
        panel::{OnBack, SidePanel},
        puzzles::{BattleBar, BattleMenu, BattleMenuActions, BattleMenuState, BattlePanel},
    },
    enums::{
        puzzles::{BattleStatus, PuzzleVerdict},
        sound::Sound,
    },
    models::{
        play::{ClientMessage, LiveEvent, ServerMessage},
        puzzles::{BattleChallenge, BattleSnapshot, BattleSummary, Puzzle},
        social::Friend,
    },
    services::puzzles::{battle_left, current_puzzle},
    views::{
        live::{LiveConnection, SessionSource},
        sound::play_sound,
        ticker::start_ticker,
    },
};

/// How often the battle's clock is redrawn.
const TICK: Duration = Duration::from_millis(200);
/// The pause after a solved puzzle before the next, to see it solved.
const NEXT_DELAY: Duration = Duration::from_millis(300);

/// A battle just began, so the Puzzles page shows it wherever it was.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BattleBegan;

/// A battle just ended: the panel's wins and rating are fetched again.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BattleEnded;

/// Puzzle Battle: pick a random opponent or an online friend, then race on shared puzzles live.
pub struct BattleView {
    connection: Entity<LiveConnection>,
    session: SessionSource,
    client: ServerClient,
    _live: Subscription,
    solver: Entity<PuzzleSolver>,
    _verdicts: Subscription,
    /// The battle the player is in or has just finished, and when it last arrived.
    battle: Option<(BattleSnapshot, Instant)>,
    /// Its puzzles, read once when it began.
    puzzles: Vec<Puzzle>,
    /// The player's own answers and mistakes, counted as made so the next puzzle comes at once.
    answered: u32,
    mistakes: u32,
    /// The puzzle on the board, by its place in the battle.
    shown: Option<usize>,
    summary: BattleSummary,
    friends: Vec<Friend>,
    picked: Option<String>,
    sent: Vec<BattleChallenge>,
    received: Vec<BattleChallenge>,
    seeking: bool,
    /// Battles asked for count for the rating; the server makes a guest's unrated whatever.
    rated: bool,
    /// A request went out, so a refusal that comes back is this panel's to show.
    awaiting: bool,
    problem: Option<String>,
    on_back: OnBack,
    _ticker: Option<Task<()>>,
    _next: Option<Task<()>>,
}

impl EventEmitter<BattleBegan> for BattleView {}
impl EventEmitter<BattleEnded> for BattleView {}

impl BattleView {
    pub fn new(
        connection: Entity<LiveConnection>,
        session: SessionSource,
        cx: &mut Context<Self>,
    ) -> Self {
        let _live = cx.subscribe(&connection, |view, _, event: &LiveEvent, cx| {
            view.on_event(event, cx);
        });
        let solver = cx.new(PuzzleSolver::new);
        let _verdicts = cx.subscribe(&solver, |view, _, verdict: &PuzzleVerdict, cx| {
            view.judged(*verdict, cx);
        });
        Self {
            client: connection.read(cx).client(),
            connection,
            session,
            _live,
            solver,
            _verdicts,
            battle: None,
            puzzles: Vec::new(),
            answered: 0,
            mistakes: 0,
            shown: None,
            summary: BattleSummary::default(),
            friends: Vec::new(),
            picked: None,
            sent: Vec::new(),
            received: Vec::new(),
            seeking: false,
            rated: true,
            awaiting: false,
            problem: None,
            on_back: Rc::new(|_, _| {}),
            _ticker: None,
            _next: None,
        }
    }

    pub fn on_back(&mut self, on_back: impl Fn(&mut Window, &mut App) + 'static) {
        self.on_back = Rc::new(on_back);
    }

    pub fn set_summary(&mut self, summary: BattleSummary, cx: &mut Context<Self>) {
        self.summary = summary;
        cx.notify();
    }

    /// The panel came on screen: connected, with the friends who are online fetched.
    pub fn opened(&mut self, cx: &mut Context<Self>) {
        self.problem = None;
        self.connection
            .update(cx, |connection, cx| connection.connect(cx));
        self.fetch_friends(cx);
    }

    /// A battle is being played: its clock ticks and its puzzles are on the board.
    fn is_playing(&self) -> bool {
        self.battle
            .as_ref()
            .is_some_and(|(battle, _)| battle.status == BattleStatus::Ongoing)
    }

    fn fetch_friends(&self, cx: &mut Context<Self>) {
        let Some(token) = (self.session)(cx).map(|session| session.token) else {
            return;
        };
        let request = self.client.friends(&token);
        cx.spawn(async move |view, cx| {
            // Without them the panel offers a random opponent alone.
            if let Ok(friends) = request.await {
                let _ = view.update(cx, |view, cx| {
                    view.friends = friends
                        .friends
                        .into_iter()
                        .filter(|friend| friend.online)
                        .collect();
                    if view
                        .picked
                        .as_ref()
                        .is_some_and(|picked| !view.friends.iter().any(|f| &f.id == picked))
                    {
                        view.picked = None;
                    }
                    cx.notify();
                });
            }
        })
        .detach();
    }

    fn send(&mut self, message: ClientMessage, cx: &mut Context<Self>) {
        self.connection.update(cx, |connection, cx| {
            connection.connect(cx);
            connection.send(message);
        });
        self.awaiting = true;
        self.problem = None;
        cx.notify();
    }

    /// Play: a random opponent, or an invitation to the friend picked.
    fn play(&mut self, cx: &mut Context<Self>) {
        let message = match self.picked.clone() {
            Some(user_id) => ClientMessage::ChallengeBattle {
                user_id,
                rated: self.rated,
            },
            None => ClientMessage::SeekBattle { rated: self.rated },
        };
        self.send(message, cx);
    }

    /// Stops looking for an opponent, or withdraws the invitation sent.
    fn cancel(&mut self, cx: &mut Context<Self>) {
        if self.seeking {
            self.send(ClientMessage::CancelBattleSeek, cx);
        }
        if let Some(sent) = self.sent.first() {
            let challenge_id = sent.id.clone();
            self.send(ClientMessage::DeclineBattle { challenge_id }, cx);
        }
    }

    /// The panel is being left: a search for an opponent is called off.
    pub fn closing(&mut self, cx: &mut Context<Self>) {
        if self.seeking {
            self.send(ClientMessage::CancelBattleSeek, cx);
        }
    }

    /// Battle invitations waiting for the player, for the rail's count.
    pub fn invitations(&self) -> usize {
        self.received.len()
    }

    /// Redraws the clock; false once the battle is over.
    fn tick(&mut self, cx: &mut Context<Self>) -> bool {
        cx.notify();
        self.is_playing()
    }

    /// Gives up the battle being played.
    fn resign(&mut self, cx: &mut Context<Self>) {
        if let Some((battle, _)) = &self.battle {
            let battle_id = battle.id.clone();
            self.send(ClientMessage::ResignBattle { battle_id }, cx);
        }
    }

    /// From a finished battle back to the panel.
    fn leave(&mut self, cx: &mut Context<Self>) {
        self.battle = None;
        self.shown = None;
        self._ticker = None;
        self.solver.update(cx, |solver, cx| solver.clear(cx));
        cx.notify();
    }

    fn on_event(&mut self, event: &LiveEvent, cx: &mut Context<Self>) {
        let LiveEvent::Message(message) = event else {
            return;
        };
        match message.as_ref() {
            ServerMessage::Battle { battle } => self.arrived((**battle).clone(), cx),
            ServerMessage::BattleSeeking => {
                self.seeking = true;
                self.awaiting = false;
            }
            ServerMessage::BattleSeekCancelled => {
                self.seeking = false;
                self.awaiting = false;
            }
            ServerMessage::BattleChallenges { sent, received } => {
                let new_one = received
                    .iter()
                    .any(|invitation| !self.received.iter().any(|old| old.id == invitation.id));
                if new_one {
                    play_sound(Sound::Notify, cx);
                }
                self.sent = sent.clone();
                self.received = received.clone();
                self.awaiting = false;
            }
            ServerMessage::FriendsChanged => self.fetch_friends(cx),
            // Only a refusal of this panel's own request is its to show.
            ServerMessage::Error { message } if self.awaiting => {
                self.awaiting = false;
                self.problem = Some(message.clone());
            }
            ServerMessage::Error { .. }
            | ServerMessage::Seeking { .. }
            | ServerMessage::SeekCancelled
            | ServerMessage::Game { .. }
            | ServerMessage::Challenges { .. }
            | ServerMessage::ChatMessage { .. }
            | ServerMessage::ConversationsChanged => return,
        }
        cx.notify();
    }

    /// The battle as the server has it: a new one starts on its first puzzle; the rest follows.
    fn arrived(&mut self, battle: BattleSnapshot, cx: &mut Context<Self>) {
        let is_new = self
            .battle
            .as_ref()
            .is_none_or(|(old, _)| old.id != battle.id);
        if is_new {
            self.puzzles = battle
                .puzzles
                .iter()
                .filter_map(|text| text.read().ok())
                .collect();
            self.answered = 0;
            self.mistakes = 0;
            self.shown = None;
            self.seeking = false;
            self.awaiting = false;
            if battle.status == BattleStatus::Ongoing {
                play_sound(Sound::GameStart, cx);
                self._ticker = Some(start_ticker(TICK, cx, BattleView::tick));
                cx.emit(BattleBegan);
            }
        }
        // The server counts every answer it has: ahead of the board, as after reconnecting, the
        // board catches up; behind it, the answers on their way will arrive.
        let counted = battle.me.score + battle.me.mistakes;
        let behind = counted > self.answered;
        if behind {
            self.answered = counted;
            self.mistakes = battle.me.mistakes;
        }
        // Finished since the last word, or already over when it first arrived, as after
        // reconnecting.
        let finished_now = battle.status == BattleStatus::Finished
            && self
                .battle
                .as_ref()
                .is_none_or(|(old, _)| old.status == BattleStatus::Ongoing || old.id != battle.id);
        self.battle = Some((battle, Instant::now()));
        if finished_now {
            // A resign is answered by the battle ending, not by a word of its own.
            self.awaiting = false;
            play_sound(Sound::GameEnd, cx);
            cx.emit(BattleEnded);
            self._ticker = None;
            self.solver.update(cx, |solver, cx| solver.clear(cx));
        } else if is_new || behind {
            self.show_current(cx);
        }
        cx.notify();
    }

    /// Puts the player's current puzzle on the board, or clears it once they are out.
    fn show_current(&mut self, cx: &mut Context<Self>) {
        if !self.is_playing() {
            return;
        }
        let current = current_puzzle(self.answered, self.mistakes, self.puzzles.len());
        if current == self.shown {
            return;
        }
        self.shown = current;
        match current.and_then(|index| self.puzzles.get(index).cloned()) {
            Some(puzzle) => self.solver.update(cx, |solver, cx| solver.load(puzzle, cx)),
            None => self.solver.update(cx, |solver, cx| solver.clear(cx)),
        }
    }

    /// A solved puzzle scores, a wrong move is a mistake; the server hears it and the next follows.
    fn judged(&mut self, verdict: PuzzleVerdict, cx: &mut Context<Self>) {
        let Some(battle_id) = self.battle.as_ref().map(|(battle, _)| battle.id.clone()) else {
            return;
        };
        let solved = match verdict {
            PuzzleVerdict::Correct => return,
            PuzzleVerdict::Solved => true,
            PuzzleVerdict::Wrong => false,
        };
        self.answered += 1;
        if !solved {
            self.mistakes += 1;
        }
        self.connection
            .read(cx)
            .send(ClientMessage::BattleAnswer { battle_id, solved });
        if solved {
            self._next = Some(cx.spawn(async move |view, cx| {
                cx.background_executor().timer(NEXT_DELAY).await;
                let _ = view.update(cx, |view, cx| view.show_current(cx));
            }));
        } else {
            self.show_current(cx);
        }
        cx.notify();
    }

    fn menu(&self, cx: &mut Context<Self>) -> BattleMenu {
        let page = cx.entity().downgrade();
        let on_back = Rc::clone(&self.on_back);
        let (pick, play, cancel) = (page.clone(), page.clone(), page.clone());
        let (accept, decline, rated) = (page.clone(), page.clone(), page);
        BattleMenu::new(
            BattleMenuState {
                summary: self.summary,
                friends: self.friends.clone(),
                picked: self.picked.clone(),
                received: self.received.clone(),
                sent: self.sent.clone(),
                seeking: self.seeking,
                problem: self.problem.clone(),
                guest: (self.session)(cx).is_some_and(|session| session.user.guest),
                rated: self.rated,
            },
            BattleMenuActions {
                on_back,
                on_pick: Rc::new(move |friend, _, cx| {
                    let _ = pick.update(cx, |view, cx| {
                        view.picked = friend;
                        cx.notify();
                    });
                }),
                on_play: Rc::new(move |_, cx| {
                    let _ = play.update(cx, |view, cx| view.play(cx));
                }),
                on_cancel: Rc::new(move |_, cx| {
                    let _ = cancel.update(cx, |view, cx| view.cancel(cx));
                }),
                on_accept: Rc::new(move |challenge_id, _, cx| {
                    let _ = accept.update(cx, |view, cx| {
                        view.send(ClientMessage::AcceptBattle { challenge_id }, cx);
                    });
                }),
                on_rated: Rc::new(move |on, _, cx| {
                    let _ = rated.update(cx, |view, cx| {
                        view.rated = on;
                        cx.notify();
                    });
                }),
                on_decline: Rc::new(move |challenge_id, _, cx| {
                    let _ = decline.update(cx, |view, cx| {
                        view.send(ClientMessage::DeclineBattle { challenge_id }, cx);
                    });
                }),
            },
        )
    }

    fn battle_page(&self, battle: &BattleSnapshot, since: Instant, cx: &mut Context<Self>) -> Div {
        let page = cx.entity().downgrade();
        let (again, back, resign) = (page.clone(), page.clone(), page);
        let result = battle.result.map(|result| (result, battle.rating_change));
        let panel = BattlePanel::new(
            (battle_left(battle, since.elapsed()), battle.rated),
            (battle.me.score, battle.opponent.score),
            result,
            (
                Rc::new(move |_, cx| {
                    let _ = again.update(cx, |view, cx| {
                        view.leave(cx);
                        view.play(cx);
                    });
                }),
                Rc::new(move |_, cx| {
                    let _ = back.update(cx, |view, cx| view.leave(cx));
                }),
            ),
        )
        .on_resign(move |_, cx| {
            let _ = resign.update(cx, |view, cx| view.resign(cx));
        });
        let column = BoardColumn::new(self.solver.clone())
            .top(BattleBar::new(battle.opponent.clone()))
            .bottom(BattleBar::new(battle.me.clone()));
        board_page(column, SidePanel::new().child(panel))
    }

    #[cfg(test)]
    pub fn solver(&self) -> &Entity<PuzzleSolver> {
        &self.solver
    }
}

impl Render for BattleView {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        match self.battle.clone() {
            Some((battle, since)) => self.battle_page(&battle, since, cx),
            None => board_page(
                BoardColumn::new(self.solver.clone()),
                SidePanel::new().child(self.menu(cx)),
            ),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::{
        rc::Rc,
        sync::{Arc, Mutex},
    };

    use axum::{
        Json, Router,
        extract::ws::{Message, WebSocket, WebSocketUpgrade},
        routing::get,
    };
    use gpui_kit::{AppContext as _, Entity, TestAppContext, VisualTestContext};
    use serde_json::{Value, json};

    use super::BattleView;
    use crate::{
        clients::server::ServerClient,
        enums::puzzles::{BattleResult, BattleStatus},
        models::account::{Session, User},
        testing::{click, wait_until},
        views::live::{LiveConnection, SessionSource},
    };

    /// What the fake server was sent, in order.
    type Heard = Arc<Mutex<Vec<String>>>;

    fn battle(status: &str, my_mistakes: u32) -> Value {
        json!({ "type": "battle", "battle": {
            "id": "b1", "rated": true, "duration_ms": 180000, "left_ms": 170000,
            "status": status,
            "result": if status == "finished" { json!("won") } else { Value::Null },
            "rating_change": if status == "finished" { json!(9) } else { Value::Null },
            "me": { "id": "ann", "username": "ann", "rating": 1500, "score": 0, "mistakes": my_mistakes },
            "opponent": { "id": "bob", "username": "bob", "rating": 1480, "score": 0, "mistakes": 3 },
            "puzzles": [{
                "id": "5laTS", "rating": 1593, "plays": 2015, "themes": ["mateIn3"],
                "fen": "7k/pp2r1p1/2pb3r/3p1B1Q/3N4/8/P2q2PP/4R2K w - - 1 1",
                "last_move": "h7h8",
                "solution": ["h5e8", "e7e8", "e1e8", "d6f8", "e8f8"]
            }]
        }})
    }

    /// Bob is waiting: a seek starts the battle at once, and Ann's first answer ends it.
    async fn bob_battles(mut socket: WebSocket, heard: Heard) {
        while let Some(Ok(Message::Text(text))) = socket.recv().await {
            let message: Value = serde_json::from_str(text.as_str()).expect("json");
            let kind = message["type"].as_str().unwrap_or_default().to_owned();
            let line = match kind.as_str() {
                "battle_answer" => format!("battle_answer {}", message["solved"]),
                "seek_battle" => format!("seek_battle rated {}", message["rated"]),
                _ => kind.clone(),
            };
            heard.lock().expect("the list").push(line);
            let answer = match kind.as_str() {
                "seek_battle" => battle("ongoing", 0),
                "battle_answer" => battle("finished", 1),
                _ => continue,
            };
            if socket
                .send(Message::Text(answer.to_string().into()))
                .await
                .is_err()
            {
                return;
            }
        }
    }

    fn battle_view(cx: &mut TestAppContext) -> (Entity<BattleView>, Heard, &mut VisualTestContext) {
        cx.executor().allow_parking();
        cx.update(gpui_kit::init);
        let heard = Heard::default();
        let socket_heard = Arc::clone(&heard);
        let client = ServerClient::serving(
            Router::new()
                .route(
                    "/api/social/friends",
                    get(|| async {
                        Json(json!({ "friends": [], "incoming": [], "outgoing": [] }))
                    }),
                )
                .route(
                    "/api/ws",
                    get(move |upgrade: WebSocketUpgrade| async move {
                        upgrade.on_upgrade(move |socket| bob_battles(socket, socket_heard))
                    }),
                ),
        );
        let session = Session {
            token: "abc".to_owned(),
            user: User {
                id: "ann".to_owned(),
                email: "ann@example.com".to_owned(),
                username: "ann".to_owned(),
                guest: false,
            },
        };
        let session: SessionSource = Rc::new(move |_| Some(session.clone()));
        let (view, cx) = cx.add_window_view(|_, cx| {
            let live = cx.new(|_| LiveConnection::new(client.clone(), Rc::clone(&session)));
            BattleView::new(live, session, cx)
        });
        (view, heard, cx)
    }

    #[gpui_kit::test]
    fn a_battle_is_found_played_on_the_board_and_ends_with_the_result(cx: &mut TestAppContext) {
        let (view, heard, cx) = battle_view(cx);
        view.update(cx, |view, cx| view.opened(cx));
        click(cx, "battle-play");
        wait_until(cx, |cx| view.read_with(cx, |view, _| view.is_playing()));
        let loaded = view.read_with(cx, |view, cx| view.solver().read(cx).run().is_some());
        assert!(loaded, "the first puzzle is on the board");

        // The queen takes the rook on h6: not the solution, so a mistake.
        click(cx, "square-h5");
        click(cx, "square-h6");
        wait_until(cx, |_| {
            heard
                .lock()
                .expect("the list")
                .iter()
                .any(|line| line == "battle_answer false")
        });
        wait_until(cx, |cx| view.read_with(cx, |view, _| !view.is_playing()));
        let ended = view.read_with(cx, |view, _| {
            view.battle
                .as_ref()
                .map(|(battle, _)| (battle.status, battle.result))
        });
        assert_eq!(
            ended,
            Some((BattleStatus::Finished, Some(BattleResult::Won)))
        );
    }

    #[gpui_kit::test]
    fn an_unrated_battle_is_asked_for_when_rated_is_off(cx: &mut TestAppContext) {
        let (view, heard, cx) = battle_view(cx);
        view.update(cx, |view, cx| {
            view.opened(cx);
            view.rated = false;
        });
        click(cx, "battle-play");
        wait_until(cx, |_| {
            heard
                .lock()
                .expect("the list")
                .iter()
                .any(|line| line == "seek_battle rated false")
        });
    }
}
