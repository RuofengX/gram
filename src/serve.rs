use crate::{
    executor::Executor,
    scraper::{DownloadConfig, HistoryConfig},
    types::FrozenSession,
};
use anyhow::bail;
use axum::{
    Json, Router,
    body::Body,
    extract::{Path, State, WebSocketUpgrade, ws::Message},
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::{get, post},
};
use grammers_client::{grammers_tl_types as tl, types::PackedChat};
use serde::Deserialize;
use std::{fmt::Display, sync::Arc};
use tokio::sync::{mpsc, oneshot};
use tracing::{error, info, instrument, warn};
use uuid::Uuid;

#[derive(Debug)]
struct AppError(anyhow::Error);

impl Display for AppError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}
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

pub fn app(executor: Executor) -> Router {
    let state = AppState::new(executor);
    Router::new()
        .nest("/ctrl", control(state.clone()))
        .nest("/op", operate(state.clone()))
}

fn control(state: AppState) -> Router {
    Router::new()
        .route("/login/request", post(request_login))
        .route("/login/confirm", post(confirm_login))
        .route("/login-ws", post(login_ws))
        .route("/unfreeze", post(unfreeze))
        .with_state(state)
}

#[derive(Debug, Deserialize)]
struct RequestLogin {
    phone: String,
}
#[instrument(level = "info", err, ret, skip(s))]
async fn request_login(
    State(s): State<AppState>,
    Json(config): Json<RequestLogin>,
) -> Result<Json<Uuid>> {
    let ret = s.request_login(&config.phone).await?;
    Ok(Json(ret))
}

#[derive(Debug, Deserialize)]
struct ConfirmLogin {
    login_id: Uuid,
    code: String,
}

#[instrument(level = "info", err, ret, skip(s))]
async fn confirm_login(
    State(s): State<AppState>,
    Json(config): Json<ConfirmLogin>,
) -> Result<Json<Uuid>> {
    let ret = s.confirm_login(config.login_id, &config.code).await?;
    Ok(Json(ret))
}

#[instrument(level = "info", err, ret, skip(ws, s))]
async fn login_ws(
    ws: WebSocketUpgrade,
    State(s): State<AppState>,
    Json(phone): Json<String>,
) -> Result<Response> {
    let ret = ws.on_upgrade(|mut ws| async move {
        let (tx, rx) = oneshot::channel();
        if let Err(e) = tokio::try_join!(
            async move {
                s.login_async(phone, rx).await?;
                Ok(())
            },
            async {
                match ws.recv().await {
                    Some(Ok(Message::Text(code))) => {
                        let code = code.to_string();
                        if tx.send(code).is_err() {
                            bail!("login code receiver close");
                        }
                        Ok(())
                    }
                    Some(Ok(_)) => {
                        bail!("websocket recv type error")
                    }
                    Some(Err(e)) => {
                        bail!("websocket recv error {e}")
                    }
                    None => bail!("websocket closed"),
                }
            }
        ) {
            error!("login error {e}");
        }
    });
    Ok(ret)
}

#[instrument(level = "info", err, ret, skip(s, frozen))]
async fn unfreeze(
    State(s): State<AppState>,
    Json(frozen): Json<FrozenSession>,
) -> Result<Json<Uuid>> {
    let uuid = s.unfreeze(frozen).await?;
    Ok(Json(uuid))
}

fn operate(state: AppState) -> Router {
    Router::new()
        // 生命周期相关
        .route("/{session_id}/freeze", get(freeze))
        .route("/{session_id}/logout", get(logout))
        .route("/{session_id}/check-self", get(check_self))
        // 信息
        .route("/{session_id}/info/user", post(fetch_user))
        .route("/{session_id}/info/channel", post(fetch_channel))
        // 文件
        .route("/{session_id}/file/download", post(download))
        // 聊天相关
        .route("/{session_id}/chat/resolve", post(resolve_username))
        .route("/{session_id}/chat/list", get(list_chat))
        .route("/{session_id}/chat/join", post(join_chat))
        .route("/{session_id}/chat/join-by-name", post(join_chat_name))
        .route("/{session_id}/chat/quit", post(quit_chat))
        .route("/{session_id}/chat/iter-msg", post(fetch_msg))
        .with_state(state)
}

/// 检测自身信息  
/// 通常用于登录是否成功的检查
#[instrument(level = "info", err, ret, skip(s))]
async fn check_self(
    State(s): State<AppState>,
    Path(session_id): Path<Uuid>,
) -> Result<Json<tl::types::User>> {
    let u = s.get_session(&session_id)?.value().get_self().await?;
    Ok(Json(u))
}

#[instrument(level = "info", err, ret, skip(s))]
async fn freeze(
    State(s): State<AppState>,
    Path(session_id): Path<Uuid>,
) -> Result<Json<FrozenSession>> {
    let s = s.freeze(session_id)?;
    Ok(Json(s))
}

#[instrument(level = "info", err, ret, skip(s))]
async fn logout(State(s): State<AppState>, Path(session_id): Path<Uuid>) -> Result<()> {
    s.logout(session_id).await?;
    Ok(())
}

#[instrument(level = "info", err, ret, skip(s))]
async fn join_chat(
    State(s): State<AppState>,
    Path(session_id): Path<Uuid>,
    Json(packed_chat): Json<PackedChat>,
) -> Result<()> {
    s.get_session(&session_id)?
        .value()
        .join_chat(packed_chat)
        .await?;
    Ok(())
}

#[instrument(level = "info", err, ret, skip(s))]
async fn join_chat_name(
    State(s): State<AppState>,
    Path(session_id): Path<Uuid>,
    chat_name: String,
) -> Result<()> {
    s.get_session(&session_id)?
        .value()
        .join_chat_name(&chat_name)
        .await?;
    Ok(())
}

#[instrument(level = "info", err, ret, skip(s))]
async fn resolve_username(
    State(s): State<AppState>,
    Path(session_id): Path<Uuid>,
    username: String,
) -> Result<Json<PackedChat>> {
    let ret = s
        .get_session(&session_id)?
        .value()
        .resolve_username(&username)
        .await?;
    Ok(Json(ret))
}

#[instrument(level = "info", err, ret, skip(s))]
async fn list_chat(
    State(s): State<AppState>,
    Path(session_id): Path<Uuid>,
) -> Result<Json<Vec<PackedChat>>> {
    let ret = s.get_session(&session_id)?.value().list_chats().await?;
    Ok(Json(ret))
}

#[instrument(level = "info", err, ret, skip(s))]
async fn quit_chat(
    State(s): State<AppState>,
    Path(session_id): Path<Uuid>,
    Json(packed_chat): Json<PackedChat>,
) -> Result<()> {
    info!("list chat");
    s.get_session(&session_id)?
        .value()
        .quit_chat(packed_chat)
        .await?;
    Ok(())
}

/// 拉取聊天历史记录, 将数据json格式化后写入websocket
#[instrument(level = "info", err, ret, skip(s))]
async fn fetch_msg(
    ws: WebSocketUpgrade,
    State(s): State<AppState>,
    Path(session_id): Path<Uuid>,
    Json(config): Json<HistoryConfig>,
) -> Result<Response> {
    let ret = ws.on_upgrade(async move |mut ws| {
        let (tx, mut rx) = mpsc::channel(1024);
        if let Err(e) = tokio::try_join!(
            async {
                s.fetch_history(session_id, config, tx).await?;
                anyhow::Ok(())
            },
            async {
                while let Some(msg) = rx.recv().await {
                    let msg_byte = serde_json::to_vec(&msg)?;
                    ws.send(msg_byte.into()).await?;
                }
                Ok(())
            }
        ) {
            warn!("fetch history error: {e}");
        }
    });
    Ok(ret)
}

#[instrument(level = "info", err, ret, skip(s))]
async fn fetch_user(
    State(s): State<AppState>,
    Path(session_id): Path<Uuid>,
    Json(user): Json<PackedChat>,
) -> Result<Json<tl::types::users::UserFull>> {
    let ret = s
        .get_session(&session_id)?
        .value()
        .fetch_user_info(user)
        .await?;
    Ok(Json(ret))
}

#[instrument(level = "info", err, ret, skip(s))]
async fn fetch_channel(
    State(s): State<AppState>,
    Path(session_id): Path<Uuid>,
    Json(channel): Json<PackedChat>,
) -> Result<Json<tl::types::messages::ChatFull>> {
    let ret = s
        .get_session(&session_id)?
        .value()
        .fetch_channel_info(channel)
        .await?;
    Ok(Json(ret))
}

/// 打开一个长连接, sse传输下载内容
///
/// media: 来自ws接口fetch_msg方法迭代的message.media字段
#[instrument(level = "info", err, ret, skip(s))]
async fn download(
    State(s): State<AppState>,
    Path(session_id): Path<Uuid>,
    Json(config): Json<DownloadConfig>,
) -> Result<Body> {
    let s = s.get_session(&session_id)?;
    let rx = s.value().download_media(config)?;
    let body = Body::new(rx);
    Ok(body)
}
