use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameEvent {
    pub text: String,
    pub mood: String, // "good" | "bad" | "neutral"
    pub year: i32,
    pub month: i32,
}
