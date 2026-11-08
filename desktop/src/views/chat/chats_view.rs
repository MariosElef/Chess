use std::future::Future;

use gpui_kit::{
    component::{
        ActiveTheme as _, Sizable as _,
        button::{Button, ButtonVariants},
        h_flex,
        input::{Input, InputState},
        v_flex,
    },
    prelude::FluentBuilder as _,
    *,
};

use super::chat_view::ChatView;
use crate::{
    clients::server::ServerClient,
    components::social::PersonRow,
    enums::{chat::ConversationKind, sound::Sound},
    models::{
        chat::Conversation,
        play::{LiveEvent, ServerMessage},
        server::ServerError,
        social::Friend,
    },
    services::server::error_message,
    views::{
        live::{LiveConnection, SessionSource},
        sound::play_sound,
    },
};

/// The player's conversations, a new group, and the one open: one to one with a friend or in a
/// group. The list follows the live connection: new messages, groups and members.
pub struct ChatsView {
    client: ServerClient,
    session: SessionSource,
    connection: Entity<LiveConnection>,
    _live: Subscription,
    conversations: Vec<Conversation>,
    /// The player's friends, to message, put in groups and add to them.
    friends: Vec<Friend>,
    open: Option<(Conversation, Entity<ChatView>)>,
    /// The new-group form: open or not, its name, and the friends ticked.
    making_group: bool,
    group_name: Entity<InputState>,
    group_members: Vec<String>,
    problem: Option<String>,
}

impl ChatsView {
    pub fn new(
        client: ServerClient,
        (session, connection): (SessionSource, Entity<LiveConnection>),
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let _live = cx.subscribe(&connection, |view, _, event: &LiveEvent, cx| {
            view.on_event(event, cx);
        });
        let group_name = cx.new(|cx| InputState::new(window, cx).placeholder("Group name"));
        Self {
            client,
            session,
            connection,
            _live,
            conversations: Vec::new(),
            friends: Vec::new(),
            open: None,
            making_group: false,
            group_name,
            group_members: Vec::new(),
            problem: None,
        }
    }

    /// Messages the player has not read, in the conversations not open.
    pub fn unread(&self) -> usize {
        let open = self
            .open
            .as_ref()
            .map(|(conversation, _)| conversation.id.as_str());
        self.conversations
            .iter()
            .filter(|conversation| Some(conversation.id.as_str()) != open)
            .map(|conversation| usize::try_from(conversation.unread).unwrap_or(usize::MAX))
            .sum()
    }

    pub fn set_friends(&mut self, friends: Vec<Friend>, cx: &mut Context<Self>) {
        self.friends = friends;
        cx.notify();
    }

    fn token(&self, cx: &App) -> Option<String> {
        (self.session)(cx).map(|session| session.token)
    }

    /// Fetches the list again.
    pub fn refresh(&mut self, cx: &mut Context<Self>) {
        let Some(token) = self.token(cx) else {
            return;
        };
        let request = self.client.conversations(&token);
        cx.spawn(async move |view, cx| {
            let result = request.await;
            // The view outlives the request unless the window is closing.
            let _ = view.update(cx, |view, cx| {
                match result {
                    Ok(conversations) => view.conversations = conversations,
                    Err(error) => view.problem = Some(error_message(&error)),
                }
                cx.notify();
            });
        })
        .detach();
    }

    /// Opens the conversation with a friend, started the first time.
    pub fn message_friend(&mut self, user_id: &str, window: &mut Window, cx: &mut Context<Self>) {
        let Some(token) = self.token(cx) else {
            return;
        };
        let request = self.client.open_direct(&token, user_id);
        self.open_when_ready(request, window, cx);
    }

    fn open_when_ready(
        &mut self,
        request: impl Future<Output = Result<Conversation, ServerError>> + 'static,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        cx.spawn_in(window, async move |view, cx| {
            let result = request.await;
            let _ = view.update_in(cx, |view, window, cx| match result {
                Ok(conversation) => {
                    view.refresh(cx);
                    view.open(conversation, window, cx);
                }
                Err(error) => {
                    view.problem = Some(error_message(&error));
                    cx.notify();
                }
            });
        })
        .detach();
    }

    /// Shows the conversation; its messages count as read from now.
    fn open(&mut self, conversation: Conversation, window: &mut Window, cx: &mut Context<Self>) {
        let chat = cx.new(|cx| {
            ChatView::new(
                (&self.connection, self.session.clone()),
                (conversation.id.clone(), conversation.title.clone().into()),
                window,
                cx,
            )
        });
        if let Some(listed) = self
            .conversations
            .iter_mut()
            .find(|listed| listed.id == conversation.id)
        {
            listed.unread = 0;
        }
        self.problem = None;
        self.open = Some((conversation, chat));
        cx.notify();
    }

    fn on_event(&mut self, event: &LiveEvent, cx: &mut Context<Self>) {
        match event {
            LiveEvent::Connected => self.refresh(cx),
            LiveEvent::Disconnected => {}
            LiveEvent::Message(message) => match message.as_ref() {
                ServerMessage::ConversationsChanged => self.refresh(cx),
                ServerMessage::ChatMessage { message } => {
                    let me = (self.session)(cx).map(|session| session.user.id);
                    let open = self.open.as_ref().map(|(open, _)| open.id.clone());
                    let from_another = me.as_deref() != Some(message.sender.id.as_str());
                    if from_another && open.as_deref() != Some(message.conversation_id.as_str()) {
                        play_sound(Sound::Notify, cx);
                    }
                    self.refresh(cx);
                }
                ServerMessage::Seeking { .. }
                | ServerMessage::SeekCancelled
                | ServerMessage::Game { .. }
                | ServerMessage::Error { .. }
                | ServerMessage::FriendsChanged
                | ServerMessage::Challenges { .. }
                | ServerMessage::BattleSeeking
                | ServerMessage::BattleSeekCancelled
                | ServerMessage::Battle { .. }
                | ServerMessage::BattleChallenges { .. } => {}
            },
        }
    }

    fn create_group(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(token) = self.token(cx) else {
            return;
        };
        let name = self.group_name.read(cx).value().trim().to_owned();
        let request = self.client.create_group(&token, &name, &self.group_members);
        self.making_group = false;
        self.group_members.clear();
        self.group_name
            .update(cx, |input, cx| input.set_value("", window, cx));
        self.open_when_ready(request, window, cx);
    }

    /// Adds a friend to the open group, or leaves it.
    fn change_group(
        &mut self,
        request: impl Future<Output = Result<(), ServerError>> + 'static,
        leaving: bool,
        cx: &mut Context<Self>,
    ) {
        if leaving {
            self.open = None;
        }
        cx.spawn(async move |view, cx| {
            let result = request.await;
            let _ = view.update(cx, |view, cx| {
                if let Err(error) = result {
                    view.problem = Some(error_message(&error));
                }
                view.refresh(cx);
                cx.notify();
            });
        })
        .detach();
    }

    fn conversation_row(
        &self,
        conversation: &Conversation,
        cx: &mut Context<Self>,
    ) -> Stateful<Div> {
        let opened = self
            .open
            .as_ref()
            .is_some_and(|(open, _)| open.id == conversation.id);
        let preview = conversation.last_message.as_ref().map_or_else(
            || "No messages yet".to_owned(),
            |last| format!("{}: {}", last.sender.username, last.body),
        );
        let unread = conversation.unread;
        let row = PersonRow::new(conversation.title.clone())
            .detail(shortened(&preview))
            .when(unread > 0 && !opened, |row| {
                row.action(unread_pill(unread, cx))
            });
        let listed = conversation.clone();
        div()
            .id(SharedString::from(format!(
                "conversation-{}",
                conversation.id
            )))
            .cursor_pointer()
            .rounded(cx.theme().radius)
            .when(opened, |this| this.bg(cx.theme().accent))
            .on_click(cx.listener(move |view, _, window, cx| {
                view.open(listed.clone(), window, cx);
            }))
            .child(row)
    }

    fn group_form(&self, cx: &mut Context<Self>) -> Div {
        let toggles: Vec<Button> = self
            .friends
            .iter()
            .map(|friend| {
                let id = friend.id.clone();
                let ticked = self.group_members.contains(&id);
                let button = Button::new(SharedString::from(format!("group-pick-{id}")))
                    .small()
                    .label(friend.username.clone())
                    .on_click(cx.listener(move |view, _, _, cx| {
                        if view.group_members.contains(&id) {
                            view.group_members.retain(|member| *member != id);
                        } else {
                            view.group_members.push(id.clone());
                        }
                        cx.notify();
                    }));
                if ticked {
                    button.primary()
                } else {
                    button.outline()
                }
            })
            .collect();
        v_flex()
            .gap_2()
            .p_3()
            .rounded(cx.theme().radius)
            .bg(cx.theme().muted)
            .child(Input::new(&self.group_name))
            .child(div().text_xs().child("Friends in the group"))
            .child(h_flex().flex_wrap().gap_2().children(toggles))
            .child(
                h_flex()
                    .gap_2()
                    .child(
                        Button::new("create-group")
                            .primary()
                            .small()
                            .label("Create group")
                            .on_click(cx.listener(|view, _, window, cx| {
                                view.create_group(window, cx);
                            })),
                    )
                    .child(
                        Button::new("cancel-group")
                            .ghost()
                            .small()
                            .label("Cancel")
                            .on_click(cx.listener(|view, _, _, cx| {
                                view.making_group = false;
                                cx.notify();
                            })),
                    ),
            )
    }

    /// For an open group: a button for each friend not in it yet, and Leave.
    fn group_actions(&self, conversation: &Conversation, cx: &mut Context<Self>) -> Div {
        let missing: Vec<Button> = self
            .friends
            .iter()
            .filter(|friend| {
                !conversation
                    .members
                    .iter()
                    .any(|member| member.id == friend.id)
            })
            .map(|friend| {
                let (group, friend_id) = (conversation.id.clone(), friend.id.clone());
                Button::new(SharedString::from(format!("add-to-group-{friend_id}")))
                    .outline()
                    .small()
                    .label(format!("+ {}", friend.username))
                    .on_click(cx.listener(move |view, _, _, cx| {
                        view.add_to_group(&group, &friend_id, cx);
                    }))
            })
            .collect();
        let group = conversation.id.clone();
        h_flex().flex_wrap().gap_2().children(missing).child(
            Button::new("leave-group")
                .ghost()
                .small()
                .label("Leave group")
                .on_click(cx.listener(move |view, _, _, cx| view.leave_group(&group, cx))),
        )
    }

    fn add_to_group(&mut self, group: &str, friend_id: &str, cx: &mut Context<Self>) {
        if let Some(token) = self.token(cx) {
            let request = self.client.add_to_group(&token, group, friend_id);
            self.change_group(request, false, cx);
        }
    }

    fn leave_group(&mut self, group: &str, cx: &mut Context<Self>) {
        if let Some(token) = self.token(cx) {
            let request = self.client.leave_group(&token, group);
            self.change_group(request, true, cx);
        }
    }
}

impl Render for ChatsView {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let rows: Vec<Stateful<Div>> = self
            .conversations
            .iter()
            .map(|conversation| self.conversation_row(conversation, cx))
            .collect();
        let open = self.open.clone();
        v_flex()
            .w(px(420.))
            .h_full()
            .gap_3()
            .child(
                h_flex()
                    .justify_between()
                    .child(
                        div()
                            .text_sm()
                            .text_color(cx.theme().muted_foreground)
                            .child("Chats"),
                    )
                    .child(
                        Button::new("new-group")
                            .outline()
                            .small()
                            .label("New group")
                            .on_click(cx.listener(|view, _, _, cx| {
                                view.making_group = !view.making_group;
                                cx.notify();
                            })),
                    ),
            )
            .when_some(self.problem.clone(), |this, problem| {
                this.child(div().text_xs().text_color(cx.theme().danger).child(problem))
            })
            .when(self.making_group, |this| this.child(self.group_form(cx)))
            .when(rows.is_empty(), |this| {
                this.child(
                    div()
                        .text_sm()
                        .text_color(cx.theme().muted_foreground)
                        .child("No chats yet: message a friend to start one."),
                )
            })
            .child(
                div()
                    .id("conversation-list")
                    .max_h(px(220.))
                    .overflow_y_scroll()
                    .child(v_flex().gap_1().children(rows)),
            )
            .when_some(open, |this, (conversation, chat)| {
                this.child(
                    v_flex()
                        .flex_1()
                        .min_h(px(280.))
                        .gap_2()
                        .p_3()
                        .rounded(cx.theme().radius)
                        .border_1()
                        .border_color(cx.theme().border)
                        .when(conversation.kind == ConversationKind::Group, |this| {
                            this.child(self.group_actions(&conversation, cx))
                        })
                        .child(div().flex_1().min_h_0().child(chat)),
                )
            })
    }
}

/// A red count of unread messages.
fn unread_pill(count: u32, cx: &App) -> Div {
    div()
        .px_1p5()
        .rounded_full()
        .bg(cx.theme().red)
        .text_color(white())
        .text_xs()
        .child(count.to_string())
}

/// The start of a long preview, so a row stays one line.
fn shortened(text: &str) -> String {
    const LONGEST: usize = 48;
    if text.chars().count() <= LONGEST {
        return text.to_owned();
    }
    let start: String = text.chars().take(LONGEST).collect();
    format!("{start}…")
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

    use super::ChatsView;
    use crate::{
        clients::server::ServerClient,
        enums::sound::Sound,
        models::account::{Session, User},
        testing::wait_until,
        views::{
            live::LiveConnection,
            sound::{played_sounds, record_sounds},
        },
    };

    /// What the fake server was asked, in order.
    type Asked = Arc<Mutex<Vec<String>>>;

    fn message(id: &str, conversation: &str, from: &str, body: &str) -> Value {
        json!({
            "id": id, "conversation_id": conversation, "body": body,
            "sender": { "id": from, "username": from }, "sent_at": "2026-11-08T10:00:00Z"
        })
    }

    fn direct() -> Value {
        json!({
            "id": "d1", "kind": "direct", "title": "bob", "unread": 0, "last_message": null,
            "members": [{ "id": "ann", "username": "ann" }, { "id": "bob", "username": "bob" }]
        })
    }

    /// Ann chats with Bob and is in a group with two unread messages; once she connects, Bob
    /// writes to her and the group gets a message too.
    fn server(asked: Asked) -> ServerClient {
        let (sent, read) = (Arc::clone(&asked), Arc::clone(&asked));
        ServerClient::serving(
            Router::new()
                .route(
                    "/api/chat/conversations",
                    get(|| async {
                        let group = json!({
                            "id": "g1", "kind": "group", "title": "Club", "unread": 2,
                            "last_message": message("m0", "g1", "cat", "hi all"),
                            "members": [{ "id": "ann", "username": "ann" }]
                        });
                        Json(json!([direct(), group]))
                    }),
                )
                .route("/api/chat/direct", post(|| async { Json(direct()) }))
                .route(
                    "/api/chat/conversations/{id}/messages",
                    get(|Path(id): Path<String>| async move {
                        Json(json!([message("m1", &id, "bob", "hello")]))
                    })
                    .post(
                        move |Path(id): Path<String>, Json(body): Json<Value>| async move {
                            let text = body["body"].as_str().unwrap_or_default().to_owned();
                            sent.lock()
                                .expect("the list")
                                .push(format!("send {id} {text}"));
                            (StatusCode::CREATED, Json(message("m2", &id, "ann", &text)))
                        },
                    ),
                )
                .route(
                    "/api/chat/conversations/{id}/read",
                    post(move |Path(id): Path<String>| async move {
                        read.lock().expect("the list").push(format!("read {id}"));
                        StatusCode::NO_CONTENT
                    }),
                )
                .route(
                    "/api/ws",
                    get(|upgrade: WebSocketUpgrade| async move { upgrade.on_upgrade(bob_writes) }),
                ),
        )
    }

    async fn bob_writes(mut socket: WebSocket) {
        for arrived in [
            message("m3", "d1", "bob", "your move"),
            message("m4", "g1", "cat", "anyone?"),
        ] {
            let text = json!({ "type": "chat_message", "message": arrived }).to_string();
            if socket.send(Message::Text(text.into())).await.is_err() {
                return;
            }
        }
        while let Some(Ok(_)) = socket.recv().await {}
    }

    fn chats(
        cx: &mut TestAppContext,
    ) -> (
        Entity<ChatsView>,
        Entity<LiveConnection>,
        Asked,
        &mut VisualTestContext,
    ) {
        cx.executor().allow_parking();
        cx.update(|cx| {
            gpui_kit::init(cx);
            record_sounds(cx);
        });
        let asked = Asked::default();
        let client = server(Arc::clone(&asked));
        let session = Session {
            token: "abc".to_owned(),
            user: User {
                id: "ann".to_owned(),
                email: "ann@example.com".to_owned(),
                username: "ann".to_owned(),
                guest: false,
            },
        };
        let session: crate::views::live::SessionSource = Rc::new(move |_| Some(session.clone()));
        let live =
            cx.update(|cx| cx.new(|_| LiveConnection::new(client.clone(), Rc::clone(&session))));
        let connection = live.clone();
        let (view, cx) = cx.add_window_view(|window, cx| {
            ChatsView::new(client, (session, connection), window, cx)
        });
        (view, live, asked, cx)
    }

    fn asked_for(asked: &Asked, what: &str) -> bool {
        asked
            .lock()
            .expect("the list")
            .iter()
            .any(|line| line == what)
    }

    fn open_messages(view: &Entity<ChatsView>, cx: &mut VisualTestContext) -> Vec<String> {
        view.read_with(cx, |view, cx| {
            view.open
                .as_ref()
                .map_or_else(Vec::new, |(_, chat)| chat.read(cx).bodies())
        })
    }

    #[gpui_kit::test]
    fn a_friend_is_messaged_and_their_answer_arrives_live(cx: &mut TestAppContext) {
        let (view, live, asked, cx) = chats(cx);
        view.update(cx, |view, cx| view.refresh(cx));
        wait_until(cx, |cx| {
            view.read_with(cx, |view, _| view.conversations.len() == 2)
        });
        assert_eq!(
            view.read_with(cx, |view, _| view.unread()),
            2,
            "the group's two"
        );

        view.update_in(cx, |view, window, cx| {
            view.message_friend("bob", window, cx)
        });
        wait_until(cx, |cx| open_messages(&view, cx) == ["hello"]);
        wait_until(cx, |_| asked_for(&asked, "read d1"));

        let chat = view.read_with(cx, |view, _| view.open.clone().expect("open").1);
        chat.update_in(cx, |chat, window, cx| {
            chat.write_and_send("  good game ", window, cx);
        });
        wait_until(cx, |_| asked_for(&asked, "send d1 good game"));
        wait_until(cx, |cx| open_messages(&view, cx) == ["hello", "good game"]);

        live.update(cx, |live, cx| live.connect(cx));
        wait_until(cx, |cx| open_messages(&view, cx).len() == 3);
        assert_eq!(open_messages(&view, cx)[2], "your move");
        wait_until(cx, |cx| {
            cx.update(|_, cx| played_sounds(cx))
                .contains(&Sound::Notify)
        });
        let notified = cx
            .update(|_, cx| played_sounds(cx))
            .iter()
            .filter(|sound| **sound == Sound::Notify)
            .count();
        assert_eq!(
            notified, 1,
            "only the group's message rings: the open chat is read"
        );
    }
}
