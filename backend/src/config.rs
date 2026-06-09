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
        let pg_host = env::var("PG_HOST").unwrap_or_else(|_| "192.168.50.150".into());
        let pg_port = env::var("PG_PORT").unwrap_or_else(|_| "5432".into());
        let pg_user = env::var("PG_USER").unwrap_or_else(|_| "ruoruo".into());
        let pg_password = env::var("PG_PASSWORD").unwrap_or_default();
        let pg_database = env::var("PG_DATABASE").unwrap_or_else(|_| "zhangmenriji".into());

        let database_url = format!(
            "postgres://{}:{}@{}:{}/{}",
            pg_user, pg_password, pg_host, pg_port, pg_database
        );

        Self {
            database_url,
            server_host: env::var("SERVER_HOST").unwrap_or_else(|_| "0.0.0.0".into()),
            server_port: env::var("SERVER_PORT")
                .unwrap_or_else(|_| "3000".into())
                .parse()
                .unwrap_or(3000),
        }
    }
}
