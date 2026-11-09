use std::{future::Future, sync::Arc, time::Duration};

use reqwest::{RequestBuilder, Response};
use serde::{Deserialize, de::DeserializeOwned};

use super::live_socket::LiveSocket;
use crate::{
    clients::http::{HttpRuntime, no_answer, read_json},
    enums::puzzles::RushKind,
    models::{
        account::{Device, Session, SignIn, SignUp, Stats, User},
        chat::{ChatMessage, Conversation},
        play::{GameRecord, LiveEvent, PlayedGame},
        puzzles::{
            AttemptReport, PlayedAttempt, PlayedBattle, Puzzle, PuzzleRating, PuzzleSummary,
            PuzzleText, RatingRange, RushRecord,
        },
        server::{Problem, ServerError},
        social::{FoundUser, Friends, Profile, RankingPage},
    },
};

/// How long fetching a 20 KB picture may take before the initial shows instead.
const PICTURE_TIMEOUT: Duration = Duration::from_secs(5);
/// How long sending a picture may take: longer than the server's 90 s, so its answer comes first.
const UPLOAD_TIMEOUT: Duration = Duration::from_secs(100);

/// The chess server's API, over the app's shared HTTP runtime. Cloning is cheap.
#[derive(Clone)]
pub struct ServerClient {
    http: HttpRuntime,
    base_url: Arc<str>,
}

impl ServerClient {
    pub fn new(http: HttpRuntime, base_url: &str) -> Self {
        Self {
            http,
            base_url: base_url.trim_end_matches('/').into(),
        }
    }

    pub fn sign_up(&self, body: &SignUp) -> impl Future<Output = Result<(), ServerError>> + use<> {
        let request = self.http.client().post(self.url("/api/users")).json(body);
        self.http
            .run(async move { send(request).await.map(|_| ()) })
    }

    pub fn sign_in(
        &self,
        body: &SignIn,
    ) -> impl Future<Output = Result<Session, ServerError>> + use<> {
        let request = self
            .http
            .client()
            .post(self.url("/api/sessions"))
            .json(body);
        self.http
            .run(async move { read_json(send(request).await?).await })
    }

    /// A session for the first free guest account.
    pub fn sign_in_as_guest(&self) -> impl Future<Output = Result<Session, ServerError>> + use<> {
        let request = self.http.client().post(self.url("/api/sessions/guest"));
        self.http
            .run(async move { read_json(send(request).await?).await })
    }

    /// The account `token` is signed in as.
    pub fn me(&self, token: &str) -> impl Future<Output = Result<User, ServerError>> + use<> {
        let request = self
            .http
            .client()
            .get(self.url("/api/users/me"))
            .bearer_auth(token);
        self.http
            .run(async move { read_json(send(request).await?).await })
    }

    /// The player's ratings in every mode and in puzzles, with their results.
    pub fn stats(&self, token: &str) -> impl Future<Output = Result<Stats, ServerError>> + use<> {
        let request = self
            .http
            .client()
            .get(self.url("/api/users/me/stats"))
            .bearer_auth(token);
        self.http
            .run(async move { read_json(send(request).await?).await })
    }

    /// A player's picture, waited for on this thread: the asset loader asks from its own.
    pub fn avatar_now(&self, user_id: &str, hash: &str) -> Result<Vec<u8>, ServerError> {
        let request = self
            .http
            .client()
            .get(self.url(&format!("/api/avatars/{user_id}/{hash}")))
            // A picture is small: a slow one gives up soon, holding a loader thread no longer.
            .timeout(PICTURE_TIMEOUT);
        self.http.block_on(async move {
            let response = send(request).await?;
            let bytes = response
                .bytes()
                .await
                .map_err(|error| ServerError::Unexpected(error.to_string()))?;
            Ok(bytes.to_vec())
        })
    }

    /// Makes `picture` (a JPEG, PNG, WebP or GIF file's bytes) the player's picture; its hash.
    pub fn set_avatar(
        &self,
        token: &str,
        picture: Vec<u8>,
    ) -> impl Future<Output = Result<String, ServerError>> + use<> {
        let request = self
            .http
            .client()
            .put(self.url("/api/users/me/avatar"))
            .bearer_auth(token)
            // A phone photo on a slow uplink takes longer than the usual request is allowed.
            .timeout(UPLOAD_TIMEOUT)
            .body(picture);
        self.http.run(async move {
            let set: SetAvatar = read_json(send(request).await?).await?;
            Ok(set.avatar)
        })
    }

    /// The player goes back to their initial.
    pub fn remove_avatar(
        &self,
        token: &str,
    ) -> impl Future<Output = Result<(), ServerError>> + use<> {
        let request = self
            .http
            .client()
            .delete(self.url("/api/users/me/avatar"))
            .bearer_auth(token);
        self.http
            .run(async move { send(request).await.map(|_| ()) })
    }

    /// The devices signed in to the account, this one marked.
    pub fn devices(
        &self,
        token: &str,
    ) -> impl Future<Output = Result<Vec<Device>, ServerError>> + use<> {
        let request = self
            .http
            .client()
            .get(self.url("/api/sessions"))
            .bearer_auth(token);
        self.http
            .run(async move { read_json(send(request).await?).await })
    }

    /// Signs out every device but this one.
    pub fn sign_out_others(
        &self,
        token: &str,
    ) -> impl Future<Output = Result<(), ServerError>> + use<> {
        let request = self
            .http
            .client()
            .delete(self.url("/api/sessions/others"))
            .bearer_auth(token);
        self.http
            .run(async move { send(request).await.map(|_| ()) })
    }

    /// A page of the player's finished games, the latest first: the newest, or before `before`.
    pub fn games(
        &self,
        token: &str,
        before: Option<&str>,
    ) -> impl Future<Output = Result<Vec<PlayedGame>, ServerError>> + use<> {
        self.history_page("/api/games", token, before)
    }

    /// A page of the player's finished puzzle battles, the latest first.
    pub fn battles(
        &self,
        token: &str,
        before: Option<&str>,
    ) -> impl Future<Output = Result<Vec<PlayedBattle>, ServerError>> + use<> {
        self.history_page("/api/puzzles/battles", token, before)
    }

    /// A page of the player's puzzle attempts, the latest first.
    pub fn attempts(
        &self,
        token: &str,
        before: Option<&str>,
    ) -> impl Future<Output = Result<Vec<PlayedAttempt>, ServerError>> + use<> {
        self.history_page("/api/puzzles/attempts", token, before)
    }

    /// A page of the history at `path`: the newest, or what came before `before`.
    fn history_page<T: DeserializeOwned + Send + 'static>(
        &self,
        path: &str,
        token: &str,
        before: Option<&str>,
    ) -> impl Future<Output = Result<Vec<T>, ServerError>> + use<T> {
        let params: Vec<(&str, &str)> = before.map(|id| ("before", id)).into_iter().collect();
        let url = reqwest::Url::parse_with_params(&self.url(path), params);
        let request = url.map(|url| self.http.client().get(url).bearer_auth(token));
        self.http.run(async move {
            let request = request.map_err(|_| {
                ServerError::Unexpected("the history could not be asked".to_owned())
            })?;
            read_json(send(request).await?).await
        })
    }

    /// Another player's profile: their name, picture, ratings and record.
    pub fn profile(
        &self,
        token: &str,
        user_id: &str,
    ) -> impl Future<Output = Result<Profile, ServerError>> + use<> {
        let request = self
            .http
            .client()
            .get(self.url(&format!("/api/users/{user_id}/profile")))
            .bearer_auth(token);
        self.http
            .run(async move { read_json(send(request).await?).await })
    }

    /// The leaderboard `board` from place `offset` on, with the player's own place.
    pub fn leaderboard(
        &self,
        token: &str,
        (board, offset): (&str, usize),
    ) -> impl Future<Output = Result<RankingPage, ServerError>> + use<> {
        let offset = offset.to_string();
        let url = reqwest::Url::parse_with_params(
            &self.url(&format!("/api/leaderboard/{board}")),
            [("offset", offset.as_str())],
        );
        let request = url.map(|url| self.http.client().get(url).bearer_auth(token));
        self.http.run(async move {
            let request = request.map_err(|_| {
                ServerError::Unexpected("the leaderboard could not be asked".to_owned())
            })?;
            read_json(send(request).await?).await
        })
    }

    /// One of the player's finished games, with its moves.
    pub fn game(
        &self,
        token: &str,
        id: &str,
    ) -> impl Future<Output = Result<GameRecord, ServerError>> + use<> {
        let request = self
            .http
            .client()
            .get(self.url(&format!("/api/games/{id}")))
            .bearer_auth(token);
        self.http
            .run(async move { read_json(send(request).await?).await })
    }

    /// A puzzle rated within `range`, for whoever `token` signs in.
    pub fn next_puzzle(
        &self,
        token: &str,
        range: RatingRange,
    ) -> impl Future<Output = Result<Puzzle, ServerError>> + use<> {
        let path = format!(
            "/api/puzzles/next?min_rating={}&max_rating={}",
            range.min, range.max
        );
        self.puzzle_at(token, &path)
    }

    /// A puzzle near the puzzle rating of whoever `token` signs in, the server picking.
    pub fn rated_puzzle(
        &self,
        token: &str,
    ) -> impl Future<Output = Result<Puzzle, ServerError>> + use<> {
        self.puzzle_at(token, "/api/puzzles/next?rated=true")
    }

    fn puzzle_at(
        &self,
        token: &str,
        path: &str,
    ) -> impl Future<Output = Result<Puzzle, ServerError>> + use<> {
        let request = self.http.client().get(self.url(path)).bearer_auth(token);
        self.http.run(async move {
            let text: PuzzleText = read_json(send(request).await?).await?;
            text.read().map_err(ServerError::Unexpected)
        })
    }

    /// Tells the server how a puzzle went; after a rated one, the player's new rating.
    pub fn record_attempt(
        &self,
        token: &str,
        puzzle_id: &str,
        report: AttemptReport,
    ) -> impl Future<Output = Result<Option<PuzzleRating>, ServerError>> + use<> {
        let request = self
            .http
            .client()
            .post(self.url(&format!("/api/puzzles/{puzzle_id}/attempts")))
            .bearer_auth(token)
            .json(&report);
        self.http.run(async move {
            let answer: AttemptAnswer = read_json(send(request).await?).await?;
            Ok(answer.rating)
        })
    }

    /// Stores a finished rush; the best of its kind, and whether this one set it.
    pub fn record_rush(
        &self,
        token: &str,
        kind: RushKind,
        score: u32,
    ) -> impl Future<Output = Result<RushRecord, ServerError>> + use<> {
        let body = serde_json::json!({ "kind": kind.key(), "score": score });
        let request = self
            .http
            .client()
            .post(self.url("/api/puzzles/rush"))
            .bearer_auth(token)
            .json(&body);
        self.http
            .run(async move { read_json(send(request).await?).await })
    }

    /// The puzzle rating, the best rushes ever and today, and the battle summary.
    pub fn puzzle_summary(
        &self,
        token: &str,
    ) -> impl Future<Output = Result<PuzzleSummary, ServerError>> + use<> {
        let request = self
            .http
            .client()
            .get(self.url("/api/puzzles/summary"))
            .bearer_auth(token);
        self.http
            .run(async move { read_json(send(request).await?).await })
    }

    /// Friends with their ratings and last online, requests each way, and the player's ratings.
    pub fn friends(
        &self,
        token: &str,
    ) -> impl Future<Output = Result<Friends, ServerError>> + use<> {
        let request = self
            .http
            .client()
            .get(self.url("/api/social/friends"))
            .bearer_auth(token);
        self.http
            .run(async move { read_json(send(request).await?).await })
    }

    /// Players whose name contains `query`, and where `token`'s player stands with each.
    pub fn search_players(
        &self,
        token: &str,
        query: &str,
    ) -> impl Future<Output = Result<Vec<FoundUser>, ServerError>> + use<> {
        let url = reqwest::Url::parse_with_params(&self.url("/api/social/search"), [("q", query)]);
        let request = url.map(|url| self.http.client().get(url).bearer_auth(token));
        self.http.run(async move {
            let request = request
                .map_err(|_| ServerError::Unexpected("the search could not be sent".to_owned()))?;
            read_json(send(request).await?).await
        })
    }

    /// Asks the player to be friends; if they had asked first, this accepts.
    pub fn request_friend(
        &self,
        token: &str,
        user_id: &str,
    ) -> impl Future<Output = Result<(), ServerError>> + use<> {
        let body = serde_json::json!({ "user_id": user_id });
        let request = self
            .http
            .client()
            .post(self.url("/api/social/friends/requests"))
            .bearer_auth(token)
            .json(&body);
        self.http
            .run(async move { send(request).await.map(|_| ()) })
    }

    pub fn accept_friend(
        &self,
        token: &str,
        user_id: &str,
    ) -> impl Future<Output = Result<(), ServerError>> + use<> {
        let path = format!("/api/social/friends/requests/{user_id}/accept");
        let request = self.http.client().post(self.url(&path)).bearer_auth(token);
        self.http
            .run(async move { send(request).await.map(|_| ()) })
    }

    /// Blocks the player: no friendship, request or message stays between the two.
    pub fn block(
        &self,
        token: &str,
        user_id: &str,
    ) -> impl Future<Output = Result<(), ServerError>> + use<> {
        let path = format!("/api/social/blocks/{user_id}");
        let request = self.http.client().post(self.url(&path)).bearer_auth(token);
        self.http
            .run(async move { send(request).await.map(|_| ()) })
    }

    /// Lifts the player's block.
    pub fn unblock(
        &self,
        token: &str,
        user_id: &str,
    ) -> impl Future<Output = Result<(), ServerError>> + use<> {
        let path = format!("/api/social/blocks/{user_id}");
        let request = self
            .http
            .client()
            .delete(self.url(&path))
            .bearer_auth(token);
        self.http
            .run(async move { send(request).await.map(|_| ()) })
    }

    /// Unfriends the player, declines their request, or withdraws one sent to them.
    pub fn remove_friend(
        &self,
        token: &str,
        user_id: &str,
    ) -> impl Future<Output = Result<(), ServerError>> + use<> {
        let path = format!("/api/social/friends/{user_id}");
        let request = self
            .http
            .client()
            .delete(self.url(&path))
            .bearer_auth(token);
        self.http
            .run(async move { send(request).await.map(|_| ()) })
    }

    /// The player's conversations, most recently active first.
    pub fn conversations(
        &self,
        token: &str,
    ) -> impl Future<Output = Result<Vec<Conversation>, ServerError>> + use<> {
        let request = self
            .http
            .client()
            .get(self.url("/api/chat/conversations"))
            .bearer_auth(token);
        self.http
            .run(async move { read_json(send(request).await?).await })
    }

    /// The conversation with a friend, started the first time.
    pub fn open_direct(
        &self,
        token: &str,
        user_id: &str,
    ) -> impl Future<Output = Result<Conversation, ServerError>> + use<> {
        let body = serde_json::json!({ "user_id": user_id });
        let request = self
            .http
            .client()
            .post(self.url("/api/chat/direct"))
            .bearer_auth(token)
            .json(&body);
        self.http
            .run(async move { read_json(send(request).await?).await })
    }

    /// A group of the player and some of their friends, with a name.
    pub fn create_group(
        &self,
        token: &str,
        title: &str,
        member_ids: &[String],
    ) -> impl Future<Output = Result<Conversation, ServerError>> + use<> {
        let body = serde_json::json!({ "title": title, "member_ids": member_ids });
        let request = self
            .http
            .client()
            .post(self.url("/api/chat/groups"))
            .bearer_auth(token)
            .json(&body);
        self.http
            .run(async move { read_json(send(request).await?).await })
    }

    /// Adds a friend to a group.
    pub fn add_to_group(
        &self,
        token: &str,
        conversation_id: &str,
        user_id: &str,
    ) -> impl Future<Output = Result<(), ServerError>> + use<> {
        let body = serde_json::json!({ "user_id": user_id });
        let path = format!("/api/chat/groups/{conversation_id}/members");
        let request = self
            .http
            .client()
            .post(self.url(&path))
            .bearer_auth(token)
            .json(&body);
        self.http
            .run(async move { send(request).await.map(|_| ()) })
    }

    /// The owner names their group anew.
    pub fn rename_group(
        &self,
        token: &str,
        conversation_id: &str,
        title: &str,
    ) -> impl Future<Output = Result<(), ServerError>> + use<> {
        let body = serde_json::json!({ "title": title });
        let path = format!("/api/chat/groups/{conversation_id}");
        let request = self
            .http
            .client()
            .patch(self.url(&path))
            .bearer_auth(token)
            .json(&body);
        self.http
            .run(async move { send(request).await.map(|_| ()) })
    }

    /// The owner takes a member out of their group.
    pub fn remove_from_group(
        &self,
        token: &str,
        conversation_id: &str,
        user_id: &str,
    ) -> impl Future<Output = Result<(), ServerError>> + use<> {
        let path = format!("/api/chat/groups/{conversation_id}/members/{user_id}");
        let request = self
            .http
            .client()
            .delete(self.url(&path))
            .bearer_auth(token);
        self.http
            .run(async move { send(request).await.map(|_| ()) })
    }

    pub fn leave_group(
        &self,
        token: &str,
        conversation_id: &str,
    ) -> impl Future<Output = Result<(), ServerError>> + use<> {
        let path = format!("/api/chat/groups/{conversation_id}/members/me");
        let request = self
            .http
            .client()
            .delete(self.url(&path))
            .bearer_auth(token);
        self.http
            .run(async move { send(request).await.map(|_| ()) })
    }

    /// The conversation's newest messages, oldest first.
    pub fn messages(
        &self,
        token: &str,
        conversation_id: &str,
    ) -> impl Future<Output = Result<Vec<ChatMessage>, ServerError>> + use<> {
        let path = format!("/api/chat/conversations/{conversation_id}/messages");
        let request = self.http.client().get(self.url(&path)).bearer_auth(token);
        self.http
            .run(async move { read_json(send(request).await?).await })
    }

    pub fn send_message(
        &self,
        token: &str,
        conversation_id: &str,
        body: &str,
    ) -> impl Future<Output = Result<ChatMessage, ServerError>> + use<> {
        let json = serde_json::json!({ "body": body });
        let path = format!("/api/chat/conversations/{conversation_id}/messages");
        let request = self
            .http
            .client()
            .post(self.url(&path))
            .bearer_auth(token)
            .json(&json);
        self.http
            .run(async move { read_json(send(request).await?).await })
    }

    /// Everything in the conversation is read.
    pub fn mark_read(
        &self,
        token: &str,
        conversation_id: &str,
    ) -> impl Future<Output = Result<(), ServerError>> + use<> {
        let path = format!("/api/chat/conversations/{conversation_id}/read");
        let request = self.http.client().post(self.url(&path)).bearer_auth(token);
        self.http
            .run(async move { send(request).await.map(|_| ()) })
    }

    /// The live socket for online play, signed in as `token`.
    pub fn live_socket(&self, token: &str) -> (LiveSocket, tokio::sync::mpsc::Receiver<LiveEvent>) {
        let url = format!("{}/api/ws", self.base_url.replacen("http", "ws", 1));
        LiveSocket::open(&self.http, url, token.to_owned())
    }

    /// Closes the signed-in account, with its password typed again.
    pub fn close_account(
        &self,
        token: &str,
        password: &str,
    ) -> impl Future<Output = Result<(), ServerError>> + use<> {
        let body = serde_json::json!({ "password": password });
        let request = self
            .http
            .client()
            .delete(self.url("/api/users/me"))
            .bearer_auth(token)
            .json(&body);
        self.http
            .run(async move { send(request).await.map(|_| ()) })
    }

    pub fn sign_out(&self, token: &str) -> impl Future<Output = Result<(), ServerError>> + use<> {
        let request = self
            .http
            .client()
            .delete(self.url("/api/sessions/current"))
            .bearer_auth(token);
        self.http
            .run(async move { send(request).await.map(|_| ()) })
    }

    /// A client talking to `app`, a fake server on its own runtime; for tests.
    #[cfg(test)]
    pub fn serving(app: axum::Router) -> Self {
        let http = HttpRuntime::new().expect("a runtime");
        let base_url = http.serve(app);
        Self::new(http, &base_url)
    }

    #[cfg(test)]
    pub fn block_on<T>(&self, work: impl Future<Output = T>) -> T {
        self.http.block_on(work)
    }

    fn url(&self, path: &str) -> String {
        format!("{}{path}", self.base_url)
    }
}

/// The answer to a new picture: the hash that names it.
#[derive(Deserialize)]
struct SetAvatar {
    avatar: String,
}

/// The answer to an attempt: the new rating, or none.
#[derive(Deserialize)]
struct AttemptAnswer {
    rating: Option<PuzzleRating>,
}

/// Sends the request: a success, the server's problem, or why there was no answer.
async fn send(request: RequestBuilder) -> Result<Response, ServerError> {
    let response = request.send().await.map_err(no_answer)?;
    if response.status().is_success() {
        return Ok(response);
    }
    let status = response.status();
    match response.json::<Problem>().await {
        Ok(problem) => Err(ServerError::Refused(problem)),
        Err(_) => Err(ServerError::Unexpected(format!(
            "the server answered {status}"
        ))),
    }
}

#[cfg(test)]
mod tests {
    use axum::{
        Json, Router,
        http::{StatusCode, header},
        response::IntoResponse,
        routing::{get, post},
    };
    use serde_json::json;

    use super::ServerClient;
    use crate::{
        clients::http::HttpRuntime,
        models::{
            account::{SignIn, SignUp},
            puzzles::RatingRange,
            server::ServerError,
        },
    };

    fn sign_in() -> SignIn {
        SignIn {
            login: "mario".to_owned(),
            password: "correct horse".to_owned(),
        }
    }

    #[test]
    fn a_sign_in_brings_back_the_session() {
        let app = Router::new().route(
            "/api/sessions",
            post(|| async {
                (
                    StatusCode::CREATED,
                    Json(json!({
                        "token": "abc",
                        "expires_at": "2026-12-08T00:00:00Z",
                        "user": { "id": "1", "email": "m@example.com", "username": "Mario" }
                    })),
                )
            }),
        );
        let client = ServerClient::serving(app);
        let session = client
            .block_on(client.sign_in(&sign_in()))
            .unwrap_or_else(|_| panic!("signed in"));
        assert_eq!(session.token, "abc");
        assert_eq!(session.user.username, "Mario");
    }

    #[test]
    fn a_refusal_brings_back_the_servers_problem() {
        let app = Router::new().route(
            "/api/users",
            post(|| async {
                let body = json!({
                    "type": "/problems/invalid-fields",
                    "title": "Some fields are not valid",
                    "status": 422,
                    "errors": [{ "pointer": "/email", "detail": "'x' is not an email address" }]
                });
                (
                    StatusCode::UNPROCESSABLE_ENTITY,
                    [(header::CONTENT_TYPE, "application/problem+json")],
                    body.to_string(),
                )
                    .into_response()
            }),
        );
        let client = ServerClient::serving(app);
        let body = SignUp {
            email: "x".to_owned(),
            username: "mario".to_owned(),
            password: "correct horse".to_owned(),
        };
        let Err(ServerError::Refused(problem)) = client.block_on(client.sign_up(&body)) else {
            panic!("refused with a problem");
        };
        assert_eq!(problem.problem_type, "/problems/invalid-fields");
        assert_eq!(problem.errors[0].pointer, "/email");
    }

    #[test]
    fn the_token_goes_in_the_authorization_header() {
        let app = Router::new().route(
            "/api/users/me",
            get(|headers: axum::http::HeaderMap| async move {
                let signed_in = headers
                    .get(header::AUTHORIZATION)
                    .is_some_and(|value| value == "Bearer abc");
                Json(json!({
                    "id": "1",
                    "email": "m@example.com",
                    "username": if signed_in { "Mario" } else { "nobody" }
                }))
            }),
        );
        let client = ServerClient::serving(app);
        let user = client.block_on(client.me("abc")).expect("the account");
        assert_eq!(user.username, "Mario");
    }

    #[test]
    fn no_server_is_unreachable() {
        let http = HttpRuntime::new().expect("a runtime");
        let client = ServerClient::new(http, "http://127.0.0.1:9");
        let result = client.block_on(client.sign_in(&sign_in()));
        assert!(matches!(result, Err(ServerError::Unreachable)));
    }

    fn puzzle_server(fen: &'static str) -> ServerClient {
        ServerClient::serving(Router::new().route(
            "/api/puzzles/next",
            get(move || async move {
                Json(json!({
                    "id": "5laTS", "rating": 1593, "plays": 2015,
                    "themes": ["mateIn3"], "fen": fen, "last_move": "h7h8",
                    "solution": ["h5e8", "e7e8", "e1e8", "d6f8", "e8f8"]
                }))
            }),
        ))
    }

    #[test]
    fn a_puzzle_arrives_with_its_position_and_moves_read() {
        let client = puzzle_server("7k/pp2r1p1/2pb3r/3p1B1Q/3N4/8/P2q2PP/4R2K w - - 1 1");
        let puzzle = client
            .block_on(client.next_puzzle("abc", RatingRange::DEFAULT))
            .expect("a puzzle");
        assert_eq!(puzzle.rating, 1593);
        assert_eq!(puzzle.solution.len(), 5);
        assert_eq!(puzzle.last_move, Some("h7h8".parse().expect("a move")));
    }

    #[test]
    fn a_puzzle_that_does_not_parse_is_unexpected() {
        let client = puzzle_server("not a position");
        let result = client.block_on(client.next_puzzle("abc", RatingRange::DEFAULT));
        assert!(matches!(result, Err(ServerError::Unexpected(_))));
    }
}
