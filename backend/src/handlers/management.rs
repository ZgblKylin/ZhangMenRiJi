use crate::handlers::games::AppState;
use crate::handlers::revision::{
    internal_error_response, not_found_response, parse_expected_revision, require_applied,
    state_conflict_response, verify_expected_revision,
};
use crate::models::management::ManagementRequest;
use axum::{
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
    response::IntoResponse,
    Json,
};
use uuid::Uuid;

pub async fn manage_sect(
    State(app): State<AppState>,
    Path(id): Path<Uuid>,
    headers: HeaderMap,
    Json(request): Json<ManagementRequest>,
) -> impl IntoResponse {
    let expected_revision = match parse_expected_revision(&headers) {
        Ok(revision) => revision,
        Err(response) => return response,
    };
    let loaded = match crate::db::get_game_with_revision(&app.pool, id).await {
        Ok(Some(game)) => game,
        Ok(None) => return not_found_response(),
        Err(error) => return internal_error_response(error),
    };
    if let Err(response) = verify_expected_revision(&app.pool, &loaded, expected_revision).await {
        return response;
    }
    let sect_name = loaded.sect_name.clone();
    let mut game_state = loaded.state;
    if game_state.game_over {
        let error = if game_state.game_won {
            "两载掌门卷已经封存。"
        } else {
            "山门已散，无法继续经营。"
        };
        return state_conflict_response("game_over", error, &game_state);
    }
    if game_state.pending_event.is_some() {
        return state_conflict_response("pending_event", "尚有江湖大事待掌门定夺。", &game_state);
    }

    let result = {
        let mut rng = rand::thread_rng();
        crate::logic::management::execute_management(&mut rng, &mut game_state, request)
    };
    let events = match result {
        Ok(events) => events,
        Err(message) => return (StatusCode::BAD_REQUEST, message).into_response(),
    };
    let result = match crate::db::update_game_with_events_if_revision(
        &app.pool,
        id,
        loaded.save_group_id,
        expected_revision,
        &sect_name,
        &game_state,
        &events,
    )
    .await
    {
        Ok(result) => result,
        Err(error) => return internal_error_response(error),
    };
    let revision = match require_applied(result, expected_revision) {
        Ok(revision) => revision,
        Err(response) => return response,
    };

    Json(serde_json::json!({
        "id": id,
        "save_group_id": loaded.save_group_id,
        "revision": revision,
        "sect_name": sect_name,
        "ok": true,
        "events": events,
        "state": game_state,
    }))
    .into_response()
}
