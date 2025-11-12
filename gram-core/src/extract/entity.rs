use anyhow::{Result, anyhow};
use grammers_tl_types as tl;
use std::collections::HashSet;
use tl::enums::MessageEntity;

pub fn extract_entity<'a>(msg: &'a str, msg_entity: &MessageEntity) -> Result<Option<&'a str>> {
    let result = match msg_entity {
        MessageEntity::Unknown(_) => None,
        MessageEntity::Mention(tl::types::MessageEntityMention { offset, length }) => {
            Some((offset, length))
        }
        MessageEntity::Hashtag(tl::types::MessageEntityHashtag { offset, length }) => {
            Some((offset, length))
        }
        MessageEntity::BotCommand(tl::types::MessageEntityBotCommand { offset, length }) => {
            Some((offset, length))
        }
        MessageEntity::Url(tl::types::MessageEntityUrl { offset, length }) => {
            Some((offset, length))
        }
        MessageEntity::Email(tl::types::MessageEntityEmail { offset, length }) => {
            Some((offset, length))
        }
        MessageEntity::Bold(tl::types::MessageEntityBold { offset, length }) => {
            Some((offset, length))
        }
        MessageEntity::Italic(tl::types::MessageEntityItalic { offset, length }) => {
            Some((offset, length))
        }
        MessageEntity::Code(tl::types::MessageEntityCode { offset, length }) => {
            Some((offset, length))
        }
        MessageEntity::Pre(tl::types::MessageEntityPre { offset, length, .. }) => {
            Some((offset, length))
        }
        MessageEntity::TextUrl(tl::types::MessageEntityTextUrl { offset, length, .. }) => {
            Some((offset, length))
        }
        MessageEntity::MentionName(tl::types::MessageEntityMentionName {
            offset, length, ..
        }) => Some((offset, length)),
        MessageEntity::InputMessageEntityMentionName(
            tl::types::InputMessageEntityMentionName { offset, length, .. },
        ) => Some((offset, length)),
        MessageEntity::Phone(tl::types::MessageEntityPhone { offset, length }) => {
            Some((offset, length))
        }
        MessageEntity::Cashtag(tl::types::MessageEntityCashtag { offset, length }) => {
            Some((offset, length))
        }
        MessageEntity::Underline(tl::types::MessageEntityUnderline { offset, length }) => {
            Some((offset, length))
        }
        MessageEntity::Strike(tl::types::MessageEntityStrike { offset, length }) => {
            Some((offset, length))
        }
        MessageEntity::BankCard(tl::types::MessageEntityBankCard { offset, length }) => {
            Some((offset, length))
        }
        MessageEntity::Spoiler(tl::types::MessageEntitySpoiler { offset, length }) => {
            Some((offset, length))
        }
        MessageEntity::CustomEmoji(tl::types::MessageEntityCustomEmoji {
            offset, length, ..
        }) => Some((offset, length)),
        MessageEntity::Blockquote(tl::types::MessageEntityBlockquote {
            offset, length, ..
        }) => Some((offset, length)),
    };
    if let Some((offset, length)) = result {
        let (l, r) = utf16_range_to_utf8(msg, *offset as usize, *length as usize)?;
        return Ok(Some(&msg[l..r]));
    }
    Ok(None)
}

pub fn extract_mentioned_users(
    msg: &str,
    msg_entities: &[MessageEntity],
) -> Result<(HashSet<String>, HashSet<i64>)> {
    let mut user_ids = HashSet::new();
    let mentions = msg_entities
        .into_iter()
        .filter_map(|ent| match ent {
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
        .filter_map(|(offset, length)| utf16_range_to_utf8(&msg, offset, length).ok()) // 获取以utf8计算的字节长度
        .filter_map(|(start, end)| msg.get(start..end)) // 截取用户名部分
        .filter_map(|x| x.get(1..)) // 删除@键
        .map(|x| x.to_lowercase()) // 转换小写
        .map(|x| x.to_string())
        .collect();
    Ok((mentions, user_ids))
}
/// 把 UTF-16 的 [offset, offset+len) 区间映射成 UTF-8 字节区间 [ret.0, ret.1)
///
/// 返回 `Ok((byte_start, byte_end))`，如果越界则返回 `Err`。
pub fn utf16_range_to_utf8(s: &str, offset: usize, len: usize) -> Result<(usize, usize)> {
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

    let start = utf16_to_byte_idx(offset).ok_or(anyhow!("invalid utf16 offset"))?;
    let end = utf16_to_byte_idx(offset + len).ok_or(anyhow!("invalid utf16 offset, length"))?;
    Ok((start, end))
}
