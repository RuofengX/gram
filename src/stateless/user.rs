use crate::types::ApiConfig;
use anyhow::{Result, anyhow};
use grammers_client::{grammers_tl_types as tl, InvocationError};
use uuid::Uuid;

use crate::scraper::Scraper;

pub async fn request_login(api_config: ApiConfig, phone: String) -> Result<()> {

}
