use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

use crate::types::{ChannelFull, PackedChat, UserFull};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq, Serialize, Deserialize)]
#[sea_orm(table_name = "user_chat")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Uuid,
    pub updated_at: DateTimeWithTimeZone,
    pub user_id: i64, // 引用到user_chat的id
    #[sea_orm(nullable)]
    pub user_full: Option<UserFull>, // 存在无用户名的聊天, 用户没有设置即无用户名
    #[sea_orm(nullable)]
    pub channel_full: Option<ChannelFull>, // 存在无用户名的聊天, 用户没有设置即无用户名
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation{
    
}
