use crate::tl;
use anyhow::Result;
use std::collections::HashSet;

pub mod deeplink;
pub mod entities;

/// 输入消息文本和消息entities
/// 输出用户名集合和用户ID集合
pub fn extract_usernames(
    message: &str,
    entities: Option<Vec<tl::enums::MessageEntity>>,
) -> Result<(HashSet<String>, HashSet<i64>)> {
    let mut usernames = HashSet::new();
    let mut user_ids = HashSet::new();

    // 调用Deeplink搜索
    usernames.extend(deeplink::extract_usernames(message));

    // 调用entities搜索
    if let Some(entities) = entities {
        let (mention_un, mention_uid) = entities::extract_mentioned(message, &entities)?;
        let text_url_un = entities::extract_text_url(&entities);
        usernames.extend(mention_un);
        usernames.extend(text_url_un);
        user_ids.extend(mention_uid);
    }
    Ok((usernames, user_ids))
}
