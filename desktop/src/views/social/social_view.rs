use std::{future::Future, time::Duration};

use chess_domain::TimeControl;
use gpui_kit::{
    assets::IconName,
    component::{
        ActiveTheme as _, Disableable as _, Sizable as _,
        button::{Button, ButtonVariants},
        h_flex,
        input::{Input, InputEvent, InputState},
        label::Label,
        v_flex,
    },
    prelude::FluentBuilder as _,
    *,
};

use crate::{
    clients::server::ServerClient,
    components::social::{ChallengePicker, PersonRow},
    enums::{social::Relation, sound::Sound},
    models::{
        play::{Challenge, ClientMessage, LiveEvent, ServerMessage},
        server::ServerError,
        social::{FoundUser, Friend, Friends, UserRef},
    },
    services::{
        play::{DEFAULT_TIME_CONTROL, challenge_terms},
        server::error_message,
    },
    views::{
        chat::ChatsView,
        live::{LiveConnection, SessionSource},
        sound::play_sound,
    },
};

/// How long typing pauses before the search goes out.
const SEARCH_PAUSE: Duration = Duration::from_millis(250);
/// The fewest letters worth searching for.
const SHORTEST_SEARCH: usize = 2;

/// Friends and who of them is online, friend requests, finding players by name, and
/// challenging a friend to a game. Lists come over HTTP; the live connection says when to
/// fetch them again, and carries the challenges.
pub struct SocialView {
    client: ServerClient,
    session: SessionSource,
    connection: Entity<LiveConnection>,
    _live: Subscription,
    friends: Friends,
    /// The friends have arrived once; until then the page says it is loading.
    loaded: bool,
    problem: Option<String>,
    search: Entity<InputState>,
    _search_changes: Subscription,
    found: Vec<FoundUser>,
    _search_task: Option<Task<()>>,
    sent: Vec<Challenge>,
    received: Vec<Challenge>,
    /// The friend whose challenge picker is open.
    picking_for: Option<String>,
    choice: TimeControl,
    rated: bool,
    /// A challenge went out or was answered, and the server's word on it is still to come.
    awaiting: bool,
    /// The player's conversations and the one open, beside the friends.
    chats: Entity<ChatsView>,
    _chat_changes: Subscription,
}

impl SocialView {
    pub fn new(
        client: ServerClient,
        session: SessionSource,
        connection: Entity<LiveConnection>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let _live = cx.subscribe(&connection, |view, _, event: &LiveEvent, cx| {
            view.on_event(event, cx);
        });
        let search = cx.new(|cx| InputState::new(window, cx).placeholder("Find players by name"));
        let _search_changes = cx.subscribe(&search, |view, _, event: &InputEvent, cx| {
            if let InputEvent::Change = event {
                view.search_changed(cx);
            }
        });
        let chats = cx.new(|cx| {
            ChatsView::new(
                client.clone(),
                (session.clone(), connection.clone()),
                window,
                cx,
            )
        });
        let _chat_changes = cx.observe(&chats, |_, _, cx| cx.notify());
        Self {
            client,
            session,
            connection,
            _live,
            friends: Friends::default(),
            loaded: false,
            problem: None,
            search,
            _search_changes,
            found: Vec::new(),
            _search_task: None,
            sent: Vec::new(),
            received: Vec::new(),
            picking_for: None,
            choice: DEFAULT_TIME_CONTROL.unwrap_or(TimeControl::new(3, 2)),
            rated: true,
            awaiting: false,
            chats,
            _chat_changes,
        }
    }

    /// The page came on screen: its lists are fetched afresh.
    pub fn opened(&mut self, cx: &mut Context<Self>) {
        self.refresh(cx);
        self.chats.update(cx, |chats, cx| chats.refresh(cx));
    }

    /// What waits for the player: challenges, friend requests and unread messages.
    pub fn waiting_for_me(&self, cx: &App) -> usize {
        self.received.len() + self.friends.incoming.len() + self.chats.read(cx).unread()
    }

    /// Signed in with an account: guests play unrated only.
    fn can_rate(&self, cx: &App) -> bool {
        (self.session)(cx).is_some_and(|session| !session.user.guest)
    }

    /// The signed-in token, unless signed out.
    fn account_token(&self, cx: &App) -> Option<String> {
        (self.session)(cx).map(|session| session.token)
    }

    fn refresh(&mut self, cx: &mut Context<Self>) {
        let Some(token) = self.account_token(cx) else {
            return;
        };
        let request = self.client.friends(&token);
        cx.spawn(async move |view, cx| {
            let result = request.await;
            // The page outlives the request unless the window is closing.
            let _ = view.update(cx, |view, cx| view.friends_arrived(result, cx));
        })
        .detach();
    }

    fn friends_arrived(&mut self, result: Result<Friends, ServerError>, cx: &mut Context<Self>) {
        match result {
            Ok(friends) => {
                let listed = friends.friends.clone();
                self.chats
                    .update(cx, |chats, cx| chats.set_friends(listed, cx));
                self.friends = friends;
                self.loaded = true;
            }
            Err(error) => self.problem = Some(error_message(&error)),
        }
        cx.notify();
    }

    /// Searches once typing pauses; fewer than two letters clear the results.
    fn search_changed(&mut self, cx: &mut Context<Self>) {
        let query = self.search.read(cx).value().trim().to_owned();
        let Some(token) = self.account_token(cx) else {
            return;
        };
        if query.chars().count() < SHORTEST_SEARCH {
            self.found.clear();
            self._search_task = None;
            cx.notify();
            return;
        }
        let client = self.client.clone();
        // A newer search replaces this task, which stops the older one.
        self._search_task = Some(cx.spawn(async move |view, cx| {
            cx.background_executor().timer(SEARCH_PAUSE).await;
            let result = client.search_players(&token, &query).await;
            let _ = view.update(cx, |view, cx| {
                match result {
                    Ok(found) => view.found = found,
                    Err(error) => view.problem = Some(error_message(&error)),
                }
                cx.notify();
            });
        }));
    }

    /// Sends a change to a friendship; once done, the lists and the search follow it.
    fn change_friendship(
        &mut self,
        request: impl Future<Output = Result<(), ServerError>> + 'static,
        cx: &mut Context<Self>,
    ) {
        self.problem = None;
        cx.spawn(async move |view, cx| {
            let result = request.await;
            let _ = view.update(cx, |view, cx| match result {
                Ok(()) => {
                    view.refresh(cx);
                    view.search_changed(cx);
                }
                Err(error) => {
                    view.problem = Some(error_message(&error));
                    cx.notify();
                }
            });
        })
        .detach();
    }

    fn add_friend(&mut self, user_id: &str, cx: &mut Context<Self>) {
        if let Some(token) = self.account_token(cx) {
            let request = self.client.request_friend(&token, user_id);
            self.change_friendship(request, cx);
        }
    }

    fn accept_friend(&mut self, user_id: &str, cx: &mut Context<Self>) {
        if let Some(token) = self.account_token(cx) {
            let request = self.client.accept_friend(&token, user_id);
            self.change_friendship(request, cx);
        }
    }

    /// Unfriends, declines a request, or withdraws one.
    fn remove_friend(&mut self, user_id: &str, cx: &mut Context<Self>) {
        if let Some(token) = self.account_token(cx) {
            let request = self.client.remove_friend(&token, user_id);
            self.change_friendship(request, cx);
        }
    }

    fn on_event(&mut self, event: &LiveEvent, cx: &mut Context<Self>) {
        match event {
            LiveEvent::Connected => self.refresh(cx),
            LiveEvent::Disconnected => self.awaiting = false,
            LiveEvent::Message(message) => self.on_message(message, cx),
        }
        cx.notify();
    }

    fn on_message(&mut self, message: &ServerMessage, cx: &mut Context<Self>) {
        match message {
            // The search results show where the player stands with each, so they follow too.
            ServerMessage::FriendsChanged => {
                self.refresh(cx);
                self.search_changed(cx);
            }
            ServerMessage::Challenges { sent, received } => {
                let new_one = received
                    .iter()
                    .any(|challenge| !self.received.iter().any(|old| old.id == challenge.id));
                if new_one {
                    play_sound(Sound::Notify, cx);
                }
                self.sent = sent.clone();
                self.received = received.clone();
                self.awaiting = false;
            }
            ServerMessage::Game { .. } => self.awaiting = false,
            // Only a refusal of this page's own challenge or answer is its to show.
            ServerMessage::Error { message } if self.awaiting => {
                self.awaiting = false;
                self.problem = Some(message.clone());
            }
            ServerMessage::Error { .. }
            | ServerMessage::Seeking { .. }
            | ServerMessage::SeekCancelled
            | ServerMessage::ChatMessage { .. }
            | ServerMessage::ConversationsChanged
            | ServerMessage::BattleSeeking
            | ServerMessage::BattleSeekCancelled
            | ServerMessage::Battle { .. }
            | ServerMessage::BattleChallenges { .. } => {}
        }
    }

    fn send_live(&mut self, message: ClientMessage, cx: &mut Context<Self>) {
        self.problem = None;
        self.awaiting = true;
        self.connection.update(cx, |connection, cx| {
            connection.connect(cx);
            connection.send(message);
        });
        cx.notify();
    }

    fn send_challenge(&mut self, friend_id: String, cx: &mut Context<Self>) {
        self.picking_for = None;
        let message = ClientMessage::Challenge {
            user_id: friend_id,
            initial_s: seconds(self.choice.initial),
            increment_s: seconds(self.choice.increment),
            rated: self.rated && self.can_rate(cx),
        };
        self.send_live(message, cx);
    }

    fn challenges_section(&self, cx: &mut Context<Self>) -> Option<Div> {
        if self.sent.is_empty() && self.received.is_empty() {
            return None;
        }
        let mut rows: Vec<PersonRow> = Vec::new();
        for challenge in &self.received {
            let (accept, decline) = (challenge.id.clone(), challenge.id.clone());
            let row = PersonRow::new(challenge.from.username.clone())
                .detail(format!("Challenges you · {}", challenge_terms(challenge)))
                .action(
                    Button::new(SharedString::from(format!("accept-challenge-{accept}")))
                        .primary()
                        .small()
                        .label("Accept")
                        .on_click(cx.listener(move |view, _, _, cx| {
                            let message = ClientMessage::AcceptChallenge {
                                challenge_id: accept.clone(),
                            };
                            view.send_live(message, cx);
                        })),
                )
                .action(
                    Button::new(SharedString::from(format!("decline-challenge-{decline}")))
                        .ghost()
                        .small()
                        .label("Decline")
                        .on_click(cx.listener(move |view, _, _, cx| {
                            let message = ClientMessage::DeclineChallenge {
                                challenge_id: decline.clone(),
                            };
                            view.send_live(message, cx);
                        })),
                );
            rows.push(row);
        }
        for challenge in &self.sent {
            let id = challenge.id.clone();
            let row = PersonRow::new(challenge.to.username.clone())
                .detail(format!(
                    "Waiting for an answer · {}",
                    challenge_terms(challenge)
                ))
                .action(
                    Button::new(SharedString::from(format!("cancel-challenge-{id}")))
                        .ghost()
                        .small()
                        .label("Cancel")
                        .on_click(cx.listener(move |view, _, _, cx| {
                            let message = ClientMessage::CancelChallenge {
                                challenge_id: id.clone(),
                            };
                            view.send_live(message, cx);
                        })),
                );
            rows.push(row);
        }
        Some(section("Challenges", cx).children(rows))
    }

    fn requests_section(&self, cx: &mut Context<Self>) -> Option<Div> {
        let Friends {
            incoming, outgoing, ..
        } = &self.friends;
        if incoming.is_empty() && outgoing.is_empty() {
            return None;
        }
        let mut rows: Vec<PersonRow> = Vec::new();
        for person in incoming {
            let row = PersonRow::new(person.username.clone())
                .detail("Wants to be friends")
                .action(self.friend_button("accept-friend", "Accept", person, cx))
                .action(self.remove_button("decline-friend", "Decline", person, cx));
            rows.push(row);
        }
        for person in outgoing {
            let row = PersonRow::new(person.username.clone())
                .detail("Request sent")
                .action(self.remove_button("withdraw-friend", "Cancel", person, cx));
            rows.push(row);
        }
        Some(section("Friend requests", cx).children(rows))
    }

    fn friends_section(&self, cx: &mut Context<Self>) -> Div {
        let title = format!("Friends ({})", self.friends.friends.len());
        let section = section(title, cx);
        if !self.loaded {
            return section.child(muted_line("Loading…", cx));
        }
        if self.friends.friends.is_empty() {
            return section.child(muted_line(
                "No friends yet: find players by name below.",
                cx,
            ));
        }
        let rows: Vec<Div> = self
            .friends
            .friends
            .iter()
            .map(|friend| self.friend_row(friend, cx))
            .collect();
        section.children(rows)
    }

    /// A friend, with Challenge while they are online, and the picker under them once opened.
    fn friend_row(&self, friend: &Friend, cx: &mut Context<Self>) -> Div {
        let picking = self.picking_for.as_deref() == Some(friend.id.as_str());
        let id = friend.id.clone();
        let challenge = Button::new(SharedString::from(format!("challenge-{id}")))
            .outline()
            .small()
            .icon(IconName::Swords)
            .label("Challenge")
            .disabled(!friend.online)
            .on_click(cx.listener(move |view, _, _, cx| {
                view.picking_for = Some(id.clone());
                cx.notify();
            }));
        let person = UserRef {
            id: friend.id.clone(),
            username: friend.username.clone(),
        };
        let to_message = friend.id.clone();
        let message = Button::new(SharedString::from(format!("message-{to_message}")))
            .outline()
            .small()
            .label("Message")
            .on_click(cx.listener(move |view, _, window, cx| {
                view.chats.update(cx, |chats, cx| {
                    chats.message_friend(&to_message, window, cx)
                });
            }));
        let row = PersonRow::new(friend.username.clone())
            .online(friend.online)
            .detail(if friend.online { "Online" } else { "Offline" })
            .action(message)
            .action(challenge)
            .action(self.remove_button("unfriend", "Remove", &person, cx));
        v_flex().gap_2().child(row).when(picking, |this| {
            this.child(self.picker(friend.id.clone(), cx))
        })
    }

    fn picker(&self, friend_id: String, cx: &mut Context<Self>) -> ChallengePicker {
        let view = cx.entity().downgrade();
        let (choose, rated, send, close) = (view.clone(), view.clone(), view.clone(), view);
        ChallengePicker::new(
            (self.choice, self.rated, self.can_rate(cx)),
            move |choice, _, cx| {
                let _ = choose.update(cx, |view, cx| {
                    view.choice = choice;
                    cx.notify();
                });
            },
            move |on, _, cx| {
                let _ = rated.update(cx, |view, cx| {
                    view.rated = on;
                    cx.notify();
                });
            },
            (
                move |_, cx: &mut App| {
                    let _ = send.update(cx, |view, cx| view.send_challenge(friend_id.clone(), cx));
                },
                move |_, cx: &mut App| {
                    let _ = close.update(cx, |view, cx| {
                        view.picking_for = None;
                        cx.notify();
                    });
                },
            ),
        )
    }

    fn search_section(&self, cx: &mut Context<Self>) -> Div {
        let rows: Vec<PersonRow> = self
            .found
            .iter()
            .map(|found| self.found_row(found, cx))
            .collect();
        section("Find players", cx)
            .child(Input::new(&self.search))
            .children(rows)
    }

    /// A player found, with what can be done next with them.
    fn found_row(&self, found: &FoundUser, cx: &mut Context<Self>) -> PersonRow {
        let person = UserRef {
            id: found.id.clone(),
            username: found.username.clone(),
        };
        let row = PersonRow::new(found.username.clone());
        match found.relation {
            Relation::None => {
                row.action(self.friend_button("add-friend", "Add friend", &person, cx))
            }
            Relation::RequestReceived => row
                .detail("Wants to be friends")
                .action(self.friend_button("accept-found", "Accept", &person, cx)),
            Relation::RequestSent => row.detail("Request sent"),
            Relation::Friends => row.detail("Friends"),
        }
    }

    /// Asks to be friends, which accepts a request they sent.
    fn friend_button(
        &self,
        id: &str,
        label: &'static str,
        person: &UserRef,
        cx: &mut Context<Self>,
    ) -> Button {
        let user_id = person.id.clone();
        let accepting = label == "Accept";
        Button::new(SharedString::from(format!("{id}-{user_id}")))
            .primary()
            .small()
            .when(!accepting, |this| this.icon(IconName::UserPlus))
            .label(label)
            .on_click(cx.listener(move |view, _, _, cx| {
                if accepting {
                    view.accept_friend(&user_id, cx);
                } else {
                    view.add_friend(&user_id, cx);
                }
            }))
    }

    fn remove_button(
        &self,
        id: &str,
        label: &'static str,
        person: &UserRef,
        cx: &mut Context<Self>,
    ) -> Button {
        let user_id = person.id.clone();
        Button::new(SharedString::from(format!("{id}-{user_id}")))
            .ghost()
            .small()
            .label(label)
            .on_click(cx.listener(move |view, _, _, cx| view.remove_friend(&user_id, cx)))
    }
}

impl Render for SocialView {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let people = v_flex()
            .flex_1()
            .gap_6()
            .when_some(self.problem.clone(), |this, problem| {
                this.child(div().text_sm().text_color(cx.theme().danger).child(problem))
            })
            .children(self.challenges_section(cx))
            .children(self.requests_section(cx))
            .child(self.friends_section(cx))
            .child(self.search_section(cx));
        page_frame().child(
            h_flex()
                .flex_1()
                .min_h_0()
                .items_start()
                .gap_6()
                .child(people)
                .child(self.chats.clone()),
        )
    }
}

fn page_frame() -> Div {
    v_flex()
        .size_full()
        .p_6()
        .gap_6()
        .child(Label::new("Social").text_2xl())
}

fn section(title: impl Into<SharedString>, cx: &App) -> Div {
    v_flex().w_full().max_w(px(640.)).gap_2().child(
        div()
            .text_sm()
            .text_color(cx.theme().muted_foreground)
            .child(title.into()),
    )
}

fn muted_line(text: &'static str, cx: &App) -> Div {
    div()
        .text_sm()
        .text_color(cx.theme().muted_foreground)
        .child(text)
}

fn seconds(duration: Duration) -> u32 {
    u32::try_from(duration.as_secs()).unwrap_or(u32::MAX)
}

#[cfg(test)]
mod tests {
    use std::{
        rc::Rc,
        sync::{Arc, Mutex},
    };

    use axum::{
        Json, Router,
        extract::{
            Path,
            ws::{Message, WebSocket, WebSocketUpgrade},
        },
        http::StatusCode,
        routing::{get, post},
    };
    use gpui_kit::{AppContext as _, Entity, TestAppContext, VisualTestContext};
    use serde_json::{Value, json};

    use super::SocialView;
    use crate::{
        clients::server::ServerClient,
        enums::sound::Sound,
        models::account::{Session, User},
        testing::wait_until,
        views::{
            live::{LiveConnection, SessionSource},
            sound::{played_sounds, record_sounds},
        },
    };

    /// What the fake server was asked, in order.
    type Asked = Arc<Mutex<Vec<String>>>;

    /// Ann has Bob as a friend, online, and a request from Cat; Bob challenges her as soon as
    /// she connects, and accepting it is recorded.
    fn server(asked: Asked) -> ServerClient {
        let accepted = Arc::clone(&asked);
        let socket_asked = Arc::clone(&asked);
        ServerClient::serving(
            Router::new()
                .route(
                    "/api/social/friends",
                    get(|| async {
                        Json(json!({
                            "friends": [{ "id": "bob", "username": "bob", "online": true }],
                            "incoming": [{ "id": "cat", "username": "cat" }],
                            "outgoing": []
                        }))
                    }),
                )
                .route(
                    "/api/social/friends/requests/{user_id}/accept",
                    post(move |Path(user_id): Path<String>| async move {
                        let mut asked = accepted.lock().expect("the list");
                        asked.push(format!("accept {user_id}"));
                        StatusCode::NO_CONTENT
                    }),
                )
                .route(
                    "/api/ws",
                    get(move |upgrade: WebSocketUpgrade| async move {
                        upgrade.on_upgrade(move |socket| bob_challenges(socket, socket_asked))
                    }),
                ),
        )
    }

    async fn bob_challenges(mut socket: WebSocket, asked: Asked) {
        let challenge = json!({ "type": "challenges", "sent": [], "received": [{
            "id": "c1",
            "from": { "id": "bob", "username": "bob" },
            "to": { "id": "ann", "username": "ann" },
            "initial_s": 300, "increment_s": 3, "rated": true
        }]});
        if socket
            .send(Message::Text(challenge.to_string().into()))
            .await
            .is_err()
        {
            return;
        }
        while let Some(Ok(Message::Text(text))) = socket.recv().await {
            let message: Value = serde_json::from_str(text.as_str()).expect("json");
            let kind = message["type"].as_str().unwrap_or_default().to_owned();
            let id = message["challenge_id"]
                .as_str()
                .unwrap_or_default()
                .to_owned();
            asked.lock().expect("the list").push(format!("{kind} {id}"));
        }
    }

    fn signed_in(guest: bool) -> SessionSource {
        let session = Session {
            token: "abc".to_owned(),
            user: User {
                id: "ann".to_owned(),
                email: "ann@example.com".to_owned(),
                username: "ann".to_owned(),
                guest,
            },
        };
        Rc::new(move |_| Some(session.clone()))
    }

    fn social(
        cx: &mut TestAppContext,
        guest: bool,
    ) -> (Entity<SocialView>, Asked, &mut VisualTestContext) {
        cx.executor().allow_parking();
        cx.update(|cx| {
            gpui_kit::init(cx);
            record_sounds(cx);
        });
        let asked = Asked::default();
        let client = server(Arc::clone(&asked));
        let (view, cx) = cx.add_window_view(|window, cx| {
            let session = signed_in(guest);
            let live = cx.new(|_| LiveConnection::new(client.clone(), Rc::clone(&session)));
            SocialView::new(client, session, live, window, cx)
        });
        (view, asked, cx)
    }

    fn asked_for(asked: &Asked, what: &str) -> bool {
        asked
            .lock()
            .expect("the list")
            .iter()
            .any(|line| line == what)
    }

    #[gpui_kit::test]
    fn friends_and_requests_load_and_a_request_is_accepted(cx: &mut TestAppContext) {
        let (view, asked, cx) = social(cx, false);
        view.update(cx, |view, cx| view.opened(cx));
        wait_until(cx, |cx| view.read_with(cx, |view, _| view.loaded));
        view.read_with(cx, |view, cx| {
            assert_eq!(view.friends.friends[0].username, "bob");
            assert!(view.friends.friends[0].online);
            assert_eq!(view.waiting_for_me(cx), 1, "Cat's request waits");
        });
        view.update(cx, |view, cx| view.accept_friend("cat", cx));
        wait_until(cx, |_| asked_for(&asked, "accept cat"));
    }

    #[gpui_kit::test]
    fn a_challenge_rings_and_accepting_it_is_sent(cx: &mut TestAppContext) {
        let (view, asked, cx) = social(cx, false);
        view.update(cx, |view, cx| {
            view.connection.update(cx, |live, cx| live.connect(cx));
        });
        wait_until(cx, |cx| {
            view.read_with(cx, |view, _| !view.received.is_empty())
        });
        assert!(
            cx.update(|_, cx| played_sounds(cx))
                .contains(&Sound::Notify)
        );
        view.update(cx, |view, cx| {
            let message = super::ClientMessage::AcceptChallenge {
                challenge_id: "c1".to_owned(),
            };
            view.send_live(message, cx);
        });
        wait_until(cx, |_| asked_for(&asked, "accept_challenge c1"));
    }

    #[gpui_kit::test]
    fn a_guest_has_friends_like_anyone(cx: &mut TestAppContext) {
        let (view, _, cx) = social(cx, true);
        view.update(cx, |view, cx| view.opened(cx));
        wait_until(cx, |cx| view.read_with(cx, |view, _| view.loaded));
    }
}
