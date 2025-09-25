use anyhow::{Result, anyhow};
use teloxide::{RequestError, prelude::*, types::MessageEntityKind};
use tracing::{error, info};

#[tokio::main]
async fn main() -> Result<()> {
    gram_core::log::init_tracing();

    info!("启动机器人");

    let bot = Bot::from_env();

    teloxide::repl(bot, |bot: Bot, msg: Message| async move {
        match parse(bot, msg).await {
            Ok(()) => Ok(()),
            Err(e) => {
                error!("处理消息时发生错误: {}", e);
                Ok(())
            }
        }
    })
    .await;

    Ok(())
}

async fn parse(bot: Bot, msg: Message) -> Result<()> {
    let mut username_list = Vec::new();
    let text = msg.text().ok_or(anyhow!("消息无内容"))?;
    if let Some(entities) = msg.entities() {
        for ent in entities {
            if matches!(ent.kind, MessageEntityKind::Mention)
                || matches!(ent.kind, MessageEntityKind::TextMention { .. })
            {
                let (start, end) =
                    gram_core::mention::convert::utf16_range_to_utf8(text, ent.offset, ent.length)
                        .ok_or(anyhow!("消息内包含无效Mention实体"))?;
                let username = text
                    .get(start..end)
                    .ok_or(anyhow!("Mention实体范围越界"))?
                    .to_string();
                info!("发现用户名: {}", username);
                username_list.push(username);
            }
        }
        username_list
    } 
    Ok(())
}
