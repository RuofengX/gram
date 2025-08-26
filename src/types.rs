use std::fmt;

use grammers_client::types::PackedChat;
use serde::{Deserialize, Serialize, de::Visitor};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiConfig {
    pub api_id: i32,
    pub api_hash: String,
}
impl Into<ApiConfig> for (i32, &'static str){
    fn into(self) -> ApiConfig {
        ApiConfig { api_id: self.0, api_hash: self.1.to_string() }
    }
}


#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FreezeSession {
    pub uuid: Uuid,
    #[serde(with = "serde_bytes")]
    pub value: Vec<u8>,
}

#[derive(Debug)]

pub struct TargetChat(pub PackedChat);

impl From<TargetChat> for PackedChat {
    fn from(value: TargetChat) -> Self {
        value.0
    }
}
impl Serialize for TargetChat {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_bytes(&self.0.to_bytes())
    }
}
impl<'de> Deserialize<'de> for TargetChat {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        struct PackedChatVisitor;

        impl<'de> Visitor<'de> for PackedChatVisitor {
            type Value = TargetChat;

            fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
                f.write_str("17 bytes representing a PackedChat")
            }

            // 如果格式是 bytes
            fn visit_bytes<E>(self, v: &[u8]) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                let arr: [u8; 17] = v
                    .try_into()
                    .map_err(|_| E::invalid_length(v.len(), &self))?;
                let inner = PackedChat::from_bytes(&arr).map_err(E::custom)?;
                Ok(TargetChat(inner))
            }

            // 某些序列化格式（如 bincode）会走 visit_byte_buf
            fn visit_byte_buf<E>(self, v: Vec<u8>) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                self.visit_bytes(&v)
            }
        }

        deserializer.deserialize_bytes(PackedChatVisitor)
    }
}
