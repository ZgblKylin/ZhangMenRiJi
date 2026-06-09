use axum::{routing::{get, post, delete}, Router};
use tower_http::cors::{CorsLayer, Any};
use crate::handlers::{games, decisions, advance, static_data};
use crate::handlers::games::AppState;

pub fn create_router(state: AppState) -> Router {
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    Router::new()
        .route("/api/games", post(games::create_game))
        .route("/api/games", get(games::list_games))
        .route("/api/games/{id}", get(games::get_game))
        .route("/api/games/{id}", delete(games::delete_game))
        .route("/api/games/{id}/decisions/{decision_id}", post(decisions::execute_decision))
        .route("/api/games/{id}/advance", post(advance::advance_month))
        .route("/api/decisions", get(static_data::list_decisions))
        .route("/api/martial-arts", get(static_data::list_martial_arts))
        .layer(cors)
        .with_state(state)
}
