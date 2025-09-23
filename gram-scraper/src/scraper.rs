use crate::types::{ApiConfig, ChannelFull, FrozenSession, PackedChat, UserFull};
use anyhow::{Result, anyhow, bail};
use bytes::Bytes;
use grammers_client::{
    Client, Config, InitParams,
    client::messages::MessageIter,
    grammers_tl_types as tl,
    session::{self as session_tl, Session},
    types::{Chat, LoginToken, Media},
};
use serde::Deserialize;
use std::time::Duration;
use tokio::sync::mpsc;
use tracing::{debug, error, info};

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
    params.flood_sleep_threshold = 1200;
    params.update_queue_limit = Some(0); // 不需要监听更新
    params.reconnection_policy = RETRY_POLICY;
    params
}

#[derive(Debug)]
pub struct Login(pub Client);
impl Login {
    pub async fn new(api_config: ApiConfig) -> Result<Self> {
        let session = session_tl::Session::new();
        let ApiConfig { api_id, api_hash } = api_config;
        let config = Config {
            session,
            api_id,
            api_hash,
            params: init_params(),
        };
        let client = Client::connect(config).await?;
        let ret = Self(client);
        Ok(ret)
    }

    /// 请求登录
    ///
    /// 输入手机号, 给手机号的Tg客户端发送验证码，返回登录Token, 之后使用Token和验证码登录
    pub async fn request_login(&self, phone: &str) -> Result<LoginToken> {
        let ret = self.0.request_login_code(phone).await?;
        Ok(ret)
    }

    /// 确认登录
    pub async fn confirm_login(self, login_token: LoginToken, code: &str) -> Result<Scraper> {
        self.0.sign_in(&login_token, code).await?;
        Ok(Scraper(self.0))
    }
}

#[derive(Debug)]
pub struct Scraper(Client);

impl Scraper {
    pub fn into_raw(self) -> Client {
        self.0
    }

    /// 请求登录
    ///
    /// 输入手机号, 给手机号的Tg客户端发送验证码，之后从reader中读code并登录
    pub async fn login_async(
        api_config: ApiConfig,
        phone: &str,
        code: tokio::sync::oneshot::Receiver<String>,
    ) -> Result<Self> {
        let ret = Login::new(api_config).await?;
        let login_token = ret.0.request_login_code(phone).await?;
        let code = code.await?;
        let user = ret.0.sign_in(&login_token, &code).await?;
        match user.raw {
            tl::enums::User::Empty(_) => bail!("sign in with empty user"),
            tl::enums::User::User(_u) => Ok(Self(ret.0)),
        }
    }

    /// 登出
    ///
    /// 退出登录
    pub async fn logout(self) -> Result<()> {
        self.0.sign_out().await?;
        Ok(())
    }

    /// 从冻结恢复
    ///
    /// 不需要重新登录
    pub async fn unfreeze(frozen: FrozenSession, api_config: ApiConfig) -> Result<Self> {
        let FrozenSession { data } = frozen;
        let ApiConfig { api_id, api_hash } = api_config;
        let session = Session::load(&data)?;

        let config = Config {
            session,
            api_id,
            api_hash,
            params: init_params(),
        };

        let client = Client::connect(config).await?;
        let ret = Self(client);
        Ok(ret)
    }

    /// 冻结
    ///
    /// 将session不退出保存, 下次不需要登录
    /// 调用者要保证出口IP前后一致
    pub fn freeze(&self) -> FrozenSession {
        FrozenSession {
            data: self.0.session().save(),
        }
    }
}

impl Scraper {
    pub async fn get_self(&self) -> Result<tl::types::User> {
        let me = self.0.get_me().await?;
        match me.raw {
            tl::enums::User::User(u) => Ok(u),
            tl::enums::User::Empty(_) => bail!("check failed, self is empty!"),
        }
    }

    /// https://core.telegram.org/method/contacts.resolveUsername
    pub async fn resolve_username(&self, username: &str) -> Result<Option<PackedChat>> {
        debug!("resolve username {}", username);
        let c = self.0.resolve_username(&username).await?;
        // .ok_or(anyhow!("username not found"))?;
        Ok(c.map(|x| x.pack().into()))
    }

    /// https://core.telegram.org/api/invites#public-usernames
    pub async fn join_chat(&self, PackedChat(chat): PackedChat) -> Result<Option<Chat>> {
        if let Some(c) = self.0.join_chat(chat).await? {
            debug!("joined chat: [{}]({})", c.name().unwrap_or("-"), c.id());
            Ok(Some(c))
        } else {
            debug!("chat ({}) not found", chat.id);
            Ok(None)
        }
    }

    pub async fn join_chat_name(&self, username: &str) -> Result<Option<Chat>> {
        if let Some(c) = self.resolve_username(username).await? {
            let c = self.join_chat(c).await?;
            Ok(c)
        } else {
            Ok(None)
        }
    }

    // 仅接受私有链接
    pub async fn join_chat_link(&self, link: &str) -> Result<()> {
        let chat = self
            .0
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
        let mut i = self.0.iter_dialogs();
        let mut ret = Vec::new();
        while let Some(dia) = i.next().await? {
            ret.push(dia.chat().pack().into());
        }

        info!("list all chats/dialogs, {} items", ret.len());

        Ok(ret)
    }

    pub async fn list_chats_with_username(&self) -> Result<Vec<(Option<String>, PackedChat)>> {
        let mut i = self.0.iter_dialogs();
        let mut ret = Vec::new();
        while let Some(dia) = i.next().await? {
            let username = dia.chat().username().map(|x| x.to_string());
            let chat = dia.chat().pack().into();
            ret.push((username, chat));
        }

        info!("list all chats/dialogs, {} items", ret.len());

        Ok(ret)
    }

    pub async fn fetch_user_info(&self, PackedChat(user): PackedChat) -> Result<UserFull> {
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
            .0
            .invoke(&tl::functions::users::GetFullUser { id: input_user })
            .await?;
        let tl::enums::users::UserFull::Full(ret) = ret;
        let ret = ret.full_user;
        let tl::enums::UserFull::Full(ret) = ret;
        Ok(ret.into())
    }

    pub async fn fetch_channel_info(&self, PackedChat(channel): PackedChat) -> Result<ChannelFull> {
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
            .0
            .invoke(&tl::functions::channels::GetFullChannel {
                channel: input_user,
            })
            .await?;
        let tl::enums::messages::ChatFull::Full(ret) = ret;
        let ret = ret.full_chat;
        if let tl::enums::ChatFull::ChannelFull(ret) = ret {
            Ok(ret.into())
        } else {
            bail!("target is channel but api return a user")
        }
    }

    pub async fn quit_chat(&self, PackedChat(chat): PackedChat) -> Result<()> {
        self.0.delete_dialog(chat).await?;
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
        let ret = self.0.iter_messages(config.chat);

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

impl Scraper {
    pub fn download_media(
        &self,
        config: DownloadConfig,
        tx: mpsc::Sender<Result<Bytes>>,
    ) -> Result<()> {
        let media_ex = Media::from_raw(config.media).ok_or(anyhow!("unsupport media"))?;
        let mut ret = self.0.iter_download(&media_ex);
        tokio::spawn(async move {
            loop {
                match ret.next().await {
                    Ok(Some(data)) => {
                        let _ = tx.send(Ok(data.into())).await.map_err(|e| {
                            error!("下载数据管道错误: {}", e);
                        });
                    }
                    Ok(None) => {
                        break;
                    }
                    Err(e) => {
                        let _ = tx
                            .send(Err(e.into()))
                            .await
                            .map_err(|e| {
                                error!("下载数据管道错误: {}", e);
                            })
                            .unwrap();
                        break;
                    }
                }
            }
        });
        Ok(())
    }
}
