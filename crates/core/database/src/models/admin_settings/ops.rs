use revolt_result::Result;

use crate::AdminSettings;

#[cfg(feature = "mongodb")]
mod mongodb;
mod reference;

#[async_trait]
pub trait AbstractAdminSettings: Sync + Send {
    async fn fetch_admin_settings(&self) -> Result<Option<AdminSettings>>;

    async fn upsert_admin_settings(&self, settings: &AdminSettings) -> Result<()>;
}
