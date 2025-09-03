use crate::types::{ApiConfig, FrozenSession};
use anyhow::{Result, anyhow, bail};
use bytes::Bytes;
use grammers_client::{
    Client, Config, InitParams, InvocationError,
    client::messages::MessageIter,
    grammers_tl_types::{self as tl},
    session::{self as session_tl, Session},
    types::{Downloadable, LoginToken, Media, PackedChat},
};
use http_body::Frame;
use serde::Deserialize;
use std::{pin::Pin, sync::Arc, task::Poll, time::Duration};
use tokio::sync::Mutex;
use tracing::{debug, info, warn};
use uuid::Uuid;

const FILE_MIGRATE_ERROR: i32 = 303;
const DOWNLOAD_CHUNK_SIZE: i32 = 0x1000000; // default 1MiB
const RETRY_POLICY: &'static dyn grammers_client::ReconnectionPolicy =
    &grammers_client::FixedReconnect {
        attempts: 5,
        delay: Duration::from_secs(1),
    };

fn init_params() -> InitParams {
    let mut params = InitParams::default();
    params.device_model = "DESKTOP-L2D4TG9I".to_owned();
    params.system_version = "10.0.241".to_owned();
    params.app_version = "0.1.0".to_owned();
    params.system_lang_code = "en".to_owned();
    params.lang_code = "my".to_owned();
    params.catch_up = true;
    params.server_addr = None;
    params.flood_sleep_threshold = 0;
    params.update_queue_limit = Some(0x1000000);
    params.reconnection_policy = RETRY_POLICY;
    params
}
#[derive(Debug)]
pub struct Scraper {
    uuid: Uuid,
    client: Client,
}

impl Scraper {
    pub fn into_raw(self) -> Client {
        self.client
    }

    /// 新建
    ///
    /// 新建一个会话, 需要登录才可使用
    pub async fn new(api_config: &ApiConfig) -> Result<Self> {
        let uuid = Uuid::new_v4();
        let session = session_tl::Session::new();
        let ApiConfig { api_id, api_hash } = api_config.clone();
        let config = Config {
            session,
            api_id,
            api_hash,
            params: Default::default(),
        };
        let client = Client::connect(config).await?;
        let ret = Self { uuid, client };
        Ok(ret)
    }

    /// 请求登录
    ///
    /// 输入手机号, 给手机号的Tg客户端发送验证码，之后从reader中读code并登录
    pub async fn login_async(
        &self,
        phone: &str,
        code: tokio::sync::oneshot::Receiver<String>,
    ) -> Result<tl::types::User> {
        let login_token = self.client.request_login_code(phone).await?;
        let code = code.await?;
        let user = self.client.sign_in(&login_token, &code).await?;
        match user.raw {
            tl::enums::User::Empty(_) => bail!("sign in with empty user"),
            tl::enums::User::User(u) => Ok(u),
        }
    }

    /// 请求登录
    ///
    /// 输入手机号, 给手机号的Tg客户端发送验证码，返回登录Token, 之后使用Token和验证码登录
    pub async fn request_login(&self, phone: &str) -> Result<LoginToken> {
        let ret = self.client.request_login_code(phone).await?;
        Ok(ret)
    }

    /// 确认登录
    pub async fn confirm_login(&self, login_token: LoginToken, code: &str) -> Result<()> {
        self.client.sign_in(&login_token, code).await?;
        Ok(())
    }

    /// 登出
    ///
    /// 退出登录
    pub async fn logout(self) -> Result<()> {
        self.client.sign_out().await?;
        Ok(())
    }

    /// 从冻结恢复
    ///
    /// 不需要重新登录
    pub async fn from_frozen(frozen: FrozenSession, api_config: &ApiConfig) -> Result<Self> {
        let FrozenSession { uuid, data } = frozen;
        let ApiConfig { api_id, api_hash } = api_config.clone();
        let session = Session::load(&data)?;

        let config = Config {
            session,
            api_id,
            api_hash,
            params: init_params(),
        };

        let client = Client::connect(config).await?;
        let ret = Self { uuid, client };
        Ok(ret)
    }

    /// 冻结
    ///
    /// 将session不退出保存, 下次不需要登录
    /// 调用者要保证出口IP前后一致
    pub fn freeze(self) -> FrozenSession {
        FrozenSession {
            uuid: self.uuid,
            data: self.client.session().save(),
        }
    }
}

impl Scraper {
    pub async fn get_self(&self) -> Result<tl::types::User> {
        let me = self.client.get_me().await?;
        match me.raw {
            tl::enums::User::User(u) => Ok(u),
            tl::enums::User::Empty(_) => bail!("check failed, self is empty!"),
        }
    }

    /// https://core.telegram.org/method/contacts.resolveUsername
    pub async fn resolve_username(&self, username: &str) -> Result<PackedChat> {
        debug!("resolve username {}", username);
        let c = self
            .client
            .resolve_username(&username)
            .await?
            .ok_or(anyhow!("username not found"))?;
        Ok(c.pack())
    }

    /// https://core.telegram.org/api/invites#public-usernames
    pub async fn join_chat(&self, chat: PackedChat) -> Result<()> {
        let c = self
            .client
            .join_chat(chat)
            .await?
            .ok_or(anyhow!("chat not found"))?;
        info!("joined chat: [{}]({})", c.name().unwrap_or("-"), c.id());
        Ok(())
    }

    pub async fn join_chat_name(&self, username: &str) -> Result<()> {
        let chat = self.resolve_username(username).await?;
        self.join_chat(chat).await?;
        Ok(())
    }

    // 仅接受私有链接
    pub async fn join_chat_link(&self, link: &str) -> Result<()> {
        let chat = self
            .client
            .accept_invite_link(link)
            .await?
            .ok_or(anyhow!("private chat not found"))?;
        info!(
            "joined chat link: [{}]({})",
            chat.name().unwrap_or("-"),
            chat.id()
        );
        Ok(())
    }

    pub async fn list_chats(&self) -> Result<Vec<PackedChat>> {
        let mut i = self.client.iter_dialogs();
        let mut ret = Vec::new();
        while let Some(dia) = i.next().await? {
            ret.push(dia.chat().pack());
        }

        info!("list all chats/dialogs, {} items", ret.len());

        Ok(ret)
    }

    pub async fn fetch_user_info(&self, user: PackedChat) -> Result<tl::types::users::UserFull> {
        if !user.is_user() {
            bail!("target chat not user");
        }

        let user_id = user.id;
        let access_hash = user.access_hash.ok_or(anyhow!("no access hash"))?;
        let input_user = tl::enums::InputUser::User(tl::types::InputUser {
            user_id,
            access_hash,
        });
        let ret = self
            .client
            .invoke(&tl::functions::users::GetFullUser { id: input_user })
            .await?;
        let tl::enums::users::UserFull::Full(ret) = ret;
        Ok(ret)
    }

    pub async fn fetch_channel_info(
        &self,
        channel: PackedChat,
    ) -> Result<tl::types::messages::ChatFull> {
        if !channel.is_channel() {
            bail!("target chat not channel");
        }

        let channel_id = channel.id;
        let access_hash = channel.access_hash.ok_or(anyhow!("no access hash"))?;
        let input_user = tl::enums::InputChannel::Channel(tl::types::InputChannel {
            channel_id,
            access_hash,
        });
        let ret = self
            .client
            .invoke(&tl::functions::channels::GetFullChannel {
                channel: input_user,
            })
            .await?;
        let tl::enums::messages::ChatFull::Full(ret) = ret;
        Ok(ret)
    }

    pub async fn quit_chat(&self, chat: PackedChat) -> Result<()> {
        self.client.delete_dialog(chat).await?;
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, Deserialize)]
pub struct HistoryConfig {
    pub chat: PackedChat,
    pub limit: Option<usize>,
    /// 参阅官方文档 tl::functions::messages::GetHistory
    pub offset_date: Option<i32>,
    /// 参阅官方文档 tl::functions::messages::GetHistory
    pub offset_id: Option<i32>,
}
impl Scraper {
    pub fn iter_history(&self, config: HistoryConfig) -> Result<MessageIter> {
        let ret = self.client.iter_messages(config.chat);

        let ret = if let Some(limit) = config.limit {
            ret.limit(limit)
        } else {
            ret
        };
        let ret = if let Some(max_date) = config.offset_date {
            ret.max_date(max_date)
        } else {
            ret
        };
        let ret = if let Some(offset_id) = config.offset_id {
            ret.offset_id(offset_id)
        } else {
            ret
        };

        Ok(ret)
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct DownloadConfig {
    /// media: 媒体对象, 可由Message的media字段得到, 仅当前session有效
    media: tl::enums::MessageMedia,
    offset: Option<i64>,
    chunk_size: Option<i32>,
}
impl DownloadConfig {
    pub fn new(
        media: tl::enums::MessageMedia,
        offset: Option<i64>,
        chunk_size: Option<i32>,
    ) -> Self {
        Self {
            media,
            offset,
            chunk_size,
        }
    }
    pub fn offset(&mut self, value: i64) -> &mut Self {
        self.offset = Some(value);
        self
    }
    pub fn chunk_size(&mut self, value: i32) -> &mut Self {
        self.chunk_size = Some(value);
        self
    }
}

pub struct DownloadSession {
    client: Client,
    location: tl::enums::InputFileLocation,
    size: usize,
    chunk_size: i32,
    offset: Arc<Mutex<i64>>,
    dc: Arc<Mutex<Option<u32>>>,
    future: Option<Pin<Box<dyn Future<Output = Result<Bytes>> + Send + Sync + 'static>>>,
}
impl DownloadSession {
    pub fn try_new(config: DownloadConfig, client: &Client) -> Result<Self> {
        let client = client.clone();
        let DownloadConfig {
            media,
            offset,
            chunk_size: limit,
        } = config;
        let media_ex = Media::from_raw(media).ok_or(anyhow!("unsupport media"))?;
        let size = media_ex.size().ok_or(anyhow!("media has no size"))?;
        let offset = Arc::new(Mutex::new(offset.unwrap_or(0)));
        let chunk_size = limit.unwrap_or(DOWNLOAD_CHUNK_SIZE);
        let location = media_ex
            .to_raw_input_location()
            .ok_or(anyhow!("cannot fetch media location"))?;
        Ok(Self {
            client,
            location,
            size,
            offset,
            chunk_size,
            dc: Arc::new(Mutex::new(None)),
            future: None,
        })
    }
}

impl DownloadSession {
    fn chunk_download(&self) -> impl Future<Output = Result<Bytes>> + Send + Sync + 'static {
        let client = self.client.clone();
        let location = self.location.clone();
        let limit = self.chunk_size;
        let offset = self.offset.clone();
        let dc = self.dc.clone();

        return async move {
            let offset = offset.lock().await.clone();
            let request = tl::functions::upload::GetFile {
                precise: true,
                cdn_supported: false,
                location,
                offset,
                limit,
            };
            let mut retry = 0;
            while retry < 3 {
                let res = match *dc.lock().await {
                    None => client.invoke(&request).await,
                    Some(dc) => client.invoke_in_dc(&request, dc as i32).await,
                };
                match res {
                    Ok(tl::enums::upload::File::File(f)) => return Ok(f.bytes.into()),

                    Ok(tl::enums::upload::File::CdnRedirect(_)) => {
                        bail!("server return cdn redict to a non-cdn request");
                    }
                    Err(e) => {
                        if let InvocationError::Rpc(e) = &e {
                            if e.code == FILE_MIGRATE_ERROR {
                                // redirect dc
                                *dc.lock().await = e.value;
                                continue;
                            }
                        }
                        warn!("retry download invoke error: {}", e);
                        retry += 1;
                    }
                }
            }
            bail!("retry download too much times");
        };
    }
}

// TODO: better implement futures_core::TryStream
impl http_body::Body for DownloadSession {
    type Data = Bytes;

    type Error = anyhow::Error;

    fn is_end_stream(&self) -> bool {
        if self.future.is_some() {
            // has download running
            false
        } else {
            // download finished || retry 3 times
            *self.offset.blocking_lock() >= self.size as i64
        }
    }

    fn poll_frame(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> Poll<Option<Result<Frame<<DownloadSession as http_body::Body>::Data>>>> {
        let this = self.get_mut();

        // redirect to running download if exists:
        if let Some(future) = &mut this.future {
            return future
                .as_mut()
                .poll(cx)
                .map(|r| Some(r.map(|d| Frame::data(d))));
        }

        // all download is end
        if this.is_end_stream() {
            return Poll::Ready(None);
        }

        // continue download next chunk
        this.future = Some(Box::pin(this.chunk_download()));
        return Poll::Pending;
    }
}

impl Scraper {
    pub fn download_media(&self, config: DownloadConfig) -> Result<DownloadSession> {
        DownloadSession::try_new(config, &self.client)
    }
}


