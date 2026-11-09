use std::future::Future;

use gpui_kit::{
    assets::IconName,
    component::{
        ActiveTheme as _, Icon, Sizable as _,
        button::{Button, ButtonVariants},
        h_flex, v_flex,
    },
    prelude::FluentBuilder as _,
    *,
};

use super::{chat_view::ChatView, group_form::GroupForm, new_group_form::NewGroupForm};
use crate::{
    clients::server::ServerClient,
    components::social::PersonRow,
    enums::{
        chat::{ConversationKind, GroupChange},
        sound::Sound,
    },
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

/// The player's conversations, a new group, and the one open, with a friend or in a group.
pub struct ChatsView {
    client: ServerClient,
    session: SessionSource,
    connection: Entity<LiveConnection>,
    _live: Subscription,
    conversations: Vec<Conversation>,
    /// The player's friends, to message, put in groups and add to them.
    friends: Vec<Friend>,
    open: Option<(Conversation, Entity<ChatView>)>,
    /// The Messages page is on screen, so the open chat is being read.
    shown: bool,
    problem: Option<String>,
    /// Lists asked for so far, and the newest shown: an older answer landing late is ignored.
    lists_asked: u64,
    list_shown: u64,
    /// Lists asked for before the open chat and the Group modal opened: they may not have them.
    asked_before_open: u64,
    asked_before_form: u64,
    /// Lists asked for before the Group modal's last change: they cannot show it yet.
    asked_before_change: u64,
    /// The Group modal while it is open, kept in step with the group.
    group_form: Option<WeakEntity<GroupForm>>,
}

impl ChatsView {
    pub fn new(
        client: ServerClient,
        (session, connection): (SessionSource, Entity<LiveConnection>),
        cx: &mut Context<Self>,
    ) -> Self {
        let _live = cx.subscribe(&connection, |view, _, event: &LiveEvent, cx| {
            view.on_event(event, cx);
        });
        Self {
            client,
            session,
            connection,
            _live,
            conversations: Vec::new(),
            friends: Vec::new(),
            open: None,
            shown: false,
            problem: None,
            lists_asked: 0,
            list_shown: 0,
            asked_before_open: 0,
            asked_before_form: 0,
            asked_before_change: 0,
            group_form: None,
        }
    }

    /// The chat being read: open, on the Messages page while it is on screen.
    fn reading_chat(&self) -> Option<&str> {
        self.open
            .as_ref()
            .filter(|_| self.shown)
            .map(|(conversation, _)| conversation.id.as_str())
    }

    /// Messages the player has not read: in every chat but the one on screen.
    pub fn unread(&self) -> usize {
        let open = self.reading_chat();
        self.conversations
            .iter()
            .filter(|conversation| Some(conversation.id.as_str()) != open)
            .map(|conversation| usize::try_from(conversation.unread).unwrap_or(usize::MAX))
            .sum()
    }

    /// The Messages page came on or off screen: the open chat reads new messages, or stops.
    pub fn set_shown(&mut self, shown: bool, cx: &mut Context<Self>) {
        self.shown = shown;
        if let Some((conversation, chat)) = self.open.as_ref() {
            chat.update(cx, |chat, cx| chat.set_reading(shown, cx));
            if shown {
                let id = conversation.id.clone();
                self.clear_unread(&id);
            }
        }
        cx.notify();
    }

    /// The open chat, while on screen, is read: a stale list does not bring back its count.
    fn clear_read_chat(&mut self) {
        if let Some(id) = self.reading_chat().map(str::to_owned) {
            self.clear_unread(&id);
        }
    }

    /// The chat's messages count as read here at once, before the server's list says so.
    fn clear_unread(&mut self, conversation_id: &str) {
        if let Some(listed) = self
            .conversations
            .iter_mut()
            .find(|listed| listed.id == conversation_id)
        {
            listed.unread = 0;
        }
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
        self.lists_asked += 1;
        let number = self.lists_asked;
        cx.spawn(async move |view, cx| {
            let result = request.await;
            // The view outlives the request unless the window is closing.
            let _ = view.update(cx, |view, cx| view.list_arrived(number, result, cx));
        })
        .detach();
    }

    /// The list asked for as `number`, unless a newer one was shown already.
    fn list_arrived(
        &mut self,
        number: u64,
        result: Result<Vec<Conversation>, ServerError>,
        cx: &mut Context<Self>,
    ) {
        if number < self.list_shown {
            return;
        }
        self.list_shown = number;
        match result {
            Ok(conversations) => {
                self.conversations = conversations;
                self.follow_open(number, cx);
                self.follow_group_form(number, cx);
                self.clear_read_chat();
            }
            Err(error) => {
                let problem = error_message(&error);
                // The Group modal waits for this list: it stops waiting and says why.
                if let Some(form) = self.group_form.as_ref().and_then(WeakEntity::upgrade) {
                    let shown = problem.clone();
                    form.update(cx, |form, cx| form.failed(shown, cx));
                }
                self.problem = Some(problem);
            }
        }
        cx.notify();
    }

    /// The Group modal's group as the list now has it; none once the player is out of it.
    fn follow_group_form(&self, number: u64, cx: &mut Context<Self>) {
        let Some(form) = self.group_form.as_ref().and_then(WeakEntity::upgrade) else {
            return;
        };
        // A list asked for before the last change cannot show it: the modal waits on.
        if number <= self.asked_before_change {
            return;
        }
        let id = form.read(cx).group_id().to_owned();
        let listed = self
            .conversations
            .iter()
            .find(|listed| listed.id == id)
            .cloned();
        // A list asked for before the modal opened may simply not have the group yet.
        if listed.is_none() && number <= self.asked_before_form {
            return;
        }
        form.update(cx, |form, cx| form.follow(listed, cx));
    }

    /// The open chat as the list now has it; closed once the player is not in it.
    fn follow_open(&mut self, number: u64, cx: &mut Context<Self>) {
        let Some((open, chat)) = self.open.clone() else {
            return;
        };
        let listed = self
            .conversations
            .iter()
            .find(|listed| listed.id == open.id)
            .cloned();
        match listed {
            Some(listed) => {
                let title = SharedString::from(listed.title.clone());
                chat.update(cx, |chat, cx| chat.set_title(title, cx));
                self.open = Some((listed, chat));
            }
            // A list asked for before the chat opened may simply not have it yet.
            None if number <= self.asked_before_open => {}
            None => self.open = None,
        }
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

    /// Shows the conversation; its messages count as read while the Messages page is on screen.
    fn open(&mut self, conversation: Conversation, window: &mut Window, cx: &mut Context<Self>) {
        let chat = cx.new(|cx| {
            ChatView::new(
                (&self.connection, self.session.clone()),
                (conversation.id.clone(), conversation.title.clone().into()),
                window,
                cx,
            )
        });
        chat.update(cx, |chat, cx| chat.set_reading(self.shown, cx));
        self.problem = None;
        self.open = Some((conversation, chat));
        self.asked_before_open = self.lists_asked;
        self.clear_read_chat();
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
                    let from_another = me.as_deref() != Some(message.sender.id.as_str());
                    if from_another && self.reading_chat() != Some(message.conversation_id.as_str())
                    {
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

    /// Makes the group the New group modal asked for, and opens it once made.
    fn create_group(
        &mut self,
        (name, members): (String, Vec<String>),
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(token) = self.token(cx) else {
            return;
        };
        let request = self.client.create_group(&token, &name, &members);
        self.open_when_ready(request, window, cx);
    }

    /// Opens the New group modal with the player's friends to pick from.
    fn open_new_group(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let chats = cx.entity().downgrade();
        NewGroupForm::open(
            self.friends.clone(),
            move |name, members, window, cx| {
                // The view outlives its buttons unless the window is closing.
                let _ = chats.update(cx, |chats, cx| {
                    chats.create_group((name, members), window, cx)
                });
            },
            window,
            cx,
        );
    }

    /// Sends a change to the open group; a refusal shows in the Group modal too.
    fn change_group(
        &mut self,
        (group, request): (
            &str,
            impl Future<Output = Result<(), ServerError>> + 'static,
        ),
        leaving: bool,
        cx: &mut Context<Self>,
    ) {
        let group = group.to_owned();
        if leaving {
            self.open = None;
        }
        cx.spawn(async move |view, cx| {
            let result = request.await;
            // The view outlives the request unless the window is closing.
            let _ = view.update(cx, |view, cx| {
                if let Err(error) = result {
                    let problem = error_message(&error);
                    let form = view.group_form.as_ref().and_then(WeakEntity::upgrade);
                    if let Some(form) = form.filter(|form| form.read(cx).group_id() == group) {
                        let shown = problem.clone();
                        form.update(cx, |form, cx| form.failed(shown, cx));
                    }
                    view.problem = Some(problem);
                }
                // Lists asked until now may have reached the server before the change did.
                view.asked_before_change = view.lists_asked;
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

    /// Opens the Group modal for the open group.
    fn open_group(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let (Some((group, _)), Some(session)) = (self.open.clone(), (self.session)(cx)) else {
            return;
        };
        let chats = cx.entity().downgrade();
        let id = group.id.clone();
        let form = GroupForm::open(
            (group, session.user.id, self.friends.clone()),
            move |change, _, cx| {
                // The view outlives its buttons unless the window is closing.
                let _ = chats.update(cx, |chats, cx| chats.change_group_by(&id, change, cx));
            },
            window,
            cx,
        );
        self.group_form = Some(form.downgrade());
        self.asked_before_form = self.lists_asked;
    }

    /// Sends what was changed in the Group modal.
    fn change_group_by(&mut self, group: &str, change: GroupChange, cx: &mut Context<Self>) {
        match change {
            GroupChange::Rename(title) => self.rename_group(group, &title, cx),
            GroupChange::Add(friend) => self.add_to_group(group, &friend, cx),
            GroupChange::Remove(member) => self.remove_from_group(group, &member, cx),
            GroupChange::Leave => self.leave_group(group, cx),
        }
    }

    fn rename_group(&mut self, group: &str, title: &str, cx: &mut Context<Self>) {
        if let Some(token) = self.token(cx) {
            let request = self.client.rename_group(&token, group, title);
            self.change_group((group, request), false, cx);
        }
    }

    fn remove_from_group(&mut self, group: &str, member: &str, cx: &mut Context<Self>) {
        if let Some(token) = self.token(cx) {
            let request = self.client.remove_from_group(&token, group, member);
            self.change_group((group, request), false, cx);
        }
    }

    fn add_to_group(&mut self, group: &str, friend_id: &str, cx: &mut Context<Self>) {
        if let Some(token) = self.token(cx) {
            let request = self.client.add_to_group(&token, group, friend_id);
            self.change_group((group, request), false, cx);
        }
    }

    fn leave_group(&mut self, group: &str, cx: &mut Context<Self>) {
        if let Some(token) = self.token(cx) {
            let request = self.client.leave_group(&token, group);
            self.change_group((group, request), true, cx);
        }
    }
}

impl Render for ChatsView {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        h_flex()
            .size_full()
            .items_start()
            .gap_5()
            .child(self.chat_list(cx))
            .child(self.open_chat(cx))
    }
}

impl ChatsView {
    /// The left column: the chats, newest first, with New group over them.
    fn chat_list(&self, cx: &mut Context<Self>) -> Div {
        let rows: Vec<Stateful<Div>> = self
            .conversations
            .iter()
            .map(|conversation| self.conversation_row(conversation, cx))
            .collect();
        let theme = cx.theme();
        let (muted, danger) = (theme.muted_foreground, theme.danger);
        v_flex()
            .w(px(360.))
            .h_full()
            .flex_shrink_0()
            .gap_3()
            .p_3()
            .rounded(px(8.))
            .bg(theme.muted)
            .child(
                h_flex()
                    .justify_between()
                    .child(div().font_weight(FontWeight::BOLD).child("Chats"))
                    .child(
                        Button::new("new-group")
                            .outline()
                            .small()
                            .icon(IconName::Plus)
                            .label("New group")
                            .on_click(cx.listener(|view, _, window, cx| {
                                view.open_new_group(window, cx);
                            })),
                    ),
            )
            .when_some(self.problem.clone(), |this, problem| {
                this.child(div().text_xs().text_color(danger).child(problem))
            })
            .when(rows.is_empty(), |this| {
                this.child(
                    div()
                        .text_sm()
                        .text_color(muted)
                        .child("No chats yet: message a friend from Social to start one."),
                )
            })
            .child(
                div()
                    .id("conversation-list")
                    .flex_1()
                    .min_h_0()
                    .overflow_y_scroll()
                    .child(v_flex().gap_1().children(rows)),
            )
    }

    /// The right of the page: the chat open, or a word to pick one.
    fn open_chat(&self, cx: &mut Context<Self>) -> Div {
        let theme = cx.theme();
        let card = v_flex()
            .flex_1()
            .min_w_0()
            .h_full()
            .gap_2()
            .p_4()
            .rounded(px(8.))
            .bg(theme.muted);
        let Some((conversation, chat)) = self.open.clone() else {
            return card.items_center().justify_center().child(
                v_flex()
                    .items_center()
                    .gap_2()
                    .text_color(theme.muted_foreground)
                    .child(Icon::from(IconName::Mail).size_10())
                    .child("Pick a chat to read it"),
            );
        };
        let group = conversation.kind == ConversationKind::Group;
        card.child(
            h_flex()
                .justify_between()
                .gap_3()
                .child(
                    div()
                        .flex_1()
                        .min_w_0()
                        .text_lg()
                        .font_weight(FontWeight::BOLD)
                        .child(conversation.title.clone()),
                )
                .when(group, |this| {
                    this.child(
                        Button::new("group-details")
                            .ghost()
                            .small()
                            .icon(IconName::Users)
                            .flex_shrink_0()
                            .label(members_label(conversation.members.len()))
                            .on_click(cx.listener(|view, _, window, cx| {
                                view.open_group(window, cx);
                            })),
                    )
                }),
        )
        .child(div().flex_1().min_h_0().child(chat))
    }
}

/// "1 member" or "3 members".
fn members_label(count: usize) -> String {
    let noun = if count == 1 { "member" } else { "members" };
    format!("{count} {noun}")
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

    /// Ann chats with Bob and is in a group with two unread; once she connects, both get a message.
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
        let (view, cx) =
            cx.add_window_view(|_, cx| ChatsView::new(client, (session, connection), cx));
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
        view.update(cx, |view, cx| {
            view.set_shown(true, cx);
            view.refresh(cx);
        });
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

    #[gpui_kit::test]
    fn an_open_chat_counts_as_unread_while_messages_is_off_screen(cx: &mut TestAppContext) {
        let (view, _, _, cx) = chats(cx);
        view.update(cx, |view, cx| view.refresh(cx));
        wait_until(cx, |cx| {
            view.read_with(cx, |view, _| view.conversations.len() == 2)
        });
        view.update_in(cx, |view, window, cx| {
            let group = view
                .conversations
                .iter()
                .find(|conversation| conversation.id == "g1")
                .cloned()
                .expect("the group");
            view.open(group, window, cx);
        });
        // The server still counts the group's two: the player went to another page.
        view.update(cx, |view, cx| view.refresh(cx));
        wait_until(cx, |cx| view.read_with(cx, |view, _| view.unread() == 2));
        view.update(cx, |view, cx| view.set_shown(true, cx));
        assert_eq!(
            view.read_with(cx, |view, _| view.unread()),
            0,
            "back on Messages, the open chat is read"
        );
    }
}
