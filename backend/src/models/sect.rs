use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct SectAttributes {
    pub prestige: i32,
    pub silver: i32,
    pub morality: i32,
    pub morale: i32,
}

impl Default for SectAttributes {
    fn default() -> Self {
        Self {
            prestige: 45,
            silver: 500,
            morality: 55,
            morale: 55,
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SectPolicy {
    #[default]
    Balanced,
    Martial,
    Scholarly,
    Chivalrous,
    Mercantile,
    Reclusive,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum MoralDirection {
    #[default]
    Righteous,
    Neutral,
    Villainous,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct RankRules {
    pub outer_ratio: f32,
    pub inner_ratio: f32,
}

impl Default for RankRules {
    fn default() -> Self {
        Self {
            outer_ratio: 0.4,
            inner_ratio: 0.3,
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum BuildingKind {
    #[default]
    Practice,
    Scripture,
    Warehouse,
    HerbHall,
    Intelligence,
    Affairs,
    Logistics,
}

impl BuildingKind {
    pub fn default_elder_duty(&self) -> &'static str {
        match self {
            Self::Practice => "instruct",
            Self::Scripture => "curate",
            Self::Warehouse => "audit",
            Self::HerbHall => "treat",
            Self::Intelligence => "correspond",
            Self::Affairs => "recruit",
            Self::Logistics => "maintain",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Building {
    pub id: String,
    pub name: String,
    pub kind: BuildingKind,
    pub level: i32,
    pub condition: i32,
    pub upgrading_months: i32,
    pub elder_id: Option<String>,
    pub elder_title: String,
    pub selected_duty: Option<String>,
    /// 长老事务的二级目标（如扩建目标建筑 id）。
    #[serde(default)]
    pub duty_target: Option<String>,
    pub elder_action_used: bool,
    pub work_required: i32,
    pub work_invested: i32,
}

impl Default for Building {
    fn default() -> Self {
        Self {
            id: String::new(),
            name: String::new(),
            kind: BuildingKind::default(),
            level: 1,
            condition: 100,
            upgrading_months: 0,
            elder_id: None,
            elder_title: String::new(),
            selected_duty: None,
            duty_target: None,
            elder_action_used: false,
            work_required: 0,
            work_invested: 0,
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct SectOrder {
    pub id: String,
    pub name: String,
    pub remaining_months: i32,
    pub silver_cost: i32,
    pub effect: BTreeMap<String, i32>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct ProductionTask {
    pub id: String,
    pub name: String,
    pub output_item: String,
    pub quantity: i32,
    pub remaining_months: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct SectState {
    pub id: String,
    pub name: String,
    pub country_id: String,
    pub player_controlled: bool,
    pub attributes: SectAttributes,
    pub policy: SectPolicy,
    pub moral_direction: MoralDirection,
    pub rank_rules: RankRules,
    pub buildings: Vec<Building>,
    pub inventory: BTreeMap<String, i32>,
    pub public_books: Vec<String>,
    pub martial_research: BTreeMap<String, i64>,
    pub relations: BTreeMap<String, i32>,
    pub active_orders: Vec<SectOrder>,
    pub productions: Vec<ProductionTask>,
    /// 后台自动循环炼制所处的配方索引。
    pub auto_brew_index: usize,
    /// 当前自动炼制进度（月）。无长老时每两个月增加一月进度。
    pub auto_brew_progress: i32,
}

impl Default for SectState {
    fn default() -> Self {
        Self {
            id: "player".into(),
            name: "无名派".into(),
            country_id: "song".into(),
            player_controlled: true,
            attributes: SectAttributes::default(),
            policy: SectPolicy::default(),
            moral_direction: MoralDirection::default(),
            rank_rules: RankRules::default(),
            buildings: default_buildings(),
            inventory: BTreeMap::from([
                ("粮秣".into(), 80),
                ("草药".into(), 20),
                ("精铁".into(), 10),
            ]),
            public_books: vec!["player_knowledge".into(), "hunyuan".into()],
            martial_research: BTreeMap::new(),
            relations: BTreeMap::new(),
            active_orders: vec![],
            productions: vec![],
            auto_brew_index: 0,
            auto_brew_progress: 0,
        }
    }
}

pub fn default_buildings() -> Vec<Building> {
    [
        ("practice", "演武场", BuildingKind::Practice, "传武长老"),
        ("scripture", "藏经阁", BuildingKind::Scripture, "传功长老"),
        ("warehouse", "仓库", BuildingKind::Warehouse, "司库长老"),
        ("herb_hall", "百草堂", BuildingKind::HerbHall, "司药长老"),
        (
            "intelligence",
            "天枢阁",
            BuildingKind::Intelligence,
            "天枢长老",
        ),
        ("affairs", "执事堂", BuildingKind::Affairs, "执事长老"),
        ("logistics", "庶务堂", BuildingKind::Logistics, "庶务长老"),
    ]
    .into_iter()
    .map(|(id, name, kind, elder_title)| Building {
        id: id.into(),
        name: name.into(),
        selected_duty: Some(kind.default_elder_duty().into()),
        kind,
        elder_title: elder_title.into(),
        ..Building::default()
    })
    .collect()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Country {
    pub id: String,
    pub name: String,
    pub prosperity: i32,
    pub order: i32,
}

pub fn default_countries() -> Vec<Country> {
    vec![
        Country {
            id: "yuan".into(),
            name: "大元".into(),
            prosperity: 72,
            order: 68,
        },
        Country {
            id: "song".into(),
            name: "大宋".into(),
            prosperity: 85,
            order: 62,
        },
        Country {
            id: "dali".into(),
            name: "大理".into(),
            prosperity: 70,
            order: 78,
        },
        Country {
            id: "xia".into(),
            name: "大夏".into(),
            prosperity: 58,
            order: 55,
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::Building;

    #[test]
    fn legacy_building_without_duty_target_deserializes() {
        let building: Building = serde_json::from_value(serde_json::json!({
            "id": "logistics",
            "name": "庶务堂",
            "kind": "logistics",
            "level": 2
        }))
        .unwrap();

        assert_eq!(building.id, "logistics");
        assert_eq!(building.level, 2);
        assert_eq!(building.duty_target, None);
    }
}
