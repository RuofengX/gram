
use anyhow::{Result, anyhow};
use crate::entity::{global_api_config, prelude::*, user_account, user_confirm};
use sea_orm::{ConnectionTrait, EntityTrait};
use uuid::Uuid;

pub async fn fetch_one(
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