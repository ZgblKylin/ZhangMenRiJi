use crate::db::{RevisionWriteResult, RevisionedGame};
use crate::models::GameState;
use axum::{
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    Json,
};
use sqlx::SqlitePool;
use uuid::Uuid;

const SAVE_REVISION_HEADER: &str = "x-save-revision";

pub fn game_response(
    id: Uuid,
    save_group_id: Uuid,
    revision: i64,
    sect_name: &str,
    state: &GameState,
) -> serde_json::Value {
    serde_json::json!({
        "id": id,
        "save_group_id": save_group_id,
        "revision": revision,
        "sect_name": sect_name,
        "state": state,
    })
}

pub fn revisioned_game_response(game: &RevisionedGame) -> serde_json::Value {
    let mut response = game_response(
        game.id,
        game.save_group_id,
        game.revision,
        &game.sect_name,
        &game.state,
    );
    response["current_id"] = serde_json::json!(game.current_game_id);
    response
}

pub fn parse_expected_revision(headers: &HeaderMap) -> Result<i64, Response> {
    let Some(value) = headers.get(SAVE_REVISION_HEADER) else {
        return Err((
            StatusCode::PRECONDITION_REQUIRED,
            Json(serde_json::json!({
                "code": "missing_revision",
                "error": "缺少 X-Save-Revision 请求头。",
            })),
        )
            .into_response());
    };

    let revision = value
        .to_str()
        .ok()
        .and_then(|value| value.trim().parse::<i64>().ok())
        .filter(|revision| *revision > 0);
    revision.ok_or_else(|| {
        (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({
                "code": "invalid_revision",
                "error": "X-Save-Revision 必须是正整数。",
            })),
        )
            .into_response()
    })
}

pub async fn verify_expected_revision(
    pool: &SqlitePool,
    loaded: &RevisionedGame,
    expected_revision: i64,
) -> Result<(), Response> {
    if loaded.revision == expected_revision {
        return Ok(());
    }

    match crate::db::get_current_game(pool, loaded.save_group_id).await {
        Ok(Some(current)) => Err(stale_revision_response(expected_revision, &current)),
        Ok(None) => Err(not_found_response()),
        Err(error) => Err(internal_error_response(error)),
    }
}

pub fn require_applied<T>(
    result: RevisionWriteResult<T>,
    expected_revision: i64,
) -> Result<T, Response> {
    match result {
        RevisionWriteResult::Applied(value) => Ok(value),
        RevisionWriteResult::Conflict(current) => {
            Err(stale_revision_response(expected_revision, &current))
        }
        RevisionWriteResult::NotFound => Err(not_found_response()),
    }
}

pub fn stale_revision_response(expected_revision: i64, current: &RevisionedGame) -> Response {
    (
        StatusCode::CONFLICT,
        Json(stale_revision_body(expected_revision, current)),
    )
        .into_response()
}

pub fn state_conflict_response(
    code: &'static str,
    error: &'static str,
    state: &GameState,
) -> Response {
    (
        StatusCode::CONFLICT,
        Json(serde_json::json!({
            "code": code,
            "error": error,
            "state": state,
        })),
    )
        .into_response()
}

pub fn not_found_response() -> Response {
    (
        StatusCode::NOT_FOUND,
        Json(serde_json::json!({
            "code": "not_found",
            "error": "存档不存在",
        })),
    )
        .into_response()
}

pub fn internal_error_response(error: impl std::fmt::Display) -> Response {
    (
        StatusCode::INTERNAL_SERVER_ERROR,
        Json(serde_json::json!({
            "code": "internal_error",
            "error": error.to_string(),
        })),
    )
        .into_response()
}

fn stale_revision_body(expected_revision: i64, current: &RevisionedGame) -> serde_json::Value {
    serde_json::json!({
        "code": "stale_revision",
        "error": "存档已更新，请刷新当前进度后重试。",
        "expected_revision": expected_revision,
        "actual_revision": current.revision,
        "current": revisioned_game_response(current),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::to_bytes;
    use axum::http::HeaderValue;

    #[test]
    fn revision_header_accepts_only_positive_integers() {
        let mut headers = HeaderMap::new();
        headers.insert(SAVE_REVISION_HEADER, HeaderValue::from_static("17"));
        assert_eq!(parse_expected_revision(&headers).unwrap(), 17);

        for value in ["0", "-1", "1.5", "old"] {
            headers.insert(SAVE_REVISION_HEADER, HeaderValue::from_str(value).unwrap());
            assert_eq!(
                parse_expected_revision(&headers).unwrap_err().status(),
                StatusCode::BAD_REQUEST
            );
        }
    }

    #[tokio::test]
    async fn missing_revision_header_returns_428_with_a_distinct_code() {
        let response = parse_expected_revision(&HeaderMap::new()).unwrap_err();
        assert_eq!(response.status(), StatusCode::PRECONDITION_REQUIRED);
        let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(json["code"], "missing_revision");
    }

    #[tokio::test]
    async fn stale_conflict_contains_the_complete_current_game_response() {
        let id = Uuid::new_v4();
        let save_group_id = Uuid::new_v4();
        let current = RevisionedGame {
            id,
            save_group_id,
            current_game_id: id,
            revision: 9,
            sect_name: "试剑门".into(),
            state: GameState::default(),
        };

        let response =
            require_applied::<()>(RevisionWriteResult::Conflict(current), 7).unwrap_err();
        assert_eq!(response.status(), StatusCode::CONFLICT);
        let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();

        assert_eq!(json["code"], "stale_revision");
        assert_eq!(json["expected_revision"], 7);
        assert_eq!(json["actual_revision"], 9);
        assert_eq!(json["current"]["id"], id.to_string());
        assert_eq!(json["current"]["save_group_id"], save_group_id.to_string());
        assert_eq!(json["current"]["revision"], 9);
        assert_eq!(json["current"]["sect_name"], "试剑门");
        assert!(json["current"].get("state").is_some());
    }
}
