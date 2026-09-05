use crate::handlers::revision::{
    game_response, internal_error_response, not_found_response, parse_expected_revision,
    require_applied, revisioned_game_response, verify_expected_revision,
};
use crate::logic::disciple::generate_starting_disciples;
use crate::models::game::{CreateGameRequest, GameState};
use axum::{
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
    response::IntoResponse,
    Json,
};
use sqlx::SqlitePool;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;

#[derive(Clone)]
pub struct AppState {
    pub pool: SqlitePool,
    pub config: Arc<RwLock<crate::config::AppConfig>>,
    pub config_path: Option<PathBuf>,
}

/// POST /api/games
#[axum::debug_handler]
pub async fn create_game(
    State(state): State<AppState>,
    Json(req): Json<CreateGameRequest>,
) -> impl IntoResponse {
    let mut game_state = GameState::default();
    game_state.autosave = true;
    // RNG 在块内创建和消费，不跨 await
    {
        let mut rng = rand::thread_rng();
        game_state.disciples = generate_starting_disciples(&mut rng);
        game_state.martial_arts_learned = vec!["player_knowledge".into(), "hunyuan".into()];
        game_state.event_log.push(crate::models::GameEvent {
        text: format!("掌门{}于{}新立山门，聚得二徒，草创基业。江湖险恶，万里之行始于此。左栏可检视弟子并安排行止，中栏各堂为门派经营总枢，右栏此卷为江湖纪事。每月限行三次定夺，右下按钮推演下月。", game_state.sect.name, game_state.sect.landmark),
        mood: "good".into(),
        year: 1,
        month: 1,
        category: "sect".into(),
    });
    } // rng drop here
    crate::logic::sect::hydrate_player_sect(&mut game_state, &req.sect_name);

    match crate::db::create_game(&state.pool, &req, &game_state).await {
        Ok((id, save_group_id)) => (
            StatusCode::CREATED,
            Json(game_response(
                id,
                save_group_id,
                1,
                &req.sect_name,
                &game_state,
            )),
        )
            .into_response(),
        Err(error) => internal_error_response(error),
    }
}

/// GET /api/games
#[axum::debug_handler]
pub async fn list_games(State(state): State<AppState>) -> impl IntoResponse {
    match crate::db::list_games(&state.pool).await {
        Ok(games) => {
            // DB 以一条 JOIN 查询同时读取节点和槽位 head，以下分组不会跨连接
            // 混用两个时刻的 current_id/revision。
            let mut group_ids = Vec::new();
            for game in &games {
                if !group_ids.contains(&game.save_group_id) {
                    group_ids.push(game.save_group_id);
                }
            }

            let mut groups = Vec::new();
            for save_group_id in group_ids {
                let Some(first) = games
                    .iter()
                    .find(|game| game.save_group_id == save_group_id)
                else {
                    continue;
                };
                let Some(current) = games.iter().find(|game| {
                    game.save_group_id == save_group_id && game.id == first.current_game_id
                }) else {
                    return internal_error_response(format!(
                        "槽位 {save_group_id} 的单快照列表缺少当前存档"
                    ));
                };
                let saves: Vec<_> = games
                    .iter()
                    .filter(|game| game.save_group_id == save_group_id)
                    .map(|game| {
                        serde_json::json!({
                            "id": game.id,
                            "save_group_id": game.save_group_id,
                            "revision": game.revision,
                            "save_type": game.save_type,
                            "autosave": game.autosave,
                            "year": game.year,
                            "month": game.month,
                            "updated_at": game.updated_at.to_rfc3339(),
                        })
                    })
                    .collect();

                groups.push(serde_json::json!({
                    "save_group_id": save_group_id,
                    "current_id": first.current_game_id,
                    "revision": first.revision,
                    "sect_name": current.sect_name,
                    "year": current.year,
                    "month": current.month,
                    "updated_at": first.head_updated_at.to_rfc3339(),
                    "saves": saves,
                }));
            }
            Json(serde_json::json!({ "groups": groups })).into_response()
        }
        Err(error) => internal_error_response(error),
    }
}

/// GET /api/games/:id
pub async fn get_game(State(state): State<AppState>, Path(id): Path<Uuid>) -> impl IntoResponse {
    match crate::db::get_game_with_revision(&state.pool, id).await {
        Ok(Some(game)) => Json(revisioned_game_response(&game)).into_response(),
        Ok(None) => not_found_response(),
        Err(error) => internal_error_response(error),
    }
}

/// POST /api/games/:id/saves — 将当前状态手动存入同一槽位。
pub async fn create_manual_save(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    headers: HeaderMap,
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

    let mut saved_state = loaded.state.clone();
    saved_state.autosave = false;
    let result = match crate::db::create_save_with_events_if_revision(
        &state.pool,
        id,
        loaded.save_group_id,
        expected_revision,
        &loaded.sect_name,
        &saved_state,
        false,
        &[],
    )
    .await
    {
        Ok(result) => result,
        Err(error) => return internal_error_response(error),
    };
    match require_applied(result, expected_revision) {
        Ok(created) => (
            StatusCode::CREATED,
            Json(game_response(
                created.id,
                created.save_group_id,
                created.revision,
                &loaded.sect_name,
                &saved_state,
            )),
        )
            .into_response(),
        Err(response) => response,
    }
}

/// DELETE /api/games/:id
pub async fn delete_game(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    headers: HeaderMap,
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

    let result = match crate::db::delete_game_if_revision(
        &state.pool,
        id,
        loaded.save_group_id,
        expected_revision,
    )
    .await
    {
        Ok(result) => result,
        Err(error) => return internal_error_response(error),
    };
    match require_applied(result, expected_revision) {
        Ok(current) => Json(serde_json::json!({
            "deleted_id": id,
            "save_group_id": loaded.save_group_id,
            "current": current.as_ref().map(revisioned_game_response),
        }))
        .into_response(),
        Err(response) => response,
    }
}

/// DELETE /api/save-groups/:id
pub async fn delete_save_group(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    headers: HeaderMap,
) -> impl IntoResponse {
    let expected_revision = match parse_expected_revision(&headers) {
        Ok(revision) => revision,
        Err(response) => return response,
    };
    let loaded = match crate::db::get_current_game(&state.pool, id).await {
        Ok(Some(game)) => game,
        Ok(None) => return not_found_response(),
        Err(error) => return internal_error_response(error),
    };
    if let Err(response) = verify_expected_revision(&state.pool, &loaded, expected_revision).await {
        return response;
    }

    let result =
        match crate::db::delete_save_group_if_revision(&state.pool, id, expected_revision).await {
            Ok(result) => result,
            Err(error) => return internal_error_response(error),
        };
    match require_applied(result, expected_revision) {
        Ok(()) => StatusCode::NO_CONTENT.into_response(),
        Err(response) => response,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::AppConfig;
    use crate::db::RevisionWriteResult;
    use axum::body::to_bytes;
    use axum::http::HeaderValue;
    use sqlx::sqlite::SqlitePoolOptions;

    async fn seeded_app(game_state: &GameState) -> (AppState, Uuid, Uuid) {
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .unwrap();
        crate::db::run_migrations(&pool).await.unwrap();
        let request = CreateGameRequest {
            sect_name: "试剑门".into(),
        };
        let (id, save_group_id) = crate::db::create_game(&pool, &request, game_state)
            .await
            .unwrap();
        (
            AppState {
                pool,
                config: Arc::new(RwLock::new(AppConfig::default())),
                config_path: None,
            },
            id,
            save_group_id,
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
    async fn manual_save_history_get_and_list_follow_the_slot_head_revision() {
        let initial = GameState::default();
        let (app, original_id, save_group_id) = seeded_app(&initial).await;

        let response =
            create_manual_save(State(app.clone()), Path(original_id), revision_headers(1))
                .await
                .into_response();
        assert_eq!(response.status(), StatusCode::CREATED);
        let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        let manual_id = Uuid::parse_str(json["id"].as_str().unwrap()).unwrap();
        assert_ne!(manual_id, original_id);
        assert_eq!(json["save_group_id"], save_group_id.to_string());
        assert_eq!(json["revision"], 2);
        assert_eq!(json["sect_name"], "试剑门");
        assert_eq!(json["state"]["autosave"], false);

        let mut current_state = initial.clone();
        current_state.year = 7;
        current_state.month = 4;
        current_state.autosave = false;
        let updated = crate::db::update_game_with_events_if_revision(
            &app.pool,
            manual_id,
            save_group_id,
            2,
            "新试剑门",
            &current_state,
            &[],
        )
        .await
        .unwrap();
        assert!(matches!(updated, RevisionWriteResult::Applied(3)));

        // 故意让非 head 的时间戳更晚，证明列表不以“最近一行”冒充槽位当前档。
        sqlx::query("UPDATE games SET updated_at = '2099-01-01 00:00:00' WHERE id = $1")
            .bind(original_id.to_string())
            .execute(&app.pool)
            .await
            .unwrap();
        sqlx::query("UPDATE games SET updated_at = '2000-01-01 00:00:00' WHERE id = $1")
            .bind(manual_id.to_string())
            .execute(&app.pool)
            .await
            .unwrap();

        let history = get_game(State(app.clone()), Path(original_id))
            .await
            .into_response();
        assert_eq!(history.status(), StatusCode::OK);
        let body = to_bytes(history.into_body(), usize::MAX).await.unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(json["id"], original_id.to_string());
        assert_eq!(json["save_group_id"], save_group_id.to_string());
        assert_eq!(json["revision"], 3);
        assert_eq!(json["sect_name"], "试剑门");
        assert_eq!(json["state"]["year"], 1);

        let list = list_games(State(app)).await.into_response();
        assert_eq!(list.status(), StatusCode::OK);
        let body = to_bytes(list.into_body(), usize::MAX).await.unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        let group = &json["groups"][0];
        assert_eq!(group["save_group_id"], save_group_id.to_string());
        assert_eq!(group["current_id"], manual_id.to_string());
        assert_eq!(group["revision"], 3);
        assert_eq!(group["sect_name"], "新试剑门");
        assert_eq!(group["year"], 7);
        assert_eq!(group["month"], 4);
        assert_eq!(group["saves"][0]["id"], manual_id.to_string());
        assert_eq!(group["saves"][0]["revision"], 3);
        assert_eq!(group["saves"][1]["id"], original_id.to_string());
    }

    #[tokio::test]
    async fn revisioned_delete_returns_replacement_and_rejects_a_stale_request() {
        let initial = GameState::default();
        let (app, original_id, save_group_id) = seeded_app(&initial).await;
        let manual = create_manual_save(State(app.clone()), Path(original_id), revision_headers(1))
            .await
            .into_response();
        let body = to_bytes(manual.into_body(), usize::MAX).await.unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        let manual_id = Uuid::parse_str(json["id"].as_str().unwrap()).unwrap();

        let stale = delete_game(State(app.clone()), Path(original_id), revision_headers(1))
            .await
            .into_response();
        assert_eq!(stale.status(), StatusCode::CONFLICT);
        let body = to_bytes(stale.into_body(), usize::MAX).await.unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(json["code"], "stale_revision");
        assert_eq!(json["actual_revision"], 2);

        let deleted = delete_game(State(app.clone()), Path(manual_id), revision_headers(2))
            .await
            .into_response();
        assert_eq!(deleted.status(), StatusCode::OK);
        let body = to_bytes(deleted.into_body(), usize::MAX).await.unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(json["deleted_id"], manual_id.to_string());
        assert_eq!(json["save_group_id"], save_group_id.to_string());
        assert_eq!(json["current"]["id"], original_id.to_string());
        assert_eq!(json["current"]["revision"], 3);
    }

    #[tokio::test]
    async fn save_group_delete_requires_and_compares_revision() {
        let initial = GameState::default();
        let (app, original_id, save_group_id) = seeded_app(&initial).await;

        let missing = delete_save_group(State(app.clone()), Path(save_group_id), HeaderMap::new())
            .await
            .into_response();
        assert_eq!(missing.status(), StatusCode::PRECONDITION_REQUIRED);

        let manual = create_manual_save(State(app.clone()), Path(original_id), revision_headers(1))
            .await
            .into_response();
        assert_eq!(manual.status(), StatusCode::CREATED);

        let stale = delete_save_group(State(app.clone()), Path(save_group_id), revision_headers(1))
            .await
            .into_response();
        assert_eq!(stale.status(), StatusCode::CONFLICT);

        let deleted = delete_save_group(State(app), Path(save_group_id), revision_headers(2))
            .await
            .into_response();
        assert_eq!(deleted.status(), StatusCode::NO_CONTENT);
    }
}
