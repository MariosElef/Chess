use std::{future::Future, io, sync::Arc, time::Duration};

use reqwest::{Client, RequestBuilder, Response};
use serde::de::DeserializeOwned;
use tokio::runtime::{Builder, Runtime};

use crate::models::{
    account::{Session, SignIn, SignUp, User},
    server::{Problem, ServerError},
};

/// A request that takes longer than this counts as the server being unreachable.
const REQUEST_TIMEOUT: Duration = Duration::from_secs(10);

/// The chess server's API. Requests run on the client's own small Tokio runtime, so the app's
/// executors never wait on the network; each call returns a future that can be awaited from
/// anywhere. Cloning shares the runtime and the connection pool.
#[derive(Clone)]
pub struct ServerClient {
    http: Client,
    base_url: Arc<str>,
    runtime: Arc<Runtime>,
}

impl ServerClient {
    pub fn new(base_url: &str) -> io::Result<Self> {
        let runtime = Builder::new_multi_thread()
            .worker_threads(1)
            .thread_name("chess-server-client")
            .enable_all()
            .build()?;
        let http = Client::builder()
            .timeout(REQUEST_TIMEOUT)
            .build()
            .map_err(io::Error::other)?;
        Ok(Self {
            http,
            base_url: base_url.trim_end_matches('/').into(),
            runtime: Arc::new(runtime),
        })
    }

    pub fn sign_up(&self, body: &SignUp) -> impl Future<Output = Result<(), ServerError>> + use<> {
        let request = self.http.post(self.url("/api/users")).json(body);
        self.run(async move { send(request).await.map(|_| ()) })
    }

    pub fn sign_in(
        &self,
        body: &SignIn,
    ) -> impl Future<Output = Result<Session, ServerError>> + use<> {
        let request = self.http.post(self.url("/api/sessions")).json(body);
        self.run(async move { read_json(send(request).await?).await })
    }

    /// A session for the first free guest account.
    pub fn sign_in_as_guest(&self) -> impl Future<Output = Result<Session, ServerError>> + use<> {
        let request = self.http.post(self.url("/api/sessions/guest"));
        self.run(async move { read_json(send(request).await?).await })
    }

    /// The account `token` is signed in as.
    pub fn me(&self, token: &str) -> impl Future<Output = Result<User, ServerError>> + use<> {
        let request = self.http.get(self.url("/api/users/me")).bearer_auth(token);
        self.run(async move { read_json(send(request).await?).await })
    }

    pub fn sign_out(&self, token: &str) -> impl Future<Output = Result<(), ServerError>> + use<> {
        let request = self
            .http
            .delete(self.url("/api/sessions/current"))
            .bearer_auth(token);
        self.run(async move { send(request).await.map(|_| ()) })
    }

    /// Waits for `work` on this thread; for tests, which have no app to await in.
    #[cfg(test)]
    pub fn block_on<T>(&self, work: impl Future<Output = T>) -> T {
        self.runtime.block_on(work)
    }

    /// A client talking to `app`, a fake server run on the client's own runtime at a free port.
    #[cfg(test)]
    pub fn serving(app: axum::Router) -> Self {
        let client = Self::new("http://127.0.0.1:1").expect("a client");
        let listener = client
            .block_on(tokio::net::TcpListener::bind("127.0.0.1:0"))
            .expect("a free port");
        let address = listener.local_addr().expect("an address");
        client.runtime.spawn(async move {
            axum::serve(listener, app)
                .await
                .expect("the fake server runs");
        });
        Self {
            base_url: format!("http://{address}").into(),
            ..client
        }
    }

    fn url(&self, path: &str) -> String {
        format!("{}{path}", self.base_url)
    }

    /// Starts `work` on the client's runtime; the returned future only waits for its result.
    fn run<T, F>(&self, work: F) -> impl Future<Output = Result<T, ServerError>> + use<T, F>
    where
        T: Send + 'static,
        F: Future<Output = Result<T, ServerError>> + Send + 'static,
    {
        let handle = self.runtime.spawn(work);
        async move {
            handle
                .await
                .unwrap_or_else(|error| Err(ServerError::Unexpected(error.to_string())))
        }
    }
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

async fn read_json<T: DeserializeOwned>(response: Response) -> Result<T, ServerError> {
    response
        .json()
        .await
        .map_err(|error| ServerError::Unexpected(error.to_string()))
}

/// A server that is down, or a request that timed out, is unreachable; anything else is ours.
fn no_answer(error: reqwest::Error) -> ServerError {
    if error.is_connect() || error.is_timeout() {
        ServerError::Unreachable
    } else {
        ServerError::Unexpected(error.to_string())
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
    use crate::models::{
        account::{SignIn, SignUp},
        server::ServerError,
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
        let client = ServerClient::new("http://127.0.0.1:9").expect("a client");
        let result = client.block_on(client.sign_in(&sign_in()));
        assert!(matches!(result, Err(ServerError::Unreachable)));
    }
}
