use std::sync::Arc;

use axum::{
    Json, Router,
    extract::{Path, State, WebSocketUpgrade, ws::WebSocket},
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::{Route, any, get, post},
};
use grammers_client::grammers_tl_types as tl;
use uuid::Uuid;

use crate::{executor::Executor, scraper::Scraper, types::FreezeSession};

struct AppError(anyhow::Error);

impl From<anyhow::Error> for AppError {
    fn from(value: anyhow::Error) -> Self {
        AppError(value)
    }
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Something went wrong: {}", self.0),
        )
            .into_response()
    }
}

type Result<T> = std::result::Result<T, AppError>;

type AppState = Arc<Executor>;

pub fn app(state: AppState) -> Router {
    Router::new()
        .nest("/ctrl", control(state.clone()))
        .nest("/op", operate(state.clone()))
}

fn control(state: AppState) -> Router {
    Router::new()
        .route("/login", any(login))
        .route("/unfreeze", post(unfreeze))
        .with_state(state)
}

async fn login(
    ws: WebSocketUpgrade,
    State(s): State<AppState>,
    Json(phone): Json<String>,
) -> Result<()> {
    todo!("ws交换验证码和uuid")
}

async fn unfreeze(
    State(s): State<AppState>,
    Json(frozen): Json<FreezeSession>,
) -> Result<Json<Uuid>> {
    todo!()
}

fn operate(state: AppState) -> Router {
    Router::new()
        .route("/{session_id}/check-self", get(check_self))
        .route("/{session_id}/freeze", get(freeze))
        .route("/{session_id}/logout", get(logout))
        // .route("/{session_id}/join/chat", post(todo!()))
        // .route("/{session_id}/join/chat-link", post(todo!()))
        // .route("/{session_id}/quit/chat", post(todo!()))
        // .route("/{session_id}/fetch/user", post(todo!()))
        // .route("/{session_id}/fetch/channel", post(todo!()))
        // .route("/{session_id}/iter/message", any(todo!()))
        // .route("/{session_id}/download", any(download))
        .with_state(state)
}

async fn check_self(
    State(s): State<AppState>,
    Path(session_id): Path<Uuid>,
) -> Result<Json<tl::types::User>> {
    todo!()
}

async fn freeze(
    State(s): State<AppState>,
    Path(session_id): Path<Uuid>,
) -> Result<Json<FreezeSession>> {
    todo!()
}

async fn logout(State(s): State<AppState>, Path(session_id): Path<Uuid>) -> Result<()> {
    todo!()
}

// async fn check_self(State(s): State<AppState>)
async fn download(
    ws: WebSocketUpgrade,
    State(s): State<AppState>,
    Path(session_id): Path<Uuid>,
    Json(frozen): Json<FreezeSession>,
) -> Result<Response> {
    // 启动ws连接前, 使用提供的frozen信息准备爬虫
    let scraper = Scraper::from_frozen(frozen).await?;
    // 将frozen捕获进入闭包,
    let ret = ws.on_upgrade(|socket| download_ws(socket, scraper));
    Ok(ret)
}

async fn download_ws(mut socket: WebSocket, scraper: Scraper) {
    todo!()
}
