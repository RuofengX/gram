use anyhow::Result;
use grammers_client::grammers_tl_types as tl;
use std::collections::HashSet;
use tl::enums::MessageEntity;

pub fn extract_text_url(msg_entities: &[MessageEntity]) -> HashSet<String> {
    msg_entities
        .iter()
        .flat_map(|ent| match ent {
            MessageEntity::TextUrl(tl::types::MessageEntityTextUrl { url, .. }) => {
                super::deeplink::get_username(url)
            }
            _ => None,
        })
        .collect::<HashSet<_>>()
}

/// 输入tl::enums::Message的文本和entities部分
///
/// 分析其内容是否包含entities为Mention与mentionName标记,
/// 如果有则提取用户名或用户ID  
///
/// 返回两个列表，分别包含用户名和用户ID
///
/// 相关文档: https://core.telegram.org/api/entities
pub fn extract_mentioned(
    msg: &str,
    msg_entities: &[MessageEntity],
) -> Result<(HashSet<String>, HashSet<i64>)> {
    let mut user_ids = HashSet::new();
    let mentions = msg_entities
        .into_iter()
        .flat_map(|ent| match ent {
            // messageEntityMention, Message entity mentioning a user by @username;
            // messageEntityMentionName can also be used to mention users by their ID.
            // Mentions are implemented as message entities, passed to the messages.sendMessage method:
            //   * inputMessageEntityMentionName - Used when sending messages, allows mentioning a user inline,
            //     even for users that don't have a @username
            //   * messageEntityMentionName - Incoming message counterpart of inputMessageEntityMentionName
            //   * messageEntityMention - @botfather (this entity is generated automatically server-side for @usernames
            //     in messages, no need to provide it manually)
            // 两者区别: 如果使用了inputMessageEntityMentionName, 则消息中包含的是MentionName(目前来看应该不涉及);
            // 其他情况, 包括服务器自动生成, 均为Mention, 分开返回
            MessageEntity::Mention(tl::types::MessageEntityMention {
                offset, // 长度单位为UTF-16字符长
                length,
            }) => Some((*offset as usize, *length as usize)),
            MessageEntity::MentionName(tl::types::MessageEntityMentionName {
                offset,
                length,
                user_id,
            }) => {
                user_ids.insert(*user_id);
                Some((*offset as usize, *length as usize))
            }
            _ => None,
        })
        // 长度越界直接忽略
        .flat_map(|(offset, length)| utf16_range_to_utf8(&msg, offset, length)) // 获取以utf8计算的字节长度
        .flat_map(|(start, end)| msg.get(start..end)) // 截取用户名部分
        .flat_map(|x| x.get(1..)) // 删除@键
        .map(|x| x.to_lowercase()) // 转换小写
        .map(|x| x.to_string())
        .collect();
    Ok((mentions, user_ids))
}
/// 把 UTF-16 的 [offset, offset+len) 区间映射成 UTF-8 字节区间。
///
/// 返回 `Some((byte_start, byte_end))`，如果越界则返回 `None`。
pub fn utf16_range_to_utf8(s: &str, offset: usize, len: usize) -> Option<(usize, usize)> {
    let utf16_to_byte_idx = |idx: usize| -> Option<usize> {
        let mut utf16_cnt = 0;
        for (byte_idx, ch) in s.char_indices() {
            if utf16_cnt == idx {
                return Some(byte_idx);
            }
            utf16_cnt += ch.len_utf16();
        }
        // 尾部也允许（例如空区间放在末尾）
        if utf16_cnt == idx {
            return Some(s.len());
        }
        None
    };

    let start = utf16_to_byte_idx(offset)?;
    let end = utf16_to_byte_idx(offset + len)?;
    Some((start, end))
}
