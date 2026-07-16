use crate::handlers::games::AppState;
use crate::models::management::ManagementRequest;
use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use uuid::Uuid;

pub async fn manage_sect(
    State(app): State<AppState>,
    Path(id): Path<Uuid>,
    Json(request): Json<ManagementRequest>,
) -> impl IntoResponse {
    let (sect_name, mut game_state) = match crate::db::get_game(&app.pool, id).await {
        Ok(Some(game)) => game,
        Ok(None) => return (StatusCode::NOT_FOUND, "存档不存在").into_response(),
        Err(error) => {
            return (StatusCode::INTERNAL_SERVER_ERROR, error.to_string()).into_response()
        }
    };
    let result = {
        let mut rng = rand::thread_rng();
        crate::logic::management::execute_management(&mut rng, &mut game_state, request)
    };
    let events = match result {
        Ok(events) => events,
        Err(message) => return (StatusCode::BAD_REQUEST, message).into_response(),
    };
    if let Err(error) = crate::db::update_game(&app.pool, id, &sect_name, &game_state).await {
        return (StatusCode::INTERNAL_SERVER_ERROR, error.to_string()).into_response();
    }
    let _ = crate::db::append_events(&app.pool, id, &events).await;
    Json(serde_json::json!({ "ok": true, "events": events, "state": game_state })).into_response()
}
