use crate::models::decision::all_decisions;
use crate::models::martial_art::all_martial_arts;
use axum::{response::IntoResponse, Json};

/// GET /api/decisions — 返回全部 8 种决策的静态定义
pub async fn list_decisions() -> impl IntoResponse {
    Json(serde_json::json!({ "decisions": all_decisions() }))
}

/// GET /api/martial-arts — 返回全部 5 门武学的静态数据
pub async fn list_martial_arts() -> impl IntoResponse {
    Json(serde_json::json!({ "arts": all_martial_arts() }))
}
