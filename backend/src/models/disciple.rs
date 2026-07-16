use crate::models::attributes::{
    AcquiredAttributes, ActionPlan, Aptitudes, AttributeBonuses, Department, DiscipleCondition,
    DiscipleRank, MartialProgress,
};
use serde::{Deserialize, Serialize};

/// 弟子个人掌握的一门武学。`martial_art_id` 对应武学静态表中的 id。
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(default)]
pub struct SkillEntry {
    pub martial_art_id: String,
    pub level: i32,
    pub experience: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Disciple {
    pub id: String,
    pub sect_id: Option<String>,
    /// 武学来历；改投别派后仍用于决定本门兵器基础与知识技能。
    pub origin_sect_id: Option<String>,
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
    pub attribute_bonuses: AttributeBonuses,
    /// 0 为旧存档；1 表示已采用六类技能与派生上限公式。
    pub martial_schema_version: i32,
    pub condition: DiscipleCondition,
    pub rank: DiscipleRank,
    pub merit: i64,
    pub department: Option<Department>,
    pub master_id: Option<String>,
    pub relations: std::collections::BTreeMap<String, i32>,
    pub skills: Vec<SkillEntry>,
    pub martial_progress: MartialProgress,
    pub action: Option<ActionPlan>,
    pub away_months: i32,
}

impl Default for Disciple {
    fn default() -> Self {
        Self {
            id: String::new(),
            sect_id: Some("player".into()),
            origin_sect_id: Some("player".into()),
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
            attribute_bonuses: AttributeBonuses::default(),
            martial_schema_version: 0,
            condition: DiscipleCondition::default(),
            rank: DiscipleRank::default(),
            merit: 0,
            department: None,
            master_id: None,
            relations: std::collections::BTreeMap::new(),
            skills: vec![],
            martial_progress: MartialProgress::default(),
            action: None,
            away_months: 0,
        }
    }
}
