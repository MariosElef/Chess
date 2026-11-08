//! The account API end to end, each test on its own freshly migrated database.
use axum::http::StatusCode;
use axum_test::TestServer;
use chess_backend::{AppState, Db, build_router};
use serde_json::{Value, json};

fn server(db: Db) -> TestServer {
    TestServer::new(build_router(AppState::assemble(db)))
}

async fn sign_up(server: &TestServer, email: &str, username: &str) {
    server
        .post("/api/users")
        .json(&json!({ "email": email, "username": username, "password": "correct horse" }))
        .await
        .assert_status(StatusCode::CREATED);
}

/// The new session's token.
async fn sign_in(server: &TestServer, login: &str) -> String {
    let session: Value = server
        .post("/api/sessions")
        .json(&json!({ "login": login, "password": "correct horse" }))
        .await
        .json();
    session["token"].as_str().expect("a token").to_owned()
}

#[sqlx::test]
async fn an_account_signs_up_signs_in_and_signs_out(db: Db) {
    let server = server(db);
    sign_up(&server, "Mario@Example.com", "Mario").await;
    let token = sign_in(&server, "mario@example.com").await;

    let me: Value = server
        .get("/api/users/me")
        .authorization_bearer(&token)
        .await
        .json();
    assert_eq!(me["email"], "mario@example.com", "stored lowercased");
    assert_eq!(me["username"], "Mario", "case kept");

    server
        .delete("/api/sessions/current")
        .authorization_bearer(&token)
        .await
        .assert_status_ok();
    let after = server
        .get("/api/users/me")
        .authorization_bearer(&token)
        .await;
    after.assert_status(StatusCode::UNAUTHORIZED);
    let problem: Value = after.json();
    assert_eq!(problem["type"], "/problems/sign-in-required");
}

#[sqlx::test]
async fn the_username_signs_in_too_whatever_its_case(db: Db) {
    let server = server(db);
    sign_up(&server, "mario@example.com", "Mario").await;
    assert!(!sign_in(&server, "MARIO").await.is_empty());
}

#[sqlx::test]
async fn a_wrong_password_and_an_unknown_name_answer_alike(db: Db) {
    let server = server(db);
    sign_up(&server, "mario@example.com", "Mario").await;
    for login in ["mario", "nobody"] {
        let response = server
            .post("/api/sessions")
            .json(&json!({ "login": login, "password": "wrong password" }))
            .await;
        response.assert_status(StatusCode::UNAUTHORIZED);
        let problem: Value = response.json();
        assert_eq!(problem["type"], "/problems/invalid-credentials", "{login}");
    }
}

#[sqlx::test]
async fn a_taken_email_or_username_is_a_conflict(db: Db) {
    let server = server(db);
    sign_up(&server, "mario@example.com", "Mario").await;
    let cases = [
        ("MARIO@example.com", "other", "/problems/email-taken"),
        ("other@example.com", "mario", "/problems/username-taken"),
    ];
    for (email, username, problem_type) in cases {
        let response = server
            .post("/api/users")
            .json(&json!({ "email": email, "username": username, "password": "correct horse" }))
            .await;
        response.assert_status(StatusCode::CONFLICT);
        let problem: Value = response.json();
        assert_eq!(problem["type"], problem_type);
    }
}

#[sqlx::test]
async fn every_bad_field_is_named_in_one_answer(db: Db) {
    let server = server(db);
    let response = server
        .post("/api/users")
        .json(&json!({ "email": "mario", "username": "m!", "password": "short" }))
        .await;
    response.assert_status(StatusCode::UNPROCESSABLE_ENTITY);
    let problem: Value = response.json();
    let pointers: Vec<&str> = problem["errors"]
        .as_array()
        .expect("field errors")
        .iter()
        .map(|error| error["pointer"].as_str().expect("a pointer"))
        .collect();
    assert_eq!(pointers, ["/email", "/username", "/password"]);
}

#[sqlx::test]
async fn an_unreadable_body_and_an_unknown_route_are_problems(db: Db) {
    let server = server(db);
    let unreadable = server
        .post("/api/users")
        .json(&json!({ "email": "mario@example.com" }))
        .await;
    unreadable.assert_status_bad_request();
    let problem: Value = unreadable.json();
    assert_eq!(problem["type"], "/problems/invalid-request-body");

    let missing = server.get("/api/nothing-here").await;
    missing.assert_status_not_found();
    assert_eq!(missing.header("content-type"), "application/problem+json");
}

#[sqlx::test]
async fn the_probes_answer(db: Db) {
    let server = server(db);
    server.get("/health").await.assert_status_ok();
    server.get("/ready").await.assert_status_ok();
}
