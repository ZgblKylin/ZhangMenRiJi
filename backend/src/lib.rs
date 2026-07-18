pub mod config;
mod db;
mod handlers;
mod logic;
mod models;
mod router;

use handlers::games::AppState;
use sqlx::SqlitePool;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use std::path::Path;

pub async fn run_server(config_path: Option<&Path>) -> anyhow::Result<()> {
    // 加载 .env（开发/Web 模式回退）
    dotenvy::dotenv().ok();

    // 初始化日志
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    // 加载配置（文件优先 → 环境变量 → 默认值）
    let cfg = config::Config::load(config_path);
    tracing::info!(
        "数据库: {}",
        cfg.database_url.split('@').next_back().unwrap_or("?")
    );

    // SQLite 默认以只打开模式连接；未显式指定 mode 时允许首次启动创建数据库文件。
    let connection_url = if cfg.database_url.starts_with("sqlite://")
        && !cfg.database_url.contains("mode=")
    {
        let separator = if cfg.database_url.contains('?') {
            '&'
        } else {
            '?'
        };
        format!("{}{separator}mode=rwc", cfg.database_url)
    } else {
        cfg.database_url.clone()
    };
    let pool = SqlitePool::connect(&connection_url).await?;
    tracing::info!("数据库连接成功");

    // 自动建表
    db::run_migrations(&pool).await?;

    // 构建路由
    let state = AppState { pool };
    let app = router::create_router(state);

    // 启动服务器
    let addr = format!("{}:{}", cfg.server_host, cfg.server_port);
    tracing::info!("掌门日记后端启动 → http://{}", addr);

    let listener = tokio::net::TcpListener::bind(&addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}
