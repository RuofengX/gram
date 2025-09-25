use anyhow::{Result, anyhow};
use gram_type::entity::{global_api_config, prelude::*, user_account};
use sea_orm::{ConnectionTrait, EntityTrait};

pub async fn fetch_config(
    conn: &impl ConnectionTrait,
) -> Result<(global_api_config::Model, user_account::Model)> {
    let ret = (
        GlobalApiConfig::find()
            .one(conn)
            .await?
            .ok_or(anyhow!("no api config found in db"))?,
        UserAccount::find()
            .one(conn)
            .await?
            .ok_or(anyhow!("no user account found in db"))?,
    );
    Ok(ret)
}
