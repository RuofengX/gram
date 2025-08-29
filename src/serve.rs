use std::{sync::Arc, task::Poll};

use anyhow::bail;
use axum::{
    Json, Router,
    body::Body,
    extract::{Path, State, WebSocketUpgrade, ws::Message},
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::{any, get, post},
};
use bytes::Bytes;
use grammers_client::{grammers_tl_types as tl, types::PackedChat};
use tokio::sync::{mpsc, oneshot};
use tracing::{error, warn};
use uuid::Uuid;

use crate::{executor::Executor, scraper::HistoryConfig, types::FrozenSession};

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
) -> Result<Response> {
    let ret = ws.on_upgrade(|mut ws| async move {
        let (tx, rx) = oneshot::channel();
        if let Err(e) = tokio::try_join!(
            async move {
                s.login(phone, rx).await?;
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

async fn unfreeze(
    State(s): State<AppState>,
    Json(frozen): Json<FrozenSession>,
) -> Result<Json<Uuid>> {
    let uuid = s.unfreeze(frozen).await?;
    Ok(Json(uuid))
}

fn operate(state: AppState) -> Router {
    Router::new()
        .route("/{session_id}/check-self", get(check_self))
        .route("/{session_id}/freeze", get(freeze))
        .route("/{session_id}/logout", get(logout))
        .route("/{session_id}/chat/join", post(join_chat))
        .route("/{session_id}/chat/join-link", post(join_chat_link))
        .route("/{session_id}/chat/quit", post(quit_chat))
        .route("/{session_id}/chat/iter-msg", post(fetch_msg))
        .route("/{session_id}/user/fetch", post(fetch_user))
        .route("/{session_id}/channel/fetch", post(fetch_channel))
        .route("/{session_id}/download", post(download))
        .with_state(state)
}

async fn check_self(
    State(s): State<AppState>,
    Path(session_id): Path<Uuid>,
) -> Result<Json<tl::types::User>> {
    let u = s.check_self(session_id).await?;
    Ok(Json(u))
}

async fn freeze(
    State(s): State<AppState>,
    Path(session_id): Path<Uuid>,
) -> Result<Json<FrozenSession>> {
    let s = s.freeze(session_id)?;
    Ok(Json(s))
}

async fn logout(State(s): State<AppState>, Path(session_id): Path<Uuid>) -> Result<()> {
    s.logout(session_id).await?;
    Ok(())
}

async fn join_chat(
    State(s): State<AppState>,
    Path(session_id): Path<Uuid>,
    Json(packed_chat): Json<PackedChat>,
) -> Result<()> {
    s.join_chat(session_id, packed_chat).await?;
    Ok(())
}

async fn join_chat_link(
    State(s): State<AppState>,
    Path(session_id): Path<Uuid>,
    Json(link): Json<String>,
) -> Result<()> {
    s.join_chat_link(session_id, &link).await?;
    Ok(())
}

async fn quit_chat(
    State(s): State<AppState>,
    Path(session_id): Path<Uuid>,
    Json(packed_chat): Json<PackedChat>,
) -> Result<()> {
    s.quit_chat(session_id, packed_chat).await?;
    Ok(())
}

/// 拉取聊天历史记录, 将数据json格式化后写入websocket
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

async fn fetch_user(
    State(s): State<AppState>,
    Path(session_id): Path<Uuid>,
    Json(user): Json<PackedChat>,
) -> Result<Json<tl::types::users::UserFull>> {
    let ret = s.fetch_user(session_id, user).await?;
    Ok(Json(ret))
}

async fn fetch_channel(
    State(s): State<AppState>,
    Path(session_id): Path<Uuid>,
    Json(channel): Json<PackedChat>,
) -> Result<Json<tl::types::messages::ChatFull>> {
    let ret = s.fetch_channel(session_id, channel).await?;
    Ok(Json(ret))
}

/// 打开一个长连接, sse传输下载内容
///
/// media: 来自ws接口fetch_msg方法迭代的message.media字段
async fn download(
    State(s): State<AppState>,
    Path(session_id): Path<Uuid>,
    Json(media): Json<tl::enums::MessageMedia>,
) -> Result<Body> {
    let rx = s.download_media(session_id, media).await?;
    Ok(Body::new(DownloadResponse(rx)))
}

/// todo???
pub struct DownloadResponse(mpsc::Receiver<std::result::Result<Bytes, String>>);
impl http_body::Body for DownloadResponse {
    type Data = Bytes;

    type Error = String;

    fn poll_frame(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> Poll<Option<std::result::Result<http_body::Frame<Self::Data>, Self::Error>>> {
        let this = self.get_mut();
        match this.0.poll_recv(cx) {
            Poll::Ready(data) => Poll::Ready(data.map(|a| a.map(|a| http_body::Frame::data(a)))),
            Poll::Pending => Poll::Pending,
        }
    }
}
