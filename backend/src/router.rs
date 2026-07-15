use crate::handlers::games::AppState;
use crate::handlers::{advance, decisions, games, management, static_data};
use axum::{
    routing::{delete, get, post},
    Router,
};
use tower_http::cors::{Any, CorsLayer};
use tower_http::services::ServeDir;

pub fn create_router(state: AppState) -> Router {
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    let api = Router::new()
        .route("/api/games", post(games::create_game))
        .route("/api/games", get(games::list_games))
        .route("/api/games/{id}", get(games::get_game))
        .route("/api/games/{id}", delete(games::delete_game))
        .route(
            "/api/games/{id}/decisions/{decision_id}",
            post(decisions::execute_decision),
        )
        .route("/api/games/{id}/advance", post(advance::advance_month))
        .route("/api/games/{id}/manage", post(management::manage_sect))
        .route("/api/decisions", get(static_data::list_decisions))
        .route("/api/martial-arts", get(static_data::list_martial_arts));

    // 静态文件：优先找构建产物 dist/，找不到则 fallback 到项目根（开发时浏览器直接打开 index.html）
    let static_files = ServeDir::new("../frontend/dist").fallback(ServeDir::new(".."));

    Router::new()
        .merge(api)
        .fallback_service(static_files)
        .layer(cors)
        .with_state(state)
}
