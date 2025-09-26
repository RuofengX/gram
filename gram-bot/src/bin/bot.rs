use anyhow::{Result, anyhow};
use gram_type::entity::{esse_username_full, prelude::*};
use regex::Regex;
use teloxide::{
    prelude::*,
    types::{MessageEntity, MessageEntityKind},
};
use tracing::{error, info, warn};

#[tokio::main]
async fn main() -> Result<()> {
    gram_core::log::init_tracing();

    info!("启动机器人");

    dotenv::dotenv().unwrap();
    let token = dotenv::var("TELOXIDE_TOKEN".to_owned())?;
    let bot = Bot::new(token);

    teloxide::repl(bot, |bot: Bot, msg: Message| async move {
        match parse(bot, msg).await {
            Ok(()) => {}
            Err(e) => {
                error!("处理消息时发生错误: {}", e);
            }
        };
        Ok(())
    })
    .await;

    Ok(())
}

async fn parse(_bot: Bot, msg: Message) -> Result<()> {
    let mut usernames = Vec::new();
    let text = msg.text().ok_or(anyhow!("消息无内容"))?;

    // 获取链接中的username
    fetch_username_in_link(&mut usernames, text)?;

    // 获取entities标注的username
    if let Some(entities) = msg.entities() {
        fetch_mention_username(&mut usernames, text, entities)?;
    }

    send_usernames(usernames).await?;
    Ok(())
}

fn fetch_mention_username(
    usernames: &mut Vec<String>,
    text: &str,
    entities: &[MessageEntity],
) -> Result<()> {
    for ent in entities {
        if matches!(&ent.kind, MessageEntityKind::Mention)
            || matches!(&ent.kind, MessageEntityKind::TextMention { .. })
        {
            let (start, end) =
                gram_core::mention::convert::utf16_range_to_utf8(text, ent.offset, ent.length)
                    .ok_or(anyhow!("消息内包含无效Mention实体"))?;
            let username = text
                .get(start..end)
                .ok_or(anyhow!("Mention实体范围越界"))?
                .to_string();
            checked_push(usernames, username);
        }
        if let MessageEntityKind::TextLink { url } = &ent.kind {
            if let Some(mut path_iter) = url.path_segments() {
                if let Some(username) = path_iter.next() {
                    checked_push(usernames, username.to_string());
                }
            }
        }
    }
    Ok(())
}

fn fetch_username_in_link(usernames: &mut Vec<String>, text: &str) -> Result<()> {
    let re = Regex::new(r"t\.me/([^/]+)").unwrap();
    re.find_iter(text)
        .map(|x| x.as_str().to_string())
        .for_each(|x| checked_push(usernames, x));
    Ok(())
}

fn checked_push(usernames: &mut Vec<String>, value: String) {
    if !value.starts_with("+") && value.len() > 0 {
        info!("发现用户名: {}", value);
        usernames.push(value);
    }
}

async fn send_usernames(usernames: Vec<String>) -> Result<()> {
    if usernames.len() == 0 {
        return Ok(());
    }
    let updates = usernames
        .into_iter()
        .map(|username| esse_username_full::ActiveModel {
            id: NotSet,
            updated_at: NotSet,
            source: NotSet,
            username: Set(username),
            is_valid: Set(None),
        })
        .collect::<Vec<esse_username_full::ActiveModel>>();

    warn!("录入{}个频道名", updates.len());
    let db = gram_type::entity::connect_db().await?;
    esse_username_full::Entity::insert_many(updates)
        .exec(&db)
        .await?;

    Ok(())
}
