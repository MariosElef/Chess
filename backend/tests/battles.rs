//! Puzzle Battle: matchmaking, a friend's invitation, both players' answers and the result,
//! over the live socket. Each test runs on its own freshly migrated database.
use std::{env, fs, process, time::Duration};

use axum::http::StatusCode;
use axum_test::{TestServer, TestWebSocket};
use chess_backend::{AppState, Db, LichessClient, build_router, import_puzzle_file};
use serde_json::{Value, json};

/// Long enough for any message to arrive; a test waiting longer has failed.
const PATIENCE: Duration = Duration::from_secs(5);

/// Three rows of the real Lichess file, rated 1819, 1433 and 1032, with its header.
const PUZZLE_FILE: &str = "\
PuzzleId,FEN,Moves,Rating,RatingDeviation,Popularity,NbPlays,Themes,GameUrl,OpeningTags,DailyDate
00008,r6k/pp2r2p/4Rp1Q/3p4/8/1N1P2R1/PqP2bPP/7K b - - 0 24,f2g3 e6e7 b2b1 b3c1 b1c1 h6c1,1819,77,95,10446,crushing hangingPiece long middlegame,https://lichess.org/game/787zsVup/black#48,,
0000D,5rk1/1p3ppp/pq3b2/8/8/1P1Q1N2/P4PPP/3R2K1 w - - 2 27,d3d6 f8d8 d6d8 f6d8,1433,75,96,37700,advantage endgame short,https://lichess.org/game/F8M8OS71#53,,
0000H,r4rk1/pp3ppp/8/2bqp2b/3N4/P2Bn2P/1PPN1P2/R1BQ1RK1 w - - 1 15,d1h5 d5g2,1032,78,100,92,mate mateIn1 middlegame oneMove,https://lichess.org/game/EwHRkfas#29,French_Defense French_Defense_Tarrasch_Variation,
";

async fn server(db: Db, name: &str) -> TestServer {
    let path = env::temp_dir().join(format!("chess-battle-{name}-{}.csv.zst", process::id()));
    let compressed = zstd::encode_all(PUZZLE_FILE.as_bytes(), 0).expect("compressed");
    fs::write(&path, compressed).expect("written");
    import_puzzle_file(&db, path.clone())
        .await
        .expect("imported");
    fs::remove_file(path).expect("removed");
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

async fn connect(server: &TestServer, token: &str) -> TestWebSocket {
    server
        .get_websocket("/api/ws")
        .authorization_bearer(token)
        .await
        .into_websocket()
        .await
}

/// The next message of `kind` that passes `wanted`, skipping any other.
async fn next_where(
    socket: &mut TestWebSocket,
    kind: &str,
    wanted: impl Fn(&Value) -> bool,
) -> Value {
    tokio::time::timeout(PATIENCE, async {
        loop {
            let message: Value = socket.receive_json().await;
            if message["type"] == kind && wanted(&message) {
                return message;
            }
        }
    })
    .await
    .unwrap_or_else(|_| panic!("no {kind} message arrived"))
}

async fn answer(socket: &mut TestWebSocket, battle: &str, solved: bool) {
    socket
        .send_json(&json!({ "type": "battle_answer", "battle_id": battle, "solved": solved }))
        .await;
}

#[sqlx::test]
async fn two_players_battle_and_the_one_who_solves_more_wins(db: Db) {
    let server = server(db, "random").await;
    let (ann, _) = account(&server, "ann").await;
    let (bob, _) = account(&server, "bob").await;
    let mut ann_socket = connect(&server, &ann).await;
    let mut bob_socket = connect(&server, &bob).await;
    for socket in [&mut ann_socket, &mut bob_socket] {
        socket
            .send_json(&json!({ "type": "seek_battle", "rated": true }))
            .await;
    }
    let started = next_where(&mut ann_socket, "battle", |_| true).await["battle"].clone();
    let battle = started["id"].as_str().expect("an id").to_owned();
    assert_eq!(started["status"], "ongoing");
    assert_eq!(started["opponent"]["username"], "bob");
    assert!(!started["puzzles"].as_array().expect("puzzles").is_empty());
    assert_eq!(started["me"]["rating"], 1500);

    answer(&mut ann_socket, &battle, true).await;
    let seen = next_where(&mut bob_socket, "battle", |message| {
        message["battle"]["opponent"]["score"] == 1
    })
    .await;
    assert_eq!(
        seen["battle"]["me"]["score"], 0,
        "Bob sees Ann's point live"
    );

    // Ann runs out of puzzles after her point and two misses; Bob strikes out.
    for _ in 0..2 {
        answer(&mut ann_socket, &battle, false).await;
    }
    for _ in 0..3 {
        answer(&mut bob_socket, &battle, false).await;
    }
    let over = next_where(&mut ann_socket, "battle", |message| {
        message["battle"]["status"] == "finished"
    })
    .await["battle"]
        .clone();
    assert_eq!(over["result"], "won");
    assert!(over["rating_change"].as_i64().expect("rated") > 0);

    let summary: Value = server
        .get("/api/puzzles/summary")
        .authorization_bearer(&ann)
        .await
        .json();
    assert_eq!(summary["battle"]["wins_this_month"], 1);
    assert!(
        summary["battle"]["rating"]["rating"]
            .as_i64()
            .expect("a rating")
            > 1500
    );
}

#[sqlx::test]
async fn a_friend_is_invited_and_the_battle_starts_when_they_accept(db: Db) {
    let server = server(db, "friend").await;
    let (ann, ann_id) = account(&server, "ann").await;
    let (bob, bob_id) = account(&server, "bob").await;
    server
        .post("/api/social/friends/requests")
        .authorization_bearer(&ann)
        .json(&json!({ "user_id": bob_id }))
        .await;
    server
        .post(&format!("/api/social/friends/requests/{ann_id}/accept"))
        .authorization_bearer(&bob)
        .await
        .assert_status(StatusCode::NO_CONTENT);
    let mut ann_socket = connect(&server, &ann).await;
    let mut bob_socket = connect(&server, &bob).await;

    ann_socket
        .send_json(&json!({ "type": "challenge_battle", "user_id": bob_id, "rated": false }))
        .await;
    let waiting = next_where(&mut bob_socket, "battle_challenges", |message| {
        !message["received"].as_array().expect("a list").is_empty()
    })
    .await;
    let invitation = waiting["received"][0]["id"]
        .as_str()
        .expect("an id")
        .to_owned();
    assert_eq!(waiting["received"][0]["from"]["username"], "ann");

    bob_socket
        .send_json(&json!({ "type": "accept_battle", "challenge_id": invitation }))
        .await;
    let started = next_where(&mut ann_socket, "battle", |_| true).await["battle"].clone();
    assert_eq!(started["opponent"]["username"], "bob");
    assert_eq!(started["rated"], false);
    assert_eq!(
        started["me"]["rating"],
        Value::Null,
        "unrated: no ratings shown"
    );
}

#[sqlx::test]
async fn resigning_ends_the_battle_and_the_other_player_wins(db: Db) {
    let server = server(db, "resign").await;
    let (ann, _) = account(&server, "ann").await;
    let (bob, _) = account(&server, "bob").await;
    let mut ann_socket = connect(&server, &ann).await;
    let mut bob_socket = connect(&server, &bob).await;
    for socket in [&mut ann_socket, &mut bob_socket] {
        socket
            .send_json(&json!({ "type": "seek_battle", "rated": false }))
            .await;
    }
    let started = next_where(&mut ann_socket, "battle", |_| true).await["battle"].clone();
    let battle = started["id"].as_str().expect("an id").to_owned();
    // Ann is ahead, but gives up: Bob wins all the same.
    answer(&mut ann_socket, &battle, true).await;
    ann_socket
        .send_json(&json!({ "type": "resign_battle", "battle_id": battle }))
        .await;
    let over = next_where(&mut bob_socket, "battle", |message| {
        message["battle"]["status"] == "finished"
    })
    .await;
    assert_eq!(over["battle"]["result"], "won");
    let history = |token: &str| {
        server
            .get("/api/puzzles/battles")
            .authorization_bearer(token)
    };
    let bobs: Value = history(&bob).await.json();
    assert_eq!(bobs[0]["result"], "won");
    assert_eq!(bobs[0]["opponent"], "ann");
    assert_eq!(bobs[0]["their_score"], 1, "Ann solved one before resigning");
    assert_eq!(bobs[0]["rated"], false);
    assert_eq!(
        bobs[0]["rating_change"],
        Value::Null,
        "a casual battle moves nothing"
    );
    let anns: Value = history(&ann).await.json();
    assert_eq!(anns[0]["result"], "lost");
    assert_eq!(anns[0]["my_score"], 1);
}

#[sqlx::test]
async fn closing_an_account_mid_battle_resigns_it(db: Db) {
    let server = server(db, "closing").await;
    let (ann, _) = account(&server, "ann").await;
    let (bob, _) = account(&server, "bob").await;
    let mut ann_socket = connect(&server, &ann).await;
    let mut bob_socket = connect(&server, &bob).await;
    for socket in [&mut ann_socket, &mut bob_socket] {
        socket
            .send_json(&json!({ "type": "seek_battle", "rated": true }))
            .await;
    }
    next_where(&mut bob_socket, "battle", |_| true).await;
    server
        .delete("/api/users/me")
        .authorization_bearer(&ann)
        .json(&json!({ "password": "correct horse" }))
        .await
        .assert_status(StatusCode::NO_CONTENT);
    let over = next_where(&mut bob_socket, "battle", |message| {
        message["battle"]["status"] == "finished"
    })
    .await;
    assert_eq!(over["battle"]["result"], "won");
}
