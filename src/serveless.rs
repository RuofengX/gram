use crate::{
    entity::{
        prelude::{GlobalApiConfig, UserAccount, UserScraper},
        user_scraper,
    },
    scraper::Scraper,
    stdin_read_line,
    types::{ApiConfig, FrozenSession},
};
use anyhow::Result;
use anyhow::anyhow;
use sea_orm::{
    ActiveValue::{NotSet, Set},
    Database, IntoActiveModel,
    prelude::*,
};
use tokio::sync::oneshot;
use tracing::{debug, info, warn};

pub async fn login_async(
    api_config: ApiConfig,
    phone: String,
    code: oneshot::Receiver<String>,
) -> Result<FrozenSession> {
    let scraper = Scraper::login_async(api_config, &phone, code).await?;
    Ok(scraper.freeze())
}

pub async fn unfreeze_and<R>(
    frozen: FrozenSession,
    api_config: ApiConfig,
    with: impl Fn(&Scraper) -> Result<R>,
) -> Result<R> {
    let scraper = Scraper::unfreeze(frozen, api_config).await?;
    with(&scraper)
}

pub async fn connect_db() -> Result<DatabaseConnection> {
    dotenv::dotenv().unwrap();
    let url = dotenv::var("DATABASE_URL".to_owned())?;
    let db = Database::connect(url).await?;
    Ok(db)
}

pub async fn resume_scraper(db: &impl ConnectionTrait) -> Result<Option<Scraper>> {
    info!("获取暂停会话");
    let ret = if let Some(scraper) = UserScraper::find()
        .filter(user_scraper::Column::InUse.eq(false))
        .one(db)
        .await?
    {
        debug!("{:?}", scraper.frozen_session);

        debug!("get relate api_config");
        let api_config = scraper
            .find_related(GlobalApiConfig)
            .one(db)
            .await?
            .ok_or(anyhow!("relate global_api_config not found"))?;
        debug!("{:?}", api_config);

        debug!("create scraper instance");
        let s = Scraper::unfreeze(scraper.frozen_session.clone(), api_config.into()).await?;

        debug!("set db scraper state");
        let mut scraper = scraper.into_active_model();
        scraper.in_use = Set(true);
        scraper.update(db).await?;

        Some(s)
    } else {
        warn!("未找到暂停的会话");
        None
    };
    Ok(ret)
}

pub async fn create_scraper_from_stdin(db: &impl ConnectionTrait) -> Result<(Uuid, Scraper)> {
    info!("从stdin输入验证码登录以创建爬虫会话");

    debug!("get global_api_config");
    let api_config = GlobalApiConfig::find()
        .one(db)
        .await?
        .ok_or(anyhow!("global_api_config not found"))?;
    let config_id = api_config.id;

    debug!("get user_account");
    let account = UserAccount::find()
        .one(db)
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
        api_config: Set(config_id),
        account: Set(account.id),
        frozen_session: Set(frozen),
        in_use: Set(true),
        ..Default::default()
    };
    let uuid = scraper_model.insert(db).await?.id;

    Ok((uuid, scraper))
}
