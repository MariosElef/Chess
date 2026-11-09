//! Direct chats between friends, group chats and a game's chat; each test on its own freshly
//! migrated database.
use std::time::Duration;

use axum::http::StatusCode;
use axum_test::{TestServer, TestWebSocket};
use chess_backend::{AppState, Db, LichessClient, build_router};
use serde_json::{Value, json};

/// Long enough for any message to arrive; a test waiting longer has failed.
const PATIENCE: Duration = Duration::from_secs(5);

fn server(db: Db) -> TestServer {
    let lichess = LichessClient::new("http://127.0.0.1:9").expect("a client");
    TestServer::builder()
        .http_transport()
        .build(build_router(AppState::assemble(db, lichess)))
}

/// An account signed in: its token and id.
async fn account(server: &TestServer, name: &str) -> (String, String) {
    server
        .post("/api/users")
        .json(&json!({ "email": format!("{name}@example.com"), "username": name, "password": "correct horse" }))
        .await;
    let session: Value = server
        .post("/api/sessions")
        .json(&json!({ "login": name, "password": "correct horse" }))
        .await
        .json();
    (
        session["token"].as_str().expect("a token").to_owned(),
        session["user"]["id"].as_str().expect("an id").to_owned(),
    )
}

async fn befriend(
    server: &TestServer,
    (a_token, a_id): &(String, String),
    (b_token, b_id): &(String, String),
) {
    server
        .post("/api/social/friends/requests")
        .authorization_bearer(a_token)
        .json(&json!({ "user_id": b_id }))
        .await;
    server
        .post(&format!("/api/social/friends/requests/{a_id}/accept"))
        .authorization_bearer(b_token)
        .await
        .assert_status(StatusCode::NO_CONTENT);
}

async fn connect(server: &TestServer, token: &str) -> TestWebSocket {
    server
        .get_websocket("/api/ws")
        .authorization_bearer(token)
        .await
        .into_websocket()
        .await
}

/// The next message of `kind`, skipping any other.
async fn next(socket: &mut TestWebSocket, kind: &str) -> Value {
    tokio::time::timeout(PATIENCE, async {
        loop {
            let message: Value = socket.receive_json().await;
            if message["type"] == kind {
                return message;
            }
        }
    })
    .await
    .unwrap_or_else(|_| panic!("no {kind} message arrived"))
}

async fn open_direct(server: &TestServer, token: &str, other: &str) -> Value {
    let response = server
        .post("/api/chat/direct")
        .authorization_bearer(token)
        .json(&json!({ "user_id": other }))
        .await;
    response.assert_status_ok();
    response.json()
}

async fn send(server: &TestServer, token: &str, conversation: &str, body: &str) -> StatusCode {
    server
        .post(&format!("/api/chat/conversations/{conversation}/messages"))
        .authorization_bearer(token)
        .json(&json!({ "body": body }))
        .await
        .status_code()
}

async fn conversations(server: &TestServer, token: &str) -> Value {
    server
        .get("/api/chat/conversations")
        .authorization_bearer(token)
        .await
        .json()
}

#[sqlx::test]
async fn friends_chat_and_each_message_arrives_at_once(db: Db) {
    let server = server(db);
    let ann = account(&server, "ann").await;
    let bob = account(&server, "bob").await;
    befriend(&server, &ann, &bob).await;
    let mut bob_socket = connect(&server, &bob.0).await;

    let chat = open_direct(&server, &ann.0, &bob.1).await;
    assert_eq!(chat["title"], "bob", "named after the other player");
    let again = open_direct(&server, &bob.0, &ann.1).await;
    assert_eq!(again["id"], chat["id"], "a pair has one conversation");

    let id = chat["id"].as_str().expect("an id");
    assert_eq!(
        send(&server, &ann.0, id, "  hi bob  ").await,
        StatusCode::CREATED
    );
    let arrived = next(&mut bob_socket, "chat_message").await;
    assert_eq!(arrived["message"]["body"], "hi bob");
    assert_eq!(arrived["message"]["sender"]["username"], "ann");

    let listed = conversations(&server, &bob.0).await;
    assert_eq!(listed[0]["unread"], 1);
    assert_eq!(listed[0]["last_message"]["body"], "hi bob");
    server
        .post(&format!("/api/chat/conversations/{id}/read"))
        .authorization_bearer(&bob.0)
        .await
        .assert_status(StatusCode::NO_CONTENT);
    assert_eq!(conversations(&server, &bob.0).await[0]["unread"], 0);

    let page: Value = server
        .get(&format!("/api/chat/conversations/{id}/messages"))
        .authorization_bearer(&bob.0)
        .await
        .json();
    assert_eq!(page.as_array().map(Vec::len), Some(1));
}

#[sqlx::test]
async fn only_friends_chat_and_only_members_read(db: Db) {
    let server = server(db);
    let ann = account(&server, "ann").await;
    let bob = account(&server, "bob").await;
    let cat = account(&server, "cat").await;
    befriend(&server, &ann, &bob).await;
    let refused = server
        .post("/api/chat/direct")
        .authorization_bearer(&ann.0)
        .json(&json!({ "user_id": cat.1 }))
        .await;
    refused.assert_status(StatusCode::FORBIDDEN);
    let chat = open_direct(&server, &ann.0, &bob.1).await;
    let id = chat["id"].as_str().expect("an id");
    assert_eq!(
        send(&server, &cat.0, id, "let me in").await,
        StatusCode::NOT_FOUND
    );
    assert_eq!(
        send(&server, &ann.0, id, "").await,
        StatusCode::UNPROCESSABLE_ENTITY
    );
}

#[sqlx::test]
async fn a_group_is_made_joined_and_left(db: Db) {
    let server = server(db);
    let ann = account(&server, "ann").await;
    let bob = account(&server, "bob").await;
    let cat = account(&server, "cat").await;
    befriend(&server, &ann, &bob).await;
    befriend(&server, &ann, &cat).await;
    let mut cat_socket = connect(&server, &cat.0).await;

    let made = server
        .post("/api/chat/groups")
        .authorization_bearer(&ann.0)
        .json(&json!({ "title": "Club", "member_ids": [bob.1] }))
        .await;
    made.assert_status(StatusCode::CREATED);
    let group: Value = made.json();
    let id = group["id"].as_str().expect("an id");
    assert_eq!(group["title"], "Club");
    assert_eq!(group["members"].as_array().map(Vec::len), Some(2));

    server
        .post(&format!("/api/chat/groups/{id}/members"))
        .authorization_bearer(&ann.0)
        .json(&json!({ "user_id": cat.1 }))
        .await
        .assert_status_ok();
    next(&mut cat_socket, "conversations_changed").await;
    assert_eq!(
        send(&server, &bob.0, id, "hello all").await,
        StatusCode::CREATED
    );
    let arrived = next(&mut cat_socket, "chat_message").await;
    assert_eq!(arrived["message"]["body"], "hello all");

    server
        .delete(&format!("/api/chat/groups/{id}/members/me"))
        .authorization_bearer(&cat.0)
        .await
        .assert_status(StatusCode::NO_CONTENT);
    assert_eq!(
        send(&server, &cat.0, id, "still here?").await,
        StatusCode::NOT_FOUND
    );
    assert_eq!(conversations(&server, &cat.0).await, json!([]));
}

#[sqlx::test]
async fn an_online_game_has_its_own_chat_for_the_two_players(db: Db) {
    let server = server(db);
    let guest = |server: &TestServer| {
        let request = server.post("/api/sessions/guest");
        async move {
            let session: Value = request.await.json();
            session["token"].as_str().expect("a token").to_owned()
        }
    };
    let (a, b) = (guest(&server).await, guest(&server).await);
    let mut first = connect(&server, &a).await;
    let mut second = connect(&server, &b).await;
    for socket in [&mut first, &mut second] {
        socket
            .send_json(
                &json!({ "type": "seek", "initial_s": 180, "increment_s": 0, "rated": false }),
            )
            .await;
    }
    let game = next(&mut first, "game").await["game"].clone();
    let chat = game["chat_id"].as_str().expect("a chat");
    assert_eq!(
        send(&server, &a, chat, "good luck").await,
        StatusCode::CREATED
    );
    let arrived = next(&mut second, "chat_message").await;
    assert_eq!(arrived["message"]["body"], "good luck");
    assert_eq!(
        conversations(&server, &a).await,
        json!([]),
        "a game's chat is with the game, not in the list"
    );
}

#[sqlx::test]
async fn both_players_of_a_game_can_close_their_accounts(db: Db) {
    let server = server(db);
    let ann = account(&server, "ann").await;
    let bob = account(&server, "bob").await;
    let mut ann_socket = connect(&server, &ann.0).await;
    let mut bob_socket = connect(&server, &bob.0).await;
    for socket in [&mut ann_socket, &mut bob_socket] {
        socket
            .send_json(
                &json!({ "type": "seek", "initial_s": 180, "increment_s": 0, "rated": false }),
            )
            .await;
    }
    let game = next(&mut ann_socket, "game").await["game"].clone();
    assert!(game["chat_id"].is_string(), "the game has its chat");
    // The game's chat empties as each leaves; its game still points at it, so it stays.
    for (token, _) in [&ann, &bob] {
        server
            .delete("/api/users/me")
            .authorization_bearer(token)
            .json(&json!({ "password": "correct horse" }))
            .await
            .assert_status(StatusCode::NO_CONTENT);
    }
}

#[sqlx::test]
async fn a_direct_chat_goes_quiet_once_they_are_not_friends(db: Db) {
    let server = server(db);
    let ann = account(&server, "ann").await;
    let bob = account(&server, "bob").await;
    befriend(&server, &ann, &bob).await;
    let chat = open_direct(&server, &ann.0, &bob.1).await;
    let id = chat["id"].as_str().expect("an id");
    assert_eq!(send(&server, &bob.0, id, "hi").await, StatusCode::CREATED);
    server
        .post(&format!("/api/social/blocks/{}", bob.1))
        .authorization_bearer(&ann.0)
        .await
        .assert_status(StatusCode::NO_CONTENT);
    assert_eq!(
        send(&server, &bob.0, id, "hello?").await,
        StatusCode::FORBIDDEN
    );
    assert_eq!(
        send(&server, &ann.0, id, "bye").await,
        StatusCode::FORBIDDEN
    );
}

#[sqlx::test]
async fn only_the_owner_renames_and_removes_and_leaving_passes_it_on(db: Db) {
    let server = server(db);
    let ann = account(&server, "ann").await;
    let bob = account(&server, "bob").await;
    let cat = account(&server, "cat").await;
    befriend(&server, &ann, &bob).await;
    befriend(&server, &ann, &cat).await;
    let group: Value = server
        .post("/api/chat/groups")
        .authorization_bearer(&ann.0)
        .json(&json!({ "title": "club", "member_ids": [bob.1, cat.1] }))
        .await
        .json();
    let id = group["id"].as_str().expect("an id").to_owned();
    assert_eq!(group["owner_id"], ann.1.as_str());
    let rename = |token: &str, title: &str| {
        server
            .patch(&format!("/api/chat/groups/{id}"))
            .authorization_bearer(token)
            .json(&json!({ "title": title }))
    };
    let remove = |token: &str, user: &str| {
        server
            .delete(&format!("/api/chat/groups/{id}/members/{user}"))
            .authorization_bearer(token)
    };

    rename(&bob.0, "bobs")
        .await
        .assert_status(StatusCode::FORBIDDEN);
    remove(&bob.0, &cat.1)
        .await
        .assert_status(StatusCode::FORBIDDEN);
    let renamed: Value = rename(&ann.0, " chess club ").await.json();
    assert_eq!(renamed["title"], "chess club");
    remove(&ann.0, &cat.1)
        .await
        .assert_status(StatusCode::NO_CONTENT);
    assert_eq!(
        conversations(&server, &cat.0).await,
        json!([]),
        "Cat is out"
    );

    server
        .delete(&format!("/api/chat/groups/{id}/members/me"))
        .authorization_bearer(&ann.0)
        .await
        .assert_status(StatusCode::NO_CONTENT);
    let bobs = conversations(&server, &bob.0).await;
    assert_eq!(bobs[0]["owner_id"], bob.1.as_str(), "Bob owns it now");
    assert_eq!(bobs[0]["title"], "chess club");
}

#[sqlx::test]
async fn no_one_joins_a_group_with_someone_who_blocked_them(db: Db) {
    let server = server(db);
    let ann = account(&server, "ann").await;
    let bob = account(&server, "bob").await;
    let cat = account(&server, "cat").await;
    befriend(&server, &ann, &bob).await;
    befriend(&server, &ann, &cat).await;
    server
        .post(&format!("/api/social/blocks/{}", bob.1))
        .authorization_bearer(&cat.0)
        .await
        .assert_status(StatusCode::NO_CONTENT);
    server
        .post("/api/chat/groups")
        .authorization_bearer(&ann.0)
        .json(&json!({ "title": "club", "member_ids": [bob.1, cat.1] }))
        .await
        .assert_status(StatusCode::FORBIDDEN);
    let group: Value = server
        .post("/api/chat/groups")
        .authorization_bearer(&ann.0)
        .json(&json!({ "title": "club", "member_ids": [bob.1] }))
        .await
        .json();
    let id = group["id"].as_str().expect("an id");
    server
        .post(&format!("/api/chat/groups/{id}/members"))
        .authorization_bearer(&ann.0)
        .json(&json!({ "user_id": cat.1 }))
        .await
        .assert_status(StatusCode::FORBIDDEN);
}

#[sqlx::test]
async fn a_block_inside_a_group_still_lets_others_join(db: Db) {
    let server = server(db);
    let ann = account(&server, "ann").await;
    let bob = account(&server, "bob").await;
    let cat = account(&server, "cat").await;
    befriend(&server, &ann, &bob).await;
    befriend(&server, &ann, &cat).await;
    let group: Value = server
        .post("/api/chat/groups")
        .authorization_bearer(&ann.0)
        .json(&json!({ "title": "club", "member_ids": [bob.1] }))
        .await
        .json();
    let id = group["id"].as_str().expect("an id");
    server
        .post(&format!("/api/social/blocks/{}", ann.1))
        .authorization_bearer(&bob.0)
        .await
        .assert_status(StatusCode::NO_CONTENT);
    server
        .post(&format!("/api/chat/groups/{id}/members"))
        .authorization_bearer(&ann.0)
        .json(&json!({ "user_id": cat.1 }))
        .await
        .assert_status_ok();
}
