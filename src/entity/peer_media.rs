use crate::types::{FileType, MessageMedia};
use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq)]
#[sea_orm(table_name = "peer_media")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Uuid,
    pub updated_at: DateTimeWithTimeZone,
    pub user_scraper: Uuid,
    pub history: Uuid,
    #[sea_orm(column_type = "JsonBinary")]
    pub message_media: MessageMedia,
    #[sea_orm(column_type = "JsonBinary", nullable)]
    pub file_type: Option<FileType>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(has_many = "super::peer_file_part::Entity")]
    PeerFilePart,
    #[sea_orm(
        belongs_to = "super::peer_history::Entity",
        from = "Column::History",
        to = "super::peer_history::Column::Id",
        on_update = "NoAction",
        on_delete = "NoAction"
    )]
    PeerHistory,
    #[sea_orm(
        belongs_to = "super::user_scraper::Entity",
        from = "Column::UserScraper",
        to = "super::user_scraper::Column::Id",
        on_update = "NoAction",
        on_delete = "NoAction"
    )]
    UserScraper,
}

impl Related<super::peer_file_part::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::PeerFilePart.def()
    }
}

impl Related<super::peer_history::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::PeerHistory.def()
    }
}

impl Related<super::user_scraper::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::UserScraper.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
