//! The leaderboards, each test on its own freshly migrated database.
use axum_test::TestServer;
use chess_backend::{AppState, Db, LichessClient, build_router};
use serde_json::{Value, json};

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
    (
        session["token"].as_str().expect("a token").to_owned(),
        session["user"]["id"].as_str().expect("an id").to_owned(),
    )
}

/// A guest signed in: its token and id.
async fn guest(server: &TestServer) -> (String, String) {
    let session: Value = server.post("/api/sessions/guest").await.json();
    (
        session["token"].as_str().expect("a token").to_owned(),
        session["user"]["id"].as_str().expect("an id").to_owned(),
    )
}

/// Gives player `id` a rating in `category` after `played` rated results.
async fn rate(db: &Db, id: &str, category: &str, (rating, deviation, played): (f64, f64, i32)) {
    sqlx::query(
        "INSERT INTO play.ratings (user_id, category_id, rating, deviation, games_played)
         VALUES ($1::uuid, $2, $3, $4, $5)",
    )
    .bind(id)
    .bind(category)
    .bind(rating)
    .bind(deviation)
    .bind(played)
    .execute(db)
    .await
    .expect("a rating");
}

async fn board(server: &TestServer, token: &str, path: &str) -> Value {
    server.get(path).authorization_bearer(token).await.json()
}

fn names(board: &Value) -> Vec<String> {
    board["rows"]
        .as_array()
        .expect("rows")
        .iter()
        .map(|row| row["username"].as_str().expect("a name").to_owned())
        .collect()
}

#[sqlx::test]
async fn the_leaderboard_ranks_open_accounts_and_shows_the_askers_place(db: Db) {
    let server = server(db.clone());
    let (ann, ann_id) = account(&server, "ann").await;
    let (bob, bob_id) = account(&server, "bob").await;
    let (_, cat_id) = account(&server, "cat").await;
    let (_, dan_id) = account(&server, "dan").await;
    let (visitor, visitor_id) = guest(&server).await;
    rate(&db, &ann_id, "blitz", (1700.0, 60.0, 5)).await;
    rate(&db, &bob_id, "blitz", (1600.0, 200.0, 5)).await;
    rate(&db, &cat_id, "blitz", (1800.0, 60.0, 5)).await;
    rate(&db, &dan_id, "blitz", (1900.0, 60.0, 5)).await;
    rate(&db, &visitor_id, "blitz", (2000.0, 60.0, 5)).await;
    // Dan closes his account: he leaves the board.
    sqlx::query("UPDATE account.users SET deleted_at = now() WHERE username = 'dan'")
        .execute(&db)
        .await
        .expect("closed");

    let blitz = board(&server, &bob, "/api/leaderboard/blitz").await;
    assert_eq!(
        names(&blitz),
        ["cat", "ann", "bob"],
        "no guest, no closed account"
    );
    assert_eq!(blitz["rows"][0]["place"], 1);
    assert_eq!(
        blitz["rows"][2]["provisional"], true,
        "Bob's rating is not settled yet"
    );
    assert_eq!(blitz["me"]["place"], 3);
    assert_eq!(blitz["more"], false);

    let as_guest = board(&server, &visitor, "/api/leaderboard/blitz").await;
    assert!(as_guest["me"].is_null(), "a guest has no place");
    let off_the_board = board(&server, &ann, "/api/leaderboard/bullet").await;
    assert!(off_the_board["me"].is_null(), "Ann has no bullet rating");
    server
        .get("/api/leaderboard/chess960")
        .authorization_bearer(&ann)
        .await
        .assert_status_not_found();
}

#[sqlx::test]
async fn equal_ratings_share_a_place_and_no_result_keeps_a_player_off(db: Db) {
    let server = server(db.clone());
    let (ann, ann_id) = account(&server, "ann").await;
    let (_, bob_id) = account(&server, "bob").await;
    let (_, cat_id) = account(&server, "cat").await;
    // Both show 1712: they share second place, listed by name.
    rate(&db, &ann_id, "puzzle", (1711.6, 60.0, 9)).await;
    rate(&db, &bob_id, "puzzle", (1712.4, 60.0, 9)).await;
    rate(&db, &cat_id, "puzzle", (1800.0, 60.0, 9)).await;
    let (_, dan_id) = account(&server, "dan").await;
    rate(&db, &dan_id, "puzzle", (2400.0, 60.0, 0)).await;
    let puzzles = board(&server, &ann, "/api/leaderboard/puzzle").await;
    assert_eq!(
        names(&puzzles),
        ["cat", "ann", "bob"],
        "Dan has never played"
    );
    let places: Vec<&Value> = puzzles["rows"]
        .as_array()
        .expect("rows")
        .iter()
        .map(|row| &row["place"])
        .collect();
    assert_eq!(places, [&json!(1), &json!(2), &json!(2)]);
}

#[sqlx::test]
async fn the_leaderboard_comes_fifty_places_at_a_time(db: Db) {
    let server = server(db.clone());
    let (ann, _) = account(&server, "ann").await;
    for number in 0..51 {
        let (_, id) = account(&server, &format!("player{number:02}")).await;
        let rating = f64::from(1000 + number);
        rate(&db, &id, "puzzle_battle", (rating, 60.0, 3)).await;
    }
    let first = board(&server, &ann, "/api/leaderboard/puzzle_battle").await;
    assert_eq!(names(&first).len(), 50);
    assert_eq!(first["more"], true, "the fifty-first waits for Load more");
    let second = board(&server, &ann, "/api/leaderboard/puzzle_battle?offset=50").await;
    assert_eq!(names(&second), ["player00"]);
    assert_eq!(second["rows"][0]["place"], 51);
    assert_eq!(second["more"], false);
    let before_the_top = board(&server, &ann, "/api/leaderboard/puzzle_battle?offset=-5").await;
    assert_eq!(
        before_the_top["rows"][0]["place"], 1,
        "a negative offset starts at the top"
    );
}

#[sqlx::test]
async fn the_average_board_ranks_each_players_mean_over_what_they_have_played(db: Db) {
    let server = server(db.clone());
    let (ann, ann_id) = account(&server, "ann").await;
    let (_, bob_id) = account(&server, "bob").await;
    let (_, cat_id) = account(&server, "cat").await;
    let (visitor, visitor_id) = guest(&server).await;
    // Ann averages 1600 over three boards; a board she never played does not pull her down.
    rate(&db, &ann_id, "blitz", (1800.0, 60.0, 10)).await;
    rate(&db, &ann_id, "puzzle", (1500.0, 60.0, 20)).await;
    rate(&db, &ann_id, "puzzle_battle", (1500.0, 60.0, 4)).await;
    rate(&db, &ann_id, "bullet", (900.0, 350.0, 0)).await;
    // Bob has one board only, and averages higher; Cat has never finished a rated result.
    rate(&db, &bob_id, "rapid", (1700.0, 60.0, 3)).await;
    rate(&db, &cat_id, "classical", (2500.0, 60.0, 0)).await;
    rate(&db, &visitor_id, "blitz", (2000.0, 60.0, 5)).await;

    let average = board(&server, &ann, "/api/leaderboard/average").await;
    assert_eq!(names(&average), ["bob", "ann"], "no guest, no one unplayed");
    assert_eq!(average["rows"][1]["rating"], 1600);
    assert_eq!(
        average["rows"][1]["played"], 34,
        "every rated result counts"
    );
    assert_eq!(average["me"]["place"], 2);
    assert_eq!(average["more"], false);
    let as_guest = board(&server, &visitor, "/api/leaderboard/average").await;
    assert!(as_guest["me"].is_null(), "a guest has no place");
}
