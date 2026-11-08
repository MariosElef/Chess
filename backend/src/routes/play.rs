use axum::{
    Router,
    extract::{
        State,
        ws::{Message, WebSocket, WebSocketUpgrade},
    },
    response::Response,
    routing::get,
};

use crate::{
    errors::AppError,
    extract::SignedIn,
    models::play::{
        messages::{ClientMessage, ServerMessage},
        player::Player,
    },
    services::play::PlayHub,
    state::AppState,
};

pub fn router() -> Router<AppState> {
    Router::new().route("/ws", get(connect))
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
    let hub = state.play.clone();
    Ok(upgrade.on_upgrade(move |socket| serve(socket, hub, player)))
}

/// Passes the player's messages to the hub and the hub's to the player until either side goes.
async fn serve(mut socket: WebSocket, hub: PlayHub, player: Player) {
    let mut connection = hub.connect(&player);
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
