use crate::routes::require_server_not_frozen;
use revolt_database::{
    util::{permissions::DatabasePermissionQuery, reference::Reference},
    Database, Invite, User,
};
use revolt_permissions::{calculate_server_permissions, ChannelPermission};
use revolt_result::Result;
use rocket::State;
use rocket_empty::EmptyResponse;

/// # Delete Invite
///
/// Delete an invite by its id.
#[openapi(tag = "Invites")]
#[delete("/<target>")]
pub async fn delete(
    db: &State<Database>,
    user: User,
    target: Reference<'_>,
) -> Result<EmptyResponse> {
    let invite = target.as_invite(db).await?;

    if user.id == invite.creator() {
        if let Invite::Server { server, .. } = &invite {
            require_server_not_frozen(db, server).await?;
        }

        db.delete_invite(invite.code()).await
    } else {
        match invite {
            Invite::Server { code, server, .. } => {
                require_server_not_frozen(db, &server).await?;

                let server = db.fetch_server(&server).await?;
                let mut query = DatabasePermissionQuery::new(db, &user).server(&server);
                calculate_server_permissions(&mut query)
                    .await
                    .throw_if_lacking_channel_permission(ChannelPermission::ManageServer)?;

                db.delete_invite(&code).await
            }
            _ => unreachable!(),
        }
    }
    .map(|_| EmptyResponse)
}
