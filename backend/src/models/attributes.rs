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

/// 丹药、事件与长期修炼带来的永久上限修正。派生上限重算时不会丢失这些值。
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

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub enum DiscipleRank {
    Chore,
    #[default]
    Outer,
    Inner,
    Elder,
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
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct ActionPlan {
    pub kind: ActionKind,
    pub target_id: Option<String>,
    pub martial_art_id: Option<String>,
    pub assigned_by: Option<String>,
    pub remaining_months: i32,
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
