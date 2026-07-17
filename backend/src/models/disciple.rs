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
    /// 基础技能 id 到当前准备战斗武学 id 的映射；知识使用 `knowledge` 键自动选择。
    #[serde(alias = "equipped_skills")]
    pub prepared_skills: std::collections::BTreeMap<String, String>,
    pub loyalty: i32,
    pub months_in_sect: i32,
    pub alive: bool,
    pub age: i32,
    pub aptitudes: Aptitudes,
    pub attributes: AcquiredAttributes,
    pub attribute_bonuses: AttributeBonuses,
    /// 0 为旧存档；1 为六类技能；2 为准备武学与独立修炼上限。
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
    /// 弟子自行支配的银两，旧存档缺失时从零开始。
    #[serde(default)]
    pub personal_silver: i32,
    /// 已由门派发给弟子随身携带的口粮。
    #[serde(default)]
    pub personal_rations: i32,
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
            prepared_skills: std::collections::BTreeMap::new(),
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
            personal_silver: 0,
            personal_rations: 0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Disciple;

    #[test]
    fn legacy_skill_preparation_field_is_migrated() {
        let disciple: Disciple = serde_json::from_value(serde_json::json!({
            "equipped_skills": { "basic_force": "hunyuan" }
        }))
        .unwrap();

        assert_eq!(
            disciple.prepared_skills.get("basic_force"),
            Some(&"hunyuan".to_string())
        );

        let serialized = serde_json::to_value(disciple).unwrap();
        assert!(serialized.get("prepared_skills").is_some());
        assert!(serialized.get("equipped_skills").is_none());
    }

    #[test]
    fn legacy_disciple_defaults_personal_resources_to_zero() {
        let disciple: Disciple = serde_json::from_value(serde_json::json!({
            "id": "legacy"
        }))
        .unwrap();

        assert_eq!(disciple.personal_silver, 0);
        assert_eq!(disciple.personal_rations, 0);
    }
}
