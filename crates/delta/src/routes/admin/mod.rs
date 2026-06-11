use authifier::{models::PasswordReset, Authifier};
use iso8601_timestamp::Timestamp;
use revolt_database::{
    AdminServerOverride, AdminSettings, AdminUserOverride, Database, PartialUser, Server, User,
    ADMIN_PERMISSION_CREATE_SERVERS, ADMIN_PERMISSION_MANAGE_ADMIN,
    ADMIN_PERMISSION_MANAGE_SERVERS, ADMIN_PERMISSION_MANAGE_UPLOAD_LIMITS,
    ADMIN_PERMISSION_MANAGE_USERS,
};
use revolt_result::{create_error, Result};
use revolt_rocket_okapi::revolt_okapi::openapi3::OpenApi;
use rocket::serde::{json::Json, Deserialize, Serialize};
use rocket::{Route, State};
use rocket_empty::EmptyResponse;

#[derive(Serialize)]
struct AdminUserEntry {
    user: User,
    default_admin: bool,
    permissions: Vec<String>,
}

#[derive(Serialize)]
struct AdminServerEntry {
    server: Server,
    state: Option<AdminServerOverride>,
}

#[derive(Deserialize)]
struct UpdateSettingsPayload {
    settings: AdminSettings,
}

#[derive(Deserialize)]
struct RenameUserPayload {
    username: String,
}

#[derive(Deserialize)]
struct SuspendUserPayload {
    duration_days: Option<usize>,
    reason: Option<Vec<String>>,
}

#[derive(Deserialize)]
struct ServerTimeoutPayload {
    timeout_until: Option<Timestamp>,
}

#[derive(Serialize)]
struct PasswordResetResponse {
    token: String,
    expires_at: Timestamp,
}

pub fn routes() -> (Vec<Route>, OpenApi) {
    (
        routes![
            fetch_settings,
            update_settings,
            list_users,
            rename_user,
            suspend_user,
            unsuspend_user,
            reset_password,
            delete_user,
            list_servers,
            freeze_server,
            unfreeze_server,
            delete_server,
        ],
        OpenApi::default(),
    )
}

async fn require_any_admin_permission(
    db: &Database,
    user: &User,
    permissions: &[&str],
) -> Result<()> {
    for permission in permissions {
        if user.has_admin_permission(db, permission).await? {
            return Ok(());
        }
    }

    Err(create_error!(MissingUserPermission {
        permission: permissions.join(",")
    }))
}

fn merge_upload_limit_settings(
    mut current: AdminSettings,
    incoming: AdminSettings,
) -> AdminSettings {
    current.default_upload_limits = incoming.default_upload_limits;

    for (role_id, incoming_role) in incoming.roles {
        if let Some(role) = current.roles.get_mut(&role_id) {
            role.upload_limits = incoming_role.upload_limits;
        }
    }

    for (user_id, incoming_user) in incoming.users {
        let user = current.users.entry(user_id).or_insert(AdminUserOverride {
            roles: Vec::new(),
            permissions: Vec::new(),
            upload_limits: Default::default(),
        });

        user.upload_limits = incoming_user.upload_limits;
    }

    current
}

#[get("/settings")]
async fn fetch_settings(db: &State<Database>, user: User) -> Result<Json<AdminSettings>> {
    require_any_admin_permission(
        db,
        &user,
        &[
            ADMIN_PERMISSION_MANAGE_ADMIN,
            ADMIN_PERMISSION_MANAGE_USERS,
            ADMIN_PERMISSION_MANAGE_SERVERS,
            ADMIN_PERMISSION_MANAGE_UPLOAD_LIMITS,
        ],
    )
    .await?;

    Ok(Json(db.fetch_admin_settings().await?.unwrap_or_default()))
}

#[put("/settings", data = "<data>")]
async fn update_settings(
    db: &State<Database>,
    user: User,
    data: Json<UpdateSettingsPayload>,
) -> Result<EmptyResponse> {
    let mut settings = data.into_inner().settings;
    settings.id = revolt_database::ADMIN_SETTINGS_ID.to_string();

    if user
        .has_admin_permission(db, ADMIN_PERMISSION_MANAGE_ADMIN)
        .await?
    {
        db.upsert_admin_settings(&settings).await?;
    } else if user
        .has_admin_permission(db, ADMIN_PERMISSION_MANAGE_UPLOAD_LIMITS)
        .await?
    {
        let current = db.fetch_admin_settings().await?.unwrap_or_default();
        let upload_limit_settings = merge_upload_limit_settings(current, settings);
        db.upsert_admin_settings(&upload_limit_settings).await?;
    } else {
        return Err(create_error!(MissingUserPermission {
            permission: ADMIN_PERMISSION_MANAGE_ADMIN.to_string()
        }));
    }

    Ok(EmptyResponse)
}

#[get("/users")]
async fn list_users(db: &State<Database>, user: User) -> Result<Json<Vec<AdminUserEntry>>> {
    user.require_admin_permission(db, ADMIN_PERMISSION_MANAGE_USERS)
        .await?;

    let settings = db.fetch_admin_settings().await?.unwrap_or_default();
    let bootstrap_admin_id = User::bootstrap_admin_id(db).await?;
    let users = db
        .fetch_all_users()
        .await?
        .into_iter()
        .map(|listed_user| {
            let default_admin = bootstrap_admin_id.as_ref() == Some(&listed_user.id);
            let mut permissions = if default_admin {
                vec![
                    ADMIN_PERMISSION_CREATE_SERVERS.to_string(),
                    ADMIN_PERMISSION_MANAGE_ADMIN.to_string(),
                    ADMIN_PERMISSION_MANAGE_USERS.to_string(),
                    ADMIN_PERMISSION_MANAGE_SERVERS.to_string(),
                    ADMIN_PERMISSION_MANAGE_UPLOAD_LIMITS.to_string(),
                ]
            } else {
                settings
                    .user_permissions(&listed_user.id)
                    .into_iter()
                    .collect::<Vec<_>>()
            };
            permissions.sort();

            AdminUserEntry {
                user: listed_user,
                default_admin,
                permissions,
            }
        })
        .collect();

    Ok(Json(users))
}

#[patch("/users/<target>/username", data = "<data>")]
async fn rename_user(
    db: &State<Database>,
    user: User,
    target: String,
    data: Json<RenameUserPayload>,
) -> Result<Json<User>> {
    user.require_admin_permission(db, ADMIN_PERMISSION_MANAGE_USERS)
        .await?;

    let mut target_user = db.fetch_user(&target).await?;
    target_user
        .update_username(db, data.into_inner().username)
        .await?;
    Ok(Json(target_user))
}

#[post("/users/<target>/suspend", data = "<data>")]
async fn suspend_user(
    db: &State<Database>,
    user: User,
    target: String,
    data: Json<SuspendUserPayload>,
) -> Result<EmptyResponse> {
    user.require_admin_permission(db, ADMIN_PERMISSION_MANAGE_USERS)
        .await?;

    if user.id == target {
        return Err(create_error!(CannotTimeoutYourself));
    }

    let data = data.into_inner();
    let mut target_user = db.fetch_user(&target).await?;
    target_user
        .suspend(db, data.duration_days, data.reason)
        .await?;

    Ok(EmptyResponse)
}

#[post("/users/<target>/unsuspend")]
async fn unsuspend_user(
    db: &State<Database>,
    authifier: &State<Authifier>,
    user: User,
    target: String,
) -> Result<EmptyResponse> {
    user.require_admin_permission(db, ADMIN_PERMISSION_MANAGE_USERS)
        .await?;

    let mut target_user = db.fetch_user(&target).await?;
    target_user
        .update(
            db,
            PartialUser {
                flags: Some(0),
                suspended_until: None,
                ..Default::default()
            },
            vec![],
        )
        .await?;

    if let Ok(mut account) = authifier.database.find_account(&target).await {
        account.disabled = false;
        account
            .save(authifier)
            .await
            .map_err(|_| create_error!(InternalError))?;
    }

    Ok(EmptyResponse)
}

#[post("/users/<target>/password-reset")]
async fn reset_password(
    db: &State<Database>,
    authifier: &State<Authifier>,
    user: User,
    target: String,
) -> Result<Json<PasswordResetResponse>> {
    user.require_admin_permission(db, ADMIN_PERMISSION_MANAGE_USERS)
        .await?;

    db.fetch_user(&target).await?;

    let token = nanoid::nanoid!(32);
    let expires_at = Timestamp::now_utc()
        .checked_add(iso8601_timestamp::Duration::days(1))
        .ok_or_else(|| create_error!(InternalError))?;
    let mut account = authifier
        .database
        .find_account(&target)
        .await
        .map_err(|_| create_error!(InternalError))?;
    account.password_reset = Some(PasswordReset {
        token: token.clone(),
        expiry: expires_at,
    });
    account
        .save(authifier)
        .await
        .map_err(|_| create_error!(InternalError))?;

    Ok(Json(PasswordResetResponse { token, expires_at }))
}

#[delete("/users/<target>")]
async fn delete_user(
    db: &State<Database>,
    authifier: &State<Authifier>,
    user: User,
    target: String,
) -> Result<EmptyResponse> {
    user.require_admin_permission(db, ADMIN_PERMISSION_MANAGE_USERS)
        .await?;

    if user.id == target {
        return Err(create_error!(InvalidOperation));
    }

    let mut target_user = db.fetch_user(&target).await?;
    target_user.mark_deleted(db).await?;

    if let Ok(mut account) = authifier.database.find_account(&target).await {
        account.disabled = true;
        let _ = account.save(authifier).await;
        let _ = account.delete_all_sessions(authifier, None).await;
    }

    Ok(EmptyResponse)
}

#[get("/servers")]
async fn list_servers(db: &State<Database>, user: User) -> Result<Json<Vec<AdminServerEntry>>> {
    user.require_admin_permission(db, ADMIN_PERMISSION_MANAGE_SERVERS)
        .await?;

    let settings = db.fetch_admin_settings().await?.unwrap_or_default();
    let servers = db
        .fetch_all_servers()
        .await?
        .into_iter()
        .map(|server| AdminServerEntry {
            state: settings.servers.get(&server.id).cloned(),
            server,
        })
        .collect();

    Ok(Json(servers))
}

#[post("/servers/<target>/freeze", data = "<data>")]
async fn freeze_server(
    db: &State<Database>,
    user: User,
    target: String,
    data: Json<ServerTimeoutPayload>,
) -> Result<EmptyResponse> {
    user.require_admin_permission(db, ADMIN_PERMISSION_MANAGE_SERVERS)
        .await?;

    db.fetch_server(&target).await?;
    let mut settings = db.fetch_admin_settings().await?.unwrap_or_default();
    settings.servers.insert(
        target,
        AdminServerOverride {
            frozen: true,
            timeout_until: data.into_inner().timeout_until,
        },
    );
    db.upsert_admin_settings(&settings).await?;
    Ok(EmptyResponse)
}

#[post("/servers/<target>/unfreeze")]
async fn unfreeze_server(
    db: &State<Database>,
    user: User,
    target: String,
) -> Result<EmptyResponse> {
    user.require_admin_permission(db, ADMIN_PERMISSION_MANAGE_SERVERS)
        .await?;

    let mut settings = db.fetch_admin_settings().await?.unwrap_or_default();
    settings.servers.remove(&target);
    db.upsert_admin_settings(&settings).await?;
    Ok(EmptyResponse)
}

#[delete("/servers/<target>")]
async fn delete_server(db: &State<Database>, user: User, target: String) -> Result<EmptyResponse> {
    user.require_admin_permission(db, ADMIN_PERMISSION_MANAGE_SERVERS)
        .await?;

    let server = db.fetch_server(&target).await?;
    server.delete(db).await?;

    let mut settings = db.fetch_admin_settings().await?.unwrap_or_default();
    settings.servers.remove(&target);
    db.upsert_admin_settings(&settings).await?;

    Ok(EmptyResponse)
}
