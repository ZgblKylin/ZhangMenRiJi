use crate::models::attributes::{
    AcquiredAttributes, ActionPlan, Aptitudes, Department, DiscipleCondition, DiscipleRank,
    MartialProgress,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Disciple {
    pub id: String,
    pub name: String,
    pub talent: i32,
    pub inner_power: i32,
    pub martial_art: String,
    pub loyalty: i32,
    pub months_in_sect: i32,
    pub alive: bool,
    pub age: i32,
    pub aptitudes: Aptitudes,
    pub attributes: AcquiredAttributes,
    pub condition: DiscipleCondition,
    pub rank: DiscipleRank,
    pub merit: i64,
    pub department: Option<Department>,
    pub master_id: Option<String>,
    pub relations: std::collections::BTreeMap<String, i32>,
    pub martial_progress: MartialProgress,
    pub action: Option<ActionPlan>,
    pub away_months: i32,
}

impl Default for Disciple {
    fn default() -> Self {
        Self {
            id: String::new(),
            name: String::new(),
            talent: 20,
            inner_power: 30,
            martial_art: "hunyuan".into(),
            loyalty: 60,
            months_in_sect: 0,
            alive: true,
            age: 18,
            aptitudes: Aptitudes::default(),
            attributes: AcquiredAttributes::default(),
            condition: DiscipleCondition::default(),
            rank: DiscipleRank::default(),
            merit: 0,
            department: None,
            master_id: None,
            relations: std::collections::BTreeMap::new(),
            martial_progress: MartialProgress::default(),
            action: None,
            away_months: 0,
        }
    }
}
