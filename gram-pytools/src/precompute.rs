use anyhow::{anyhow, Result};
use grammers_tl_types as tl;

pub fn deserialize_msg(msg: &str) -> Result<tl::enums::Message> {
    serde_json::from_str::<tl::enums::Message>(msg)
        .map_err(|_| anyhow!("not json-format tl::enums::message"))
}
