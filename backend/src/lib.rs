mod config;
mod db;
mod handlers;
mod logic;
mod models;
mod router;

use handlers::games::AppState;
use sqlx::PgPool;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

pub async fn run_server() -> anyhow::Result<()> {
    // 加载 .env（开发环境）
    dotenvy::dotenv().ok();

    // 初始化日志
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    // 加载配置
    let cfg = config::Config::from_env();
    tracing::info!(
        "数据库: {}",
        cfg.database_url.split('@').last().unwrap_or("?")
    );

    // 连接数据库
    let pool = PgPool::connect(&cfg.database_url).await?;
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
