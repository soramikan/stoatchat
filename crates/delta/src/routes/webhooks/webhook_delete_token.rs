use crate::routes::require_channel_server_not_frozen;
use revolt_database::{util::reference::Reference, Database};
use revolt_result::Result;
use rocket::State;
use rocket_empty::EmptyResponse;

/// # Deletes a webhook
///
/// Deletes a webhook with a token
#[openapi(tag = "Webhooks")]
#[delete("/<webhook_id>/<token>")]
pub async fn webhook_delete_token(
    db: &State<Database>,
    webhook_id: Reference<'_>,
    token: String,
) -> Result<EmptyResponse> {
    let webhook = webhook_id.as_webhook(db).await?;
    webhook.assert_token(&token)?;
    let channel = db.fetch_channel(&webhook.channel_id).await?;
    require_channel_server_not_frozen(db, &channel).await?;

    webhook.delete(db).await.map(|_| EmptyResponse)
}
