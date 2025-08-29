use anyhow::{Result, anyhow, bail};
use grammers_client::{
    Client, Config, InvocationError,
    client::messages::MessageIter,
    grammers_tl_types as tl,
    session::{self as session_tl, Session},
    types::{Chat, Downloadable, Media, PackedChat},
};
use tokio::io::{AsyncWrite, AsyncWriteExt};
use tracing::{info, warn};
use uuid::Uuid;

use crate::types::{ApiConfig, FreezeSession};

const FILE_MIGRATE_ERROR: i32 = 303;
const DOWNLOAD_CHUNK_SIZE: usize = 16 * 1024 * 1024;

#[derive(Debug)]
pub struct Scraper {
    uuid: Uuid,
    api_config: ApiConfig,
    client: Client,
}

impl Scraper {
    /// 新建
    ///
    /// 新建一个会话, 需要登录才可使用
    pub async fn new(api_config: ApiConfig) -> Result<Self> {
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
        let ret = Self {
            uuid,
            client,
            api_config,
        };
        Ok(ret)
    }

    /// 请求登录
    ///
    /// 输入手机号, 给手机号的Tg客户端发送验证码，之后从reader中读code并登录
    pub async fn login(
        &self,
        login_phone: &str,
        code_reader: tokio::sync::oneshot::Receiver<String>,
    ) -> Result<tl::types::User> {
        let login_token = self.client.request_login_code(login_phone).await?;
        let code = code_reader.await?;
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
    pub async fn from_frozen(frozen: FreezeSession) -> Result<Self> {
        let FreezeSession {
            uuid,
            value,
            api_config,
        } = frozen;
        let ApiConfig { api_id, api_hash } = api_config.clone();
        let session = Session::load(&value)?;
        let config = Config {
            session,
            api_id,
            api_hash,
            params: Default::default(),
        };

        let client = Client::connect(config).await?;
        let ret = Self {
            uuid,
            client,
            api_config,
        };
        Ok(ret)
    }

    /// 冻结
    ///
    /// 将session不退出保存, 下次不需要登录
    /// 调用者要保证出口IP前后一致
    pub fn freeze(self) -> FreezeSession {
        FreezeSession {
            uuid: self.uuid,
            value: self.client.session().save(),
            api_config: self.api_config,
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
    pub async fn join_chat(&self, target_chat: PackedChat) -> Result<Option<Chat>> {
        let ret = self.client.join_chat(target_chat).await?;
        Ok(ret)
    }

    pub async fn join_chat_link(&self, link: &str) -> Result<Option<Chat>> {
        let ret = self.client.accept_invite_link(link).await?;
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

    pub fn start_fetch_message(&self, chat: PackedChat) -> Result<MessageIter> {
        let ret = self.client.iter_messages(chat);
        Ok(ret)
    }

    pub async fn quit_chat(&self, chat: PackedChat) -> Result<()> {
        self.client.delete_dialog(chat).await?;
        Ok(())
    }

    /// 下载聊天中的媒体对象
    ///
    /// media: 媒体对象, 可由Message得到
    /// writer: 下载数据存储写入位置
    pub async fn download_media(
        &self,
        media: Media,
        mut writer: impl AsyncWrite + Send + Unpin + 'static,
    ) -> Result<()> {
        if let Some(location) = media.to_raw_input_location() {
            let size = media.size().ok_or(anyhow!("media has no size"))? as i64;
            let mut offset = 0i64;

            let (tx, mut rx) = tokio::sync::mpsc::channel::<Vec<u8>>(1024);

            tokio::try_join!(
                async {
                    while let Some(buf) = rx.recv().await {
                        writer.write_all(&buf).await?;
                        writer.flush().await?;
                    }
                    anyhow::Ok(())
                },
                async {
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
                                None => self.client.invoke(&request).await,
                                Some(dc) => self.client.invoke_in_dc(&request, dc as i32).await,
                            };

                            match res {
                                Ok(tl::enums::upload::File::File(file)) => {
                                    tx.send(file.bytes).await?;
                                }
                                Ok(tl::enums::upload::File::CdnRedirect(_)) => {
                                    bail!(
                                        "API returned File::CdnRedirect even though cdn_supported = false"
                                    );
                                }
                                Err(InvocationError::Rpc(e)) => {
                                    if e.code == FILE_MIGRATE_ERROR {
                                        dc = Some(e.value.ok_or(anyhow!(
                                            "api returned dc redirect, but not provide dc"
                                        ))?);
                                        times -= 1;
                                        info!("file download redirect to dc {}", dc.unwrap());
                                        continue;
                                    }
                                    bail!("download error: {e}");
                                }
                                Err(e) => bail!("download error: {e}"),
                            }
                        }
                    } // end of `while offset < size`
                    Ok(())
                } // end of second arm of tokio::join!
            )?; // end of `tokio::join!`
            Ok(())
        } else {
            warn!("media {media:?} has no location");
            bail!("media cannot download")
        }
    }
}
