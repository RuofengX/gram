use crate::format::deserialize_msg;
use anyhow::Result;
use grammers_client::grammers_tl_types as tl;
use std::collections::HashSet;

pub mod deeplink;
pub mod entities;

/// 输入json格式的tl::enums::Message  
/// 通过扫描DeepLink和提取Mention  
/// 返回用户名集合与用户ID集合  
pub fn extract_usernames_json(msg: &str) -> Result<(HashSet<String>, HashSet<i64>)> {
    let msg = deserialize_msg(msg)?;
    extract_usernames(msg)
}

/// 输入tl::enums::Message  
/// 通过扫描DeepLink和提取Mention  
/// 返回用户名集合与用户ID集合  
pub fn extract_usernames(msg: tl::enums::Message) -> Result<(HashSet<String>, HashSet<i64>)> {
    let mut user_names = HashSet::new();
    let mut user_ids = HashSet::new();
    match msg {
        tl::enums::Message::Message(msg) => {
            // 调用Deeplink搜索
            let usernames = deeplink::extract_usernames_json(&msg.message);
            user_names.extend(usernames);

            // 调用entities搜索
            if let Some(entities) = msg.entities {
                let (mention_un, mention_uid) =
                    entities::extract_mentioned(msg.message, &entities)?;
                let text_url_un = entities::extract_text_url(&entities);
                user_names.extend(mention_un);
                user_names.extend(text_url_un);
                user_ids.extend(mention_uid);
            }
        }
        _ => (),
    };
    Ok((user_names, user_ids))
}
