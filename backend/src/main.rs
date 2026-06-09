mod config;
mod router;
mod models;
mod logic;
mod db;
mod handlers;

use sqlx::PgPool;
use handlers::games::AppState;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() {
    // 加载 .env（开发环境）
    dotenvy::dotenv().ok();

    // 初始化日志
    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::try_from_default_env()
            .unwrap_or_else(|_| "info".into()))
        .with(tracing_subscriber::fmt::layer())
        .init();

    // 加载配置
    let cfg = config::Config::from_env();
    tracing::info!("数据库: {}", cfg.database_url.split('@').last().unwrap_or("?"));

    // 连接数据库
    let pool = PgPool::connect(&cfg.database_url).await.expect("数据库连接失败");
    tracing::info!("数据库连接成功");

    // 构建路由
    let state = AppState { pool };
    let app = router::create_router(state);

    // 启动服务器
    let addr = format!("{}:{}", cfg.server_host, cfg.server_port);
    tracing::info!("掌门日记后端启动 → http://{}", addr);

    let listener = tokio::net::TcpListener::bind(&addr).await.expect("端口绑定失败");
    axum::serve(listener, app).await.expect("服务器异常退出");
}
