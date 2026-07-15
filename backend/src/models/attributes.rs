use serde::{Deserialize, Serialize};
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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum DiscipleCondition {
    Healthy,
    Exhausted,
    Unconscious,
    SeriouslyInjured,
    Dead,
}

impl Default for DiscipleCondition {
    fn default() -> Self {
        Self::Healthy
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub enum DiscipleRank {
    Chore,
    Outer,
    Inner,
    Elder,
}

impl Default for DiscipleRank {
    fn default() -> Self {
        Self::Outer
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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ActionKind {
    Read,
    Teach,
    Spar,
    TemperBody,
    CultivateNeili,
    Meditate,
    SectMission,
    Wander,
    Recover,
}

impl Default for ActionKind {
    fn default() -> Self {
        Self::CultivateNeili
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct ActionPlan {
    pub kind: ActionKind,
    pub target_id: Option<String>,
    pub martial_art_id: Option<String>,
    pub assigned_by: Option<String>,
    pub remaining_months: i32,
}

impl Default for ActionPlan {
    fn default() -> Self {
        Self {
            kind: ActionKind::default(),
            target_id: None,
            martial_art_id: None,
            assigned_by: None,
            remaining_months: 0,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct MartialProgress {
    /// 每门武学的熟练造诣，键为武学 id。
    pub proficiencies: BTreeMap<String, i64>,
    pub specialties: Vec<String>,
    pub private_books: Vec<String>,
}
