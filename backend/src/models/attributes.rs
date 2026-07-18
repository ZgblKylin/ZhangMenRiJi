use serde::{Deserialize, Deserializer, Serialize};
use std::collections::BTreeMap;

/// 侠客行式先天天赋。数值越高，相关修习与恢复越有利。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Aptitudes {
    pub strength: i32,
    pub intelligence: i32,
    pub constitution: i32,
    pub agility: i32,
    pub fortune: i32,
}

impl Default for Aptitudes {
    fn default() -> Self {
        Self {
            strength: 20,
            intelligence: 20,
            constitution: 20,
            agility: 20,
            fortune: 20,
        }
    }
}

impl Aptitudes {
    /// 基础拳脚、知识、基础内功、基础轻功每十级分别增益一项有效天赋。
    /// 招架与兵器只参与战斗，不直接改变人物天赋。
    pub fn with_skill_bonuses(
        &self,
        unarmed_level: i32,
        knowledge_level: i32,
        force_level: i32,
        dodge_level: i32,
    ) -> Self {
        Self {
            strength: self.strength + unarmed_level.max(0) / 10,
            intelligence: self.intelligence + knowledge_level.max(0) / 10,
            constitution: self.constitution + force_level.max(0) / 10,
            agility: self.agility + dodge_level.max(0) / 10,
            fortune: self.fortune,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct ResourcePool {
    pub current: i32,
    pub maximum: i32,
}

impl Default for ResourcePool {
    fn default() -> Self {
        Self {
            current: 100,
            maximum: 100,
        }
    }
}

/// 后天状态。内力、精力既是资源，也是内功与冥想修为的根基。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct AcquiredAttributes {
    pub qi: ResourcePool,
    pub spirit: ResourcePool,
    pub neili: ResourcePool,
    pub energy: ResourcePool,
    pub attainment: i64,
    pub reputation: i32,
    pub morality: i32,
    pub sect_loyalty: i32,
}

/// 丹药、事件带来的永久上限修正。打坐、冥想练成的上限直接保存在资源池中。
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct AttributeBonuses {
    pub qi: i32,
    pub spirit: i32,
    pub neili: i32,
    pub energy: i32,
}

impl Default for AcquiredAttributes {
    fn default() -> Self {
        Self {
            qi: ResourcePool::default(),
            spirit: ResourcePool::default(),
            neili: ResourcePool {
                current: 30,
                maximum: 30,
            },
            energy: ResourcePool {
                current: 50,
                maximum: 50,
            },
            attainment: 0,
            reputation: 0,
            morality: 50,
            sect_loyalty: 60,
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum DiscipleCondition {
    #[default]
    Healthy,
    Exhausted,
    Unconscious,
    SeriouslyInjured,
    Dead,
}

#[derive(Debug, Clone, Default, Serialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub enum DiscipleRank {
    Chore,
    #[default]
    Outer,
    Inner,
}

impl<'de> Deserialize<'de> for DiscipleRank {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let stored = String::deserialize(deserializer)?;
        match stored.as_str() {
            "chore" => Ok(Self::Chore),
            "outer" => Ok(Self::Outer),
            // v3.0 早期存档中的长老是等级；新版载入时保留其内门身份，
            // 再由建筑负责人字段决定是否担任长老。
            "inner" | "elder" => Ok(Self::Inner),
            _ => Err(serde::de::Error::unknown_variant(
                &stored,
                &["chore", "outer", "inner", "elder"],
            )),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Department {
    Transmission,
    Library,
    Apothecary,
    Treasury,
    Stewardship,
    ExternalAffairs,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ActionKind {
    Read,
    Practice,
    Teach,
    Spar,
    TemperBody,
    #[default]
    CultivateNeili,
    Meditate,
    SectMission,
    Wander,
    Recover,
    Maintain,
    Construct,
    Produce,
    Business,
    Gather,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum JourneyOutcome {
    Success,
    Partial,
    Failed,
}

/// 外派与游历在出发时即固定的旅程卷宗。旧存档缺失时由下一次月结稳定补建。
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct JourneyProgress {
    pub id: String,
    pub template_id: String,
    pub destination_id: String,
    pub difficulty: i32,
    pub total_months: i32,
    pub elapsed_months: i32,
    pub encounter_id: Option<String>,
    pub encounter_resolved: bool,
    pub outcome: Option<JourneyOutcome>,
    pub settled: bool,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct ActionPlan {
    pub kind: ActionKind,
    pub target_id: Option<String>,
    pub martial_art_id: Option<String>,
    pub assigned_by: Option<String>,
    pub remaining_months: i32,
    /// 外出任务的口粮是否已经从仓库申领，防止跨月重复支取。
    #[serde(default)]
    pub rations_claimed: bool,
    /// 一程一档：模板、目的地、难度、奇遇与最终结果只抽取一次。
    #[serde(default)]
    pub journey: Option<JourneyProgress>,
}

/// 单门武学的修习进度。经验达到下一级平方后升级，沿用侠客行 MUD 的技能门槛。
#[derive(Debug, Clone, Default, Serialize, PartialEq, Eq)]
pub struct SkillProgress {
    pub level: i32,
    pub experience: i64,
}

impl SkillProgress {
    pub fn new(level: i32, experience: i64) -> Self {
        let mut progress = Self {
            level: level.max(0),
            experience: experience.max(0),
        };
        progress.normalize();
        progress
    }

    pub fn experience_to_next_level(&self) -> i64 {
        i64::from(self.level.saturating_add(1)).pow(2)
    }

    /// 增加该门武学的经验，返回本次提升的等级数。
    pub fn gain_experience(&mut self, amount: i64) -> i32 {
        self.experience = self.experience.saturating_add(amount.max(0));
        let old_level = self.level;
        self.normalize();
        self.level - old_level
    }

    fn normalize(&mut self) {
        while self.experience >= self.experience_to_next_level() {
            self.experience -= self.experience_to_next_level();
            self.level = self.level.saturating_add(1);
        }
    }
}

impl<'de> Deserialize<'de> for SkillProgress {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(untagged)]
        enum StoredProgress {
            Current {
                #[serde(default)]
                level: i32,
                #[serde(default, alias = "exp")]
                experience: i64,
            },
            Legacy(i64),
        }

        Ok(match StoredProgress::deserialize(deserializer)? {
            StoredProgress::Current { level, experience } => Self::new(level, experience),
            StoredProgress::Legacy(level) => {
                Self::new(level.clamp(0, i64::from(i32::MAX)) as i32, 0)
            }
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct MartialProgress {
    /// 每门武学的独立等级与经验，键为武学 id。
    pub proficiencies: BTreeMap<String, SkillProgress>,
    pub specialties: Vec<String>,
    pub private_books: Vec<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn basic_skills_add_one_aptitude_per_ten_levels() {
        let innate = Aptitudes::default();
        let effective = innate.with_skill_bonuses(39, 40, 21, 19);
        assert_eq!(effective.strength, 23);
        assert_eq!(effective.intelligence, 24);
        assert_eq!(effective.constitution, 22);
        assert_eq!(effective.agility, 21);
        assert_eq!(effective.fortune, innate.fortune);
    }

    #[test]
    fn legacy_elder_rank_loads_as_inner_disciple() {
        let rank: DiscipleRank = serde_json::from_str("\"elder\"").unwrap();
        assert_eq!(rank, DiscipleRank::Inner);
        assert_eq!(serde_json::to_string(&rank).unwrap(), "\"inner\"");
    }

    #[test]
    fn legacy_action_plan_without_journey_still_loads() {
        let plan: ActionPlan = serde_json::from_value(serde_json::json!({
            "kind": "wander",
            "remaining_months": 2,
            "rations_claimed": true
        }))
        .unwrap();

        assert_eq!(plan.kind, ActionKind::Wander);
        assert_eq!(plan.remaining_months, 2);
        assert!(plan.rations_claimed);
        assert!(plan.journey.is_none());
    }
}
