//! The ten shared guest accounts, each test on its own freshly migrated database.
use axum::http::StatusCode;
use axum_test::TestServer;
use chess_backend::{AppState, Db, build_router};
use serde_json::{Value, json};

fn server(db: Db) -> TestServer {
    TestServer::new(build_router(AppState::assemble(db)))
}

/// Signs in as a guest: the guest's username and the session's token.
async fn guest(server: &TestServer) -> (String, String) {
    let response = server.post("/api/sessions/guest").await;
    response.assert_status(StatusCode::CREATED);
    let session: Value = response.json();
    (
        session["user"]["username"]
            .as_str()
            .expect("a username")
            .to_owned(),
        session["token"].as_str().expect("a token").to_owned(),
    )
}

#[sqlx::test]
async fn guests_are_handed_out_in_order_and_freed_by_signing_out(db: Db) {
    let server = server(db);
    let (first, first_token) = guest(&server).await;
    let (second, _) = guest(&server).await;
    assert_eq!((first.as_str(), second.as_str()), ("Guest", "Guest2"));

    let me: Value = server
        .get("/api/users/me")
        .authorization_bearer(&first_token)
        .await
        .json();
    assert_eq!(me["guest"], true);

    server
        .delete("/api/sessions/current")
        .authorization_bearer(&first_token)
        .await
        .assert_status_ok();
    let (again, _) = guest(&server).await;
    assert_eq!(again, "Guest", "signing out gave Guest back");
}

#[sqlx::test]
async fn an_eleventh_guest_is_asked_to_sign_up(db: Db) {
    let server = server(db);
    let mut names = Vec::new();
    for _ in 0..10 {
        names.push(guest(&server).await.0);
    }
    assert_eq!(names.last().map(String::as_str), Some("Guest10"));
    let refused = server.post("/api/sessions/guest").await;
    refused.assert_status(StatusCode::CONFLICT);
    let problem: Value = refused.json();
    assert_eq!(problem["type"], "/problems/all-guests-busy");
}

#[sqlx::test]
async fn guests_take_no_password_and_their_names_are_taken(db: Db) {
    let server = server(db);
    server
        .post("/api/sessions")
        .json(&json!({ "login": "Guest", "password": "anything at all" }))
        .await
        .assert_status(StatusCode::UNAUTHORIZED);
    let sign_up = server
        .post("/api/users")
        .json(&json!({ "email": "me@example.com", "username": "guest3", "password": "correct horse" }))
        .await;
    sign_up.assert_status(StatusCode::CONFLICT);
    let problem: Value = sign_up.json();
    assert_eq!(problem["type"], "/problems/username-taken");
}

#[sqlx::test]
async fn a_real_account_is_not_a_guest(db: Db) {
    let server = server(db);
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
    assert_eq!(session["user"]["guest"], false);
}

#[sqlx::test]
async fn two_guests_at_once_never_share_an_account(db: Db) {
    let server = server(db);
    let (first, second) = tokio::join!(guest(&server), guest(&server));
    assert_ne!(first.0, second.0);
}
