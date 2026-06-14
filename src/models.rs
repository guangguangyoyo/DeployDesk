use serde::{Deserialize, Serialize};
use uuid::Uuid;

pub const AUTH_METHOD_PASSWORD: &str = "password";
pub const AUTH_METHOD_KEY: &str = "key";
pub const DEPLOY_MODE_BUILD: &str = "build";
pub const DEPLOY_MODE_UPLOAD: &str = "upload";
pub const MIN_RELEASES: u32 = 3;
pub const DEFAULT_MAX_RELEASES: u32 = 15;
pub const DEFAULT_BUILD_OUTPUT_DIR: &str = "dist";
pub const DEFAULT_SYMLINK_NAME: &str = "current";

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
pub struct Project {
    pub id: String,
    pub name: String,
    pub local_path: String,
    pub build_command: String,
    pub remote_host: String,
    pub remote_port: u16,
    pub username: String,
    pub auth_method: String,
    pub password_or_key: String,
    pub remote_deploy_path: String,
    #[serde(default = "default_build_output_dir")]
    pub build_output_dir: String,
    #[serde(default = "default_symlink_name")]
    pub symlink_name: String,
    #[serde(default = "default_deploy_mode")]
    pub deploy_mode: String,
    #[serde(default = "default_max_releases")]
    pub max_releases: u32,
}

impl Project {
    pub fn blank() -> Self {
        Self {
            id: String::new(),
            name: String::new(),
            local_path: String::new(),
            build_command: "npm run build".to_string(),
            remote_host: String::new(),
            remote_port: 22,
            username: "root".to_string(),
            auth_method: AUTH_METHOD_PASSWORD.to_string(),
            password_or_key: String::new(),
            remote_deploy_path: "/var/www/html/project".to_string(),
            build_output_dir: default_build_output_dir(),
            symlink_name: default_symlink_name(),
            deploy_mode: default_deploy_mode(),
            max_releases: default_max_releases(),
        }
    }

    pub fn normalize(&mut self) {
        if self.id.trim().is_empty() {
            self.id = Uuid::new_v4().to_string();
        }

        if self.remote_port == 0 {
            self.remote_port = 22;
        }

        if self.auth_method != AUTH_METHOD_PASSWORD && self.auth_method != AUTH_METHOD_KEY {
            self.auth_method = AUTH_METHOD_PASSWORD.to_string();
        }

        if self.deploy_mode != DEPLOY_MODE_BUILD && self.deploy_mode != DEPLOY_MODE_UPLOAD {
            self.deploy_mode = default_deploy_mode();
        }

        if self.build_output_dir.trim().is_empty() {
            self.build_output_dir = default_build_output_dir();
        }

        if self.symlink_name.trim().is_empty() {
            self.symlink_name = default_symlink_name();
        }

        if self.max_releases < MIN_RELEASES {
            self.max_releases = MIN_RELEASES;
        }
    }

    pub fn server_label(&self) -> String {
        format!(
            "{}@{}:{}",
            self.username, self.remote_host, self.remote_port
        )
    }

    pub fn effective_symlink_name(&self) -> &str {
        let symlink_name = self.symlink_name.trim();
        if symlink_name.is_empty() {
            DEFAULT_SYMLINK_NAME
        } else {
            symlink_name
        }
    }

    pub fn uses_build_step(&self) -> bool {
        self.deploy_mode == DEPLOY_MODE_BUILD
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Release {
    pub name: String,
    pub remark: String,
    pub is_current: bool,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct LogEntry {
    pub id: String,
    pub timestamp: String,
    pub project_name: String,
    pub action: String,
    pub status: String,
    pub details: String,
}

pub fn default_max_releases() -> u32 {
    DEFAULT_MAX_RELEASES
}

pub fn default_deploy_mode() -> String {
    DEPLOY_MODE_BUILD.to_string()
}

pub fn default_build_output_dir() -> String {
    DEFAULT_BUILD_OUTPUT_DIR.to_string()
}

pub fn default_symlink_name() -> String {
    DEFAULT_SYMLINK_NAME.to_string()
}
