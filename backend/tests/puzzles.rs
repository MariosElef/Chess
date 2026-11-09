//! Puzzles: from the imported database by rating range, or from a fake Lichess before any are
//! imported. Each test runs on its own freshly migrated database.
use std::{env, fs, process};

use axum::{Json, Router, extract::Query, http::StatusCode, routing::get};
use axum_test::TestServer;
use chess_backend::{AppState, Db, LichessClient, build_router, import_puzzle_file};
use serde::Deserialize;
use serde_json::{Value, json};

/// Three rows of the real Lichess file, rated 1819, 1433 and 1032, with its header.
const PUZZLE_FILE: &str = "\
PuzzleId,FEN,Moves,Rating,RatingDeviation,Popularity,NbPlays,Themes,GameUrl,OpeningTags,DailyDate
00008,r6k/pp2r2p/4Rp1Q/3p4/8/1N1P2R1/PqP2bPP/7K b - - 0 24,f2g3 e6e7 b2b1 b3c1 b1c1 h6c1,1819,77,95,10446,crushing hangingPiece long middlegame,https://lichess.org/game/787zsVup/black#48,,
0000D,5rk1/1p3ppp/pq3b2/8/8/1P1Q1N2/P4PPP/3R2K1 w - - 2 27,d3d6 f8d8 d6d8 f6d8,1433,75,96,37700,advantage endgame short,https://lichess.org/game/F8M8OS71#53,,
0000H,r4rk1/pp3ppp/8/2bqp2b/3N4/P2Bn2P/1PPN1P2/R1BQ1RK1 w - - 1 15,d1h5 d5g2,1032,78,100,92,mate mateIn1 middlegame oneMove,https://lichess.org/game/EwHRkfas#29,French_Defense French_Defense_Tarrasch_Variation,
";

#[derive(Deserialize)]
struct Difficulty {
    difficulty: String,
}

/// A fake Lichess on a free port that names the difficulty it was asked for in the puzzle id.
async fn fake_lichess() -> String {
    let app = Router::new().route(
        "/api/puzzle/next",
        get(|Query(asked): Query<Difficulty>| async move {
            Json(json!({
                "puzzle": {
                    "id": format!("lichess-{}", asked.difficulty),
                    "rating": 1593, "plays": 2015,
                    "solution": ["h5e8", "e7e8", "e1e8", "d6f8", "e8f8"],
                    "themes": ["mateIn3"],
                    "fen": "7k/pp2r1p1/2pb3r/3p1B1Q/3N4/8/P2q2PP/4R2K w - - 1 1",
                    "lastMove": "h7h8"
                }
            }))
        }),
    );
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("a free port");
    let address = listener.local_addr().expect("an address");
    tokio::spawn(async move {
        axum::serve(listener, app)
            .await
            .expect("the fake Lichess runs");
    });
    format!("http://{address}")
}

fn server(db: Db, lichess_url: &str) -> TestServer {
    let lichess = LichessClient::new(lichess_url).expect("a client");
    TestServer::new(build_router(AppState::assemble(db, lichess)))
}

async fn guest_token(server: &TestServer) -> String {
    let session: Value = server.post("/api/sessions/guest").await.json();
    session["token"].as_str().expect("a token").to_owned()
}

/// The three-puzzle file, compressed as Lichess ships it, imported.
async fn import_three(db: &Db, name: &str) -> u64 {
    let path = env::temp_dir().join(format!("chess-{name}-{}.csv.zst", process::id()));
    let compressed = zstd::encode_all(PUZZLE_FILE.as_bytes(), 0).expect("compressed");
    fs::write(&path, compressed).expect("written");
    let written = import_puzzle_file(db, path.clone())
        .await
        .expect("imported");
    fs::remove_file(path).expect("removed");
    written
}

async fn next(server: &TestServer, token: &str, query: &str) -> axum_test::TestResponse {
    server
        .get(&format!("/api/puzzles/next{query}"))
        .authorization_bearer(token)
        .await
}

#[sqlx::test]
async fn an_imported_puzzle_is_served_within_the_range_asked(db: Db) {
    assert_eq!(import_three(&db, "range").await, 3);
    let server = server(db, &fake_lichess().await);
    let token = guest_token(&server).await;
    for _ in 0..10 {
        let puzzle: Value = next(&server, &token, "?min_rating=1400&max_rating=1500")
            .await
            .json();
        assert_eq!(puzzle["id"], "0000D", "the only puzzle rated 1400 to 1500");
        assert_eq!(
            puzzle["last_move"], "d3d6",
            "the opponent's move is made first"
        );
        assert_eq!(puzzle["solution"], json!(["f8d8", "d6d8", "f6d8"]));
        assert_eq!(puzzle["themes"], json!(["advantage", "endgame", "short"]));
    }
}

#[sqlx::test]
async fn importing_again_updates_instead_of_duplicating(db: Db) {
    import_three(&db, "again-1").await;
    assert_eq!(import_three(&db, "again-2").await, 3);
    let server = server(db, &fake_lichess().await);
    let token = guest_token(&server).await;
    let wide: Value = next(&server, &token, "").await.json();
    assert!(["00008", "0000D", "0000H"].contains(&wide["id"].as_str().expect("an id")));
}

#[sqlx::test]
async fn a_range_with_no_puzzle_or_a_backwards_one_is_refused(db: Db) {
    import_three(&db, "refused").await;
    let server = server(db, &fake_lichess().await);
    let token = guest_token(&server).await;
    let empty = next(&server, &token, "?min_rating=2500&max_rating=2600").await;
    empty.assert_status_not_found();
    let problem: Value = empty.json();
    assert_eq!(problem["type"], "/problems/no-puzzle-in-range");
    let backwards = next(&server, &token, "?min_rating=1600&max_rating=1200").await;
    backwards.assert_status(StatusCode::UNPROCESSABLE_ENTITY);
}

#[sqlx::test]
async fn before_any_import_lichess_is_asked_at_the_nearest_difficulty(db: Db) {
    let server = server(db, &fake_lichess().await);
    let token = guest_token(&server).await;
    let easy: Value = next(&server, &token, "?min_rating=600&max_rating=900")
        .await
        .json();
    assert_eq!(easy["id"], "lichess-easiest");
    let hard: Value = next(&server, &token, "?min_rating=2100&max_rating=2400")
        .await
        .json();
    assert_eq!(hard["id"], "lichess-hardest");
}

#[sqlx::test]
async fn puzzles_need_signing_in(db: Db) {
    let server = server(db, &fake_lichess().await);
    server
        .get("/api/puzzles/next")
        .await
        .assert_status(StatusCode::UNAUTHORIZED);
}

#[sqlx::test]
async fn lichess_being_down_is_a_problem_of_its_own(db: Db) {
    let server = server(db, "http://127.0.0.1:9");
    let token = guest_token(&server).await;
    let response = next(&server, &token, "").await;
    response.assert_status(StatusCode::SERVICE_UNAVAILABLE);
    let problem: Value = response.json();
    assert_eq!(problem["type"], "/problems/lichess-unavailable");
}

/// A real account's token, signed up and signed in.
async fn account_token(server: &TestServer) -> String {
    server
        .post("/api/users")
        .json(&json!({ "email": "mario@example.com", "username": "Mario", "password": "correct horse" }))
        .await
        .assert_status(StatusCode::CREATED);
    let session: Value = server
        .post("/api/sessions")
        .json(&json!({ "login": "mario", "password": "correct horse" }))
        .await
        .json();
    session["token"].as_str().expect("a token").to_owned()
}

async fn attempt(server: &TestServer, token: &str, puzzle_id: &str, solved: bool) -> Value {
    let response = server
        .post(&format!("/api/puzzles/{puzzle_id}/attempts"))
        .authorization_bearer(token)
        .json(&json!({ "solved": solved, "rated": true, "duration_ms": 12000 }))
        .await;
    response.assert_status(StatusCode::CREATED);
    response.json()
}

async fn summary(server: &TestServer, token: &str) -> Value {
    server
        .get("/api/puzzles/summary")
        .authorization_bearer(token)
        .await
        .json()
}

#[sqlx::test]
async fn a_rated_attempt_moves_the_puzzle_rating(db: Db) {
    import_three(&db, "rated").await;
    let server = server(db, &fake_lichess().await);
    let token = account_token(&server).await;
    assert_eq!(
        summary(&server, &token).await["rating"],
        Value::Null,
        "no rating yet"
    );

    let solved = attempt(&server, &token, "0000D", true).await;
    let rating = &solved["rating"];
    assert!(
        rating["rating"].as_i64().expect("a rating") > 1500,
        "beat a 1433 puzzle"
    );
    assert!(rating["change"].as_i64().expect("a change") > 0);
    assert_eq!(
        rating["provisional"], true,
        "one result is not enough to be sure"
    );

    let failed = attempt(&server, &token, "0000H", false).await;
    assert!(failed["rating"]["change"].as_i64().expect("a change") < 0);
    let shown = summary(&server, &token).await;
    assert_eq!(shown["rating"]["rating"], failed["rating"]["rating"]);
    assert_eq!(shown["guest"], false);
}

#[sqlx::test]
async fn an_attempted_puzzle_is_not_served_again(db: Db) {
    import_three(&db, "seen").await;
    let server = server(db, &fake_lichess().await);
    let token = account_token(&server).await;
    attempt(&server, &token, "0000D", true).await;
    let again = next(&server, &token, "?min_rating=1400&max_rating=1500").await;
    again.assert_status_not_found();
}

#[sqlx::test]
async fn a_guest_keeps_no_rating_attempts_or_scores(db: Db) {
    import_three(&db, "guest").await;
    let server = server(db, &fake_lichess().await);
    let token = guest_token(&server).await;
    let solved = attempt(&server, &token, "0000D", true).await;
    assert_eq!(solved["rating"], Value::Null);
    let again: Value = next(&server, &token, "?min_rating=1400&max_rating=1500")
        .await
        .json();
    assert_eq!(again["id"], "0000D", "nothing was remembered for the guest");
    let rush: Value = server
        .post("/api/puzzles/rush")
        .authorization_bearer(&token)
        .json(&json!({ "kind": "survival", "score": 7 }))
        .await
        .json();
    assert_eq!(rush["new_best"], false);
    let shown = summary(&server, &token).await;
    assert_eq!(shown["guest"], true);
    assert_eq!(shown["best_rush"]["survival"], Value::Null);
}

#[sqlx::test]
async fn the_best_rush_of_each_kind_is_kept(db: Db) {
    let server = server(db, &fake_lichess().await);
    let token = account_token(&server).await;
    let rush = |score: i32| {
        server
            .post("/api/puzzles/rush")
            .authorization_bearer(&token)
            .json(&json!({ "kind": "three_minutes", "score": score }))
    };
    let first: Value = rush(5).await.json();
    assert_eq!(
        (first["best"].clone(), first["new_best"].clone()),
        (json!(5), json!(true))
    );
    let lower: Value = rush(3).await.json();
    assert_eq!(
        (lower["best"].clone(), lower["new_best"].clone()),
        (json!(5), json!(false))
    );
    let shown = summary(&server, &token).await;
    assert_eq!(shown["best_rush"]["three_minutes"], 5);
    assert_eq!(shown["best_rush"]["five_minutes"], Value::Null);
    assert_eq!(shown["best_today"]["three_minutes"], 5, "played today");
}

#[sqlx::test]
async fn attempts_are_listed_newest_first_with_their_rating_change(db: Db) {
    import_three(&db, "history").await;
    let server = server(db, &fake_lichess().await);
    let token = account_token(&server).await;
    let first = attempt(&server, &token, "0000D", true).await;
    let second = attempt(&server, &token, "0000H", false).await;
    let listed: Value = server
        .get("/api/puzzles/attempts")
        .authorization_bearer(&token)
        .await
        .json();
    assert_eq!(listed[0]["puzzle_id"], "0000H", "the latest first");
    assert_eq!(listed[0]["solved"], false);
    assert_eq!(listed[0]["puzzle_rating"], 1032);
    assert_eq!(listed[0]["duration_ms"], 12000);
    assert_eq!(listed[0]["rating_change"], second["rating"]["change"]);
    assert_eq!(listed[1]["rating_change"], first["rating"]["change"]);
}

#[sqlx::test]
async fn a_rated_ask_serves_puzzles_near_the_players_rating_and_widens_when_they_run_out(db: Db) {
    import_three(&db, "near").await;
    let server = server(db, &fake_lichess().await);
    let token = account_token(&server).await;
    // No rating yet is 1500: of 1819, 1433 and 1032, only 1433 is within 150.
    for _ in 0..5 {
        let puzzle: Value = next(&server, &token, "?rated=true").await.json();
        assert_eq!(puzzle["id"], "0000D", "the one nearest 1500");
    }
    // Solving it lifts the rating; 1819 is the nearest left, and 1032 is out of reach.
    attempt(&server, &token, "0000D", true).await;
    let puzzle: Value = next(&server, &token, "?rated=true").await.json();
    assert_eq!(puzzle["id"], "00008");
    // With every near puzzle done, the far one still comes rather than none at all.
    attempt(&server, &token, "00008", true).await;
    let puzzle: Value = next(&server, &token, "?rated=true").await.json();
    assert_eq!(puzzle["id"], "0000H");
}
