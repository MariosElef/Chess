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
        .json(&json!({
            "email": format!("{name}@example.com"),
            "username": name,
            "password": "correct horse",
        }))
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

/// The next message of any of `kinds`.
async fn next_of(socket: &mut TestWebSocket, kinds: &[&str]) -> Value {
    tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            let message: Value = socket.receive_json().await;
            if kinds.iter().any(|kind| message["type"] == *kind) {
                return message;
            }
        }
    })
    .await
    .unwrap_or_else(|_| panic!("none of {kinds:?} arrived"))
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

/// The account `token` signs in as, as the server describes it.
async fn me(server: &TestServer, token: &str) -> Value {
    server
        .get("/api/users/me")
        .authorization_bearer(token)
        .await
        .json()
}

async fn whoami(server: &TestServer, token: &str) -> String {
    me(server, token).await["username"]
        .as_str()
        .expect("a name")
        .to_owned()
}

/// An account's id.
async fn user_id(server: &TestServer, token: &str) -> String {
    me(server, token).await["id"]
        .as_str()
        .expect("an id")
        .to_owned()
}

async fn play(socket: &mut TestWebSocket, game_id: &str, uci: &str) {
    socket
        .send_json(&json!({ "type": "move", "game_id": game_id, "uci": uci }))
        .await;
}

/// Two accounts who just played: White resigned. Their sockets, the game, and their tokens.
async fn finished<'a>(
    server: &TestServer,
    tokens: (&'a str, &'a str),
) -> (TestWebSocket, TestWebSocket, String, (&'a str, &'a str)) {
    let (mut white, mut black, id, tokens) = paired(server, tokens, 180, false).await;
    white
        .send_json(&json!({ "type": "resign", "game_id": id }))
        .await;
    next(&mut black, "game").await;
    next(&mut white, "game").await;
    (white, black, id, tokens)
}

/// Asks for a rematch of `game` over `socket`.
async fn ask_rematch(socket: &mut TestWebSocket, game: &str) {
    socket
        .send_json(&json!({ "type": "offer_rematch", "game_id": game }))
        .await;
}

/// The next challenges message whose `side` list holds a rematch (`wanted`) or holds none.
async fn rematches(socket: &mut TestWebSocket, side: &str, wanted: bool) -> Value {
    loop {
        let challenges = next(socket, "challenges").await;
        let has = challenges[side].as_array().is_some_and(|list| {
            list.iter()
                .any(|challenge| challenge["rematch_of"].is_string())
        });
        if has == wanted {
            return challenges;
        }
    }
}

/// The first rematch received, from the next challenges message that has one.
async fn received_rematch(socket: &mut TestWebSocket) -> Value {
    rematches(socket, "received", true).await["received"][0].clone()
}

/// The kinds of every message `socket` hears over the next second.
async fn kinds_heard(socket: &mut TestWebSocket) -> Vec<String> {
    let mut kinds = Vec::new();
    let listening = async {
        loop {
            let message: Value = socket.receive_json().await;
            kinds.push(message["type"].as_str().unwrap_or_default().to_owned());
        }
    };
    let _ = tokio::time::timeout(Duration::from_secs(1), listening).await;
    kinds
}

/// Makes the two accounts friends; their ids.
async fn befriend(server: &TestServer, (first, second): (&str, &str)) -> (String, String) {
    let (first_id, second_id) = (user_id(server, first).await, user_id(server, second).await);
    server
        .post("/api/social/friends/requests")
        .authorization_bearer(first)
        .json(&json!({ "user_id": second_id }))
        .await;
    server
        .post(&format!("/api/social/friends/requests/{first_id}/accept"))
        .authorization_bearer(second)
        .await;
    (first_id, second_id)
}

/// A friend's challenge to `user_id`: five minutes, unrated.
fn challenge_to(user_id: &str) -> Value {
    json!({
        "type": "challenge",
        "user_id": user_id,
        "initial_s": 300,
        "increment_s": 0,
        "rated": false,
    })
}

/// The next challenges message whose received list has `count` challenges.
async fn received_count(socket: &mut TestWebSocket, count: usize) -> Value {
    loop {
        let challenges = next(socket, "challenges").await;
        if challenges["received"].as_array().map(Vec::len) == Some(count) {
            return challenges;
        }
    }
}

/// The next game other than `old`, as `socket` hears it.
async fn new_game(socket: &mut TestWebSocket, old: &str) -> Value {
    loop {
        let game = next(socket, "game").await["game"].clone();
        if game["id"] != old {
            return game;
        }
    }
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

#[sqlx::test]
async fn a_rematch_asked_by_both_starts_with_colours_swapped(db: Db) {
    let server = server(db);
    let (a, b) = (account(&server, "ann").await, account(&server, "bob").await);
    let (mut white, mut black, id, (white_token, black_token)) = finished(&server, (&a, &b)).await;
    ask_rematch(&mut white, &id).await;
    let offer = received_rematch(&mut black).await;
    assert_eq!(
        offer["rematch_of"],
        id.as_str(),
        "not a friend's challenge, a rematch"
    );
    assert_eq!(offer["initial_s"], 180);
    ask_rematch(&mut black, &id).await;
    let again = new_game(&mut white, &id).await;
    assert_eq!(
        again["white"]["username"],
        whoami(&server, black_token).await,
        "Black plays White this time"
    );
    assert_eq!(
        again["black"]["username"],
        whoami(&server, white_token).await
    );
}

#[sqlx::test]
async fn a_rematch_is_refused_for_a_game_still_going(db: Db) {
    let server = server(db);
    let (a, b) = (account(&server, "ann").await, account(&server, "bob").await);
    let (mut white, _black, id, _) = paired(&server, (&a, &b), 180, false).await;
    ask_rematch(&mut white, &id).await;
    let refused = next(&mut white, "rematch_refused").await;
    assert_eq!(
        refused["game_id"],
        id.as_str(),
        "said of that rematch, not as a stray error"
    );
    assert_eq!(
        refused["message"],
        "That game can no longer be played again"
    );
}

#[sqlx::test]
async fn a_rematch_is_refused_for_a_game_the_player_was_not_in(db: Db) {
    let server = server(db);
    let (a, b) = (account(&server, "ann").await, account(&server, "bob").await);
    let (_white, _black, id, _) = finished(&server, (&a, &b)).await;
    let cat = account(&server, "cat").await;
    let mut stranger = connect(&server, &cat).await;
    ask_rematch(&mut stranger, &id).await;
    let refused = next(&mut stranger, "rematch_refused").await;
    assert_eq!(
        refused["message"],
        "That game can no longer be played again"
    );
}

#[sqlx::test]
async fn rematches_sent_at_the_same_moment_start_one_game(db: Db) {
    let server = server(db);
    let (a, b) = (account(&server, "ann").await, account(&server, "bob").await);
    let (mut white, mut black, id, _) = finished(&server, (&a, &b)).await;
    ask_rematch(&mut white, &id).await;
    ask_rematch(&mut black, &id).await;
    for socket in [&mut white, &mut black] {
        let again = new_game(socket, &id).await;
        assert_eq!(
            again["status"], "ongoing",
            "one new game, not two waiting offers"
        );
    }
}

#[sqlx::test]
async fn a_rematch_accepted_from_the_toast_starts_between_strangers(db: Db) {
    let server = server(db);
    let (a, b) = (account(&server, "ann").await, account(&server, "bob").await);
    let (mut white, mut black, id, (_, black_token)) = finished(&server, (&a, &b)).await;
    ask_rematch(&mut white, &id).await;
    let offer = received_rematch(&mut black).await;
    assert_eq!(offer["wait_s"], 60);
    assert!(
        offer["seconds_left"]
            .as_u64()
            .is_some_and(|left| left <= 60)
    );
    black
        .send_json(&json!({ "type": "accept_rematch", "game_id": id }))
        .await;
    let again = new_game(&mut black, &id).await;
    assert_eq!(
        again["white"]["username"],
        whoami(&server, black_token).await
    );
}

#[sqlx::test]
async fn a_declined_rematch_goes_for_both(db: Db) {
    let server = server(db);
    let (a, b) = (account(&server, "ann").await, account(&server, "bob").await);
    let (mut white, mut black, id, _) = finished(&server, (&a, &b)).await;
    ask_rematch(&mut white, &id).await;
    let offer = received_rematch(&mut black).await;
    black
        .send_json(&json!({ "type": "decline_challenge", "challenge_id": offer["id"] }))
        .await;
    rematches(&mut white, "sent", false).await;
}

#[sqlx::test]
async fn a_rematch_withdrawn_by_its_game_goes_for_both(db: Db) {
    let server = server(db);
    let (a, b) = (account(&server, "ann").await, account(&server, "bob").await);
    let (mut white, mut black, id, _) = finished(&server, (&a, &b)).await;
    ask_rematch(&mut white, &id).await;
    received_rematch(&mut black).await;
    white
        .send_json(&json!({ "type": "cancel_rematch", "game_id": id }))
        .await;
    rematches(&mut black, "received", false).await;
}

#[sqlx::test]
async fn asking_again_keeps_the_rematch_already_waiting(db: Db) {
    let server = server(db);
    let (a, b) = (account(&server, "ann").await, account(&server, "bob").await);
    let (mut white, mut black, id, _) = finished(&server, (&a, &b)).await;
    ask_rematch(&mut white, &id).await;
    let first = received_rematch(&mut black).await;
    // As another device of the player's asking too: the rematch waiting stands.
    ask_rematch(&mut white, &id).await;
    let told = next(&mut white, "rematch_refused").await;
    assert_eq!(
        told["message"],
        "You asked for this rematch on another device"
    );
    let sent = rematches(&mut white, "sent", true).await;
    assert_eq!(
        sent["sent"][0]["id"], first["id"],
        "not withdrawn and asked anew"
    );
}

#[sqlx::test]
async fn a_rematch_is_stored_with_its_game_and_fixed_colours(db: Db) {
    let server = server(db.clone());
    let (a, b) = (account(&server, "ann").await, account(&server, "bob").await);
    let (mut white, mut black, id, _) = finished(&server, (&a, &b)).await;
    ask_rematch(&mut white, &id).await;
    let offer = received_rematch(&mut black).await;
    let stored: (Option<String>, String) = sqlx::query_as(
        "SELECT rematch_of::text, challenger_colour FROM play.challenges WHERE id = $1::uuid",
    )
    .bind(offer["id"].as_str().expect("an id"))
    .fetch_one(&db)
    .await
    .expect("the record");
    assert_eq!(
        stored,
        (Some(id.clone()), "black".to_owned()),
        "White asks, and plays Black"
    );
}

#[sqlx::test]
async fn the_time_a_rematch_has_left_counts_down(db: Db) {
    // The sleep below is a floor, so more than a second has always gone by after it.
    let server = server(db);
    let (a, b) = (account(&server, "ann").await, account(&server, "bob").await);
    let (mut white, mut black, id, _) = finished(&server, (&a, &b)).await;
    ask_rematch(&mut white, &id).await;
    let first = received_rematch(&mut black).await;
    // White's own list from the posting is read first, so the next one is the later list.
    rematches(&mut white, "sent", true).await;
    tokio::time::sleep(Duration::from_millis(1100)).await;
    // Any change to the lists sends them again, with the time left as it is then.
    ask_rematch(&mut white, &id).await;
    let later = rematches(&mut white, "sent", true).await["sent"][0].clone();
    let left = |offer: &Value| offer["seconds_left"].as_u64().expect("seconds left");
    assert!(
        left(&later) < left(&first),
        "{} then {}",
        left(&first),
        left(&later)
    );
}

#[sqlx::test]
async fn a_rematch_is_refused_once_ten_minutes_have_gone(db: Db) {
    let server = server(db.clone());
    let (a, b) = (account(&server, "ann").await, account(&server, "bob").await);
    let (mut white, _black, id, _) = finished(&server, (&a, &b)).await;
    sqlx::query(
        "UPDATE play.games SET finished_at = now() - interval '11 minutes' WHERE id = $1::uuid",
    )
    .bind(&id)
    .execute(&db)
    .await
    .expect("the game aged");
    ask_rematch(&mut white, &id).await;
    let refused = next(&mut white, "rematch_refused").await;
    assert_eq!(
        refused["message"],
        "That game can no longer be played again"
    );
}

#[sqlx::test]
async fn a_rematch_never_reaches_whoever_has_the_account_now(db: Db) {
    let server = server(db.clone());
    let (a, b) = (account(&server, "ann").await, guest(&server).await);
    let (mut white, mut black, id, (white_token, black_token)) = finished(&server, (&a, &b)).await;
    // The guest's sessions now all began after the game, as one handed to someone new.
    let (asker, other) = if white_token == a {
        (&mut white, black_token)
    } else {
        (&mut black, white_token)
    };
    let other_id = user_id(&server, other).await;
    sqlx::query("UPDATE account.sessions SET created_at = now() WHERE user_id = $1::uuid")
        .bind(&other_id)
        .execute(&db)
        .await
        .expect("sessions renewed");
    ask_rematch(asker, &id).await;
    let refused = next(asker, "rematch_refused").await;
    let said = refused["message"].as_str().expect("a message");
    assert!(said.ends_with("is not taking a rematch"), "{said}");
}

#[sqlx::test]
async fn a_block_refuses_a_rematch_and_drops_one_waiting(db: Db) {
    let server = server(db);
    let (a, b) = (account(&server, "ann").await, account(&server, "bob").await);
    let (mut white, mut black, id, (white_token, black_token)) = finished(&server, (&a, &b)).await;
    ask_rematch(&mut white, &id).await;
    received_rematch(&mut black).await;
    let white_id = user_id(&server, white_token).await;
    server
        .post(&format!("/api/social/blocks/{white_id}"))
        .authorization_bearer(black_token)
        .await;
    rematches(&mut black, "received", false).await;
    ask_rematch(&mut white, &id).await;
    let refused = next(&mut white, "rematch_refused").await;
    let said = refused["message"].as_str().expect("a message");
    assert!(said.ends_with("is not taking a rematch"), "{said}");
}

#[sqlx::test]
async fn unfriending_and_a_friends_challenge_leave_a_rematch_waiting(db: Db) {
    let server = server(db);
    let (a, b) = (account(&server, "ann").await, account(&server, "bob").await);
    let (mut white, mut black, id, (white_token, black_token)) = finished(&server, (&a, &b)).await;
    let (white_id, black_id) = befriend(&server, (white_token, black_token)).await;
    ask_rematch(&mut white, &id).await;
    received_rematch(&mut black).await;
    // A friend's challenge from the same player does not withdraw their rematch.
    white.send_json(&challenge_to(&black_id)).await;
    received_count(&mut black, 2).await;
    server
        .delete(&format!("/api/social/friends/{white_id}"))
        .authorization_bearer(black_token)
        .await;
    let after = received_count(&mut black, 1).await;
    assert_eq!(
        after["received"][0]["rematch_of"],
        id.as_str(),
        "the rematch stays"
    );
}

#[sqlx::test]
async fn a_player_in_another_game_is_refused_a_rematch(db: Db) {
    let server = server(db);
    let (a, b, c) = (
        account(&server, "ann").await,
        account(&server, "bob").await,
        account(&server, "cat").await,
    );
    // The first game's sockets stay open: its opponent is still online and free.
    let (_white, _black, id, (white_token, _)) = finished(&server, (&a, &b)).await;
    // White sits down with Cat, then asks to replay the first game.
    let (first, second, _, (first_token, _)) = paired(&server, (white_token, &c), 300, false).await;
    // `paired` hands the sockets back White first: the first game's player may be either.
    let (mut busy, _cat) = if first_token == white_token {
        (first, second)
    } else {
        (second, first)
    };
    ask_rematch(&mut busy, &id).await;
    let refused = next(&mut busy, "rematch_refused").await;
    assert_eq!(refused["message"], "Finish your game first");
}

#[sqlx::test]
async fn a_rematch_leaves_a_friends_challenge_and_a_new_challenge_replaces_the_old(db: Db) {
    let server = server(db);
    let (a, b) = (account(&server, "ann").await, account(&server, "bob").await);
    let (mut white, mut black, id, (white_token, black_token)) = finished(&server, (&a, &b)).await;
    let (_, black_id) = befriend(&server, (white_token, black_token)).await;
    white.send_json(&challenge_to(&black_id)).await;
    let first = received_count(&mut black, 1).await["received"][0]["id"].clone();
    ask_rematch(&mut white, &id).await;
    received_count(&mut black, 2).await;
    white.send_json(&challenge_to(&black_id)).await;
    let mut ids = Vec::new();
    while ids.len() != 2 || ids.contains(&first) {
        let challenges = received_count(&mut black, 2).await;
        ids = challenges["received"]
            .as_array()
            .expect("a list")
            .iter()
            .map(|challenge| challenge["id"].clone())
            .collect();
    }
}

#[sqlx::test]
async fn only_the_player_who_asked_can_withdraw_a_rematch(db: Db) {
    let server = server(db);
    let (a, b) = (account(&server, "ann").await, account(&server, "bob").await);
    let (mut white, mut black, id, _) = finished(&server, (&a, &b)).await;
    ask_rematch(&mut white, &id).await;
    received_rematch(&mut black).await;
    black
        .send_json(&json!({ "type": "cancel_rematch", "game_id": id }))
        .await;
    // White's list from the posting, then the one asking again resends: it is still there.
    ask_rematch(&mut white, &id).await;
    rematches(&mut white, "sent", true).await;
    rematches(&mut white, "sent", true).await;
}

#[sqlx::test]
async fn accepting_a_rematch_that_has_gone_is_refused_as_that_games(db: Db) {
    let server = server(db);
    let (a, b) = (account(&server, "ann").await, account(&server, "bob").await);
    let (mut white, mut black, id, _) = finished(&server, (&a, &b)).await;
    ask_rematch(&mut white, &id).await;
    received_rematch(&mut black).await;
    white
        .send_json(&json!({ "type": "cancel_rematch", "game_id": id }))
        .await;
    rematches(&mut black, "received", false).await;
    black
        .send_json(&json!({ "type": "accept_rematch", "game_id": id }))
        .await;
    let refused = next(&mut black, "rematch_refused").await;
    assert_eq!(refused["game_id"], id.as_str());
    assert_eq!(refused["message"], "That rematch is no longer waiting");
    // Not turned into a rematch of Black's own.
    rematches(&mut white, "received", false).await;
}

#[sqlx::test]
async fn two_challenges_accepted_at_once_seat_a_player_only_once(db: Db) {
    let server = server(db.clone());
    let (a, b, c) = (
        account(&server, "ann").await,
        account(&server, "bob").await,
        account(&server, "cat").await,
    );
    let (ann_id, bob_id) = befriend(&server, (&a, &b)).await;
    let (_, cat_id) = befriend(&server, (&a, &c)).await;
    let (mut bob, mut cat) = (connect(&server, &b).await, connect(&server, &c).await);
    // Ann on two Macs, each with both challenges.
    let (mut mac, mut imac) = (connect(&server, &a).await, connect(&server, &a).await);
    bob.send_json(&challenge_to(&ann_id)).await;
    cat.send_json(&challenge_to(&ann_id)).await;
    let waiting = received_count(&mut imac, 2).await;
    received_count(&mut mac, 2).await;
    let from = |name_id: &str| {
        waiting["received"]
            .as_array()
            .expect("a list")
            .iter()
            .find(|challenge| challenge["from"]["id"] == name_id)
            .expect("their challenge")["id"]
            .clone()
    };
    let accept =
        |challenge_id: Value| json!({ "type": "accept_challenge", "challenge_id": challenge_id });
    mac.send_json(&accept(from(&bob_id))).await;
    imac.send_json(&accept(from(&cat_id))).await;
    // However the two raced, one accept was refused, on the Mac that sent it, and one game began.
    let (on_mac, on_imac) = tokio::join!(kinds_heard(&mut mac), kinds_heard(&mut imac));
    let errors = [&on_mac, &on_imac]
        .iter()
        .map(|kinds| {
            kinds
                .iter()
                .filter(|kind| *kind == "invite_refused")
                .count()
        })
        .sum::<usize>();
    assert_eq!(errors, 1, "{on_mac:?} {on_imac:?}");
    let games: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM play.games WHERE white_id = $1::uuid OR black_id = $1::uuid",
    )
    .bind(&ann_id)
    .fetch_one(&db)
    .await
    .expect("a count");
    assert_eq!(games, 1);
}

#[sqlx::test]
async fn a_rematch_of_an_opponent_gone_offline_is_refused(db: Db) {
    let server = server(db);
    let (a, b) = (account(&server, "ann").await, account(&server, "bob").await);
    let (mut white, black, id, _) = finished(&server, (&a, &b)).await;
    drop(black);
    // The server hears the socket close on its own time: until then the rematch is posted.
    loop {
        ask_rematch(&mut white, &id).await;
        let answer = next_of(&mut white, &["rematch_refused", "challenges"]).await;
        if answer["type"] == "rematch_refused" {
            let said = answer["message"].as_str().expect("a message");
            assert!(said.ends_with("is not taking a rematch"), "{said}");
            break;
        }
        white
            .send_json(&json!({ "type": "cancel_rematch", "game_id": id }))
            .await;
        rematches(&mut white, "sent", false).await;
    }
}

#[sqlx::test]
async fn the_players_pictures_reach_the_board(db: Db) {
    let server = server(db.clone());
    let (a, b) = (account(&server, "ann").await, account(&server, "bob").await);
    let ann_id = user_id(&server, &a).await;
    sqlx::query("UPDATE account.users SET avatar_hash = 'h1' WHERE id = $1::uuid")
        .bind(&ann_id)
        .execute(&db)
        .await
        .expect("a picture");
    let (_white, _black, _, _) = paired(&server, (&a, &b), 180, false).await;
    // A new socket is sent the game going on.
    let mut again = connect(&server, &a).await;
    let game = next(&mut again, "game").await["game"].clone();
    let ann = [&game["white"], &game["black"]]
        .into_iter()
        .find(|side| side["id"] == ann_id.as_str())
        .expect("Ann at the board");
    assert_eq!(ann["avatar"], "h1");
}

#[sqlx::test]
async fn a_new_rematch_replaces_the_senders_rematch_of_another_game(db: Db) {
    let server = server(db);
    let (a, b, c) = (
        account(&server, "ann").await,
        account(&server, "bob").await,
        account(&server, "cat").await,
    );
    let (mut first_white, mut first_black, first, (first_token, _)) =
        finished(&server, (&a, &b)).await;
    let (mut second_white, mut second_black, second, (second_token, _)) =
        finished(&server, (&a, &c)).await;
    // Ann's socket in each game, whichever colour she had.
    let (ann_first, bob) = if first_token == a {
        (&mut first_white, &mut first_black)
    } else {
        (&mut first_black, &mut first_white)
    };
    ask_rematch(ann_first, &first).await;
    received_rematch(bob).await;
    let (ann_second, _cat) = if second_token == a {
        (&mut second_white, &mut second_black)
    } else {
        (&mut second_black, &mut second_white)
    };
    ask_rematch(ann_second, &second).await;
    rematches(bob, "received", false).await;
}

#[sqlx::test]
async fn a_player_signed_in_again_since_the_game_is_told_to_start_a_new_one(db: Db) {
    let server = server(db.clone());
    let (a, b) = (account(&server, "ann").await, account(&server, "bob").await);
    let (mut white, _black, id, (white_token, _)) = finished(&server, (&a, &b)).await;
    let white_id = user_id(&server, white_token).await;
    sqlx::query("UPDATE account.sessions SET created_at = now() WHERE user_id = $1::uuid")
        .bind(&white_id)
        .execute(&db)
        .await
        .expect("sessions renewed");
    ask_rematch(&mut white, &id).await;
    let refused = next(&mut white, "rematch_refused").await;
    assert_eq!(
        refused["message"],
        "You signed in again since that game: start a new one"
    );
}

#[sqlx::test]
async fn accepting_a_rematch_by_its_id_is_refused_as_that_games(db: Db) {
    let server = server(db.clone());
    let (a, b) = (account(&server, "ann").await, account(&server, "bob").await);
    let (mut white, mut black, id, (white_token, black_token)) = finished(&server, (&a, &b)).await;
    ask_rematch(&mut white, &id).await;
    let offer = received_rematch(&mut black).await;
    // A block stored behind the hub's back, so the rematch is still waiting when accepted.
    sqlx::query("INSERT INTO social.blocks (blocker_id, blocked_id) VALUES ($1::uuid, $2::uuid)")
        .bind(user_id(&server, black_token).await)
        .bind(user_id(&server, white_token).await)
        .execute(&db)
        .await
        .expect("a block");
    black
        .send_json(&json!({ "type": "accept_challenge", "challenge_id": offer["id"] }))
        .await;
    let refused = next(&mut black, "rematch_refused").await;
    assert_eq!(refused["game_id"], id.as_str());
}

#[sqlx::test]
async fn a_refusal_answers_only_the_device_that_asked(db: Db) {
    let server = server(db);
    let (a, b) = (account(&server, "ann").await, account(&server, "bob").await);
    let (mut white, _black, id, (white_token, _)) = finished(&server, (&a, &b)).await;
    // White's second Mac.
    let mut second = connect(&server, white_token).await;
    ask_rematch(&mut white, &id).await;
    rematches(&mut second, "sent", true).await;
    ask_rematch(&mut second, &id).await;
    let told = next(&mut second, "rematch_refused").await;
    assert_eq!(
        told["message"],
        "You asked for this rematch on another device"
    );
    // The Mac that asked hears the lists both times, and no refusal between them.
    for _ in 0..2 {
        let heard = next_of(&mut white, &["rematch_refused", "challenges"]).await;
        assert_eq!(heard["type"], "challenges", "{heard}");
    }
}

#[sqlx::test]
async fn an_aborted_game_can_be_asked_for_again(db: Db) {
    let server = server(db);
    let (a, b) = (account(&server, "ann").await, account(&server, "bob").await);
    let (mut white, mut black, id, _) = paired(&server, (&a, &b), 180, false).await;
    white
        .send_json(&json!({ "type": "abort", "game_id": id }))
        .await;
    next(&mut black, "game").await;
    ask_rematch(&mut white, &id).await;
    let offer = received_rematch(&mut black).await;
    assert_eq!(offer["rematch_of"], id.as_str());
}
