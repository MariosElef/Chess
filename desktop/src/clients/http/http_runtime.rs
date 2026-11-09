use std::{future::Future, io, sync::Arc, time::Duration};

use reqwest::Client;
use tokio::runtime::{Builder, Runtime};

use crate::models::server::ServerError;

/// A request that takes longer than this counts as the other side being unreachable.
const REQUEST_TIMEOUT: Duration = Duration::from_secs(10);

/// The app's one HTTP client and the small Tokio runtime it runs on, shared by every client.
#[derive(Clone)]
pub struct HttpRuntime {
    http: Client,
    runtime: Arc<Runtime>,
}

impl HttpRuntime {
    pub fn new() -> io::Result<Self> {
        let runtime = Builder::new_multi_thread()
            .worker_threads(1)
            .thread_name("chess-network")
            .enable_all()
            .build()?;
        let http = Client::builder()
            .timeout(REQUEST_TIMEOUT)
            .user_agent(user_agent())
            .build()
            .map_err(io::Error::other)?;
        Ok(Self {
            http,
            runtime: Arc::new(runtime),
        })
    }

    pub fn client(&self) -> &Client {
        &self.http
    }

    /// Starts `work` on the runtime; the returned future only waits for its result.
    pub fn run<T, F>(&self, work: F) -> impl Future<Output = Result<T, ServerError>> + use<T, F>
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

    /// Runs `work` on the runtime in the background, for as long as it takes.
    pub fn spawn(&self, work: impl Future<Output = ()> + Send + 'static) {
        self.runtime.spawn(work);
    }

    /// Waits for `work` on this thread; for tests, which have no app to await in.
    #[cfg(test)]
    pub fn block_on<T>(&self, work: impl Future<Output = T>) -> T {
        self.runtime.block_on(work)
    }

    /// Serves `app` as a fake server on this runtime at a free port; its base URL.
    #[cfg(test)]
    pub fn serve(&self, app: axum::Router) -> String {
        let listener = self
            .block_on(tokio::net::TcpListener::bind("127.0.0.1:0"))
            .expect("a free port");
        let address = listener.local_addr().expect("an address");
        self.runtime.spawn(async move {
            axum::serve(listener, app)
                .await
                .expect("the fake server runs");
        });
        format!("http://{address}")
    }
}

/// How the app names itself to the server, which lists it among the account's devices.
fn user_agent() -> String {
    format!(
        "Chess {} ({})",
        env!("CARGO_PKG_VERSION"),
        std::env::consts::OS
    )
}
