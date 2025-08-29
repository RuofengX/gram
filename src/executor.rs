use std::collections::HashMap;

use uuid::Uuid;

use crate::scraper::Scraper;

pub struct Executor {
    scrapers: HashMap<Uuid, Scraper>,
}
