use grammers_tl_types::enums::MessageEntity;
use anyhow::Result;
use grammers_client::grammers_tl_types as tl;
use serde::{Deserialize, Serialize};

/// 将telethon的entities的json列表转换为grammers的entities
pub fn deserialize_telethon_entities(entities: &str) -> Result<Vec<MessageEntity>> {
    let entities: Vec<TelethonEntity> = serde_json::from_str(entities)?;
    let ret = entities.into_iter().map(|x| x.into()).collect();
    Ok(ret)
}

/// 将telethon的entity的json对象转换为grammers的entity
pub fn deserialize_telethon_entity(entity: &str) -> Result<MessageEntity> {
    let ret: TelethonEntity = serde_json::from_str(entity)?;
    Ok(ret.into())
}

#[derive(Serialize, Deserialize)]
#[serde(tag = "_")]
enum TelethonEntity {
    MessageEntityUnknown(tl::types::MessageEntityUnknown),
    MessageEntityMention(tl::types::MessageEntityMention),
    MessageEntityHashtag(tl::types::MessageEntityHashtag),
    MessageEntityBotCommand(tl::types::MessageEntityBotCommand),
    MessageEntityUrl(tl::types::MessageEntityUrl),
    MessageEntityEmail(tl::types::MessageEntityEmail),
    MessageEntityBold(tl::types::MessageEntityBold),
    MessageEntityItalic(tl::types::MessageEntityItalic),
    MessageEntityCode(tl::types::MessageEntityCode),
    MessageEntityPre(tl::types::MessageEntityPre),
    MessageEntityTextUrl(tl::types::MessageEntityTextUrl),
    MessageEntityMentionName(tl::types::MessageEntityMentionName),
    MessageEntityInputMessageEntityMentionName(tl::types::InputMessageEntityMentionName),
    MessageEntityPhone(tl::types::MessageEntityPhone),
    MessageEntityCashtag(tl::types::MessageEntityCashtag),
    MessageEntityUnderline(tl::types::MessageEntityUnderline),
    MessageEntityStrike(tl::types::MessageEntityStrike),
    MessageEntityBankCard(tl::types::MessageEntityBankCard),
    MessageEntitySpoiler(tl::types::MessageEntitySpoiler),
    MessageEntityCustomEmoji(tl::types::MessageEntityCustomEmoji),
    MessageEntityBlockquote(tl::types::MessageEntityBlockquote),
}
impl Into<MessageEntity> for TelethonEntity {
    fn into(self) -> MessageEntity {
        match self {
            TelethonEntity::MessageEntityUnknown(unknown) => MessageEntity::Unknown(unknown),
            TelethonEntity::MessageEntityMention(mention) => MessageEntity::Mention(mention),
            TelethonEntity::MessageEntityHashtag(hashtag) => MessageEntity::Hashtag(hashtag),
            TelethonEntity::MessageEntityBotCommand(bc) => MessageEntity::BotCommand(bc),
            TelethonEntity::MessageEntityUrl(url) => MessageEntity::Url(url),
            TelethonEntity::MessageEntityEmail(email) => MessageEntity::Email(email),
            TelethonEntity::MessageEntityBold(b) => MessageEntity::Bold(b),
            TelethonEntity::MessageEntityItalic(i) => MessageEntity::Italic(i),
            TelethonEntity::MessageEntityCode(code) => MessageEntity::Code(code),
            TelethonEntity::MessageEntityPre(pre) => MessageEntity::Pre(pre),
            TelethonEntity::MessageEntityTextUrl(text_url) => MessageEntity::TextUrl(text_url),
            TelethonEntity::MessageEntityMentionName(mention) => {
                MessageEntity::MentionName(mention)
            }
            TelethonEntity::MessageEntityInputMessageEntityMentionName(
                input_message_entity_mention_name,
            ) => MessageEntity::InputMessageEntityMentionName(input_message_entity_mention_name),
            TelethonEntity::MessageEntityPhone(phone) => MessageEntity::Phone(phone),
            TelethonEntity::MessageEntityCashtag(c) => MessageEntity::Cashtag(c),
            TelethonEntity::MessageEntityUnderline(u) => MessageEntity::Underline(u),
            TelethonEntity::MessageEntityStrike(s) => MessageEntity::Strike(s),
            TelethonEntity::MessageEntityBankCard(bc) => MessageEntity::BankCard(bc),
            TelethonEntity::MessageEntitySpoiler(spo) => MessageEntity::Spoiler(spo),
            TelethonEntity::MessageEntityCustomEmoji(e) => MessageEntity::CustomEmoji(e),
            TelethonEntity::MessageEntityBlockquote(b) => MessageEntity::Blockquote(b),
        }
    }
}
