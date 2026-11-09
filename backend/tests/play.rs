//! Live games over the socket, each test on its own freshly migrated database.
use std::time::Duration;

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

async fn guest(server: &TestServer) -> String {
    let session: Value = server.post("/api/sessions/guest").await.json();
    session["token"].as_str().expect("a token").to_owned()
}

async fn account(server: &TestServer, name: &str) -> String {
    server
        .post("/api/users")
        .json(&json!({ "email": format!("{name}@example.com"), "username": name, "password": "correct horse" }))
        .await;
    let session: Value = server
        .post("/api/sessions")
        .json(&json!({ "login": name, "password": "correct horse" }))
        .await
        .json();
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

async fn seek(socket: &mut TestWebSocket, initial_s: u32, rated: bool) {
    socket
        .send_json(
            &json!({ "type": "seek", "initial_s": initial_s, "increment_s": 0, "rated": rated }),
        )
        .await;
}

/// Two players paired: their sockets, white's first, the game's id, and their tokens, white's
/// first.
async fn paired<'a>(
    server: &TestServer,
    tokens: (&'a str, &'a str),
    initial_s: u32,
    rated: bool,
) -> (TestWebSocket, TestWebSocket, String, (&'a str, &'a str)) {
    let mut first = connect(server, tokens.0).await;
    let mut second = connect(server, tokens.1).await;
    seek(&mut first, initial_s, rated).await;
    next(&mut first, "seeking").await;
    seek(&mut second, initial_s, rated).await;
    let game = next(&mut first, "game").await["game"].clone();
    next(&mut second, "game").await;
    let id = game["id"].as_str().expect("an id").to_owned();
    let first_name = whoami(server, tokens.0).await;
    if game["white"]["username"] == first_name {
        (first, second, id, tokens)
    } else {
        (second, first, id, (tokens.1, tokens.0))
    }
}

async fn whoami(server: &TestServer, token: &str) -> String {
    let me: Value = server
        .get("/api/users/me")
        .authorization_bearer(token)
        .await
        .json();
    me["username"].as_str().expect("a name").to_owned()
}

async fn play(socket: &mut TestWebSocket, game_id: &str, uci: &str) {
    socket
        .send_json(&json!({ "type": "move", "game_id": game_id, "uci": uci }))
        .await;
}

#[sqlx::test]
async fn two_players_are_paired_and_see_each_move(db: Db) {
    let server = server(db);
    let (a, b) = (guest(&server).await, guest(&server).await);
    let (mut white, mut black, id, _) = paired(&server, (&a, &b), 180, false).await;
    play(&mut white, &id, "e2e4").await;
    for socket in [&mut white, &mut black] {
        let game = next(socket, "game").await["game"].clone();
        assert_eq!(game["moves"], json!(["e2e4"]));
        assert_eq!(
            game["running"], "black",
            "black's clock runs after white's first move"
        );
        assert_eq!(game["status"], "ongoing");
    }
}

#[sqlx::test]
async fn out_of_turn_and_illegal_moves_are_refused(db: Db) {
    let server = server(db);
    let (a, b) = (guest(&server).await, guest(&server).await);
    let (mut white, mut black, id, _) = paired(&server, (&a, &b), 180, false).await;
    play(&mut black, &id, "e7e5").await;
    assert_eq!(
        next(&mut black, "error").await["message"],
        "It is not your turn"
    );
    play(&mut white, &id, "e2e5").await;
    assert!(
        next(&mut white, "error").await["message"]
            .as_str()
            .expect("a sentence")
            .contains("not a legal move")
    );
}

#[sqlx::test]
async fn resigning_ends_the_game_for_both(db: Db) {
    let server = server(db.clone());
    let (a, b) = (guest(&server).await, guest(&server).await);
    let (mut white, mut black, id, _) = paired(&server, (&a, &b), 180, false).await;
    white
        .send_json(&json!({ "type": "resign", "game_id": id }))
        .await;
    let game = next(&mut black, "game").await["game"].clone();
    assert_eq!(game["status"], "finished");
    assert_eq!(game["result"], "resignation");
    assert_eq!(game["winner"], "black");
    let stored: (String, Option<String>) =
        sqlx::query_as("SELECT status, winner FROM play.games WHERE id = $1::uuid")
            .bind(&id)
            .fetch_one(&db)
            .await
            .expect("the game row");
    assert_eq!(stored, ("finished".to_owned(), Some("black".to_owned())));
}

#[sqlx::test]
async fn a_draw_offered_and_accepted_ends_it_drawn(db: Db) {
    let server = server(db);
    let (a, b) = (guest(&server).await, guest(&server).await);
    let (mut white, mut black, id, _) = paired(&server, (&a, &b), 180, false).await;
    white
        .send_json(&json!({ "type": "offer_draw", "game_id": id }))
        .await;
    assert_eq!(
        next(&mut black, "game").await["game"]["draw_offer"],
        "white"
    );
    black
        .send_json(&json!({ "type": "accept_draw", "game_id": id }))
        .await;
    let game = next(&mut white, "game").await["game"].clone();
    let game = if game["status"] == "finished" {
        game
    } else {
        next(&mut white, "game").await["game"].clone()
    };
    assert_eq!(game["result"], "draw_agreement");
    assert_eq!(game["winner"], Value::Null);
}

#[sqlx::test]
async fn an_instant_reply_still_costs_time(db: Db) {
    let server = server(db);
    let (a, b) = (guest(&server).await, guest(&server).await);
    let (mut white, mut black, id, _) = paired(&server, (&a, &b), 180, false).await;
    play(&mut white, &id, "e2e4").await;
    next(&mut black, "game").await;
    play(&mut black, &id, "e7e5").await;
    let game = next(&mut white, "game").await["game"].clone();
    let game = if game["moves"].as_array().map(Vec::len) == Some(2) {
        game
    } else {
        next(&mut white, "game").await["game"].clone()
    };
    let black_ms = game["black_ms"].as_u64().expect("a clock");
    assert!(
        black_ms <= 179_000,
        "a premove costs at least a second, left {black_ms}"
    );
}

#[sqlx::test]
async fn running_out_of_time_loses(db: Db) {
    let server = server(db);
    let (a, b) = (guest(&server).await, guest(&server).await);
    let (mut white, mut black, id, _) = paired(&server, (&a, &b), 1, false).await;
    play(&mut white, &id, "e2e4").await;
    next(&mut black, "game").await;
    let game = next(&mut black, "game").await["game"].clone();
    assert_eq!(game["result"], "timeout", "black's second ran out");
    assert_eq!(game["winner"], "white");
}

#[sqlx::test]
async fn a_rated_game_moves_both_ratings(db: Db) {
    let server = server(db.clone());
    let (a, b) = (account(&server, "ann").await, account(&server, "bob").await);
    let (mut white, mut black, id, _) = paired(&server, (&a, &b), 180, true).await;
    for (mover, uci) in [(0, "f2f3"), (1, "e7e5"), (0, "g2g4"), (1, "d8h4")] {
        let socket = if mover == 0 { &mut white } else { &mut black };
        play(socket, &id, uci).await;
        next(&mut white, "game").await;
    }
    let game = next(&mut black, "game").await["game"].clone();
    let game = loop_until_finished(&mut black, game).await;
    assert_eq!(game["result"], "checkmate");
    assert_eq!(game["winner"], "black");
    let changes = &game["rating_changes"];
    assert!(changes["black"].as_i64().expect("a change") > 0);
    assert!(changes["white"].as_i64().expect("a change") < 0);
    let rated: i64 =
        sqlx::query_scalar("SELECT count(*) FROM play.rating_history WHERE game_id = $1::uuid")
            .bind(&id)
            .fetch_one(&db)
            .await
            .expect("a count");
    assert_eq!(rated, 2, "one history row per player");
}

async fn loop_until_finished(socket: &mut TestWebSocket, mut game: Value) -> Value {
    while game["status"] != "finished" {
        game = next(socket, "game").await["game"].clone();
    }
    game
}

#[sqlx::test]
async fn a_guest_cannot_seek_a_rated_game(db: Db) {
    let server = server(db);
    let token = guest(&server).await;
    let mut socket = connect(&server, &token).await;
    seek(&mut socket, 180, true).await;
    assert!(
        next(&mut socket, "error").await["message"]
            .as_str()
            .expect("a sentence")
            .starts_with("Guests play unrated")
    );
}

#[sqlx::test]
async fn reconnecting_puts_the_player_back_in_the_game(db: Db) {
    let server = server(db);
    let (a, b) = (guest(&server).await, guest(&server).await);
    let (mut white, mut black, id, (_, black_token)) = paired(&server, (&a, &b), 180, false).await;
    play(&mut white, &id, "d2d4").await;
    next(&mut black, "game").await;
    black.close().await;
    let mut again = connect(&server, black_token).await;
    let game = next(&mut again, "game").await["game"].clone();
    assert_eq!(game["moves"], json!(["d2d4"]));
}

#[sqlx::test]
async fn a_game_called_off_before_both_moved_is_aborted_and_frees_both(db: Db) {
    let server = server(db.clone());
    let (a, b) = (guest(&server).await, guest(&server).await);
    let (mut white, mut black, id, _) = paired(&server, (&a, &b), 180, false).await;
    play(&mut white, &id, "e2e4").await;
    next(&mut black, "game").await;
    black
        .send_json(&json!({ "type": "abort", "game_id": id }))
        .await;
    let mut game = next(&mut white, "game").await["game"].clone();
    while game["status"] == "ongoing" {
        game = next(&mut white, "game").await["game"].clone();
    }
    assert_eq!(game["status"], "aborted");
    assert_eq!(game["result"], Value::Null);
    let status: String = sqlx::query_scalar("SELECT status FROM play.games WHERE id = $1::uuid")
        .bind(&id)
        .fetch_one(&db)
        .await
        .expect("the game row");
    assert_eq!(status, "aborted");
    seek(&mut white, 180, false).await;
    next(&mut white, "seeking").await;
}

#[sqlx::test]
async fn once_both_have_moved_abort_is_refused(db: Db) {
    let server = server(db);
    let (a, b) = (guest(&server).await, guest(&server).await);
    let (mut white, mut black, id, _) = paired(&server, (&a, &b), 180, false).await;
    play(&mut white, &id, "e2e4").await;
    next(&mut black, "game").await;
    play(&mut black, &id, "e7e5").await;
    next(&mut white, "game").await;
    white
        .send_json(&json!({ "type": "abort", "game_id": id }))
        .await;
    assert_eq!(
        next(&mut white, "error").await["message"],
        "Both sides have moved; resign instead"
    );
}

#[sqlx::test]
async fn a_finished_game_is_in_both_histories_with_its_moves(db: Db) {
    let server = server(db);
    let (a, b) = (account(&server, "ann").await, account(&server, "bob").await);
    let (mut white, mut black, id, (white_token, black_token)) =
        paired(&server, (&a, &b), 180, true).await;
    for (mover, uci) in [(0, "f2f3"), (1, "e7e5"), (0, "g2g4"), (1, "d8h4")] {
        let socket = if mover == 0 { &mut white } else { &mut black };
        play(socket, &id, uci).await;
        next(&mut white, "game").await;
    }
    let game = next(&mut black, "game").await["game"].clone();
    let game = loop_until_finished(&mut black, game).await;
    let history = |token: &str| server.get("/api/games").authorization_bearer(token);
    let lost: Value = history(white_token).await.json();
    assert_eq!(lost[0]["id"], id.as_str());
    assert_eq!(lost[0]["result"], "lost");
    assert_eq!(lost[0]["outcome"], "checkmate");
    assert_eq!(lost[0]["color"], "white");
    assert_eq!(lost[0]["plies"], 4);
    assert_eq!(lost[0]["rating_change"], game["rating_changes"]["white"]);
    let won: Value = history(black_token).await.json();
    assert_eq!(won[0]["result"], "won");
    assert_eq!(won[0]["opponent"], whoami(&server, white_token).await);
    let record: Value = server
        .get(&format!("/api/games/{id}"))
        .authorization_bearer(black_token)
        .await
        .json();
    assert_eq!(record["moves"], json!(["f2f3", "e7e5", "g2g4", "d8h4"]));
    let older: Value = server
        .get(&format!("/api/games?before={id}"))
        .authorization_bearer(black_token)
        .await
        .json();
    assert_eq!(older, json!([]), "nothing before the only game");
    let stranger = account(&server, "cat").await;
    server
        .get(&format!("/api/games/{id}"))
        .authorization_bearer(&stranger)
        .await
        .assert_status_not_found();
}
