use revolt_config::config;
use revolt_database::{Database, Member, Server, User};
use revolt_models::v0;
use revolt_result::{create_error, Result};

use rocket::serde::json::Json;
use rocket::State;
use validator::Validate;

/// # Create Server
///
/// Create a new server.
#[openapi(tag = "Server Information")]
#[post("/create", data = "<data>")]
pub async fn create_server(
    db: &State<Database>,
    user: User,
    data: Json<v0::DataCreateServer>,
) -> Result<Json<v0::CreateServerLegacyResponse>> {
    if user.bot.is_some() {
        return Err(create_error!(IsBot));
    }

    let config = config().await;

    let settings = db.fetch_admin_settings().await?.unwrap_or_default();
    let config_restricts_creation = !config
        .features
        .limits
        .global
        .restrict_server_creation
        .is_empty();
    let admin_restricts_creation = settings.server_creation.restricted;
    let user_can_create_by_config = config
        .features
        .limits
        .global
        .restrict_server_creation
        .contains(&user.id);
    let user_can_create_by_admin =
        user.is_default_admin(db).await? || settings.can_create_server(&user.id);

    if (config_restricts_creation || admin_restricts_creation)
        && !user_can_create_by_config
        && !user_can_create_by_admin
    {
        return Err(create_error!(CantCreateServers));
    }

    let data = data.into_inner();
    data.validate().map_err(|error| {
        create_error!(FailedValidation {
            error: error.to_string()
        })
    })?;

    user.can_acquire_server(db).await?;

    let (server, channels) = Server::create(db, data, &user, true).await?;
    let (_, channels) = Member::create(db, &server, &user, Some(channels)).await?;

    Ok(Json(v0::CreateServerLegacyResponse {
        server: server.into(),
        channels: channels.into_iter().map(|channel| channel.into()).collect(),
    }))
}
