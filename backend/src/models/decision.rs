use serde::{Deserialize, Serialize};

/// 决策定义（静态数据，8种）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DecisionDef {
    pub id: String,
    pub title: String,
    pub desc: String,
    pub cost: i32,
    pub cost_type: String, // "silver" | "none"
    pub req_injury_max: Option<i32>,  // 要求伤势 ≤ 此值
    pub req_silver_min: Option<i32>,  // 要求银两 ≥ 此值
    pub req_disciples_min: Option<i32>, // 要求弟子 ≥ 此值
    pub req_unlearned_arts: bool,     // 要求有未学武学
}

/// 返回全部 8 种决策的静态定义
pub fn all_decisions() -> Vec<DecisionDef> {
    vec![
        DecisionDef {
            id: "recruit".into(), title: "张贴招贤榜".into(),
            desc: "遣人在山下城镇张贴招贤榜文，吸引江湖人士前来投奔。花费库银五十两。".into(),
            cost: 50, cost_type: "silver".into(),
            req_injury_max: None, req_silver_min: Some(50),
            req_disciples_min: None, req_unlearned_arts: false,
        },
        DecisionDef {
            id: "train".into(), title: "闭关练功".into(),
            desc: "掌门亲自督导，率众弟子闭关苦练武学。可提升弟子内力与忠诚。".into(),
            cost: 0, cost_type: "none".into(),
            req_injury_max: Some(29), req_silver_min: None,
            req_disciples_min: None, req_unlearned_arts: false,
        },
        DecisionDef {
            id: "mission".into(), title: "遣弟子行侠".into(),
            desc: "派遣弟子下山行侠仗义，或可赚取银两与声望。成败视弟子武艺而定。".into(),
            cost: 0, cost_type: "none".into(),
            req_injury_max: None, req_silver_min: None,
            req_disciples_min: Some(1), req_unlearned_arts: false,
        },
        DecisionDef {
            id: "repair".into(), title: "修缮山门".into(),
            desc: "拨银修缮门派建筑，改善弟子起居环境。花费库银六十两。".into(),
            cost: 60, cost_type: "silver".into(),
            req_injury_max: None, req_silver_min: Some(60),
            req_disciples_min: None, req_unlearned_arts: false,
        },
        DecisionDef {
            id: "diplomacy".into(), title: "拜会邻派".into(),
            desc: "携礼拜访周边门派，修睦关系增声望。花费库银四十两。".into(),
            cost: 40, cost_type: "silver".into(),
            req_injury_max: None, req_silver_min: Some(40),
            req_disciples_min: None, req_unlearned_arts: false,
        },
        DecisionDef {
            id: "rest".into(), title: "静养疗伤".into(),
            desc: "掌门暂歇俗务，闭关静养以疗伤势。可显著恢复掌门伤势。".into(),
            cost: 0, cost_type: "none".into(),
            req_injury_max: None, req_silver_min: None,
            req_disciples_min: None, req_unlearned_arts: false,
        },
        DecisionDef {
            id: "study".into(), title: "研习武功".into(),
            desc: "掌门闭关钻研，试图领悟新的武学。花费库银三十两购置药材辅助。".into(),
            cost: 30, cost_type: "silver".into(),
            req_injury_max: None, req_silver_min: Some(30),
            req_disciples_min: None, req_unlearned_arts: true,
        },
        DecisionDef {
            id: "teach".into(), title: "传功授艺".into(),
            desc: "掌门亲自为弟子传授武艺，可令数名弟子武学精进。花费库银二十两备办药材。".into(),
            cost: 20, cost_type: "silver".into(),
            req_injury_max: None, req_silver_min: Some(20),
            req_disciples_min: Some(1), req_unlearned_arts: false,
        },
    ]
}
