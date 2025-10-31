use grammers_client::grammers_tl_types as tl;
use std::collections::HashSet;
use tl::enums::MessageEntity;

pub fn extract_text_url(msg_entities: &[MessageEntity]) -> HashSet<String> {
    msg_entities
        .iter()
        .flat_map(|ent| match ent {
            MessageEntity::TextUrl(tl::types::MessageEntityTextUrl { url, .. }) => {
                super::deeplink::get_username(url)
            }
            _ => None,
        })
        .collect::<HashSet<_>>()
}

