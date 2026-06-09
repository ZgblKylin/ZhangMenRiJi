use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TournamentRecord {
    pub year: i32,
    pub rank: i32,
    pub total_sects: i32,
    pub power: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TournamentResult {
    pub rank: i32,
    pub total_sects: i32,
    pub power: i32,
    pub desc_text: String,
    pub reward_silver: i32,
    pub reward_prestige: i32,
}
