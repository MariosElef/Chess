//! Friends, search, who is online, and challenges between friends; each test on its own
//! freshly migrated database.
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

async fn guest(server: &TestServer) -> String {
    let session: Value = server.post("/api/sessions/guest").await.json();
    session["token"].as_str().expect("a token").to_owned()
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

/// The next challenges message with something in it.
async fn challenges(socket: &mut TestWebSocket) -> Value {
    loop {
        let message = next(socket, "challenges").await;
        let empty = message["sent"].as_array().is_some_and(Vec::is_empty)
            && message["received"].as_array().is_some_and(Vec::is_empty);
        if !empty {
            return message;
        }
    }
}

async fn request(server: &TestServer, token: &str, user_id: &str) -> Value {
    let response = server
        .post("/api/social/friends/requests")
        .authorization_bearer(token)
        .json(&json!({ "user_id": user_id }))
        .await;
    response.assert_status(StatusCode::CREATED);
    response.json()
}

async fn friends(server: &TestServer, token: &str) -> Value {
    server
        .get("/api/social/friends")
        .authorization_bearer(token)
        .await
        .json()
}

/// Ann and Bob as friends: their tokens and ids.
async fn friends_ann_and_bob(server: &TestServer) -> ((String, String), (String, String)) {
    let ann = account(server, "ann").await;
    let bob = account(server, "bob").await;
    request(server, &ann.0, &bob.1).await;
    server
        .post(&format!("/api/social/friends/requests/{}/accept", ann.1))
        .authorization_bearer(&bob.0)
        .await
        .assert_status(StatusCode::NO_CONTENT);
    (ann, bob)
}

#[sqlx::test]
async fn a_request_accepted_makes_two_friends(db: Db) {
    let server = server(db);
    let (ann_token, ann_id) = account(&server, "ann").await;
    let (bob_token, bob_id) = account(&server, "bob").await;
    let sent = request(&server, &ann_token, &bob_id).await;
    assert_eq!(sent["relation"], "request_sent");
    assert_eq!(
        friends(&server, &ann_token).await["outgoing"][0]["username"],
        "bob"
    );
    assert_eq!(
        friends(&server, &bob_token).await["incoming"][0]["username"],
        "ann"
    );

    server
        .post(&format!("/api/social/friends/requests/{ann_id}/accept"))
        .authorization_bearer(&bob_token)
        .await
        .assert_status(StatusCode::NO_CONTENT);
    let listed = friends(&server, &ann_token).await;
    assert_eq!(listed["friends"][0]["username"], "bob");
    assert_eq!(listed["friends"][0]["online"], false);
    assert_eq!(listed["incoming"], json!([]));
    assert_eq!(listed["outgoing"], json!([]));
}

#[sqlx::test]
async fn asking_someone_who_asked_you_accepts(db: Db) {
    let server = server(db);
    let (ann_token, ann_id) = account(&server, "ann").await;
    let (bob_token, bob_id) = account(&server, "bob").await;
    request(&server, &ann_token, &bob_id).await;
    let back = request(&server, &bob_token, &ann_id).await;
    assert_eq!(back["relation"], "friends");
}

#[sqlx::test]
async fn unfriending_and_declining_go_back_to_strangers(db: Db) {
    let server = server(db);
    let ((ann_token, _), (bob_token, bob_id)) = friends_ann_and_bob(&server).await;
    server
        .delete(&format!("/api/social/friends/{bob_id}"))
        .authorization_bearer(&ann_token)
        .await
        .assert_status(StatusCode::NO_CONTENT);
    assert_eq!(friends(&server, &bob_token).await["friends"], json!([]));
    let again = request(&server, &ann_token, &bob_id).await;
    assert_eq!(again["relation"], "request_sent", "asking again works");
}

#[sqlx::test]
async fn search_finds_by_part_of_a_name_and_says_where_you_stand(db: Db) {
    let server = server(db);
    let (ann_token, _) = account(&server, "ann").await;
    let (_, bob_id) = account(&server, "bobby").await;
    account(&server, "rob").await;
    guest(&server).await;
    request(&server, &ann_token, &bob_id).await;
    let found: Value = server
        .get("/api/social/search?q=OB")
        .authorization_bearer(&ann_token)
        .await
        .json();
    let names: Vec<&str> = found
        .as_array()
        .expect("a list")
        .iter()
        .map(|user| user["username"].as_str().expect("a name"))
        .collect();
    assert_eq!(names, ["bobby", "rob"]);
    assert_eq!(found[0]["relation"], "request_sent");
    assert_eq!(found[1]["relation"], "none");
    let guests: Value = server
        .get("/api/social/search?q=guest")
        .authorization_bearer(&ann_token)
        .await
        .json();
    assert_eq!(guests[0]["username"], "Guest", "guests are found too");
    let wildcard: Value = server
        .get("/api/social/search?q=%25%25")
        .authorization_bearer(&ann_token)
        .await
        .json();
    assert_eq!(wildcard, json!([]), "% is a letter, not a wildcard");
}

#[sqlx::test]
async fn guests_make_friends_like_anyone(db: Db) {
    let server = server(db);
    let first: Value = server.post("/api/sessions/guest").await.json();
    let second: Value = server.post("/api/sessions/guest").await.json();
    let token = |session: &Value| session["token"].as_str().expect("a token").to_owned();
    let id = |session: &Value| session["user"]["id"].as_str().expect("an id").to_owned();
    request(&server, &token(&first), &id(&second)).await;
    let back = request(&server, &token(&second), &id(&first)).await;
    assert_eq!(back["relation"], "friends");
    let listed = friends(&server, &token(&first)).await;
    assert_eq!(listed["friends"][0]["username"], "Guest2");
}

#[sqlx::test]
async fn a_friend_coming_online_is_heard_and_listed(db: Db) {
    let server = server(db);
    let ((ann_token, _), (bob_token, _)) = friends_ann_and_bob(&server).await;
    let mut ann = connect(&server, &ann_token).await;
    let _bob = connect(&server, &bob_token).await;
    next(&mut ann, "friends_changed").await;
    let listed = friends(&server, &ann_token).await;
    assert_eq!(listed["friends"][0]["online"], true);
}

#[sqlx::test]
async fn a_request_is_heard_at_once(db: Db) {
    let server = server(db);
    let (ann_token, _) = account(&server, "ann").await;
    let (bob_token, bob_id) = account(&server, "bob").await;
    let mut bob = connect(&server, &bob_token).await;
    request(&server, &ann_token, &bob_id).await;
    next(&mut bob, "friends_changed").await;
}

#[sqlx::test]
async fn an_accepted_challenge_starts_the_game_for_both(db: Db) {
    let server = server(db);
    let ((ann_token, ann_id), (bob_token, bob_id)) = friends_ann_and_bob(&server).await;
    let mut ann = connect(&server, &ann_token).await;
    let mut bob = connect(&server, &bob_token).await;
    ann.send_json(&json!({ "type": "challenge", "user_id": bob_id, "initial_s": 300, "increment_s": 3, "rated": true }))
        .await;
    let received = challenges(&mut bob).await;
    let challenge = &received["received"][0];
    assert_eq!(challenge["from"]["id"], ann_id.as_str());
    assert_eq!(challenge["initial_s"], 300);
    assert_eq!(challenges(&mut ann).await["sent"][0]["id"], challenge["id"]);

    bob.send_json(&json!({ "type": "accept_challenge", "challenge_id": challenge["id"] }))
        .await;
    for socket in [&mut ann, &mut bob] {
        let game = next(socket, "game").await["game"].clone();
        assert_eq!(game["rated"], true);
        assert_eq!(game["initial_s"], 300);
        assert_eq!(game["increment_s"], 3);
    }
}

#[sqlx::test]
async fn a_declined_challenge_goes_for_both(db: Db) {
    let server = server(db);
    let ((ann_token, _), (bob_token, bob_id)) = friends_ann_and_bob(&server).await;
    let mut ann = connect(&server, &ann_token).await;
    let mut bob = connect(&server, &bob_token).await;
    ann.send_json(&json!({ "type": "challenge", "user_id": bob_id, "initial_s": 180, "increment_s": 0, "rated": false }))
        .await;
    let id = challenges(&mut bob).await["received"][0]["id"].clone();
    challenges(&mut ann).await;
    bob.send_json(&json!({ "type": "decline_challenge", "challenge_id": id }))
        .await;
    let after = next(&mut ann, "challenges").await;
    assert_eq!(after["sent"], json!([]));
}

#[sqlx::test]
async fn only_friends_who_are_online_can_be_challenged(db: Db) {
    let server = server(db);
    let ((ann_token, _), (_, bob_id)) = friends_ann_and_bob(&server).await;
    let (_, cat_id) = account(&server, "cat").await;
    let mut ann = connect(&server, &ann_token).await;
    ann.send_json(&json!({ "type": "challenge", "user_id": bob_id, "initial_s": 180, "increment_s": 0, "rated": false }))
        .await;
    assert_eq!(next(&mut ann, "error").await["message"], "bob is offline");
    ann.send_json(&json!({ "type": "challenge", "user_id": cat_id, "initial_s": 180, "increment_s": 0, "rated": false }))
        .await;
    assert_eq!(
        next(&mut ann, "error").await["message"],
        "You can only challenge friends"
    );
}

#[sqlx::test]
async fn unfriending_calls_off_a_challenge_between_them(db: Db) {
    let server = server(db);
    let ((ann_token, _), (bob_token, bob_id)) = friends_ann_and_bob(&server).await;
    let mut ann = connect(&server, &ann_token).await;
    let mut bob = connect(&server, &bob_token).await;
    ann.send_json(&json!({ "type": "challenge", "user_id": bob_id, "initial_s": 180, "increment_s": 0, "rated": false }))
        .await;
    challenges(&mut bob).await;
    server
        .delete(&format!("/api/social/friends/{bob_id}"))
        .authorization_bearer(&ann_token)
        .await
        .assert_status(StatusCode::NO_CONTENT);
    let after = next(&mut bob, "challenges").await;
    assert_eq!(after["received"], json!([]));
}

#[sqlx::test]
async fn starting_a_game_calls_off_waiting_challenges(db: Db) {
    let server = server(db);
    let ((ann_token, _), (bob_token, bob_id)) = friends_ann_and_bob(&server).await;
    let (cat_token, _) = account(&server, "cat").await;
    let mut ann = connect(&server, &ann_token).await;
    let mut bob = connect(&server, &bob_token).await;
    let mut cat = connect(&server, &cat_token).await;
    ann.send_json(&json!({ "type": "challenge", "user_id": bob_id, "initial_s": 180, "increment_s": 0, "rated": false }))
        .await;
    challenges(&mut bob).await;
    for socket in [&mut ann, &mut cat] {
        socket
            .send_json(
                &json!({ "type": "seek", "initial_s": 60, "increment_s": 0, "rated": false }),
            )
            .await;
    }
    next(&mut ann, "game").await;
    let after = next(&mut bob, "challenges").await;
    assert_eq!(after["received"], json!([]), "Ann is playing Cat now");
}

#[sqlx::test]
async fn friends_come_with_when_they_were_last_online_and_their_ratings(db: Db) {
    let server = server(db.clone());
    let ((ann_token, _), (bob_token, bob_id)) = friends_ann_and_bob(&server).await;
    let listed = friends(&server, &ann_token).await;
    assert!(
        listed["friends"][0]["away_s"].is_number(),
        "Bob never opened the app, but signed in: his session says when"
    );
    assert_eq!(
        listed["friends"][0]["ratings"],
        json!({}),
        "he has not played"
    );
    assert_eq!(listed["my_ratings"], json!({}));
    let mut ann = connect(&server, &ann_token).await;
    let bob = connect(&server, &bob_token).await;
    next(&mut ann, "friends_changed").await;
    drop(bob);
    next(&mut ann, "friends_changed").await;
    let listed = friends(&server, &ann_token).await;
    assert_eq!(listed["friends"][0]["online"], false);
    assert!(
        listed["friends"][0]["away_s"].is_number(),
        "Bob was just here"
    );
    // The app being open was stored, not just the session's use.
    let bob_id: uuid::Uuid = bob_id.parse().expect("a uuid");
    let stored: bool =
        sqlx::query_scalar("SELECT last_online_at IS NOT NULL FROM account.users WHERE id = $1")
            .bind(bob_id)
            .fetch_one(&db)
            .await
            .expect("Bob's row");
    assert!(stored, "Bob's last time online is kept");
}

#[sqlx::test]
async fn blocking_ends_the_friendship_and_only_the_blocker_can_lift_it(db: Db) {
    let server = server(db);
    let ((ann_token, ann_id), (bob_token, bob_id)) = friends_ann_and_bob(&server).await;
    let block = |token: &str, id: &str| {
        server
            .post(&format!("/api/social/blocks/{id}"))
            .authorization_bearer(token)
    };
    block(&ann_token, &bob_id)
        .await
        .assert_status(StatusCode::NO_CONTENT);
    let anns = friends(&server, &ann_token).await;
    assert_eq!(anns["friends"], json!([]));
    assert_eq!(anns["blocked"][0]["username"], "bob");
    let bobs = friends(&server, &bob_token).await;
    assert_eq!(bobs["friends"], json!([]));
    assert_eq!(bobs["blocked"], json!([]), "Bob is never told");

    let asked = request(&server, &bob_token, &ann_id).await;
    assert_eq!(asked["relation"], "request_sent", "it looks unanswered");
    assert_eq!(friends(&server, &ann_token).await["incoming"], json!([]));
    let mine = request(&server, &ann_token, &bob_id).await;
    assert_eq!(mine["relation"], "blocked", "Ann must unblock first");

    // Bob cannot lift Ann's block.
    server
        .delete(&format!("/api/social/blocks/{ann_id}"))
        .authorization_bearer(&bob_token)
        .await
        .assert_status(StatusCode::NO_CONTENT);
    assert_eq!(
        friends(&server, &ann_token).await["blocked"][0]["username"],
        "bob"
    );

    server
        .delete(&format!("/api/social/blocks/{bob_id}"))
        .authorization_bearer(&ann_token)
        .await
        .assert_status(StatusCode::NO_CONTENT);
    assert_eq!(friends(&server, &ann_token).await["blocked"], json!([]));
    let again = request(&server, &bob_token, &ann_id).await;
    assert_eq!(again["relation"], "request_sent");
    assert_eq!(
        friends(&server, &ann_token).await["incoming"][0]["username"],
        "bob",
        "after unblocking, a request arrives"
    );
}

#[sqlx::test]
async fn signing_out_other_devices_ends_their_sessions_and_sockets(db: Db) {
    let server = server(db);
    let ((desk, _), (bob, _)) = friends_ann_and_bob(&server).await;
    let laptop: Value = server
        .post("/api/sessions")
        .add_header(
            axum::http::header::USER_AGENT,
            axum::http::HeaderValue::from_static("Chess 0.1.0 (macos)"),
        )
        .json(&json!({ "login": "ann", "password": "correct horse" }))
        .await
        .json();
    let laptop = laptop["token"].as_str().expect("a token").to_owned();
    let _laptop_socket = connect(&server, &laptop).await;
    assert_eq!(friends(&server, &bob).await["friends"][0]["online"], true);

    let devices: Value = server
        .get("/api/sessions")
        .authorization_bearer(&desk)
        .await
        .json();
    assert_eq!(devices.as_array().map(Vec::len), Some(2));
    let current: Vec<&Value> = devices
        .as_array()
        .expect("a list")
        .iter()
        .filter(|device| device["current"] == true)
        .collect();
    assert_eq!(current.len(), 1, "the asking device is marked");
    assert_eq!(current[0]["device"], "Unknown device");
    assert_eq!(
        devices[0]["current"], true,
        "asking uses it, so it is the last used"
    );
    assert_eq!(devices[1]["device"], "Chess app on macOS");

    server
        .delete("/api/sessions/others")
        .authorization_bearer(&desk)
        .await
        .assert_status(StatusCode::NO_CONTENT);
    server
        .get("/api/users/me")
        .authorization_bearer(&laptop)
        .await
        .assert_status(StatusCode::UNAUTHORIZED);
    server
        .get("/api/users/me")
        .authorization_bearer(&desk)
        .await
        .assert_status_ok();
    tokio::time::timeout(PATIENCE, async {
        while friends(&server, &bob).await["friends"][0]["online"] == true {
            tokio::task::yield_now().await;
        }
    })
    .await
    .expect("the laptop's socket closes, so Ann goes offline");
}

#[sqlx::test]
async fn signing_out_closes_the_sockets_of_that_session(db: Db) {
    let server = server(db);
    let ((ann, _), (bob, _)) = friends_ann_and_bob(&server).await;
    // Ann's session in a second app, as when two apps share one saved sign-in.
    let _shared = connect(&server, &ann).await;
    assert_eq!(friends(&server, &bob).await["friends"][0]["online"], true);
    server
        .delete("/api/sessions/current")
        .authorization_bearer(&ann)
        .await
        .assert_status_ok();
    tokio::time::timeout(PATIENCE, async {
        while friends(&server, &bob).await["friends"][0]["online"] == true {
            tokio::task::yield_now().await;
        }
    })
    .await
    .expect("the session's socket closes, so Ann goes offline");
}

#[sqlx::test]
async fn signing_out_leaves_the_sockets_of_other_sessions_open(db: Db) {
    let server = server(db);
    let ((ann, _), _) = friends_ann_and_bob(&server).await;
    let laptop: Value = server
        .post("/api/sessions")
        .json(&json!({ "login": "ann", "password": "correct horse" }))
        .await
        .json();
    let laptop = laptop["token"].as_str().expect("a token").to_owned();
    let mut laptop_socket = connect(&server, &laptop).await;
    server
        .delete("/api/sessions/current")
        .authorization_bearer(&ann)
        .await
        .assert_status_ok();
    // The laptop's socket still answers.
    laptop_socket
        .send_json(&json!({ "type": "seek", "initial_s": 180, "increment_s": 2, "rated": false }))
        .await;
    tokio::time::timeout(PATIENCE, async {
        loop {
            let message: Value = laptop_socket.receive_json().await;
            if message["type"] == "seeking" {
                return;
            }
        }
    })
    .await
    .expect("the laptop is still connected");
}

#[sqlx::test]
async fn two_players_can_block_each_other_and_each_lifts_only_their_own(db: Db) {
    let server = server(db);
    let ((ann_token, ann_id), (bob_token, bob_id)) = friends_ann_and_bob(&server).await;
    for (token, id) in [(&ann_token, &bob_id), (&bob_token, &ann_id)] {
        server
            .post(&format!("/api/social/blocks/{id}"))
            .authorization_bearer(token)
            .await
            .assert_status(StatusCode::NO_CONTENT);
    }
    assert_eq!(
        friends(&server, &bob_token).await["blocked"][0]["username"],
        "ann"
    );
    server
        .delete(&format!("/api/social/blocks/{bob_id}"))
        .authorization_bearer(&ann_token)
        .await
        .assert_status(StatusCode::NO_CONTENT);
    assert_eq!(friends(&server, &ann_token).await["blocked"], json!([]));
    assert_eq!(
        friends(&server, &bob_token).await["blocked"][0]["username"],
        "ann",
        "Bob's block stays"
    );
    let asked = request(&server, &ann_token, &bob_id).await;
    assert_eq!(asked["relation"], "request_sent");
    assert_eq!(friends(&server, &bob_token).await["incoming"], json!([]));
}

#[sqlx::test]
async fn a_guest_has_no_history_from_the_last_holder(db: Db) {
    let server = server(db);
    let token = guest(&server).await;
    for path in [
        "/api/games",
        "/api/puzzles/battles",
        "/api/puzzles/attempts",
    ] {
        let listed: Value = server.get(path).authorization_bearer(&token).await.json();
        assert_eq!(listed, json!([]), "{path}");
    }
}

#[sqlx::test]
async fn a_friends_picture_comes_with_them(db: Db) {
    let server = server(db);
    let ((ann_token, _), (bob_token, _)) = friends_ann_and_bob(&server).await;
    let mut png = Vec::new();
    image::RgbImage::from_pixel(40, 40, image::Rgb([10, 120, 200]))
        .write_to(&mut std::io::Cursor::new(&mut png), image::ImageFormat::Png)
        .expect("a png");
    let set: Value = server
        .put("/api/users/me/avatar")
        .authorization_bearer(&bob_token)
        .bytes(png.into())
        .await
        .json();
    let listed = friends(&server, &ann_token).await;
    assert_eq!(listed["friends"][0]["avatar"], set["avatar"]);
}

#[sqlx::test]
async fn a_profile_shows_a_player_but_not_their_email(db: Db) {
    let server = server(db.clone());
    let ((ann, ann_id), (_, bob_id)) = friends_ann_and_bob(&server).await;
    let bob: Value = server
        .get(&format!("/api/users/{bob_id}/profile"))
        .authorization_bearer(&ann)
        .await
        .json();
    assert_eq!(bob["username"], "bob");
    assert_eq!(bob["relation"], "friends");
    assert_eq!(bob["mine"], false);
    assert!(bob["stats"]["modes"].is_array(), "their ratings and record");
    assert!(bob.get("email").is_none(), "an email stays private");
    let mine: Value = server
        .get(&format!("/api/users/{ann_id}/profile"))
        .authorization_bearer(&ann)
        .await
        .json();
    assert_eq!(mine["mine"], true);
    sqlx::query("UPDATE account.users SET deleted_at = now() WHERE username = 'bob'")
        .execute(&db)
        .await
        .expect("closed");
    server
        .get(&format!("/api/users/{bob_id}/profile"))
        .authorization_bearer(&ann)
        .await
        .assert_status_not_found();
}
