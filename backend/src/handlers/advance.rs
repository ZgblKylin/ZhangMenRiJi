use crate::handlers::games::AppState;
use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use uuid::Uuid;

#[derive(serde::Deserialize)]
pub struct ResolveEventRequest {
    pub option_id: String,
}

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
    if game_state.pending_event.is_some() {
        return (
            StatusCode::CONFLICT,
            Json(serde_json::json!({
                "error": "尚有江湖大事待掌门定夺。",
                "state": game_state,
            })),
        )
            .into_response();
    }

    let before = (game_state.year, game_state.month);
    // RNG 在块内消费完即 drop，不跨 await
    let (events, tournament, game_over) = {
        let mut rng = rand::thread_rng();
        crate::logic::advance::advance_month(&mut rng, &mut game_state)
    };
    crate::logic::sect::absorb_legacy_fields(&mut game_state);

    let save_id = if (game_state.year, game_state.month) != before {
        match crate::db::create_save(&state.pool, id, &sect_name, &game_state, true).await {
            Ok(Some(save_id)) => save_id,
            Ok(None) => return (StatusCode::NOT_FOUND, "存档不存在").into_response(),
            Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
        }
    } else {
        // 交互事件尚未结算，月份未推进，先保存在当前时间点中。
        if let Err(e) = crate::db::update_game(&state.pool, id, &sect_name, &game_state).await {
            return (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response();
        }
        id
    };
    let _ = crate::db::append_events(&state.pool, save_id, &events).await;
    game_state.autosave = save_id != id || game_state.autosave;

    Json(serde_json::json!({
        "id": save_id,
        "events": events,
        "tournament": tournament,
        "game_over": game_over,
        "state": game_state,
    }))
    .into_response()
}

/// POST /api/games/:id/events/resolve
pub async fn resolve_event(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Json(request): Json<ResolveEventRequest>,
) -> impl IntoResponse {
    let (sect_name, mut game_state) = match crate::db::get_game(&state.pool, id).await {
        Ok(Some(game)) => game,
        Ok(None) => return (StatusCode::NOT_FOUND, "存档不存在").into_response(),
        Err(error) => {
            return (StatusCode::INTERNAL_SERVER_ERROR, error.to_string()).into_response()
        }
    };
    let result = {
        let mut rng = rand::thread_rng();
        crate::logic::advance::resolve_pending_event(&mut rng, &mut game_state, &request.option_id)
    };
    let (events, tournament, game_over) = match result {
        Ok(result) => result,
        Err(message) => return (StatusCode::BAD_REQUEST, message).into_response(),
    };
    let save_id = match crate::db::create_save(&state.pool, id, &sect_name, &game_state, true).await
    {
        Ok(Some(save_id)) => save_id,
        Ok(None) => return (StatusCode::NOT_FOUND, "存档不存在").into_response(),
        Err(error) => {
            return (StatusCode::INTERNAL_SERVER_ERROR, error.to_string()).into_response()
        }
    };
    let _ = crate::db::append_events(&state.pool, save_id, &events).await;
    game_state.autosave = true;
    Json(serde_json::json!({
        "id": save_id,
        "events": events,
        "tournament": tournament,
        "game_over": game_over,
        "state": game_state,
    }))
    .into_response()
}
