use crate::config::AppConfig;
use crate::handlers::games::AppState;
use axum::{extract::State, http::StatusCode, response::IntoResponse, Json};

/// GET /api/config
pub async fn get_config(State(state): State<AppState>) -> Json<AppConfig> {
    Json(state.config.read().await.clone())
}

/// POST /api/config
pub async fn save_config(
    State(state): State<AppState>,
    Json(mut config): Json<AppConfig>,
) -> impl IntoResponse {
    config.db_path = config.db_path.trim().to_owned();
    if config.db_path.is_empty() {
        return (StatusCode::BAD_REQUEST, "数据库路径不能为空").into_response();
    }

    let Some(path) = state.config_path.as_deref() else {
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            "当前运行方式未提供配置文件路径",
        )
            .into_response();
    };

    if let Err(error) = config.save_to_file(path) {
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("保存配置失败: {error}"),
        )
            .into_response();
    }

    *state.config.write().await = config;
    StatusCode::NO_CONTENT.into_response()
}
