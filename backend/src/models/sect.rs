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
    /// 门派卷宗中的简述；旧存档缺失时为空。
    #[serde(default)]
    pub description: String,
    /// 门派驻地或朝廷中枢；旧存档缺失时为空。
    #[serde(default)]
    pub landmark: String,
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
            description: "新立山门，百业待兴。".into(),
            landmark: "山门".into(),
            country_id: "song".into(),
            player_controlled: true,
            attributes: SectAttributes::default(),
            policy: SectPolicy::default(),
            moral_direction: MoralDirection::default(),
            rank_rules: RankRules::default(),
            buildings: default_buildings(),
            inventory: BTreeMap::from([
                ("粮秣".into(), 50),
                ("草药".into(), 20),
                ("精铁".into(), 30),
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
    default_buildings_with_titles(
        "传武长老",
        "传功长老",
        "司库长老",
        "司药长老",
        "天枢长老",
        "执事长老",
        "庶务长老",
    )
}

pub fn default_buildings_with_titles(
    practice_title: &str,
    scripture_title: &str,
    warehouse_title: &str,
    herb_hall_title: &str,
    intelligence_title: &str,
    affairs_title: &str,
    logistics_title: &str,
) -> Vec<Building> {
    [
        ("practice", "演武场", BuildingKind::Practice, practice_title),
        (
            "scripture",
            "藏经阁",
            BuildingKind::Scripture,
            scripture_title,
        ),
        (
            "warehouse",
            "仓库",
            BuildingKind::Warehouse,
            warehouse_title,
        ),
        (
            "herb_hall",
            "百草堂",
            BuildingKind::HerbHall,
            herb_hall_title,
        ),
        (
            "intelligence",
            "天枢阁",
            BuildingKind::Intelligence,
            intelligence_title,
        ),
        ("affairs", "执事堂", BuildingKind::Affairs, affairs_title),
        (
            "logistics",
            "庶务堂",
            BuildingKind::Logistics,
            logistics_title,
        ),
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

pub fn sect_buildings(sect_id: &str) -> Vec<Building> {
    let titles = match sect_id {
        "wudang" => [
            "演武真人",
            "掌经真人",
            "司库真人",
            "丹房真人",
            "知客真人",
            "执事真人",
            "督造真人",
        ],
        "shaolin" => [
            "罗汉堂首座",
            "达摩院首座",
            "监寺",
            "药王院首座",
            "知客僧",
            "戒律院首座",
            "都寺",
        ],
        "mingjiao" => [
            "光明左使",
            "光明右使",
            "掌库使",
            "掌药使",
            "五散人",
            "掌旗使",
            "掌工使",
        ],
        "quanzhen" => [
            "演武真人",
            "藏经真人",
            "司库真人",
            "丹鼎真人",
            "知客真人",
            "巡察真人",
            "督造真人",
        ],
        "tianlong" => [
            "武僧统领",
            "参经长老",
            "库头",
            "药师长老",
            "知客长老",
            "戒律长老",
            "督工长老",
        ],
        "taohua" => [
            "奇门护法",
            "书阁护法",
            "库房护法",
            "丹房护法",
            "知客护法",
            "巡察护法",
            "督造护法",
        ],
        "gumu" => ["剑侍", "书侍", "库侍", "药侍", "知客侍", "巡察侍", "匠侍"],
        "gaibang" => [
            "执法长老",
            "传功长老",
            "掌钵长老",
            "掌药长老",
            "掌棒长老",
            "巡察长老",
            "督工长老",
        ],
        "emei" => [
            "演武师太",
            "藏经师太",
            "司库师太",
            "药师师太",
            "知客师太",
            "戒律师太",
            "督造师太",
        ],
        "riyue" => [
            "左护法",
            "右护法",
            "掌库使",
            "掌药使",
            "风雷堂主",
            "青龙堂主",
            "督造使",
        ],
        "xingxiu" => [
            "大弟子",
            "秘典使",
            "库使",
            "毒药使",
            "知客使",
            "巡察使",
            "匠使",
        ],
        "murong" => [
            "家将统领",
            "书阁统领",
            "司库统领",
            "丹房统领",
            "知客统领",
            "巡察统领",
            "督造统领",
        ],
        "dalun" => [
            "护法金刚",
            "经阁上师",
            "库头上师",
            "药王上师",
            "知客上师",
            "戒律上师",
            "督工上师",
        ],
        "court" => [
            "怯薛长",
            "翰林学士",
            "度支使",
            "太医令",
            "宣徽使",
            "御史中丞",
            "将作监",
        ],
        "lingjiu" => [
            "钧天部",
            "昊天部",
            "库使",
            "药使",
            "阳天部",
            "朱天部",
            "匠使",
        ],
        "baituo" => [
            "蛇奴统领",
            "秘典使",
            "库使",
            "药师",
            "知客使",
            "巡察使",
            "匠使",
        ],
        "xueshan" => [
            "剑术教头",
            "书阁长老",
            "司库长老",
            "药师长老",
            "知客长老",
            "巡察长老",
            "督造长老",
        ],
        "tiandihui" => [
            "香主",
            "书阁香主",
            "掌库香主",
            "掌药香主",
            "知客香主",
            "巡察香主",
            "督造香主",
        ],
        "shenlong" => [
            "黑龙使",
            "赤龙使",
            "掌库使",
            "掌药使",
            "白龙使",
            "青龙使",
            "督造使",
        ],
        "jueqing" => ["渔隐", "书隐", "库隐", "药隐", "知客隐", "巡察隐", "匠隐"],
        "wudu" => [
            "蛊师统领",
            "秘典蛊师",
            "库房蛊师",
            "药蛊师",
            "知客蛊师",
            "巡察蛊师",
            "督造蛊师",
        ],
        "qingcheng" => [
            "剑术教头",
            "书阁长老",
            "司库长老",
            "丹房长老",
            "知客长老",
            "巡察长老",
            "督造长老",
        ],
        "song_court" => [
            "马军都指挥",
            "翰林承旨",
            "三司使",
            "翰林医官",
            "客省使",
            "御史中丞",
            "将作监",
        ],
        _ => return default_buildings(),
    };

    default_buildings_with_titles(
        titles[0], titles[1], titles[2], titles[3], titles[4], titles[5], titles[6],
    )
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
    use super::{default_buildings, sect_buildings, Building, SectState};

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

    #[test]
    fn legacy_sect_defaults_new_roll_fields() {
        let sect: SectState = serde_json::from_value(serde_json::json!({
            "id": "legacy",
            "name": "旧门派"
        }))
        .unwrap();

        assert_eq!(sect.description, "");
        assert_eq!(sect.landmark, "");
    }

    #[test]
    fn sect_building_titles_match_faction_customs() {
        let expected = [
            (
                "wudang",
                "演武真人/掌经真人/司库真人/丹房真人/知客真人/执事真人/督造真人",
            ),
            (
                "shaolin",
                "罗汉堂首座/达摩院首座/监寺/药王院首座/知客僧/戒律院首座/都寺",
            ),
            (
                "mingjiao",
                "光明左使/光明右使/掌库使/掌药使/五散人/掌旗使/掌工使",
            ),
            (
                "quanzhen",
                "演武真人/藏经真人/司库真人/丹鼎真人/知客真人/巡察真人/督造真人",
            ),
            (
                "tianlong",
                "武僧统领/参经长老/库头/药师长老/知客长老/戒律长老/督工长老",
            ),
            (
                "taohua",
                "奇门护法/书阁护法/库房护法/丹房护法/知客护法/巡察护法/督造护法",
            ),
            ("gumu", "剑侍/书侍/库侍/药侍/知客侍/巡察侍/匠侍"),
            (
                "gaibang",
                "执法长老/传功长老/掌钵长老/掌药长老/掌棒长老/巡察长老/督工长老",
            ),
            (
                "emei",
                "演武师太/藏经师太/司库师太/药师师太/知客师太/戒律师太/督造师太",
            ),
            (
                "riyue",
                "左护法/右护法/掌库使/掌药使/风雷堂主/青龙堂主/督造使",
            ),
            ("xingxiu", "大弟子/秘典使/库使/毒药使/知客使/巡察使/匠使"),
            (
                "murong",
                "家将统领/书阁统领/司库统领/丹房统领/知客统领/巡察统领/督造统领",
            ),
            (
                "dalun",
                "护法金刚/经阁上师/库头上师/药王上师/知客上师/戒律上师/督工上师",
            ),
            (
                "court",
                "怯薛长/翰林学士/度支使/太医令/宣徽使/御史中丞/将作监",
            ),
            ("lingjiu", "钧天部/昊天部/库使/药使/阳天部/朱天部/匠使"),
            ("baituo", "蛇奴统领/秘典使/库使/药师/知客使/巡察使/匠使"),
            (
                "xueshan",
                "剑术教头/书阁长老/司库长老/药师长老/知客长老/巡察长老/督造长老",
            ),
            (
                "tiandihui",
                "香主/书阁香主/掌库香主/掌药香主/知客香主/巡察香主/督造香主",
            ),
            (
                "shenlong",
                "黑龙使/赤龙使/掌库使/掌药使/白龙使/青龙使/督造使",
            ),
            ("jueqing", "渔隐/书隐/库隐/药隐/知客隐/巡察隐/匠隐"),
            (
                "wudu",
                "蛊师统领/秘典蛊师/库房蛊师/药蛊师/知客蛊师/巡察蛊师/督造蛊师",
            ),
            (
                "qingcheng",
                "剑术教头/书阁长老/司库长老/丹房长老/知客长老/巡察长老/督造长老",
            ),
            (
                "song_court",
                "马军都指挥/翰林承旨/三司使/翰林医官/客省使/御史中丞/将作监",
            ),
        ];

        for (sect_id, titles) in expected {
            assert_eq!(
                sect_buildings(sect_id)
                    .iter()
                    .map(|building| building.elder_title.as_str())
                    .collect::<Vec<_>>()
                    .join("/"),
                titles,
                "unexpected building titles for {sect_id}"
            );
        }
    }

    #[test]
    fn unknown_sect_uses_default_buildings() {
        let default = default_buildings();
        let unknown = sect_buildings("huashan");

        assert_eq!(
            unknown
                .iter()
                .map(|building| (&building.id, &building.name, &building.elder_title))
                .collect::<Vec<_>>(),
            default
                .iter()
                .map(|building| (&building.id, &building.name, &building.elder_title))
                .collect::<Vec<_>>()
        );
    }
}
