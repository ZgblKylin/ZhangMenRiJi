use serde::{Deserialize, Serialize};
use std::env;
use std::path::Path;

/// 持久化到磁盘的应用配置（用户可在设置界面编辑）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    #[serde(default = "default_pg_host")]
    pub pg_host: String,
    #[serde(default = "default_pg_port")]
    pub pg_port: String,
    #[serde(default = "default_pg_user")]
    pub pg_user: String,
    #[serde(default = "default_pg_password")]
    pub pg_password: String,
    #[serde(default = "default_pg_database")]
    pub pg_database: String,
    #[serde(default = "default_server_host")]
    pub server_host: String,
    #[serde(default = "default_server_port")]
    pub server_port: String,
}

fn default_pg_host() -> String {
    "192.168.50.150".into()
}
fn default_pg_port() -> String {
    "5432".into()
}
fn default_pg_user() -> String {
    "ruoruo".into()
}
fn default_pg_password() -> String {
    String::new()
}
fn default_pg_database() -> String {
    "zhangmenriji".into()
}
fn default_server_host() -> String {
    "0.0.0.0".into()
}
fn default_server_port() -> String {
    "3000".into()
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            pg_host: default_pg_host(),
            pg_port: default_pg_port(),
            pg_user: default_pg_user(),
            pg_password: default_pg_password(),
            pg_database: default_pg_database(),
            server_host: default_server_host(),
            server_port: default_server_port(),
        }
    }
}

impl AppConfig {
    /// 从 JSON 文件加载配置
    pub fn load_from_file(path: &Path) -> Option<Self> {
        let content = std::fs::read_to_string(path).ok()?;
        serde_json::from_str(&content).ok()
    }

    /// 保存配置到 JSON 文件
    pub fn save_to_file(&self, path: &Path) -> Result<(), String> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        let json = serde_json::to_string_pretty(self).map_err(|e| e.to_string())?;
        std::fs::write(path, json).map_err(|e| e.to_string())
    }

    /// 用此配置设置进程环境变量（供后端子线程读取）
    pub fn apply_to_env(&self) {
        env::set_var("PG_HOST", &self.pg_host);
        env::set_var("PG_PORT", &self.pg_port);
        env::set_var("PG_USER", &self.pg_user);
        env::set_var("PG_PASSWORD", &self.pg_password);
        env::set_var("PG_DATABASE", &self.pg_database);
        env::set_var("SERVER_HOST", &self.server_host);
        env::set_var("SERVER_PORT", &self.server_port);
    }
}

// ── 内部运行时配置（不可变快照） ──

pub struct Config {
    pub database_url: String,
    pub server_host: String,
    pub server_port: u16,
}

impl Config {
    /// 加载配置：优先 JSON 文件 → 环境变量 → 默认值
    pub fn load(config_path: Option<&Path>) -> Self {
        // 1. 尝试从 JSON 文件加载
        if let Some(path) = config_path {
            if let Some(app_cfg) = AppConfig::load_from_file(path) {
                app_cfg.apply_to_env();
            }
        }

        // 2. 读取环境变量（文件已注入或用户手动设置）
        Self::from_env()
    }

    pub fn from_env() -> Self {
        // 尝试直接使用 DATABASE_URL
        if let Ok(url) = env::var("DATABASE_URL") {
            return Self {
                database_url: url,
                server_host: env::var("SERVER_HOST").unwrap_or_else(|_| "0.0.0.0".into()),
                server_port: env::var("SERVER_PORT")
                    .unwrap_or_else(|_| "3000".into())
                    .parse()
                    .unwrap_or(3000),
            };
        }

        // 拼合 PG_* 变量
        let host = env::var("PG_HOST").unwrap_or_else(|_| "192.168.50.150".into());
        let port = env::var("PG_PORT").unwrap_or_else(|_| "5432".into());
        let user = env::var("PG_USER").unwrap_or_else(|_| "ruoruo".into());
        let pass = env::var("PG_PASSWORD").unwrap_or_default();
        let db = env::var("PG_DATABASE").unwrap_or_else(|_| "zhangmenriji".into());

        let mut url = String::from("postgres://");
        url.push_str(&user);
        url.push(':');
        url.push_str(&pass);
        url.push('@');
        url.push_str(&host);
        url.push(':');
        url.push_str(&port);
        url.push('/');
        url.push_str(&db);

        Self {
            database_url: url,
            server_host: env::var("SERVER_HOST").unwrap_or_else(|_| "0.0.0.0".into()),
            server_port: env::var("SERVER_PORT")
                .unwrap_or_else(|_| "3000".into())
                .parse()
                .unwrap_or(3000),
        }
    }
}
