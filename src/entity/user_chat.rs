use crate::types::PackedChat;
use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq)]
#[sea_orm(table_name = "user_chat")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Uuid,
    pub updated_at: DateTimeWithTimeZone,
    pub user_scraper: Uuid,
    #[sea_orm(column_type = "JsonBinary")]
    pub packed_chat: PackedChat,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::user_scraper::Entity",
        from = "Column::UserScraper",
        to = "super::user_scraper::Column::Id",
        on_update = "NoAction",
        on_delete = "NoAction"
    )]
    UserScraper,
}

impl Related<super::user_scraper::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::UserScraper.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
