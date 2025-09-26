use anyhow::{Result, anyhow};
use grammers_client::grammers_tl_types as tl;

/// 将JSON格式的message转换为[`tl::enums::Message`]
pub fn deserialize_msg(msg: &str) -> Result<tl::enums::Message> {
    serde_json::from_str::<tl::enums::Message>(msg)
        .map_err(|_| anyhow!("not json-format tl::enums::message"))
}
