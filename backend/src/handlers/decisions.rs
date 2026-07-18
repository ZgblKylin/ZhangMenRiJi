use crate::handlers::games::AppState;
use crate::handlers::revision::{
    internal_error_response, not_found_response, parse_expected_revision, require_applied,
    state_conflict_response, verify_expected_revision,
};
use axum::{
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
    response::IntoResponse,
    Json,
};
use uuid::Uuid;

/// POST /api/games/:id/decisions/:decision_id
pub async fn execute_decision(
    State(state): State<AppState>,
    Path((id, decision_id)): Path<(Uuid, String)>,
    headers: HeaderMap,
) -> impl IntoResponse {
    let expected_revision = match parse_expected_revision(&headers) {
        Ok(revision) => revision,
        Err(response) => return response,
    };
    let loaded = match crate::db::get_game_with_revision(&state.pool, id).await {
        Ok(Some(g)) => g,
        Ok(None) => return not_found_response(),
        Err(error) => return internal_error_response(error),
    };
    if let Err(response) = verify_expected_revision(&state.pool, &loaded, expected_revision).await {
        return response;
    }
    let sect_name = loaded.sect_name.clone();
    let mut game_state = loaded.state;

    if game_state.game_over {
        let error = if game_state.game_won {
            "两载掌门卷已经封存。"
        } else {
            "山门已散，无法继续定夺。"
        };
        return state_conflict_response("game_over", error, &game_state);
    }
    if game_state.pending_event.is_some() {
        return state_conflict_response("pending_event", "尚有江湖大事待掌门定夺。", &game_state);
    }
    if game_state.decisions_used >= game_state.max_decisions {
        return (StatusCode::BAD_REQUEST, "本月决策次数已用完").into_response();
    }

    // RNG 在块内消费完即 drop，不跨 await
    let events = {
        let mut rng = rand::thread_rng();
        crate::logic::decision::execute_decision(&mut rng, &mut game_state, &decision_id)
    };
    if events.is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({
                "error": "此项议事眼下无法施行，请检查建筑完好、库银、门人在门状态与修习上限。",
                "state": game_state,
            })),
        )
            .into_response();
    }
    for disciple in &mut game_state.disciples {
        crate::logic::disciple::absorb_legacy_attributes(disciple);
    }
    crate::logic::sect::absorb_legacy_fields(&mut game_state);

    let result = match crate::db::update_game_with_events_if_revision(
        &state.pool,
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::AppConfig;
    use crate::models::game::{CreateGameRequest, GameState};
    use axum::body::to_bytes;
    use axum::http::HeaderValue;
    use sqlx::sqlite::SqlitePoolOptions;
    use std::sync::Arc;
    use tokio::sync::RwLock;

    async fn seeded_app(game_state: &GameState) -> (AppState, Uuid) {
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .unwrap();
        crate::db::run_migrations(&pool).await.unwrap();
        let request = CreateGameRequest {
            sect_name: "试剑门".into(),
        };
        let (id, _) = crate::db::create_game(&pool, &request, game_state)
            .await
            .unwrap();
        (
            AppState {
                pool,
                config: Arc::new(RwLock::new(AppConfig::default())),
                config_path: None,
            },
            id,
        )
    }

    fn revision_headers(revision: i64) -> HeaderMap {
        let mut headers = HeaderMap::new();
        headers.insert(
            "x-save-revision",
            HeaderValue::from_str(&revision.to_string()).unwrap(),
        );
        headers
    }

    #[tokio::test]
    async fn legacy_decision_id_keeps_success_envelope_and_persists_one_use() {
        let mut initial = GameState::default();
        initial.disciples.push(crate::models::Disciple {
            id: "steward".into(),
            name: "周执事".into(),
            ..crate::models::Disciple::default()
        });
        let (app, id) = seeded_app(&initial).await;

        let response = execute_decision(
            State(app.clone()),
            Path((id, "mission".into())),
            revision_headers(1),
        )
        .await
        .into_response();

        assert_eq!(response.status(), StatusCode::OK);
        let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(json["id"], id.to_string());
        assert_eq!(json["revision"], 2);
        assert_eq!(json["sect_name"], "试剑门");
        assert_eq!(json["ok"], true);
        assert_eq!(json["state"]["decisions_used"], 1);
        let stored = crate::db::get_game_with_revision(&app.pool, id)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(stored.revision, 2);
        assert_eq!(stored.state.decisions_used, 1);
    }

    #[tokio::test]
    async fn unavailable_decision_is_rejected_without_consuming_or_persisting_a_use() {
        let initial = GameState::default();
        let (app, id) = seeded_app(&initial).await;

        let response = execute_decision(
            State(app.clone()),
            Path((id, "train".into())),
            revision_headers(1),
        )
        .await
        .into_response();

        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
        let (_, stored) = crate::db::get_game(&app.pool, id).await.unwrap().unwrap();
        assert_eq!(stored.decisions_used, 0);
        assert_eq!(
            stored.sect.attributes.silver,
            initial.sect.attributes.silver
        );
    }

    #[tokio::test]
    async fn a_repeated_decision_with_the_old_revision_returns_stale_shape() {
        let mut initial = GameState::default();
        initial.disciples.push(crate::models::Disciple {
            id: "steward".into(),
            name: "周执事".into(),
            ..crate::models::Disciple::default()
        });
        let (app, id) = seeded_app(&initial).await;

        let first = execute_decision(
            State(app.clone()),
            Path((id, "mission".into())),
            revision_headers(1),
        )
        .await
        .into_response();
        assert_eq!(first.status(), StatusCode::OK);

        let stale = execute_decision(
            State(app),
            Path((id, "mission".into())),
            revision_headers(1),
        )
        .await
        .into_response();
        assert_eq!(stale.status(), StatusCode::CONFLICT);
        let body = to_bytes(stale.into_body(), usize::MAX).await.unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(json["code"], "stale_revision");
        assert_eq!(json["expected_revision"], 1);
        assert_eq!(json["actual_revision"], 2);
        assert_eq!(json["current"]["id"], id.to_string());
        assert_eq!(json["current"]["revision"], 2);
        assert!(json["current"].get("state").is_some());
    }
}
