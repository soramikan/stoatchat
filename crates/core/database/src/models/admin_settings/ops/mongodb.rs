use mongodb::options::ReplaceOptions;
use revolt_result::Result;

use crate::{AdminSettings, MongoDb, ADMIN_SETTINGS_ID};

use super::AbstractAdminSettings;

static COL: &str = "admin_settings";

#[async_trait]
impl AbstractAdminSettings for MongoDb {
    async fn fetch_admin_settings(&self) -> Result<Option<AdminSettings>> {
        query!(self, find_one_by_id, COL, ADMIN_SETTINGS_ID)
    }

    async fn upsert_admin_settings(&self, settings: &AdminSettings) -> Result<()> {
        self.col::<AdminSettings>(COL)
            .replace_one(doc! { "_id": ADMIN_SETTINGS_ID }, settings)
            .with_options(ReplaceOptions::builder().upsert(true).build())
            .await
            .map(|_| ())
            .map_err(|_| create_database_error!("replace_one", COL))
    }
}
