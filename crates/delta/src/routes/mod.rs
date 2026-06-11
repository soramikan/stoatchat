use revolt_config::Settings;
use revolt_database::{Channel, Database};
use revolt_result::{create_error, Result};
use revolt_rocket_okapi::{revolt_okapi::openapi3::OpenApi, settings::OpenApiSettings};
pub use rocket::http::Status;
pub use rocket::response::Redirect;
use rocket::{Build, Rocket, Route};

mod admin;
mod bots;
mod channels;
mod customisation;
mod invites;
mod onboard;
mod policy;
mod push;
mod root;
mod safety;
mod servers;
mod sync;
mod users;
mod webhooks;

pub(crate) async fn require_server_not_frozen(db: &Database, server_id: &str) -> Result<()> {
    if db
        .fetch_admin_settings()
        .await?
        .unwrap_or_default()
        .server_is_frozen(server_id)
    {
        return Err(create_error!(InvalidOperation));
    }

    Ok(())
}

pub(crate) async fn require_channel_server_not_frozen(
    db: &Database,
    channel: &Channel,
) -> Result<()> {
    if let Some(server_id) = channel.server() {
        require_server_not_frozen(db, server_id).await?;
    }

    Ok(())
}

type RouteDocs = (Vec<Route>, OpenApi);

pub fn mount(config: Settings, rocket: Rocket<Build>) -> Rocket<Build> {
    let settings = OpenApiSettings::default();
    let route_docs = route_docs(config.features.webhooks_enabled);

    mount_route_docs(rocket, "/".to_owned(), &settings, route_docs)
}

fn route_docs(webhooks_enabled: bool) -> Vec<(&'static str, RouteDocs)> {
    let mut route_docs = vec![
        ("/", (vec![], custom_openapi_spec())),
        ("/admin", admin::routes()),
        ("", openapi_get_routes_spec![root::root]),
        ("/users", users::routes()),
        ("/bots", bots::routes()),
        ("/channels", channels::routes()),
        ("/servers", servers::routes()),
        ("/invites", invites::routes()),
        ("/custom", customisation::routes()),
        ("/safety", safety::routes()),
        ("/auth/account", rocket_authifier::routes::account::routes()),
        ("/auth/session", rocket_authifier::routes::session::routes()),
        ("/auth/mfa", rocket_authifier::routes::mfa::routes()),
        ("/onboard", onboard::routes()),
        ("/policy", policy::routes()),
        ("/push", push::routes()),
        ("/sync", sync::routes()),
    ];

    if webhooks_enabled {
        route_docs.push(("/webhooks", webhooks::routes()));
    }

    route_docs
        .into_iter()
        .map(|(path, docs)| (path, normalize_openapi_version(docs)))
        .collect()
}

fn normalize_openapi_version((routes, mut spec): RouteDocs) -> RouteDocs {
    spec.openapi = OpenApi::default_version();
    (routes, spec)
}

fn mount_route_docs(
    mut rocket: Rocket<Build>,
    base_path: String,
    settings: &OpenApiSettings,
    route_docs: Vec<(&'static str, RouteDocs)>,
) -> Rocket<Build> {
    assert!(
        base_path == "/" || !base_path.ends_with('/'),
        "`base_path` should not end with an `/`."
    );

    let mut openapi_list = Vec::new();

    for (path, (routes, openapi)) in route_docs {
        rocket = rocket.mount(format!("{}{}", base_path, path), routes);
        openapi_list.push((path, openapi));
    }

    let openapi_docs =
        match revolt_rocket_okapi::revolt_okapi::merge::marge_spec_list(&openapi_list) {
            Ok(docs) => docs,
            Err(err) => panic!("Could not merge OpenAPI spec: {}", err),
        };

    rocket.mount(
        base_path,
        vec![revolt_rocket_okapi::get_openapi_route(
            openapi_docs,
            settings,
        )],
    )
}

fn custom_openapi_spec() -> OpenApi {
    use revolt_rocket_okapi::revolt_okapi::openapi3::*;

    let mut extensions = schemars::Map::new();
    extensions.insert(
        "x-logo".to_owned(),
        json!({
            "url": "https://stoat.chat/header.png",
            "altText": "Stoat Header"
        }),
    );

    extensions.insert(
        "x-tagGroups".to_owned(),
        json!([
          {
            "name": "Stoat",
            "tags": [
              "Core"
            ]
          },
          {
            "name": "Users",
            "tags": [
              "User Information",
              "Direct Messaging",
              "Relationships"
            ]
          },
          {
            "name": "Bots",
            "tags": [
              "Bots"
            ]
          },
          {
            "name": "Channels",
            "tags": [
              "Channel Information",
              "Channel Invites",
              "Channel Permissions",
              "Messaging",
              "Interactions",
              "Groups",
              "Voice",
              "Webhooks",
            ]
          },
          {
            "name": "Servers",
            "tags": [
              "Server Information",
              "Server Members",
              "Server Permissions"
            ]
          },
          {
            "name": "Invites",
            "tags": [
              "Invites"
            ]
          },
          {
            "name": "Customisation",
            "tags": [
              "Emojis"
            ]
          },
          {
            "name": "Platform Administration",
            "tags": [
              "Admin",
              "User Safety"
            ]
          },
          {
            "name": "Authentication",
            "tags": [
              "Account",
              "Session",
              "Onboarding",
              "MFA"
            ]
          },
          {
            "name": "Miscellaneous",
            "tags": [
              "Sync",
              "Web Push"
            ]
          }
        ]),
    );

    OpenApi {
        openapi: OpenApi::default_version(),
        info: Info {
            title: "Stoat API".to_owned(),
            description: Some("Open source user-first chat platform.".to_owned()),
            terms_of_service: Some("https://stoat.chat/terms".to_owned()),
            contact: Some(Contact {
                name: Some("Stoat".to_owned()),
                url: Some("https://stoat.chat".to_owned()),
                email: Some("contact@stoat.chat".to_owned()),
                ..Default::default()
            }),
            license: Some(License {
                name: "AGPLv3".to_owned(),
                url: Some(
                    "https://github.com/stoatchat/stoatchat/blob/main/crates/delta/LICENSE"
                        .to_owned(),
                ),
                ..Default::default()
            }),
            version: env!("CARGO_PKG_VERSION").to_string(),
            ..Default::default()
        },
        servers: vec![
            Server {
                url: "https://api.stoat.chat".to_owned(),
                description: Some("Stoat Production".to_owned()),
                ..Default::default()
            },
            Server {
                url: "https://beta.stoat.chat/api".to_owned(),
                description: Some("Stoat Beta".to_owned()),
                ..Default::default()
            },
        ],
        external_docs: Some(ExternalDocs {
            url: "https://developers.stoat.chat".to_owned(),
            description: Some("Stoat Developer Documentation".to_owned()),
            ..Default::default()
        }),
        extensions,
        tags: vec![
            Tag {
                name: "Core".to_owned(),
                description: Some(
                    "Use in your applications to determine information about the Stoat node"
                        .to_owned(),
                ),
                ..Default::default()
            },
            Tag {
                name: "User Information".to_owned(),
                description: Some("Query and fetch users on Stoat".to_owned()),
                ..Default::default()
            },
            Tag {
                name: "Direct Messaging".to_owned(),
                description: Some("Direct message other users on Stoat".to_owned()),
                ..Default::default()
            },
            Tag {
                name: "Relationships".to_owned(),
                description: Some(
                    "Manage your friendships and block list on the platform".to_owned(),
                ),
                ..Default::default()
            },
            Tag {
                name: "Bots".to_owned(),
                description: Some("Create and edit bots".to_owned()),
                ..Default::default()
            },
            Tag {
                name: "Channel Information".to_owned(),
                description: Some("Query and fetch channels on Stoat".to_owned()),
                ..Default::default()
            },
            Tag {
                name: "Channel Invites".to_owned(),
                description: Some("Create and manage invites for channels".to_owned()),
                ..Default::default()
            },
            Tag {
                name: "Channel Permissions".to_owned(),
                description: Some("Manage permissions for channels".to_owned()),
                ..Default::default()
            },
            Tag {
                name: "Messaging".to_owned(),
                description: Some("Send and manipulate messages".to_owned()),
                ..Default::default()
            },
            Tag {
                name: "Groups".to_owned(),
                description: Some("Create, invite users and manipulate groups".to_owned()),
                ..Default::default()
            },
            Tag {
                name: "Voice".to_owned(),
                description: Some("Join and talk with other users".to_owned()),
                ..Default::default()
            },
            Tag {
                name: "Server Information".to_owned(),
                description: Some("Query and fetch servers on Stoat".to_owned()),
                ..Default::default()
            },
            Tag {
                name: "Server Members".to_owned(),
                description: Some("Find and edit server members".to_owned()),
                ..Default::default()
            },
            Tag {
                name: "Server Permissions".to_owned(),
                description: Some("Manage permissions for servers".to_owned()),
                ..Default::default()
            },
            Tag {
                name: "Invites".to_owned(),
                description: Some("View, join and delete invites".to_owned()),
                ..Default::default()
            },
            Tag {
                name: "Account".to_owned(),
                description: Some("Manage your account".to_owned()),
                ..Default::default()
            },
            Tag {
                name: "Session".to_owned(),
                description: Some("Create and manage sessions".to_owned()),
                ..Default::default()
            },
            Tag {
                name: "MFA".to_owned(),
                description: Some("Multi-factor Authentication".to_owned()),
                ..Default::default()
            },
            Tag {
                name: "Onboarding".to_owned(),
                description: Some(
                    "After signing up to Stoat, users must pick a unique username".to_owned(),
                ),
                ..Default::default()
            },
            Tag {
                name: "Sync".to_owned(),
                description: Some("Upload and retrieve any JSON data between clients".to_owned()),
                ..Default::default()
            },
            Tag {
                name: "Web Push".to_owned(),
                description: Some(
                    "Subscribe to and receive Stoat push notifications while offline".to_owned(),
                ),
                ..Default::default()
            },
            Tag {
                name: "Webhooks".to_owned(),
                description: Some("Send messages from 3rd party services".to_owned()),
                ..Default::default()
            },
        ],
        ..Default::default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use revolt_rocket_okapi::revolt_okapi::merge::marge_spec_list;

    fn assert_route_docs_merge(webhooks_enabled: bool) {
        let expected_version = OpenApi::default_version();
        let route_docs = route_docs(webhooks_enabled);

        for (path, (_, spec)) in &route_docs {
            assert_eq!(
                spec.openapi, expected_version,
                "OpenAPI version mismatch before merge for route group {path}"
            );
        }

        let openapi_list: Vec<_> = route_docs
            .into_iter()
            .map(|(path, (_, spec))| (path, spec))
            .collect();

        let merged = marge_spec_list(&openapi_list).expect("OpenAPI route specs should merge");
        assert_eq!(merged.openapi, expected_version);
    }

    #[test]
    fn route_docs_merge_without_webhooks() {
        assert_route_docs_merge(false);
    }

    #[test]
    fn route_docs_merge_with_webhooks() {
        assert_route_docs_merge(true);
    }
}
