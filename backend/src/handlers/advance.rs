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

#[derive(serde::Deserialize)]
pub struct ResolveEventRequest {
    pub option_id: String,
}

/// POST /api/games/:id/advance
pub async fn advance_month(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
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
            "山门已散，无法继续推演。"
        };
        return state_conflict_response("game_over", error, &game_state);
    }
    if game_state.pending_event.is_some() {
        return state_conflict_response("pending_event", "尚有江湖大事待掌门定夺。", &game_state);
    }

    let before = (game_state.year, game_state.month);
    // RNG 在块内消费完即 drop，不跨 await
    let (events, tournament, game_over) = {
        let mut rng = rand::thread_rng();
        crate::logic::advance::advance_month(&mut rng, &mut game_state)
    };
    crate::logic::sect::absorb_legacy_fields(&mut game_state);

    let (save_id, save_group_id, revision) = if (game_state.year, game_state.month) != before {
        game_state.autosave = true;
        let result = match crate::db::create_save_with_events_if_revision(
            &state.pool,
            id,
            loaded.save_group_id,
            expected_revision,
            &sect_name,
            &game_state,
            true,
            &events,
        )
        .await
        {
            Ok(result) => result,
            Err(error) => return internal_error_response(error),
        };
        match require_applied(result, expected_revision) {
            Ok(created) => (created.id, created.save_group_id, created.revision),
            Err(response) => return response,
        }
    } else {
        // 交互事件尚未结算，月份未推进，先保存在当前时间点中。
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
        match require_applied(result, expected_revision) {
            Ok(revision) => (id, loaded.save_group_id, revision),
            Err(response) => return response,
        }
    };
    let (sect_events, world_events) = split_events(&events);

    Json(serde_json::json!({
        "id": save_id,
        "save_group_id": save_group_id,
        "revision": revision,
        "sect_name": sect_name,
        "events": events,
        "sect_events": sect_events,
        "world_events": world_events,
        "tournament": tournament,
        "game_over": game_over,
        "game_won": game_state.game_won,
        "state": game_state,
    }))
    .into_response()
}

/// POST /api/games/:id/events/resolve
pub async fn resolve_event(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    headers: HeaderMap,
    Json(request): Json<ResolveEventRequest>,
) -> impl IntoResponse {
    let expected_revision = match parse_expected_revision(&headers) {
        Ok(revision) => revision,
        Err(response) => return response,
    };
    let loaded = match crate::db::get_game_with_revision(&state.pool, id).await {
        Ok(Some(game)) => game,
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
    let result = {
        let mut rng = rand::thread_rng();
        crate::logic::advance::resolve_pending_event(&mut rng, &mut game_state, &request.option_id)
    };
    let (events, tournament, game_over) = match result {
        Ok(result) => result,
        Err(message) => return (StatusCode::BAD_REQUEST, message).into_response(),
    };
    game_state.autosave = true;
    let result = match crate::db::create_save_with_events_if_revision(
        &state.pool,
        id,
        loaded.save_group_id,
        expected_revision,
        &sect_name,
        &game_state,
        true,
        &events,
    )
    .await
    {
        Ok(result) => result,
        Err(error) => return internal_error_response(error),
    };
    let created = match require_applied(result, expected_revision) {
        Ok(created) => created,
        Err(response) => return response,
    };
    let (sect_events, world_events) = split_events(&events);
    Json(serde_json::json!({
        "id": created.id,
        "save_group_id": created.save_group_id,
        "revision": created.revision,
        "sect_name": sect_name,
        "events": events,
        "sect_events": sect_events,
        "world_events": world_events,
        "tournament": tournament,
        "game_over": game_over,
        "game_won": game_state.game_won,
        "state": game_state,
    }))
    .into_response()
}

fn split_events(
    events: &[crate::models::GameEvent],
) -> (Vec<crate::models::GameEvent>, Vec<crate::models::GameEvent>) {
    events
        .iter()
        .cloned()
        .partition(|event| event.category != "world")
}

#[cfg(test)]
mod tests {
    use super::split_events;
    use crate::models::GameEvent;

    #[test]
    fn month_events_are_grouped_for_the_frontend() {
        let events = vec![
            GameEvent {
                text: "本门弟子练功".into(),
                mood: "good".into(),
                year: 1,
                month: 1,
                category: "sect".into(),
            },
            GameEvent {
                text: "武当弟子游历".into(),
                mood: "neutral".into(),
                year: 1,
                month: 1,
                category: "world".into(),
            },
        ];

        let (sect_events, world_events) = split_events(&events);
        assert_eq!(sect_events.len(), 1);
        assert_eq!(world_events.len(), 1);
        assert_eq!(world_events[0].category, "world");
    }
}
