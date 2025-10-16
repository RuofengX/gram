use crate::{scraper::Scraper, serveless::general::now, stdin_read_line};
use anyhow::{Result, anyhow};
use gram_type::{
    ApiConfig, FrozenSession,
    entity::{prelude::*, user_scraper},
};
use sea_orm::{
    ActiveValue::{NotSet, Set},
    IntoActiveModel, QueryOrder, TransactionTrait,
};
use tokio::sync::oneshot;
use tracing::{debug, info, warn};
use uuid::Uuid;

pub async fn login_async(
    api_config: ApiConfig,
    phone: String,
    code: oneshot::Receiver<String>,
) -> Result<FrozenSession> {
    let scraper = Scraper::login_async(api_config, &phone, code).await?;
    Ok(scraper.freeze())
}

pub async fn create_scraper_from_stdin(conn: &impl ConnectionTrait) -> Result<(Uuid, Scraper)> {
    info!("从stdin输入验证码登录以创建爬虫会话");

    debug!("get global_api_config");
    let api_config = GlobalApiConfig::find()
        .one(conn)
        .await?
        .ok_or(anyhow!("global_api_config not found"))?;
    let config_id = api_config.id;

    debug!("get user_account");
    let account = UserAccount::find()
        .one(conn)
        .await?
        .ok_or(anyhow!("user_account not found"))?;
    let phone = account.phone.clone();

    debug!("get confirm code from stdin");
    let code = stdin_read_line(format!("请输入TG号为{}的验证码", phone));

    debug!("create scraper");
    let scraper = Scraper::login_async(api_config.into(), &phone, code).await?;
    let frozen = scraper.freeze();

    debug!("insert scraper in db");
    let scraper_model = user_scraper::ActiveModel {
        id: NotSet,
        updated_at: NotSet,
        api_config: Set(config_id),
        account: Set(account.id),
        frozen_session: Set(frozen),
        in_use: Set(true),
    };
    let id = scraper_model.insert(conn).await?.id;

    Ok((id, scraper))
}

pub async fn resume_scraper(db: &impl TransactionTrait) -> Result<Option<(Uuid, Scraper)>> {
    info!("获取冻结会话");
    let trans = db.begin().await?;
    let ret = if let Some(scraper) = UserScraper::find()
        .filter(user_scraper::Column::InUse.eq(false)) // 选择未在使用的会话
        .order_by_desc(user_scraper::Column::UpdatedAt) // 始终选择最新的会话
        .one(&trans)
        .await?
    {
        debug!("{:?}", scraper.frozen_session);

        debug!("get relate api_config");
        let api_config = scraper
            .find_related(GlobalApiConfig)
            .one(&trans)
            .await?
            .ok_or(anyhow!("relate global_api_config not found"))?;
        debug!("{:?}", api_config);

        debug!("create scraper instance");
        info!("尝试启用会话: {}", scraper.id);
        let s = Scraper::unfreeze(scraper.frozen_session.clone(), api_config.into()).await?;

        debug!("set db scraper state");
        let mut scraper = scraper.into_active_model();
        scraper.in_use = Set(true);
        let id = scraper.update(&trans).await?.id;
        trans.commit().await?;

        Some((id, s))
    } else {
        warn!("未找到暂停的会话");
        None
    };
    Ok(ret)
}

pub async fn exit_scraper(
    db: &impl ConnectionTrait,
    scraper_id: Uuid,
    scraper: Scraper,
) -> Result<()> {
    info!("退出爬虫，更新数据库状态");

    debug!("freeze scraper instence");
    let frozen = scraper.freeze();
    let scraper_update = user_scraper::ActiveModel {
        id: Set(scraper_id),
        updated_at: Set(now()),
        api_config: NotSet,
        account: NotSet,
        frozen_session: Set(frozen),
        in_use: Set(false),
    };
    debug!("update scraper in db");
    scraper_update.update(db).await?;
    Ok(())
}
