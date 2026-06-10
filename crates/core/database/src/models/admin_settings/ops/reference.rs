use revolt_result::Result;

use crate::{AdminSettings, ReferenceDb, ADMIN_SETTINGS_ID};

use super::AbstractAdminSettings;

#[async_trait]
impl AbstractAdminSettings for ReferenceDb {
    async fn fetch_admin_settings(&self) -> Result<Option<AdminSettings>> {
        let admin_settings = self.admin_settings.lock().await;
        Ok(admin_settings.get(ADMIN_SETTINGS_ID).cloned())
    }

    async fn upsert_admin_settings(&self, settings: &AdminSettings) -> Result<()> {
        let mut admin_settings = self.admin_settings.lock().await;
        admin_settings.insert(ADMIN_SETTINGS_ID.to_string(), settings.clone());
        Ok(())
    }
}
