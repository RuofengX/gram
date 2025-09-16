use anyhow::Result;
use grammers_client::grammers_tl_types as tl;
use sea_orm::FromJsonQueryResult;
use serde::{Deserialize, Serialize};
use std::{
    fs::File,
    io::{Read, Write},
    path::Path,
};
use tracing::{info, trace};

use crate::entity::global_api_config;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, FromJsonQueryResult)]
pub struct ApiConfig {
    pub api_id: i32,
    pub api_hash: String,
}
impl From<(i32, &str)> for ApiConfig {
    fn from(value: (i32, &str)) -> Self {
        Self {
            api_id: value.0,
            api_hash: value.1.to_string(),
        }
    }
}
impl From<global_api_config::Model> for ApiConfig {
    fn from(value: global_api_config::Model) -> Self {
        value.api_config
    }
}

/// 冻结的会话
///
/// 会话可以离线保存, 类似应用网络静默, 冻结后系统不再分配计算资源  
/// 内含会话凭证和会话ID(UUID)
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize, FromJsonQueryResult)]
pub struct FrozenSession {
    #[serde(with = "serde_repr_base64::base64")]
    pub data: Vec<u8>,
}

impl std::fmt::Debug for FrozenSession {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FrozenSession")
            .field(
                "data",
                &self
                    .data
                    .iter()
                    .map(|b| format!("{:02x}", b))
                    .collect::<String>(),
            )
            .finish()
    }
}

impl FrozenSession {
    pub fn dumps(&self) -> Result<Vec<u8>> {
        // let ret = postcard::to_allocvec(&self)?;
        let ret = serde_json::to_vec(&self)?;
        Ok(ret)
    }

    pub fn loads(buf: &[u8]) -> Result<Self> {
        // let ret = postcard::from_bytes(buf)?;
        let ret = serde_json::from_slice(buf)?;
        Ok(ret)
    }

    pub fn dump(&self, path: impl AsRef<Path>) -> Result<()> {
        trace!("开始保存: {}", path.as_ref().display());
        let path = path.as_ref().to_path_buf();

        let mut f = File::options()
            .create(true)
            .write(true)
            .truncate(true)
            .open(&path)?;

        f.write_all(&self.dumps()?)?;
        f.flush()?;
        info!("保存至文件: {}", path.display());
        Ok(())
    }

    pub fn load(path: impl AsRef<Path>) -> Result<Self> {
        let mut f = File::options().read(true).open(path)?;
        let mut buf = Vec::new();
        f.read_to_end(&mut buf)?;
        Ok(Self::loads(&buf)?)
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, FromJsonQueryResult)]
pub struct ChannelFull(tl::types::ChannelFull);
impl Eq for ChannelFull {}
impl From<tl::types::ChannelFull> for ChannelFull {
    fn from(value: tl::types::ChannelFull) -> Self {
        Self(value)
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, FromJsonQueryResult)]
pub struct UserFull(tl::types::UserFull);
impl Eq for UserFull {}
impl From<tl::types::UserFull> for UserFull {
    fn from(value: tl::types::UserFull) -> Self {
        Self(value)
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, FromJsonQueryResult)]
pub struct MessageMedia(tl::enums::MessageMedia);
impl Eq for MessageMedia {}
impl From<tl::enums::MessageMedia> for MessageMedia {
    fn from(value: tl::enums::MessageMedia) -> Self {
        Self(value)
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, FromJsonQueryResult)]
pub struct FileType(tl::enums::storage::FileType);
impl Eq for FileType {}
impl From<tl::enums::storage::FileType> for FileType {
    fn from(value: tl::enums::storage::FileType) -> Self {
        Self(value)
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, FromJsonQueryResult)]
pub struct Message(tl::types::Message);
impl Eq for Message {}
impl From<tl::types::Message> for Message {
    fn from(value: tl::types::Message) -> Self {
        Self(value)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, FromJsonQueryResult)]
pub struct PackedChat(pub grammers_client::types::PackedChat);
impl Eq for PackedChat {}
impl From<grammers_client::types::PackedChat> for PackedChat {
    fn from(value: grammers_client::types::PackedChat) -> Self {
        Self(value)
    }
}
impl Into<grammers_client::types::PackedChat> for PackedChat {
    fn into(self) -> grammers_client::types::PackedChat {
        self.0
    }
}
