use std::collections::{HashMap, HashSet};

use iso8601_timestamp::Timestamp;
use revolt_config::FeaturesLimits;

pub const ADMIN_SETTINGS_ID: &str = "global";
pub const ADMIN_PERMISSION_MANAGE_ADMIN: &str = "manage_admin";
pub const ADMIN_PERMISSION_MANAGE_USERS: &str = "manage_users";
pub const ADMIN_PERMISSION_MANAGE_SERVERS: &str = "manage_servers";
pub const ADMIN_PERMISSION_MANAGE_UPLOAD_LIMITS: &str = "manage_upload_limits";
pub const ADMIN_PERMISSION_CREATE_SERVERS: &str = "create_servers";

auto_derived!(
    /// Global platform administration role.
    pub struct AdminRole {
        pub name: String,
        #[serde(default)]
        pub permissions: Vec<String>,
        #[serde(default)]
        pub upload_limits: HashMap<String, usize>,
    }

    /// Per-user administration overrides.
    pub struct AdminUserOverride {
        #[serde(default)]
        pub roles: Vec<String>,
        #[serde(default)]
        pub permissions: Vec<String>,
        #[serde(default)]
        pub upload_limits: HashMap<String, usize>,
    }

    /// Per-server administrative state.
    pub struct AdminServerOverride {
        #[serde(skip_serializing_if = "crate::if_false", default)]
        pub frozen: bool,
        #[serde(skip_serializing_if = "Option::is_none")]
        pub timeout_until: Option<Timestamp>,
    }

    /// Server creation policy configured by administrators.
    pub struct AdminServerCreationPolicy {
        #[serde(default)]
        pub restricted: bool,
        #[serde(default)]
        pub allowed_users: Vec<String>,
        #[serde(default)]
        pub allowed_roles: Vec<String>,
    }

    /// Global platform administration settings.
    pub struct AdminSettings {
        #[serde(rename = "_id")]
        pub id: String,
        #[serde(default)]
        pub roles: HashMap<String, AdminRole>,
        #[serde(default)]
        pub users: HashMap<String, AdminUserOverride>,
        #[serde(default)]
        pub servers: HashMap<String, AdminServerOverride>,
        #[serde(default)]
        pub default_upload_limits: HashMap<String, usize>,
        #[serde(default)]
        pub server_creation: AdminServerCreationPolicy,
    }
);

impl Default for AdminServerCreationPolicy {
    fn default() -> Self {
        Self {
            restricted: false,
            allowed_users: Vec::new(),
            allowed_roles: Vec::new(),
        }
    }
}

impl Default for AdminSettings {
    fn default() -> Self {
        Self {
            id: ADMIN_SETTINGS_ID.to_string(),
            roles: HashMap::new(),
            users: HashMap::new(),
            servers: HashMap::new(),
            default_upload_limits: HashMap::new(),
            server_creation: AdminServerCreationPolicy::default(),
        }
    }
}

impl AdminSettings {
    fn admin_permissions() -> &'static [&'static str] {
        &[
            ADMIN_PERMISSION_CREATE_SERVERS,
            ADMIN_PERMISSION_MANAGE_ADMIN,
            ADMIN_PERMISSION_MANAGE_SERVERS,
            ADMIN_PERMISSION_MANAGE_UPLOAD_LIMITS,
            ADMIN_PERMISSION_MANAGE_USERS,
        ]
    }

    pub fn user_permissions(&self, user_id: &str) -> HashSet<String> {
        let mut permissions = HashSet::new();

        if let Some(user) = self.users.get(user_id) {
            for role_id in &user.roles {
                if let Some(role) = self.roles.get(role_id) {
                    permissions.extend(role.permissions.iter().cloned());
                }
            }

            permissions.extend(user.permissions.iter().cloned());
        }

        permissions
    }

    pub fn user_has_permission(&self, user_id: &str, permission: &str) -> bool {
        self.user_permissions(user_id).contains(permission)
    }

    pub fn user_has_any_admin_permission(&self, user_id: &str) -> bool {
        let permissions = self.user_permissions(user_id);

        Self::admin_permissions()
            .iter()
            .any(|permission| permissions.contains(*permission))
    }

    pub fn has_any_admin_account(&self) -> bool {
        self.users
            .keys()
            .any(|user_id| self.user_has_any_admin_permission(user_id))
    }

    pub fn can_create_server(&self, user_id: &str) -> bool {
        if !self.server_creation.restricted {
            return true;
        }

        if self
            .server_creation
            .allowed_users
            .iter()
            .any(|id| id == user_id)
            || self.user_has_permission(user_id, ADMIN_PERMISSION_CREATE_SERVERS)
        {
            return true;
        }

        self.users.get(user_id).is_some_and(|user| {
            user.roles.iter().any(|role| {
                self.server_creation
                    .allowed_roles
                    .iter()
                    .any(|id| id == role)
            })
        })
    }

    pub fn apply_upload_limits(&self, user_id: &str, limits: &mut FeaturesLimits) {
        for (tag, limit) in &self.default_upload_limits {
            limits.file_upload_size_limit.insert(tag.clone(), *limit);
        }

        if let Some(user) = self.users.get(user_id) {
            for role_id in &user.roles {
                if let Some(role) = self.roles.get(role_id) {
                    for (tag, limit) in &role.upload_limits {
                        limits.file_upload_size_limit.insert(tag.clone(), *limit);
                    }
                }
            }

            for (tag, limit) in &user.upload_limits {
                limits.file_upload_size_limit.insert(tag.clone(), *limit);
            }
        }
    }

    pub fn server_is_frozen(&self, server_id: &str) -> bool {
        self.servers.get(server_id).is_some_and(|state| {
            state.frozen
                && state
                    .timeout_until
                    .as_ref()
                    .map_or(true, |until| until > &Timestamp::now_utc())
        })
    }
}
