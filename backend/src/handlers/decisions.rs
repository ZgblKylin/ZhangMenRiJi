use crate::handlers::games::AppState;
use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use uuid::Uuid;

/// POST /api/games/:id/decisions/:decision_id
pub async fn execute_decision(
    State(state): State<AppState>,
    Path((id, decision_id)): Path<(Uuid, String)>,
) -> impl IntoResponse {
    let (sect_name, mut game_state) = match crate::db::get_game(&state.pool, id).await {
        Ok(Some(g)) => g,
        Ok(None) => return (StatusCode::NOT_FOUND, "存档不存在").into_response(),
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    };

    if game_state.decisions_used >= game_state.max_decisions {
        return (StatusCode::BAD_REQUEST, "本月决策次数已用完").into_response();
    }

    // RNG 在块内消费完即 drop，不跨 await
    let events = {
        let mut rng = rand::thread_rng();
        crate::logic::decision::execute_decision(&mut rng, &mut game_state, &decision_id)
    };

    if let Err(e) = crate::db::update_game(&state.pool, id, &sect_name, &game_state).await {
        return (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response();
    }
    let _ = crate::db::append_events(&state.pool, id, &events).await;

    Json(serde_json::json!({
        "ok": true,
        "events": events,
        "state": game_state,
    }))
    .into_response()
}
