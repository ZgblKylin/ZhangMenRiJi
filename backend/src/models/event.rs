use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameEvent {
    pub text: String,
    pub mood: String, // "good" | "bad" | "neutral"
    pub year: i32,
    pub month: i32,
    /// "sect" 为本门纪事，"world" 为 NPC 门派与江湖纪事。
    #[serde(default = "default_category")]
    pub category: String,
}

fn default_category() -> String {
    "sect".into()
}
