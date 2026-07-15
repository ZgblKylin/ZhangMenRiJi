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

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Building {
    pub id: String,
    pub name: String,
    pub level: i32,
    pub condition: i32,
    pub upgrading_months: i32,
}

impl Default for Building {
    fn default() -> Self {
        Self {
            id: String::new(),
            name: String::new(),
            level: 1,
            condition: 100,
            upgrading_months: 0,
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

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct SectState {
    pub id: String,
    pub name: String,
    pub country_id: String,
    pub player_controlled: bool,
    pub attributes: SectAttributes,
    pub policy: SectPolicy,
    pub buildings: Vec<Building>,
    pub inventory: BTreeMap<String, i32>,
    pub public_books: Vec<String>,
    pub martial_research: BTreeMap<String, i64>,
    pub relations: BTreeMap<String, i32>,
    pub active_orders: Vec<SectOrder>,
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
            buildings: default_buildings(),
            inventory: BTreeMap::from([
                ("粮秣".into(), 80),
                ("草药".into(), 20),
                ("精铁".into(), 10),
            ]),
            public_books: vec!["hunyuan".into()],
            martial_research: BTreeMap::new(),
            relations: BTreeMap::new(),
            active_orders: vec![],
        }
    }
}

pub fn default_buildings() -> Vec<Building> {
    [
        ("scripture", "藏经阁"),
        ("treasury", "银库"),
        ("pharmacy", "药库"),
        ("inner_store", "内门仓库"),
        ("outer_store", "外门仓库"),
        ("practice", "演武场"),
    ]
    .into_iter()
    .map(|(id, name)| Building {
        id: id.into(),
        name: name.into(),
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
