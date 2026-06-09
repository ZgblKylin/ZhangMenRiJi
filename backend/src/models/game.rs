use serde::{Deserialize, Serialize};
use crate::models::{Disciple, GameEvent};
use crate::models::tournament::TournamentRecord;

/// 完整游戏状态 — 对应前端 DEFAULT_STATE
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameState {
    pub year: i32,
    pub month: i32,
    pub prestige: i32,
    pub silver: i32,
    pub morale: i32,
    pub injury: i32,
    pub disciples: Vec<Disciple>,
    pub martial_arts_learned: Vec<String>,
    pub event_log: Vec<GameEvent>,
    pub decisions_used: i32,
    pub max_decisions: i32,
    pub total_disciples_recruited: i32,
    pub game_over: bool,
    pub game_over_reason: String,
    pub tournament_history: Vec<TournamentRecord>,
    pub pending_event: Option<serde_json::Value>,
}

impl Default for GameState {
    fn default() -> Self {
        Self {
            year: 1,
            month: 1,
            prestige: 45,
            silver: 500,
            morale: 55,
            injury: 0,
            disciples: vec![],
            martial_arts_learned: vec!["hunyuan".into()],
            event_log: vec![],
            decisions_used: 0,
            max_decisions: 3,
            total_disciples_recruited: 0,
            game_over: false,
            game_over_reason: String::new(),
            tournament_history: vec![],
            pending_event: None,
        }
    }
}

/// 游戏存档摘要（列表展示用）
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct GameSummary {
    pub id: uuid::Uuid,
    pub sect_name: String,
    pub state: serde_json::Value,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

/// 创建新游戏的请求
#[derive(Debug, Deserialize)]
pub struct CreateGameRequest {
    pub sect_name: String,
}
