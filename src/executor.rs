use anyhow::{Result, anyhow};
use axum::body::Body;
use dashmap::{DashMap, mapref::one::Ref};
use grammers_client::{grammers_tl_types as tl, types::PackedChat};
use tokio::sync::{mpsc, oneshot::Receiver};
use tracing::{error, warn};
use uuid::Uuid;

use crate::{
    scraper::{DownloadConfig, HistoryConfig, Scraper},
    types::{ApiConfig, FrozenSession},
};

pub struct Executor {
    api_config: ApiConfig,
    scrapers: DashMap<Uuid, Scraper>,
}
impl Executor {
    fn get(&self, session_id: &Uuid) -> Result<Ref<'_, Uuid, Scraper>> {
        self.scrapers
            .get(session_id)
            .ok_or(anyhow!("session not exist"))
    }
}
impl Executor {
    /// 创建一新的会话, 返回会话ID(UUID)
    ///
    /// 会话ID是后续操作的凭证
    pub async fn login(&self, phone: String, code: Receiver<String>) -> Result<Uuid> {
        let scraper = Scraper::new(&self.api_config).await?;
        scraper.login(&phone, code).await?;

        let uuid = Uuid::new_v4();
        self.scrapers.insert(uuid, scraper);

        Ok(uuid)
    }

    /// 从冻结（离线保存）的会话中恢复, 返回会话ID
    pub async fn unfreeze(&self, frozen: FrozenSession) -> Result<Uuid> {
        let uuid = frozen.uuid;
        if !self.scrapers.contains_key(&uuid) {
            let s = Scraper::from_frozen(frozen, &self.api_config).await?;
            self.scrapers.insert(uuid, s);
        }
        Ok(uuid)
    }
}

impl Executor {
    /// 检测自身信息  
    /// 通常用于登录是否成功的检查
    pub async fn check_self(&self, session_id: Uuid) -> Result<tl::types::User> {
        let s = self.get(&session_id)?;
        let ret = s.value().check_self().await?;
        Ok(ret)
    }

    /// 将会话冻结, 并从本地活跃状态删除
    pub fn freeze(&self, session_id: Uuid) -> Result<FrozenSession> {
        let (_, s) = self
            .scrapers
            .remove(&session_id)
            .ok_or(anyhow!("session not exist"))?;
        let frozen = s.freeze();
        Ok(frozen)
    }

    /// 将会话登出, 且从本地活跃状态删除
    pub async fn logout(&self, session_id: Uuid) -> Result<()> {
        let (_, s) = self
            .scrapers
            .remove(&session_id)
            .ok_or(anyhow!("session not exist"))?;
        s.logout().await?;
        Ok(())
    }

    pub async fn join_chat(&self, session_id: Uuid, packed_chat: PackedChat) -> Result<()> {
        let s = self.get(&session_id)?;
        s.value().join_chat(packed_chat).await?;
        Ok(())
    }

    pub async fn join_chat_link(&self, session_id: Uuid, link: &str) -> Result<()> {
        let s = self.get(&session_id)?;
        s.value().join_chat_link(link).await?;
        Ok(())
    }

    pub async fn quit_chat(&self, session_id: Uuid, packed_chat: PackedChat) -> Result<()> {
        let s = self.get(&session_id)?;
        s.value().quit_chat(packed_chat).await?;
        Ok(())
    }

    /// 获取对话历史
    pub async fn fetch_history(
        &self,
        session_id: Uuid,
        config: HistoryConfig,
        writer: mpsc::Sender<tl::enums::Message>,
    ) -> Result<()> {
        let s = self.get(&session_id)?;
        let mut i = s.value().iter_history(config)?;
        tokio::spawn(async move {
            warn!("start fetch message from chat({})", config.chat.id);
            loop {
                match i.next().await {
                    Ok(Some(msg)) => match writer.send(msg.raw).await {
                        Ok(_) => continue,
                        Err(e) => warn!("fetch message interrupt, inner writer error: {e}"),
                    },
                    Ok(None) => {
                        warn!("message iter end");
                    }
                    Err(e) => error!("message iter error: {e}"),
                }
            }
        });
        Ok(())
    }

    pub async fn fetch_user(
        &self,
        session_id: Uuid,
        user: PackedChat,
    ) -> Result<tl::types::users::UserFull> {
        let s = self.get(&session_id)?;
        let ret = s.value().fetch_user_info(user).await?;
        Ok(ret)
    }

    pub async fn fetch_channel(
        &self,
        session_id: Uuid,
        channel: PackedChat,
    ) -> Result<tl::types::messages::ChatFull> {
        let s = self.get(&session_id)?;
        let ret = s.value().fetch_channel_info(channel).await?;
        Ok(ret)
    }

    pub async fn download_media_http(
        &self,
        session_id: Uuid,
        config: DownloadConfig,
    ) -> Result<Body> {
        let s = self.get(&session_id)?;
        let session = s.value().download_media(config)?;
        Ok(Body::new(session))
    }
}
