pub mod attributes;
pub mod decision;
pub mod disciple;
pub mod event;
pub mod game;
pub mod management;
pub mod martial_art;
pub mod medicine;
// 固定名册先作为静态模型 API 提供，世界生成接入前不触发 dead_code 告警。
#[allow(dead_code)]
pub mod named_npc;
pub mod sect;
pub mod tournament;

pub use disciple::{Disciple, SkillEntry};
pub use event::GameEvent;
pub use game::GameState;
