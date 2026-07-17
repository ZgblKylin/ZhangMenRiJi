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

fn game_response(
    id: Uuid,
    save_group_id: Uuid,
    sect_name: String,
    game_state: crate::models::GameState,
) -> serde_json::Value {
    serde_json::json!({
        "id": id,
        "save_group_id": save_group_id,
        "sect_name": sect_name,
        "state": game_state,
    })
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
                game_state.martial_arts_learned = vec!["player_knowledge".into(), "hunyuan".into()];
            } // rng drop here
            crate::logic::sect::hydrate_player_sect(&mut game_state, &req.sect_name);
            let _ = crate::db::update_game(&state.pool, id, &req.sect_name, &game_state).await;
            let save_group_id = crate::db::get_game_group(&state.pool, id)
                .await
                .ok()
                .flatten()
                .unwrap_or(id);
            (
                StatusCode::CREATED,
                Json(game_response(id, save_group_id, req.sect_name, game_state)),
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
            // 查询结果已按更新时间倒序；槽位及槽内存档自然都保持“最近优先”。
            let mut groups: Vec<serde_json::Value> = Vec::new();
            for game in games {
                let save = serde_json::json!({
                    "id": game.id,
                    "save_group_id": game.save_group_id,
                    "save_type": game.save_type,
                    "autosave": game.autosave,
                    "year": game.year,
                    "month": game.month,
                    "updated_at": game.updated_at.to_rfc3339(),
                });
                if let Some(group) = groups.iter_mut().find(|group| {
                    group.get("save_group_id").and_then(|v| v.as_str())
                        == Some(&game.save_group_id.to_string())
                }) {
                    group["saves"].as_array_mut().unwrap().push(save);
                } else {
                    groups.push(serde_json::json!({
                        "save_group_id": game.save_group_id,
                        "sect_name": game.sect_name,
                        "year": game.year,
                        "month": game.month,
                        "updated_at": game.updated_at.to_rfc3339(),
                        "saves": [save],
                    }));
                }
            }
            Json(serde_json::json!({ "groups": groups })).into_response()
        }
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

/// GET /api/games/:id
pub async fn get_game(State(state): State<AppState>, Path(id): Path<Uuid>) -> impl IntoResponse {
    match crate::db::get_game(&state.pool, id).await {
        Ok(Some((sect_name, game_state))) => {
            match crate::db::get_game_group(&state.pool, id).await {
                Ok(Some(group_id)) => {
                    Json(game_response(id, group_id, sect_name, game_state)).into_response()
                }
                Ok(None) => (StatusCode::NOT_FOUND, "存档不存在").into_response(),
                Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
            }
        }
        Ok(None) => (StatusCode::NOT_FOUND, "存档不存在").into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

/// POST /api/games/:id/saves — 将当前状态手动存入同一槽位。
pub async fn create_manual_save(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> impl IntoResponse {
    let (sect_name, game_state) = match crate::db::get_game(&state.pool, id).await {
        Ok(Some(game)) => game,
        Ok(None) => return (StatusCode::NOT_FOUND, "存档不存在").into_response(),
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    };
    match crate::db::create_save(&state.pool, id, &sect_name, &game_state, false).await {
        Ok(Some(save_id)) => {
            let group_id = crate::db::get_game_group(&state.pool, save_id)
                .await
                .ok()
                .flatten()
                .unwrap_or(save_id);
            (
                StatusCode::CREATED,
                Json(game_response(save_id, group_id, sect_name, {
                    let mut saved = game_state;
                    saved.autosave = false;
                    saved
                })),
            )
                .into_response()
        }
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

/// DELETE /api/save-groups/:id
pub async fn delete_save_group(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> impl IntoResponse {
    match crate::db::delete_save_group(&state.pool, id).await {
        Ok(true) => StatusCode::NO_CONTENT.into_response(),
        Ok(false) => (StatusCode::NOT_FOUND, "槽位不存在").into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}
