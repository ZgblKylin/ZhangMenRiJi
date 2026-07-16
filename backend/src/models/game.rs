use crate::models::sect::{default_countries, Country, SectState};
use crate::models::tournament::TournamentRecord;
use crate::models::{Disciple, GameEvent};
use serde::{Deserialize, Serialize};

/// 完整游戏状态 — 对应前端 DEFAULT_STATE
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct GameState {
    pub schema_version: i32,
    /// 当前存档是否由月令推进自动生成；旧存档反序列化时默认为 false。
    pub autosave: bool,
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
    pub sect: SectState,
    pub npc_sects: Vec<SectState>,
    pub npc_disciples: Vec<Disciple>,
    pub countries: Vec<Country>,
    pub world_seed: u64,
}

impl Default for GameState {
    fn default() -> Self {
        Self {
            schema_version: 3,
            autosave: false,
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
            sect: SectState::default(),
            npc_sects: vec![],
            npc_disciples: vec![],
            countries: default_countries(),
            world_seed: rand::random(),
        }
    }
}

/// 游戏存档摘要（列表展示用）
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct GameSummary {
    pub id: uuid::Uuid,
    pub save_group_id: uuid::Uuid,
    pub save_type: String,
    pub sect_name: String,
    pub state: serde_json::Value,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

/// 创建新游戏的请求
#[derive(Debug, Deserialize)]
pub struct CreateGameRequest {
    pub sect_name: String,
}
