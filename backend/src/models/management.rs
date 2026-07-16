use crate::models::attributes::{ActionKind, Department, DiscipleRank};
use crate::models::sect::SectPolicy;
use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "action", rename_all = "snake_case")]
pub enum ManagementRequest {
    AssignAction {
        disciple_id: String,
        kind: ActionKind,
        target_id: Option<String>,
        martial_art_id: Option<String>,
    },
    EquipSkill {
        disciple_id: String,
        basic_skill_id: String,
        martial_art_id: String,
    },
    SetPolicy {
        policy: SectPolicy,
    },
    UpgradeBuilding {
        building_id: String,
    },
    RepairBuilding {
        building_id: String,
    },
    Recruit,
    SetPersonnel {
        disciple_id: String,
        rank: DiscipleRank,
        department: Option<Department>,
    },
    Expel {
        disciple_id: String,
    },
    IssueItem {
        disciple_id: String,
        item: String,
        quantity: i32,
    },
    IssueOrder {
        order_id: String,
    },
    LibraryAdd {
        martial_art_id: String,
    },
    ResearchMartial {
        martial_art_id: String,
    },
    ResearchNewMartial,
    Exchange {
        sect_id: String,
    },
    RequestManual {
        sect_id: String,
        martial_art_id: String,
    },
}
