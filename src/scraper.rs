use anyhow::{Result, anyhow, bail};
use grammers_client::{
    Client, Config, InvocationError,
    client::messages::MessageIter,
    grammers_tl_types as tl,
    session::{self as session_tl, Session},
    types::{Downloadable, Media, PackedChat},
};
use serde::Deserialize;
use tokio::sync::mpsc;
use tracing::{error, info, warn};
use uuid::Uuid;

use crate::types::{ApiConfig, FrozenSession};

const FILE_MIGRATE_ERROR: i32 = 303;
const DOWNLOAD_CHUNK_SIZE: usize = 16 * 1024 * 1024;

#[derive(Debug)]
pub struct Scraper {
    uuid: Uuid,
    client: Client,
}

impl Scraper {
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
    pub async fn login(
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
            params: Default::default(),
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
    pub async fn check_self(&self) -> Result<tl::types::User> {
        let me = self.client.get_me().await?;
        match me.raw {
            tl::enums::User::User(u) => Ok(u),
            tl::enums::User::Empty(_) => bail!("check failed, self is empty!"),
        }
    }
    pub async fn join_chat(&self, chat: PackedChat) -> Result<()> {
        let ret = self.client.join_chat(chat).await?;
        match ret {
            Some(c) => info!("joined chat: [{}]({})", c.name().unwrap_or("-"), c.id()),
            None => warn!("client join chat return None value"),
        }
        Ok(())
    }

    pub async fn join_chat_link(&self, link: &str) -> Result<()> {
        let ret = self.client.accept_invite_link(link).await?;
        match ret {
            Some(c) => info!(
                "joined chat link: [{}]({})",
                c.name().unwrap_or("-"),
                c.id()
            ),
            None => warn!("client join chat link return None value"),
        }
        Ok(())
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

impl Scraper {
    /// 后台下载聊天中的媒体对象
    ///
    /// media: 媒体对象, 可由Message的media字段得到, 仅当前session有效
    /// tx: 下载数据分块写入位置
    pub async fn start_download(
        &self,
        media: tl::enums::MessageMedia,
        tx: mpsc::Sender<std::result::Result<bytes::Bytes, String>>,
    ) -> () {
        let client = self.client.clone();
        // TODO: 使用异步循环来编写，充分利用?语法糖
        tokio::spawn(async move {
            let media = Media::from_raw(media);
            if media.is_none() {
                tx.send(Err("media type not supported".to_owned())).await;
                return;
            }
            let media = media.unwrap();

            if let Some(location) = media.to_raw_input_location() {
                let size = media.size();
                if size.is_none() {
                    tx.send(Err("media has no size".to_owned())).await;
                    return;
                }
                let size = size.unwrap() as i64;

                let mut offset = 0i64;

                while offset < size {
                    let location = location.clone();

                    let request = tl::functions::upload::GetFile {
                        precise: true,
                        cdn_supported: false,
                        location,
                        offset,
                        limit: DOWNLOAD_CHUNK_SIZE as i32, // 1 MB
                    };

                    offset += DOWNLOAD_CHUNK_SIZE as i64;

                    let mut times = 0;
                    let mut dc = None;

                    while times <= 3 {
                        times += 1;
                        let res = match dc {
                            None => client.invoke(&request).await,
                            Some(dc) => client.invoke_in_dc(&request, dc as i32).await,
                        };

                        let payload = match res {
                            Ok(tl::enums::upload::File::File(file)) => Ok(file.bytes.into()),
                            Ok(tl::enums::upload::File::CdnRedirect(_)) => Err(
                                "API returned File::CdnRedirect even though cdn_supported = false"
                                    .to_owned(),
                            ),
                            Err(InvocationError::Rpc(e)) => {
                                if e.code == FILE_MIGRATE_ERROR {
                                    // dc redirect
                                    match e.value {
                                        Some(value) => {
                                            dc = Some(value);
                                            times -= 1;
                                            info!("file download redirect to dc {}", value);
                                            continue;
                                        }
                                        None => {
                                            Err("api returned dc redirect, but not dc provided"
                                                .to_owned())
                                        }
                                    }
                                } else {
                                    // invoce error
                                    Err("download error: {e}".to_owned())
                                }
                            }
                            Err(e) => Err("download error: {e}".to_owned()),
                        };

                        match payload {
                            Ok(payload) => {
                                tx.send(Ok(payload)).await;
                            }
                            Err(e) => {
                                error!("download error: {}", e);
                                tx.send(Err(e)).await;
                            }
                        };
                    }
                }
            } else {
                warn!("media {media:?} has no location, download fail");
            }
        });
    }
}

#[derive(Debug, Clone, Copy, Deserialize)]
pub struct HistoryConfig {
    pub chat: PackedChat,
    pub limit: usize,
    /// 参阅官方文档 tl::functions::messages::GetHistory
    pub offset_date: i32,
    /// 参阅官方文档 tl::functions::messages::GetHistory
    pub offset_id: i32,
}
impl Scraper {
    pub fn iter_history(&self, config: HistoryConfig) -> Result<MessageIter> {
        let ret = self
            .client
            .iter_messages(config.chat)
            .limit(config.limit)
            .max_date(config.offset_date)
            .offset_id(config.offset_id);

        Ok(ret)
    }
}
