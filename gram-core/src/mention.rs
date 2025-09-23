mod convert;

use crate::{format::deserialize_msg, mention::convert::utf16_range_to_utf8};
use anyhow::Result;
use grammers_tl_types as tl;
use std::collections::HashSet;

/// 输入json格式的tl::enums::Message  
/// 分析其内容是否包含entities为Mention与mentionName标记,
/// 如果有则提取用户名或用户ID  
///
/// 返回两个列表，分别包含用户名和用户ID
///
/// 相关文档: https://core.telegram.org/api/entities
pub fn get_mentioned(msg: &str) -> Result<(HashSet<String>, HashSet<i64>)> {
    let msg = deserialize_msg(msg)?;
    let mut ret_user_id = HashSet::new();
    let mentions = match msg {
        tl::enums::Message::Message(msg) => {
            if let Some(entities) = msg.entities {
                entities
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
                        tl::enums::MessageEntity::Mention(tl::types::MessageEntityMention {
                            offset, // 长度单位为UTF-16字符长
                            length,
                        }) => Some((offset as usize, length as usize)),
                        tl::enums::MessageEntity::MentionName(
                            tl::types::MessageEntityMentionName {
                                offset,
                                length,
                                user_id,
                            },
                        ) => {
                            ret_user_id.insert(user_id);
                            Some((offset as usize, length as usize))
                        }
                        _ => None,
                    })
                    // 长度越界直接忽略
                    .flat_map(|(offset, length)| {
                        utf16_range_to_utf8(&msg.message, offset as usize, length as usize)
                    }) // 获取以utf8计算的字节长度
                    .flat_map(|(start, end)| msg.message.get(start..end)) // 截取用户名部分
                    .flat_map(|x| x.get(1..)) // 删除@键
                    .map(|x| x.to_lowercase()) // 转换小写
                    .map(|x| x.to_string())
                    .collect()
            } else {
                Default::default()
            }
        }
        _ => Default::default(),
    };
    Ok((mentions, ret_user_id))
}
