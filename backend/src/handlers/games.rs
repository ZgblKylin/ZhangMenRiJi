use crate::logic::disciple::generate_starting_disciples;
use crate::models::game::CreateGameRequest;
use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use sqlx::PgPool;
use uuid::Uuid;

#[derive(Clone)]
pub struct AppState {
    pub pool: PgPool,
}

/// POST /api/games
#[axum::debug_handler]
pub async fn create_game(
    State(state): State<AppState>,
    Json(req): Json<CreateGameRequest>,
) -> impl IntoResponse {
    match crate::db::create_game(&state.pool, &req).await {
        Ok((id, mut game_state)) => {
            // RNG 在块内创建和消费，不跨 await
            {
                let mut rng = rand::thread_rng();
                game_state.disciples = generate_starting_disciples(&mut rng);
                game_state.martial_arts_learned = vec!["hunyuan".into()];
            } // rng drop here
            let _ = crate::db::update_game(&state.pool, id, &req.sect_name, &game_state).await;
            (
                StatusCode::CREATED,
                Json(serde_json::json!({
                    "id": id,
                    "sect_name": req.sect_name,
                    "state": game_state,
                })),
            )
                .into_response()
        }
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

/// GET /api/games
#[axum::debug_handler]
pub async fn list_games(State(state): State<AppState>) -> impl IntoResponse {
    match crate::db::list_games(&state.pool).await {
        Ok(games) => {
            let list: Vec<serde_json::Value> = games
                .iter()
                .map(|g| {
                    serde_json::json!({
                        "id": g.id,
                        "sect_name": g.sect_name,
                        "state": g.state,
                        "updated_at": g.updated_at.to_rfc3339(),
                    })
                })
                .collect();
            Json(serde_json::json!({ "games": list })).into_response()
        }
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

/// GET /api/games/:id
pub async fn get_game(State(state): State<AppState>, Path(id): Path<Uuid>) -> impl IntoResponse {
    match crate::db::get_game(&state.pool, id).await {
        Ok(Some((sect_name, game_state))) => Json(serde_json::json!({
            "id": id,
            "sect_name": sect_name,
            "state": game_state,
        }))
        .into_response(),
        Ok(None) => (StatusCode::NOT_FOUND, "存档不存在").into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

/// DELETE /api/games/:id
pub async fn delete_game(State(state): State<AppState>, Path(id): Path<Uuid>) -> impl IntoResponse {
    match crate::db::delete_game(&state.pool, id).await {
        Ok(true) => StatusCode::NO_CONTENT.into_response(),
        Ok(false) => (StatusCode::NOT_FOUND, "存档不存在").into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}
