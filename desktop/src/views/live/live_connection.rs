use std::rc::Rc;

use gpui_kit::*;

use crate::{
    clients::server::{LiveSocket, ServerClient},
    models::{
        account::Session,
        play::{ClientMessage, LiveEvent},
    },
};

/// Reads the signed-in session, for the live socket and for knowing which player is "me".
pub type SessionSource = Rc<dyn Fn(&App) -> Option<Session>>;

/// The one live connection to the server while signed in, shared by online play and the Social
/// page: each listens to its events and sends through it. It reconnects on its own.
pub struct LiveConnection {
    client: ServerClient,
    session: SessionSource,
    socket: Option<LiveSocket>,
    connected: bool,
    _events: Option<Task<()>>,
}

impl EventEmitter<LiveEvent> for LiveConnection {}

impl LiveConnection {
    pub fn new(client: ServerClient, session: SessionSource) -> Self {
        Self {
            client,
            session,
            socket: None,
            connected: false,
            _events: None,
        }
    }

    /// Opens the socket for the signed-in player, unless it is open.
    pub fn connect(&mut self, cx: &mut Context<Self>) {
        if self.socket.is_some() {
            return;
        }
        let Some(session) = (self.session)(cx) else {
            return;
        };
        let (socket, mut events) = self.client.live_socket(&session.token);
        self.socket = Some(socket);
        self._events = Some(cx.spawn(async move |connection, cx| {
            while let Some(event) = events.recv().await {
                let passed_on =
                    connection.update(cx, |connection, cx| connection.pass_on(event, cx));
                if passed_on.is_err() {
                    break;
                }
            }
        }));
    }

    /// The server's API, for the pages listening here that also fetch over HTTP.
    pub fn client(&self) -> ServerClient {
        self.client.clone()
    }

    /// Queues `message`; it goes as soon as the connection is up.
    pub fn send(&self, message: ClientMessage) {
        if let Some(socket) = &self.socket {
            socket.send(message);
        }
    }

    /// Asked to connect, but not connected right now: it is being tried again.
    pub fn is_reconnecting(&self) -> bool {
        self.socket.is_some() && !self.connected
    }

    fn pass_on(&mut self, event: LiveEvent, cx: &mut Context<Self>) {
        match &event {
            LiveEvent::Connected => self.connected = true,
            LiveEvent::Disconnected => self.connected = false,
            LiveEvent::Message(_) => {}
        }
        cx.emit(event);
        cx.notify();
    }
}
