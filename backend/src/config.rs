use std::env;

pub struct Config {
    pub database_url: String,
    pub server_host: String,
    pub server_port: u16,
}

impl Config {
    pub fn from_env() -> Self {
        // 1. 尝试直接使用 DATABASE_URL
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

        // 2. 拼合 PG_* 变量
        let host = env::var("PG_HOST").unwrap_or_else(|_| "192.168.50.150".into());
        let port = env::var("PG_PORT").unwrap_or_else(|_| "5432".into());
        let user = env::var("PG_USER").unwrap_or_else(|_| "ruoruo".into());
        let pass = env::var("PG_PASSWORD").unwrap_or_default();
        let db = env::var("PG_DATABASE").unwrap_or_else(|_| "zhangmenriji".into());

        // 用 push_str 拼合，避免 format! 中 {} 被误处理
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
