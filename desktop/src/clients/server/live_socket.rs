use std::time::Duration;

use futures_util::{SinkExt as _, StreamExt as _};
use tokio::{net::TcpStream, sync::mpsc};
use tokio_tungstenite::{
    MaybeTlsStream, WebSocketStream, connect_async,
    tungstenite::{
        Error, Message,
        client::IntoClientRequest as _,
        http::{HeaderValue, StatusCode},
    },
};

use crate::{
    clients::http::HttpRuntime,
    models::play::{ClientMessage, LiveEvent, ServerMessage},
};

/// How long to wait before connecting again after the connection drops.
const RETRY_AFTER: Duration = Duration::from_secs(2);
/// Messages queued each way.
const QUEUE: usize = 64;

type Socket = WebSocketStream<MaybeTlsStream<TcpStream>>;

/// The open line to the server for live play, reconnecting on its own whenever it drops.
pub struct LiveSocket {
    outgoing: mpsc::Sender<ClientMessage>,
}

/// How a conversation over one connection ended.
enum Ended {
    /// The connection dropped: try again.
    Lost,
    /// The app let go of the socket: stop.
    Released,
}

impl LiveSocket {
    /// Connects to `url` (`ws://…/api/ws`) as `token`; the events arrive on the receiver.
    pub fn open(
        http: &HttpRuntime,
        url: String,
        token: String,
    ) -> (Self, mpsc::Receiver<LiveEvent>) {
        let (outgoing, mut outbox) = mpsc::channel(QUEUE);
        let (events, inbox) = mpsc::channel(QUEUE);
        http.spawn(async move { keep_connected(&url, &token, &mut outbox, &events).await });
        (Self { outgoing }, inbox)
    }

    /// Queues `message`; it goes as soon as the connection is up.
    pub fn send(&self, message: ClientMessage) {
        // Only a socket already closed for good refuses it, and nothing is listening then.
        let _ = self.outgoing.try_send(message);
    }
}

/// Connects, talks until the connection drops, waits and connects again, until the app lets go.
async fn keep_connected(
    url: &str,
    token: &str,
    outbox: &mut mpsc::Receiver<ClientMessage>,
    events: &mpsc::Sender<LiveEvent>,
) {
    loop {
        match connect(url, token).await {
            Connecting::Open(socket) => {
                if events.send(LiveEvent::Connected).await.is_err() {
                    return;
                }
                if let Ended::Released = converse(*socket, outbox, events).await {
                    return;
                }
            }
            Connecting::SignedOut => {
                // Nothing to retry: the app hears it and stops using the session.
                let _ = events.send(LiveEvent::SignedOut).await;
                return;
            }
            Connecting::Failed => {}
        }
        if events.send(LiveEvent::Disconnected).await.is_err() || outbox.is_closed() {
            return;
        }
        tokio::time::sleep(RETRY_AFTER).await;
    }
}

/// How an attempt to connect went.
enum Connecting {
    /// Connected: the socket to talk over.
    Open(Box<Socket>),
    /// The server turned the session away: it was signed out.
    SignedOut,
    /// No connection, for now: try again.
    Failed,
}

async fn connect(url: &str, token: &str) -> Connecting {
    let Ok(mut request) = url.into_client_request() else {
        return Connecting::Failed;
    };
    let Ok(bearer) = HeaderValue::from_str(&format!("Bearer {token}")) else {
        return Connecting::Failed;
    };
    request.headers_mut().insert("authorization", bearer);
    match connect_async(request).await {
        Ok((socket, _)) => Connecting::Open(Box::new(socket)),
        Err(Error::Http(response)) if response.status() == StatusCode::UNAUTHORIZED => {
            Connecting::SignedOut
        }
        Err(_) => Connecting::Failed,
    }
}

/// Passes the server's messages to the app and the app's to the server.
async fn converse(
    mut socket: Socket,
    outbox: &mut mpsc::Receiver<ClientMessage>,
    events: &mpsc::Sender<LiveEvent>,
) -> Ended {
    loop {
        tokio::select! {
            incoming = socket.next() => match incoming {
                Some(Ok(Message::Text(text))) => {
                    if let Ok(message) = serde_json::from_str::<ServerMessage>(text.as_str())
                        && events.send(LiveEvent::Message(Box::new(message))).await.is_err()
                    {
                        return Ended::Released;
                    }
                }
                Some(Ok(Message::Binary(_) | Message::Ping(_) | Message::Pong(_) | Message::Frame(_))) => {}
                Some(Ok(Message::Close(_)) | Err(_)) | None => return Ended::Lost,
            },
            outgoing = outbox.recv() => match outgoing {
                Some(message) => {
                    let Ok(text) = serde_json::to_string(&message) else {
                        continue;
                    };
                    if socket.send(Message::text(text)).await.is_err() {
                        return Ended::Lost;
                    }
                }
                None => {
                    // Closing politely; the app is gone either way.
                    let _ = socket.close(None).await;
                    return Ended::Released;
                }
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use axum::{
        Router,
        extract::ws::{Message, WebSocketUpgrade},
        http::{HeaderMap, StatusCode},
        response::IntoResponse as _,
        routing::get,
    };

    use super::LiveSocket;
    use crate::{
        clients::http::HttpRuntime,
        models::play::{ClientMessage, LiveEvent, ServerMessage},
    };

    /// A server that answers a seek with "seeking", and only for the bearer `abc`.
    fn echo_server(http: &HttpRuntime) -> String {
        let app = Router::new().route(
            "/api/ws",
            get(|headers: HeaderMap, upgrade: WebSocketUpgrade| async move {
                let bearer = headers.get("authorization");
                // A session signed out: turned away before the socket opens.
                if bearer.is_some_and(|value| value == "Bearer revoked") {
                    return StatusCode::UNAUTHORIZED.into_response();
                }
                let signed_in = bearer.is_some_and(|value| value == "Bearer abc");
                upgrade.on_upgrade(move |mut socket| async move {
                    while let Some(Ok(Message::Text(_))) = socket.recv().await {
                        let answer = if signed_in {
                            r#"{"type":"seeking","initial_s":180,"increment_s":2,"rated":false}"#
                        } else {
                            r#"{"type":"error","message":"Sign in required"}"#
                        };
                        if socket.send(Message::Text(answer.into())).await.is_err() {
                            return;
                        }
                    }
                })
                .into_response()
            }),
        );
        http.serve(app).replacen("http", "ws", 1)
    }

    #[test]
    fn a_seek_goes_out_and_the_answer_comes_back() {
        let http = HttpRuntime::new().expect("a runtime");
        let url = format!("{}/api/ws", echo_server(&http));
        let (socket, mut events) = LiveSocket::open(&http, url, "abc".to_owned());
        assert_eq!(http.block_on(events.recv()), Some(LiveEvent::Connected));
        socket.send(ClientMessage::Seek {
            initial_s: 180,
            increment_s: 2,
            rated: false,
        });
        let answer = http.block_on(events.recv());
        assert_eq!(
            answer,
            Some(LiveEvent::Message(Box::new(ServerMessage::Seeking {
                initial_s: 180,
                increment_s: 2,
                rated: false
            })))
        );
    }

    #[test]
    fn no_server_is_a_disconnection_tried_again() {
        let http = HttpRuntime::new().expect("a runtime");
        let (_socket, mut events) = LiveSocket::open(
            &http,
            "ws://127.0.0.1:9/api/ws".to_owned(),
            "abc".to_owned(),
        );
        assert_eq!(http.block_on(events.recv()), Some(LiveEvent::Disconnected));
    }

    #[test]
    fn a_session_the_server_turns_away_is_signed_out_and_not_retried() {
        let http = HttpRuntime::new().expect("a runtime");
        let url = format!("{}/api/ws", echo_server(&http));
        let (_socket, mut events) = LiveSocket::open(&http, url, "revoked".to_owned());
        assert_eq!(http.block_on(events.recv()), Some(LiveEvent::SignedOut));
        assert_eq!(http.block_on(events.recv()), None, "it stops trying");
    }
}
