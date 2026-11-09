use gpui_kit::{
    assets::IconName,
    component::{
        ActiveTheme as _, Icon, h_flex,
        input::{Input, InputEvent, InputState},
        v_flex,
    },
    prelude::FluentBuilder as _,
    *,
};

use crate::{
    clients::server::ServerClient,
    components::{
        panel::{START_GREEN, StartButton},
        social::avatar,
    },
    models::{
        chat::ChatMessage,
        play::{LiveEvent, ServerMessage},
        server::ServerError,
    },
    services::{server::error_message, social::face_of},
    views::live::{LiveConnection, SessionSource},
};

/// One conversation: its messages, the player's own on the right, and a field to write in.
pub struct ChatView {
    client: ServerClient,
    session: SessionSource,
    conversation_id: String,
    title: SharedString,
    messages: Vec<ChatMessage>,
    problem: Option<String>,
    input: Entity<InputState>,
    _input_events: Subscription,
    _live: Subscription,
    /// On screen: what arrives is read. Off it, as while another page shows, it stays unread.
    reading: bool,
    /// The history came back: until then, an empty chat is not yet known to be empty.
    loaded: bool,
    /// Whether it draws its own title; a page that titles it already turns this off.
    headed: bool,
    /// Kept at the newest message as messages come; scrolling up reads older ones.
    scroll: ScrollHandle,
}

impl ChatView {
    pub fn new(
        (connection, session): (&Entity<LiveConnection>, SessionSource),
        (conversation_id, title): (String, SharedString),
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let input = cx.new(|cx| InputState::new(window, cx).placeholder("Write a message"));
        let _input_events = cx.subscribe_in(&input, window, send_on_enter);
        let _live = cx.subscribe(connection, |view, _, event: &LiveEvent, cx| {
            if let LiveEvent::Message(message) = event
                && let ServerMessage::ChatMessage { message } = message.as_ref()
            {
                view.arrived(message.clone(), cx);
            }
        });
        let mut view = Self {
            client: connection.read(cx).client(),
            session,
            conversation_id,
            title,
            messages: Vec::new(),
            problem: None,
            input,
            _input_events,
            _live,
            reading: true,
            loaded: false,
            headed: true,
            scroll: ScrollHandle::new(),
        };
        view.load(cx);
        view
    }

    /// Leaves the title to the page around it.
    pub fn without_header(mut self) -> Self {
        self.headed = false;
        self
    }

    /// The conversation was renamed.
    pub fn set_title(&mut self, title: SharedString, cx: &mut Context<Self>) {
        if self.title != title {
            self.title = title;
            cx.notify();
        }
    }

    /// Comes on screen or goes off it; coming back reads what arrived meanwhile.
    pub fn set_reading(&mut self, reading: bool, cx: &mut Context<Self>) {
        let back = reading && !self.reading;
        self.reading = reading;
        if back {
            self.mark_read(cx);
        }
    }

    pub fn conversation_id(&self) -> &str {
        &self.conversation_id
    }

    /// The messages shown, oldest first.
    #[cfg(test)]
    pub fn bodies(&self) -> Vec<String> {
        self.messages
            .iter()
            .map(|message| message.body.clone())
            .collect()
    }

    /// Writes `text` in the field and sends it, as typing and pressing Enter would.
    #[cfg(test)]
    pub fn write_and_send(&mut self, text: &str, window: &mut Window, cx: &mut Context<Self>) {
        self.input
            .update(cx, |input, cx| input.set_value(text.to_owned(), window, cx));
        self.send(window, cx);
    }

    fn token(&self, cx: &App) -> Option<String> {
        (self.session)(cx).map(|session| session.token)
    }

    /// Fetches the newest messages, and marks them read while on screen.
    fn load(&mut self, cx: &mut Context<Self>) {
        let Some(token) = self.token(cx) else {
            return;
        };
        let request = self.client.messages(&token, &self.conversation_id);
        cx.spawn(async move |view, cx| {
            let result = request.await;
            // The view outlives the request unless its page is gone.
            let _ = view.update(cx, |view, cx| {
                match result {
                    Ok(messages) => {
                        view.messages = messages;
                        view.loaded = true;
                        view.scroll.scroll_to_bottom();
                    }
                    Err(error) => view.problem = Some(error_message(&error)),
                }
                if view.reading {
                    view.mark_read(cx);
                }
                cx.notify();
            });
        })
        .detach();
    }

    /// A live message: kept if it is this conversation's, and read while on screen.
    fn arrived(&mut self, message: ChatMessage, cx: &mut Context<Self>) {
        if message.conversation_id != self.conversation_id {
            return;
        }
        self.add(message);
        if self.reading {
            self.mark_read(cx);
        }
        cx.notify();
    }

    /// Adds a message once, whether it came back from sending or over the connection first.
    /// Adds a message once, and brings it into view.
    fn add(&mut self, message: ChatMessage) {
        if !self.messages.iter().any(|kept| kept.id == message.id) {
            self.messages.push(message);
            self.scroll.scroll_to_bottom();
        }
    }

    fn mark_read(&self, cx: &mut Context<Self>) {
        if let Some(token) = self.token(cx) {
            // Only the unread count depends on it; a failure leaves it as it was.
            cx.background_spawn(self.client.mark_read(&token, &self.conversation_id))
                .detach();
        }
    }

    fn send(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let body = self.input.read(cx).value().trim().to_owned();
        let Some(token) = self.token(cx).filter(|_| !body.is_empty()) else {
            return;
        };
        let request = self
            .client
            .send_message(&token, &self.conversation_id, &body);
        self.input
            .update(cx, |input, cx| input.set_value("", window, cx));
        cx.spawn(async move |view, cx| {
            let result = request.await;
            let _ = view.update(cx, |view, cx| view.sent(result, cx));
        })
        .detach();
    }

    fn sent(&mut self, result: Result<ChatMessage, ServerError>, cx: &mut Context<Self>) {
        match result {
            Ok(message) => {
                self.problem = None;
                self.add(message);
            }
            Err(error) => self.problem = Some(error_message(&error)),
        }
        cx.notify();
    }

    fn bubble(&self, message: &ChatMessage, me: Option<&str>, cx: &App) -> Div {
        let mine = me == Some(message.sender.id.as_str());
        let theme = cx.theme();
        let bubble = v_flex()
            .max_w(relative(0.75))
            .px_3()
            .py_1p5()
            .rounded(theme.radius)
            // Theirs stand out from the muted card the chat sits on.
            .when(!mine, |this| {
                this.bg(theme.background).child(
                    div()
                        .text_xs()
                        .text_color(theme.muted_foreground)
                        .child(message.sender.username.clone()),
                )
            })
            .when(mine, |this| this.bg(rgb(START_GREEN)).text_color(white()))
            .child(message.body.clone());
        let sender = &message.sender;
        let face = face_of(&sender.id, &sender.username, sender.avatar.as_deref());
        let picture = avatar(&face.name, face.picture.as_deref(), px(28.), cx);
        // Each message beside its sender's picture: theirs on the left, the player's on the right.
        let row = h_flex().w_full().items_end().gap_2();
        if mine {
            row.justify_end().child(bubble).child(picture)
        } else {
            row.child(picture).child(bubble)
        }
    }
}

impl ChatView {
    /// The chat's own title, with its icon.
    fn header(&self, cx: &App) -> Div {
        h_flex()
            .gap_2()
            .text_sm()
            .font_weight(FontWeight::SEMIBOLD)
            .text_color(cx.theme().muted_foreground)
            .child(Icon::from(IconName::Mail).size_4())
            .child(self.title.clone())
    }

    /// The messages, newest at the bottom, or a word that there are none yet.
    fn messages(&self, cx: &App) -> Stateful<Div> {
        let me = (self.session)(cx).map(|session| session.user.id);
        // Newest first in a column drawn bottom up: the latest message sits at the bottom.
        let bubbles: Vec<Div> = self
            .messages
            .iter()
            .rev()
            .map(|message| self.bubble(message, me.as_deref(), cx))
            .collect();
        let none_yet = self.loaded && bubbles.is_empty();
        div()
            .id("chat-messages")
            .flex_1()
            .min_h_0()
            .overflow_y_scroll()
            .track_scroll(&self.scroll)
            .when(none_yet, |this| {
                this.flex()
                    .items_center()
                    .justify_center()
                    .text_sm()
                    .text_color(cx.theme().muted_foreground)
                    .child("No messages yet")
            })
            .child(v_flex().flex_col_reverse().gap_1p5().children(bubbles))
    }

    /// The field to write in and the Send button.
    fn compose_row(&self, cx: &mut Context<Self>) -> Div {
        let view = cx.entity().downgrade();
        h_flex()
            .gap_2()
            // A click here is the field's: the page under it must not take the focus back.
            .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
            .child(div().flex_1().child(Input::new(&self.input)))
            .child(
                StartButton::new("send-message", "Send", move |window, cx| {
                    // The chat outlives its button unless its page is gone.
                    let _ = view.update(cx, |view, cx| view.send(window, cx));
                })
                .small(),
            )
    }
}

impl Render for ChatView {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .size_full()
            .gap_2()
            .when(self.headed, |this| this.child(self.header(cx)))
            .child(self.messages(cx))
            .when_some(self.problem.clone(), |this, problem| {
                this.child(div().text_xs().text_color(cx.theme().danger).child(problem))
            })
            .child(self.compose_row(cx))
    }
}

/// Enter sends what is written.
fn send_on_enter(
    view: &mut ChatView,
    _: &Entity<InputState>,
    event: &InputEvent,
    window: &mut Window,
    cx: &mut Context<ChatView>,
) {
    if let InputEvent::PressEnter { .. } = event {
        view.send(window, cx);
    }
}
