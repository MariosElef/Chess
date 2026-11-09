mod games;

use axum::{
    Router,
    extract::{
        State,
        ws::{Message, WebSocket, WebSocketUpgrade},
    },
    response::Response,
    routing::get,
};
use uuid::Uuid;

use crate::{
    errors::AppError,
    extract::SignedIn,
    models::play::{
        messages::{ClientMessage, ServerMessage},
        player::Player,
    },
    state::AppState,
};

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/ws", get(connect))
        .merge(games::router())
}

/// The live socket: seeks, moves, resignations and draws go in; games come out.
async fn connect(
    State(state): State<AppState>,
    signed_in: SignedIn,
    upgrade: WebSocketUpgrade,
) -> Result<Response, AppError> {
    let user = state.users.get(signed_in.user_id).await?;
    let player = Player {
        id: user.id,
        username: user.username,
        guest: user.guest,
    };
    let session = signed_in.session_id;
    Ok(upgrade.on_upgrade(move |socket| serve(socket, state, (player, session))))
}

/// Passes the player's messages to the hub and the hub's to the player until either side goes.
async fn serve(mut socket: WebSocket, state: AppState, (player, session): (Player, Uuid)) {
    let hub = state.play.clone();
    let mut connection = hub.connect(&player, session);
    // Signed out elsewhere as this socket opened: closing the others missed it, so it goes now.
    if !state.sessions.is_live(session).await.unwrap_or(false) {
        hub.disconnect(&player, connection.id);
        return;
    }
    loop {
        tokio::select! {
            incoming = socket.recv() => match incoming {
                Some(Ok(Message::Text(text))) => match serde_json::from_str::<ClientMessage>(&text) {
                    Ok(message) => hub.handle(&player, message).await,
                    Err(error) => {
                        let refusal = ServerMessage::Error {
                            message: format!("That message could not be read: {error}"),
                        };
                        if send(&mut socket, &refusal).await.is_err() {
                            break;
                        }
                    }
                },
                Some(Ok(Message::Binary(_) | Message::Ping(_) | Message::Pong(_))) => {}
                Some(Ok(Message::Close(_)) | Err(_)) | None => break,
            },
            outgoing = connection.messages.recv() => match outgoing {
                Some(message) => {
                    if send(&mut socket, &message).await.is_err() {
                        break;
                    }
                }
                None => break,
            },
        }
    }
    hub.disconnect(&player, connection.id);
}

async fn send(socket: &mut WebSocket, message: &ServerMessage) -> Result<(), axum::Error> {
    let text = serde_json::to_string(message).map_err(axum::Error::new)?;
    socket.send(Message::Text(text.into())).await
}
