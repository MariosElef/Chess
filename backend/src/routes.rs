mod account;
mod health;
mod puzzles;

use std::time::Duration;

use axum::{Router, http::StatusCode};
use tower::ServiceBuilder;
use tower_http::{
    catch_panic::CatchPanicLayer,
    limit::RequestBodyLimitLayer,
    request_id::{MakeRequestUuid, PropagateRequestIdLayer, SetRequestIdLayer},
    timeout::TimeoutLayer,
    trace::{DefaultMakeSpan, DefaultOnResponse, TraceLayer},
};
use tracing::Level;

use crate::{
    errors::{AppError, ChessCode},
    state::AppState,
};

/// A request that takes longer is answered 408, which also lets shutdown finish.
const REQUEST_TIMEOUT: Duration = Duration::from_secs(30);
/// No request this server takes needs more.
const BODY_LIMIT_BYTES: usize = 64 * 1024;

/// Every route under `/api` behind the middleware; the health probes after it, so they make no
/// log lines.
pub fn build_router(state: AppState) -> Router {
    let api = Router::new()
        .merge(account::router())
        .nest("/puzzles", puzzles::router());
    let middleware = ServiceBuilder::new()
        .layer(SetRequestIdLayer::x_request_id(MakeRequestUuid))
        .layer(
            TraceLayer::new_for_http()
                .make_span_with(DefaultMakeSpan::new().level(Level::INFO))
                .on_response(DefaultOnResponse::new().level(Level::INFO)),
        )
        .layer(PropagateRequestIdLayer::x_request_id())
        .layer(TimeoutLayer::with_status_code(
            StatusCode::REQUEST_TIMEOUT,
            REQUEST_TIMEOUT,
        ))
        .layer(CatchPanicLayer::new())
        .layer(RequestBodyLimitLayer::new(BODY_LIMIT_BYTES));
    Router::new()
        .nest("/api", api)
        .fallback(not_found)
        .layer(middleware)
        .merge(health::router())
        .with_state(state)
}

async fn not_found() -> AppError {
    ChessCode::RouteNotFound.into()
}
