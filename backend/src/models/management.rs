use crate::models::attributes::{ActionKind, Department, DiscipleRank};
use crate::models::sect::{MoralDirection, SectPolicy};
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
    SetMoralDirection {
        direction: MoralDirection,
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
    AssignElder {
        building_id: String,
        disciple_id: Option<String>,
    },
    SetElderDuty {
        building_id: String,
        duty_id: String,
    },
    Expel {
        disciple_id: String,
    },
    IssueItem {
        disciple_id: String,
        item: String,
        quantity: i32,
    },
    BrewPill {
        recipe_id: String,
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
        disciple_id: Option<String>,
    },
    RequestManual {
        sect_id: String,
        martial_art_id: String,
        disciple_id: Option<String>,
    },
}
