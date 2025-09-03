use std::{
    fs::File,
    io::{Read, Write},
    path::Path,
};

use anyhow::Result;
use serde::{Deserialize, Serialize};
use tracing::{info, trace};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiConfig {
    pub api_id: i32,
    pub api_hash: String,
}
impl Into<ApiConfig> for (i32, &'static str) {
    fn into(self) -> ApiConfig {
        ApiConfig {
            api_id: self.0,
            api_hash: self.1.to_string(),
        }
    }
}

/// 冻结的会话
///
/// 会话可以离线保存, 类似应用网络静默, 冻结后系统不再分配计算资源  
/// 内含会话凭证和会话ID(UUID)
#[derive(Clone, Serialize, Deserialize)]
pub struct FrozenSession {
    pub uuid: Uuid,
    #[serde(with = "serde_repr_base64::base64")]
    pub data: Vec<u8>,
}
// serde_json::to_string_pretty(&self).unwrap().fmt(f)
impl std::fmt::Debug for FrozenSession {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FrozenSession")
            .field("uuid", &self.uuid)
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

pub struct JsonLinesResponse {}
