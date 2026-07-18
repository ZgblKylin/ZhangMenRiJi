use serde::{Deserialize, Serialize};

/// 分散在七座建筑中的掌门议事。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DecisionDef {
    pub id: String,
    pub title: String,
    pub desc: String,
    pub cost: i32,
    pub cost_type: String,              // "silver" | "none"
    pub req_injury_max: Option<i32>,    // 要求伤势 ≤ 此值
    pub req_silver_min: Option<i32>,    // 要求银两 ≥ 此值
    pub req_disciples_min: Option<i32>, // 要求弟子 ≥ 此值
    pub req_unlearned_arts: bool,       // 要求有未学武学
}

/// 返回全部建筑议事定义。
pub fn all_decisions() -> Vec<DecisionDef> {
    vec![
        DecisionDef {
            id: "recruit".into(),
            title: "张贴招贤榜".into(),
            desc: "遣人在山下城镇张贴招贤榜文，吸引江湖人士前来投奔。花费库银五十两。".into(),
            cost: 50,
            cost_type: "silver".into(),
            req_injury_max: None,
            req_silver_min: Some(50),
            req_disciples_min: None,
            req_unlearned_arts: false,
        },
        DecisionDef {
            id: "train".into(),
            title: "演武督课".into(),
            desc: "掌门在演武场督导当月无差事的健康门人，依各自真实所学增长武学经验；进境受个人与门派上限约束。".into(),
            cost: 0,
            cost_type: "none".into(),
            req_injury_max: Some(29),
            req_silver_min: None,
            req_disciples_min: Some(1),
            req_unlearned_arts: false,
        },
        DecisionDef {
            id: "mission".into(),
            title: "承接乡里委托".into(),
            desc: "由执事堂以门派名义承接乡里委托，须有至少一名健康、在门且空闲的门人当值；只结算门派银两与声望，不重复弟子外派收益。".into(),
            cost: 0,
            cost_type: "none".into(),
            req_injury_max: None,
            req_silver_min: None,
            req_disciples_min: Some(1),
            req_unlearned_arts: false,
        },
        DecisionDef {
            id: "rest".into(),
            title: "静养疗伤".into(),
            desc: "掌门暂歇俗务，闭关静养以疗伤势。可显著恢复掌门伤势。".into(),
            cost: 0,
            cost_type: "none".into(),
            req_injury_max: None,
            req_silver_min: None,
            req_disciples_min: None,
            req_unlearned_arts: false,
        },
        DecisionDef {
            id: "study".into(),
            title: "闭阁推演新谱".into(),
            desc: "掌门在完好的藏经阁中推演本门新谱；若未定稿，则转为提高已有公册的门派可授上限。花费库银三十两。".into(),
            cost: 30,
            cost_type: "silver".into(),
            req_injury_max: None,
            req_silver_min: Some(30),
            req_disciples_min: None,
            req_unlearned_arts: false,
        },
        DecisionDef {
            id: "research".into(),
            title: "集众研创新武学".into(),
            desc: "召集门中高手群策群力，共同开创一门本派新武学。需拨库银一百二十两。".into(),
            cost: 120,
            cost_type: "silver".into(),
            req_injury_max: None,
            req_silver_min: Some(120),
            req_disciples_min: None,
            req_unlearned_arts: true,
        },
        DecisionDef {
            id: "teach".into(),
            title: "开坛传武".into(),
            desc: "由在门熟手向自己的弟子或关系亲近的同门传授确已掌握的武学；根基、身份、知识、造诣与门派参研上限均须合规。".into(),
            cost: 20,
            cost_type: "silver".into(),
            req_injury_max: None,
            req_silver_min: Some(20),
            req_disciples_min: Some(2),
            req_unlearned_arts: false,
        },
    ]
}
