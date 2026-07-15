use crate::handlers::games::AppState;
use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use uuid::Uuid;

/// POST /api/games/:id/advance
pub async fn advance_month(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> impl IntoResponse {
    let (sect_name, mut game_state) = match crate::db::get_game(&state.pool, id).await {
        Ok(Some(g)) => g,
        Ok(None) => return (StatusCode::NOT_FOUND, "存档不存在").into_response(),
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    };

    // RNG 在块内消费完即 drop，不跨 await
    let (events, tournament, game_over) = {
        let mut rng = rand::thread_rng();
        crate::logic::advance::advance_month(&mut rng, &mut game_state)
    };

    if let Err(e) = crate::db::update_game(&state.pool, id, &sect_name, &game_state).await {
        return (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response();
    }
    let _ = crate::db::append_events(&state.pool, id, &events).await;

    Json(serde_json::json!({
        "events": events,
        "tournament": tournament,
        "game_over": game_over,
        "state": game_state,
    }))
    .into_response()
}
