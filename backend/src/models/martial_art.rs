use serde::{Deserialize, Serialize};
use std::sync::OnceLock;

/// 技能在人物面板中的六个固定门类。知识是基础技能，但不参与战斗。
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub enum SkillCategory {
    Unarmed,
    Parry,
    Dodge,
    Force,
    Weapon,
    Knowledge,
}

impl SkillCategory {
    /// 正常传授的战斗门类。招架保留为基础技能及旧存档分类，不再生成门派通用招架。
    pub const COMBAT: [Self; 4] = [
        Self::Unarmed,
        Self::Dodge,
        Self::Force,
        Self::Weapon,
    ];

    pub const fn slug(self) -> &'static str {
        match self {
            Self::Unarmed => "unarmed",
            Self::Parry => "parry",
            Self::Dodge => "dodge",
            Self::Force => "force",
            Self::Weapon => "weapon",
            Self::Knowledge => "knowledge",
        }
    }

    fn display(self, weapon_type: &str) -> String {
        match self {
            Self::Unarmed => "拳脚".into(),
            Self::Parry => "招架".into(),
            Self::Dodge => "轻功".into(),
            Self::Force => "内功".into(),
            Self::Weapon => weapon_type.into(),
            Self::Knowledge => "知识".into(),
        }
    }
}

/// 门派武学的传授层次；长老沿用内门层。
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub enum MartialTier {
    Basic,
    Chore,
    Outer,
    Inner,
}

impl MartialTier {
    pub const fn slug(self) -> &'static str {
        match self {
            Self::Basic => "basic",
            Self::Chore => "chore",
            Self::Outer => "outer",
            Self::Inner => "inner",
        }
    }
}

/// 一门基础技能、知识或门派战斗武学的静态定义。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MartialArt {
    pub id: String,
    pub name: String,
    #[serde(rename = "type")]
    pub art_type: String,
    pub category: SkillCategory,
    pub tier: MartialTier,
    pub is_combat: bool,
    pub desc: String,
    pub atk: i32,
    pub def: i32,
    pub spd: i32,
    #[serde(rename = "req_talent")]
    pub req_talent: i32,
    pub sect_id: Option<String>,
    /// 对应的基础技能 id。知识技能没有此项。
    pub basic_skill: String,
    pub difficulty: i32,
    /// 除拳脚、兵器外，是否也可准备为招架式。
    #[serde(default)]
    pub usable_for_parry: bool,
}

#[derive(Clone, Copy)]
struct SectMartialTemplate {
    id: &'static str,
    knowledge: &'static str,
    weapon_basic: &'static str,
    weapon_type: &'static str,
    signature_id: &'static str,
    signature_category: SkillCategory,
    /// 每层依次为拳脚、轻功、内功、旧制招架、兵器；旧制招架仅用于存档兼容。
    tiers: [[&'static str; 5]; 3],
}

const SECT_MARTIALS: &[SectMartialTemplate] = &[
    SectMartialTemplate {
        id: "wudang",
        knowledge: "道家心法",
        weapon_basic: "basic_sword",
        weapon_type: "剑法",
        signature_id: "taiji",
        signature_category: SkillCategory::Force,
        tiers: [
            [
                "武当长拳",
                "武当身法",
                "太极心法（初）",
                "武当招架（初）",
                "武当剑法（初）",
            ],
            [
                "太极拳",
                "梯云纵（初）",
                "太极心法（中）",
                "武当招架（中）",
                "武当剑法（中）",
            ],
            ["太极拳经", "梯云纵", "太极神功", "太极招架", "太极剑法"],
        ],
    },
    SectMartialTemplate {
        id: "huashan",
        knowledge: "儒门心法",
        weapon_basic: "basic_sword",
        weapon_type: "剑法",
        signature_id: "dugu",
        signature_category: SkillCategory::Weapon,
        tiers: [
            [
                "华山长拳",
                "华山身法",
                "华山心法（初）",
                "华山招架（初）",
                "华山剑法（初）",
            ],
            [
                "劈石破玉拳",
                "华山轻功",
                "紫霞功（初）",
                "华山招架（中）",
                "华山剑法",
            ],
            ["混元掌", "神行百变", "紫霞神功", "独孤招架", "独孤九剑"],
        ],
    },
    SectMartialTemplate {
        id: "mingjiao",
        knowledge: "圣火心法",
        weapon_basic: "basic_blade",
        weapon_type: "刀法",
        signature_id: "qiankun",
        signature_category: SkillCategory::Force,
        tiers: [
            [
                "明教长拳",
                "光明身法",
                "圣火功（初）",
                "明教招架（初）",
                "烈焰刀法",
            ],
            [
                "鹰爪擒拿手",
                "青蝠身法",
                "圣火功（中）",
                "明教招架（中）",
                "风雷刀法",
            ],
            [
                "七伤拳",
                "乾坤挪移身法",
                "乾坤大挪移",
                "乾坤招架",
                "圣火令法",
            ],
        ],
    },
    SectMartialTemplate {
        id: "quanzhen",
        knowledge: "玄门心法",
        weapon_basic: "basic_sword",
        weapon_type: "剑法",
        signature_id: "xiantian",
        signature_category: SkillCategory::Force,
        tiers: [
            [
                "全真长拳",
                "金雁功（初）",
                "全真心法",
                "全真招架（初）",
                "全真剑法（初）",
            ],
            [
                "三花聚顶掌",
                "金雁功",
                "先天功（初）",
                "全真招架（中）",
                "全真剑法",
            ],
            ["空明拳", "天罡北斗步", "先天功", "北斗招架", "一炁化三清剑"],
        ],
    },
    SectMartialTemplate {
        id: "tianlong",
        knowledge: "禅宗心法",
        weapon_basic: "basic_sword",
        weapon_type: "剑法",
        signature_id: "liumai",
        signature_category: SkillCategory::Unarmed,
        tiers: [
            [
                "天龙指法",
                "天龙身法",
                "枯禅心法",
                "天龙招架（初）",
                "慈悲剑法",
            ],
            [
                "一阳指（初）",
                "段氏身法",
                "枯荣禅功",
                "天龙招架（中）",
                "哀牢山剑法",
            ],
            ["六脉神剑", "凌波微步", "枯荣神功", "枯荣招架", "六脉剑阵"],
        ],
    },
    SectMartialTemplate {
        id: "taohua",
        knowledge: "奇门五行",
        weapon_basic: "basic_sword",
        weapon_type: "剑法",
        signature_id: "luoying",
        signature_category: SkillCategory::Unarmed,
        tiers: [
            [
                "碧波掌法",
                "桃花身法",
                "碧波心法",
                "桃花招架（初）",
                "落英剑法（初）",
            ],
            [
                "兰花拂穴手",
                "灵鳌步",
                "碧海潮生功",
                "桃花招架（中）",
                "玉箫剑法",
            ],
            ["落英神剑掌", "旋风扫叶腿", "奇门玄功", "弹指招架", "玉箫剑"],
        ],
    },
    SectMartialTemplate {
        id: "shaolin",
        knowledge: "禅宗心法",
        weapon_basic: "basic_staff",
        weapon_type: "棍法",
        signature_id: "yijin",
        signature_category: SkillCategory::Force,
        tiers: [
            ["罗汉拳", "少林身法", "少林心法", "少林招架（初）", "韦陀棍"],
            [
                "大金刚拳",
                "一苇渡江",
                "混元一气功",
                "少林招架（中）",
                "伏魔棍法",
            ],
            ["拈花指", "八步赶蝉", "易筋经", "金刚不坏体", "达摩剑法"],
        ],
    },
    SectMartialTemplate {
        id: "gumu",
        knowledge: "古墓心法",
        weapon_basic: "basic_sword",
        weapon_type: "剑法",
        signature_id: "yunu",
        signature_category: SkillCategory::Force,
        tiers: [
            [
                "美女拳法",
                "古墓身法",
                "古墓心法（初）",
                "古墓招架（初）",
                "玉女剑法（初）",
            ],
            [
                "天罗地网掌",
                "捕雀功",
                "玉女心法",
                "古墓招架（中）",
                "玉女剑法",
            ],
            [
                "黯然销魂掌",
                "玉女身法",
                "玉女心经",
                "玉女招架",
                "玉女素心剑",
            ],
        ],
    },
    SectMartialTemplate {
        id: "gaibang",
        knowledge: "江湖阅历",
        weapon_basic: "basic_staff",
        weapon_type: "棍法",
        signature_id: "dagou",
        signature_category: SkillCategory::Weapon,
        tiers: [
            [
                "丐帮长拳",
                "逍遥游（初）",
                "混天气功（初）",
                "丐帮招架（初）",
                "叫花棒法",
            ],
            ["莲花掌", "逍遥游", "混天气功", "丐帮招架（中）", "疯魔杖法"],
            ["降龙十八掌", "四方步", "擒龙功", "降龙招架", "打狗棒法"],
        ],
    },
    SectMartialTemplate {
        id: "emei",
        knowledge: "佛学心法",
        weapon_basic: "basic_sword",
        weapon_type: "剑法",
        signature_id: "jiuyin",
        signature_category: SkillCategory::Force,
        tiers: [
            [
                "金顶绵掌",
                "峨嵋身法",
                "峨嵋心法",
                "峨嵋招架（初）",
                "峨嵋剑法（初）",
            ],
            [
                "飘雪穿云掌",
                "诸天化身步",
                "临济十二庄",
                "峨嵋招架（中）",
                "回风拂柳剑",
            ],
            ["截手九式", "金顶云踪", "九阴真经", "佛光招架", "灭绝剑法"],
        ],
    },
    SectMartialTemplate {
        id: "riyue",
        knowledge: "日月教义",
        weapon_basic: "basic_sword",
        weapon_type: "剑法",
        signature_id: "kuihua",
        signature_category: SkillCategory::Force,
        tiers: [
            [
                "日月掌法",
                "日月身法",
                "日月心法",
                "日月招架（初）",
                "日月剑法",
            ],
            [
                "吸星掌",
                "鬼魅身法",
                "吸星大法",
                "日月招架（中）",
                "辟邪剑法（初）",
            ],
            ["葵花指", "葵花身法", "葵花宝典", "葵花招架", "辟邪剑法"],
        ],
    },
    SectMartialTemplate {
        id: "xingxiu",
        knowledge: "毒技心法",
        weapon_basic: "basic_staff",
        weapon_type: "杖法",
        signature_id: "huagong",
        signature_category: SkillCategory::Force,
        tiers: [
            [
                "星宿掌",
                "星宿身法",
                "星宿心法",
                "星宿招架（初）",
                "星宿杖法",
            ],
            [
                "抽髓掌",
                "摘星身法",
                "化功心法",
                "星宿招架（中）",
                "天山杖法",
            ],
            ["三阴蜈蚣爪", "飞星术", "化功大法", "毒功招架", "天山杖"],
        ],
    },
    SectMartialTemplate {
        id: "murong",
        knowledge: "经世心法",
        weapon_basic: "basic_sword",
        weapon_type: "剑法",
        signature_id: "douzhuan",
        signature_category: SkillCategory::Dodge,
        tiers: [
            [
                "慕容长拳",
                "燕子身法",
                "慕容心法",
                "慕容招架（初）",
                "慕容剑法",
            ],
            ["参合指", "燕灵身法", "参合功", "以彼之道", "龙城剑法"],
            ["参合指法", "斗转星移", "斗转心法", "斗转招架", "慕容家传剑"],
        ],
    },
    SectMartialTemplate {
        id: "dalun",
        knowledge: "密宗心法",
        weapon_basic: "basic_staff",
        weapon_type: "杖法",
        signature_id: "longxiang",
        signature_category: SkillCategory::Force,
        tiers: [
            [
                "大轮掌",
                "雪山身法",
                "密宗心法",
                "大轮招架（初）",
                "金刚杵法",
            ],
            [
                "大手印",
                "移形换位",
                "龙象功（初）",
                "大轮招架（中）",
                "降魔杵法",
            ],
            ["火焰刀", "无上大挪移", "龙象般若功", "龙象招架", "五轮大转"],
        ],
    },
    SectMartialTemplate {
        id: "court",
        knowledge: "兵法韬略",
        weapon_basic: "basic_blade",
        weapon_type: "刀法",
        signature_id: "xuantian",
        signature_category: SkillCategory::Force,
        tiers: [
            ["御前拳法", "御前身法", "宫门心法", "御前招架", "御林刀法"],
            [
                "大擒拿手",
                "八方步",
                "玄天功（初）",
                "大内招架",
                "血滴子刀法",
            ],
            ["凝血神爪", "神行百变", "玄天无极功", "玄天招架", "金蛇剑法"],
        ],
    },
    SectMartialTemplate {
        id: "lingjiu",
        knowledge: "逍遥心法",
        weapon_basic: "basic_sword",
        weapon_type: "剑法",
        signature_id: "bahuang",
        signature_category: SkillCategory::Force,
        tiers: [
            [
                "灵鹫掌法",
                "灵鹫身法",
                "灵鹫心法",
                "灵鹫招架（初）",
                "灵鹫剑法",
            ],
            [
                "天山折梅手",
                "月影舞步",
                "小无相功",
                "灵鹫招架（中）",
                "天羽奇剑",
            ],
            [
                "天山六阳掌",
                "凌波微步",
                "八荒六合唯我独尊功",
                "逍遥招架",
                "逍遥剑法",
            ],
        ],
    },
    SectMartialTemplate {
        id: "baituo",
        knowledge: "西域毒经",
        weapon_basic: "basic_staff",
        weapon_type: "杖法",
        signature_id: "hamagong",
        signature_category: SkillCategory::Unarmed,
        tiers: [
            [
                "白驼掌",
                "白驼身法",
                "白驼心法",
                "白驼招架（初）",
                "灵蛇杖法（初）",
            ],
            [
                "灵蛇拳",
                "瞬息千里",
                "逆行经脉",
                "白驼招架（中）",
                "灵蛇杖法",
            ],
            ["蛤蟆功", "蛇行狸翻", "九阴逆运", "蛤蟆招架", "神驼雪山掌杖"],
        ],
    },
    SectMartialTemplate {
        id: "xueshan",
        knowledge: "雪山心法",
        weapon_basic: "basic_sword",
        weapon_type: "剑法",
        signature_id: "xueshan",
        signature_category: SkillCategory::Weapon,
        tiers: [
            [
                "雪山掌法",
                "雪山身法",
                "雪山心法（初）",
                "雪山招架（初）",
                "入门十三剑",
            ],
            [
                "雪影擒拿手",
                "踏雪无痕",
                "雪山心法",
                "雪山招架（中）",
                "雪山剑法（初）",
            ],
            ["金乌刀掌", "凌霄飞渡", "雪山神功", "凌霄招架", "雪山剑法"],
        ],
    },
    SectMartialTemplate {
        id: "tiandihui",
        knowledge: "反清义理",
        weapon_basic: "basic_blade",
        weapon_type: "刀法",
        signature_id: "ningxue",
        signature_category: SkillCategory::Unarmed,
        tiers: [
            ["洪门拳", "洪门身法", "洪门心法", "洪门招架", "地堂刀法"],
            [
                "凝血爪（初）",
                "百胜步",
                "凝血心法",
                "天地会招架",
                "百胜刀法",
            ],
            ["凝血神爪", "神行百变", "凝血神功", "英雄招架", "英雄刀法"],
        ],
    },
    SectMartialTemplate {
        id: "shenlong",
        knowledge: "神龙教义",
        weapon_basic: "basic_staff",
        weapon_type: "杖法",
        signature_id: "shenlong",
        signature_category: SkillCategory::Unarmed,
        tiers: [
            [
                "神龙掌",
                "蛇岛身法",
                "神龙心法",
                "神龙招架（初）",
                "腾蛇杖法",
            ],
            [
                "化骨绵掌",
                "游蛇步",
                "神龙心法（中）",
                "神龙招架（中）",
                "五蛇杖法",
            ],
            ["神龙八式", "神龙无影步", "神龙神功", "神龙招架", "神龙杖法"],
        ],
    },
    SectMartialTemplate {
        id: "jueqing",
        knowledge: "绝情心法",
        weapon_basic: "basic_blade",
        weapon_type: "刀法",
        signature_id: "yinyang",
        signature_category: SkillCategory::Weapon,
        tiers: [
            [
                "绝情掌",
                "绝情身法",
                "绝情心法（初）",
                "绝情招架（初）",
                "绝情刀法",
            ],
            [
                "闭穴功",
                "铁掌身法",
                "阴阳心法",
                "绝情招架（中）",
                "阴阳双刃",
            ],
            ["铁掌功", "水上漂", "阴阳神功", "阴阳招架", "阴阳倒乱刃法"],
        ],
    },
    SectMartialTemplate {
        id: "wudu",
        knowledge: "五毒秘传",
        weapon_basic: "basic_whip",
        weapon_type: "鞭法",
        signature_id: "wudu",
        signature_category: SkillCategory::Unarmed,
        tiers: [
            [
                "五毒掌",
                "五毒身法",
                "五毒心法",
                "五毒招架（初）",
                "软索鞭法",
            ],
            [
                "千蛛万毒手",
                "金蛇游身",
                "五毒真气",
                "五毒招架（中）",
                "金蛇鞭法",
            ],
            ["五毒神掌", "幻雾身法", "五毒神功", "毒砂招架", "金蛇锥法"],
        ],
    },
    SectMartialTemplate {
        id: "qingcheng",
        knowledge: "道家心法",
        weapon_basic: "basic_sword",
        weapon_type: "剑法",
        signature_id: "songfeng",
        signature_category: SkillCategory::Weapon,
        tiers: [
            [
                "青城长拳",
                "青城身法",
                "青城心法",
                "青城招架（初）",
                "松风剑法（初）",
            ],
            [
                "摧心掌",
                "无影幻脚",
                "鹤唳心法",
                "青城招架（中）",
                "松风剑法",
            ],
            [
                "青城摧心掌",
                "蜀道难",
                "鹤唳九霄神功",
                "松风招架",
                "青城绝命剑",
            ],
        ],
    },
];

fn martial_art(
    id: impl Into<String>,
    name: impl Into<String>,
    category: SkillCategory,
    tier: MartialTier,
    sect_id: Option<&str>,
    basic_skill: impl Into<String>,
    weapon_type: &str,
) -> MartialArt {
    let id = id.into();
    let power = match tier {
        MartialTier::Basic => 1,
        MartialTier::Chore => 2,
        MartialTier::Outer => 4,
        MartialTier::Inner => 6,
    };
    let (atk, def, spd) = match category {
        SkillCategory::Unarmed => (power + 2, power, power),
        SkillCategory::Parry => (power, power + 2, power),
        SkillCategory::Dodge => (power, power, power + 2),
        SkillCategory::Force => (power + 1, power + 2, power),
        SkillCategory::Weapon => (power + 2, power + 1, power + 1),
        SkillCategory::Knowledge => (0, 0, 0),
    };
    let is_combat = category != SkillCategory::Knowledge;
    MartialArt {
        usable_for_parry: matches!(id.as_str(), "douzhuan" | "qiankun" | "riyue_outer_force") || category == SkillCategory::Parry,
        id,
        name: name.into(),
        art_type: category.display(weapon_type),
        category,
        tier,
        is_combat,
        desc: if is_combat {
            match tier {
                MartialTier::Basic => "江湖通行的基础功架，是修习各门绝艺的根基。",
                MartialTier::Chore => "本门杂役弟子所习的入门功夫。",
                MartialTier::Outer => "本门外门弟子所习的进阶功夫。",
                MartialTier::Inner => "本门内门真传，非根基深厚者不能得授。",
            }
        } else {
            "本门义理与修心之学；约束门派武学上限，并增益悟性与精力。"
        }
        .into(),
        atk,
        def,
        spd,
        req_talent: match tier {
            MartialTier::Basic => 0,
            MartialTier::Chore => 12,
            MartialTier::Outer => 20,
            MartialTier::Inner => 30,
        },
        sect_id: sect_id.map(str::to_owned),
        basic_skill: basic_skill.into(),
        difficulty: match (tier, category) {
            (_, SkillCategory::Knowledge) => 18,
            (MartialTier::Basic, _) => 8,
            (MartialTier::Chore, _) => 14,
            (MartialTier::Outer, _) => 26,
            (MartialTier::Inner, _) => 42,
        },
    }
}

fn basic_arts() -> Vec<MartialArt> {
    [
        ("basic_unarmed", "基本拳脚", SkillCategory::Unarmed, "拳脚"),
        ("basic_parry", "基本招架", SkillCategory::Parry, "招架"),
        ("basic_dodge", "基本轻功", SkillCategory::Dodge, "轻功"),
        ("basic_force", "基本内功", SkillCategory::Force, "内功"),
        ("basic_sword", "基本剑法", SkillCategory::Weapon, "剑法"),
        ("basic_blade", "基本刀法", SkillCategory::Weapon, "刀法"),
        ("basic_staff", "基本棍杖", SkillCategory::Weapon, "棍法"),
        ("basic_whip", "基本鞭法", SkillCategory::Weapon, "鞭法"),
    ]
    .into_iter()
    .map(|(id, name, category, weapon_type)| {
        martial_art(
            id,
            name,
            category,
            MartialTier::Basic,
            None,
            id,
            weapon_type,
        )
    })
    .collect()
}

fn player_arts() -> Vec<MartialArt> {
    let mut arts = vec![martial_art(
        "player_knowledge",
        "本门心法",
        SkillCategory::Knowledge,
        MartialTier::Basic,
        Some("player"),
        "",
        "知识",
    )];
    let definitions = [
        ("taixu", "太虚剑法", SkillCategory::Weapon, "basic_sword"),
        (
            "jiuyang",
            "九阳烈掌",
            SkillCategory::Unarmed,
            "basic_unarmed",
        ),
        ("xuanbing", "玄冰心经", SkillCategory::Force, "basic_force"),
        ("zhuifeng", "追风步", SkillCategory::Dodge, "basic_dodge"),
        ("hunyuan", "混元功", SkillCategory::Force, "basic_force"),
    ];
    arts.extend(definitions.into_iter().map(|(id, name, category, basic)| {
        martial_art(
            id,
            name,
            category,
            MartialTier::Inner,
            Some("player"),
            basic,
            "剑法",
        )
    }));
    arts
}

fn sect_arts(template: &SectMartialTemplate) -> Vec<MartialArt> {
    let mut arts = vec![martial_art(
        knowledge_skill_id(template.id),
        template.knowledge,
        SkillCategory::Knowledge,
        MartialTier::Basic,
        Some(template.id),
        "",
        "知识",
    )];
    for (tier_index, tier) in [MartialTier::Chore, MartialTier::Outer, MartialTier::Inner]
        .into_iter()
        .enumerate()
    {
        for (category_index, category) in [
            (0, SkillCategory::Unarmed),
            (1, SkillCategory::Dodge),
            (2, SkillCategory::Force),
            (4, SkillCategory::Weapon),
        ] {
            let id = if tier == MartialTier::Chore && category == SkillCategory::Force {
                format!("{}_foundation", template.id)
            } else if tier == MartialTier::Inner && category == template.signature_category {
                template.signature_id.into()
            } else {
                format!("{}_{}_{}", template.id, tier.slug(), category.slug())
            };
            let basic = match category {
                SkillCategory::Unarmed => "basic_unarmed",
                SkillCategory::Dodge => "basic_dodge",
                SkillCategory::Force => "basic_force",
                SkillCategory::Weapon => template.weapon_basic,
                SkillCategory::Parry | SkillCategory::Knowledge => unreachable!(),
            };
            arts.push(martial_art(
                id,
                template.tiers[tier_index][category_index],
                category,
                tier,
                Some(template.id),
                basic,
                template.weapon_type,
            ));
        }
    }
    arts
}

/// 旧存档中的门派招架仍需静态名称、分类和数值，故保留在兼容目录；
/// 它们不会出现在门派课程、初始藏书或正常残卷获取池中。
fn legacy_parry_arts(template: &SectMartialTemplate) -> Vec<MartialArt> {
    [MartialTier::Chore, MartialTier::Outer, MartialTier::Inner]
        .into_iter()
        .enumerate()
        .map(|(tier_index, tier)| {
            martial_art(
                format!("{}_{}_parry", template.id, tier.slug()),
                template.tiers[tier_index][3],
                SkillCategory::Parry,
                tier,
                Some(template.id),
                "basic_parry",
                template.weapon_type,
            )
        })
        .collect()
}

fn build_martial_arts() -> Vec<MartialArt> {
    let mut arts = basic_arts();
    arts.extend(player_arts());
    arts.extend(SECT_MARTIALS.iter().flat_map(sect_arts));
    arts.extend(SECT_MARTIALS.iter().flat_map(legacy_parry_arts));
    arts
}

fn martial_registry() -> &'static [MartialArt] {
    static REGISTRY: OnceLock<Vec<MartialArt>> = OnceLock::new();
    REGISTRY.get_or_init(build_martial_arts)
}

/// 返回基础技能、玩家武学及 23 个 NPC 门派的三层完整武学表。
pub fn all_martial_arts() -> Vec<MartialArt> {
    martial_registry().to_vec()
}

pub fn martial_art_by_id(id: &str) -> Option<MartialArt> {
    let canonical = canonical_skill_id(id);
    martial_registry()
        .iter()
        .find(|art| art.id == canonical)
        .cloned()
}

pub fn knowledge_skill_id(sect_id: &str) -> String {
    if sect_id == "player" {
        "player_knowledge".into()
    } else {
        format!("{sect_id}_knowledge")
    }
}

pub fn knowledge_skill_for_art(art_id: &str) -> Option<String> {
    let art = martial_art_by_id(art_id)?;
    (art.is_combat && art.tier != MartialTier::Basic)
        .then(|| art.sect_id.as_deref().map(knowledge_skill_id))
        .flatten()
}

pub fn weapon_basic_for_sect(sect_id: &str) -> &'static str {
    SECT_MARTIALS
        .iter()
        .find(|template| template.id == sect_id)
        .map(|template| template.weapon_basic)
        .unwrap_or("basic_sword")
}

pub fn sect_ids() -> impl Iterator<Item = &'static str> {
    SECT_MARTIALS.iter().map(|template| template.id)
}

pub fn sect_combat_arts(sect_id: &str, tier: MartialTier) -> Vec<MartialArt> {
    martial_registry()
        .iter()
        .filter(|art| {
            art.sect_id.as_deref() == Some(sect_id)
                && art.is_combat
                && art.category != SkillCategory::Parry
                && art.tier == tier
        })
        .cloned()
        .collect()
}

pub fn base_skill_ids(sect_id: &str) -> [String; 6] {
    [
        "basic_unarmed".into(),
        "basic_parry".into(),
        "basic_dodge".into(),
        "basic_force".into(),
        weapon_basic_for_sect(sect_id).into(),
        knowledge_skill_id(sect_id),
    ]
}

/// v2/v3 早期存档曾以中文技能名作为 id，载入时统一到稳定英文 id。
pub fn canonical_skill_id(id: &str) -> String {
    match id {
        "基本拳脚" | "基本指法" => "basic_unarmed",
        "基本招架" => "basic_parry",
        "基本轻功" => "basic_dodge",
        "基本内功" => "basic_force",
        "基本剑法" => "basic_sword",
        "基本刀法" => "basic_blade",
        "基本棍法" | "基本杖法" => "basic_staff",
        "基本鞭法" => "basic_whip",
        _ => return id.into(),
    }
    .into()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeSet;

    #[test]
    fn every_npc_sect_has_six_basics_and_three_complete_combat_tiers() {
        let arts = all_martial_arts();
        for sect_id in sect_ids() {
            assert!(base_skill_ids(sect_id)
                .iter()
                .all(|id| arts.iter().any(|art| &art.id == id)));
            for tier in [MartialTier::Chore, MartialTier::Outer, MartialTier::Inner] {
                let tier_arts = sect_combat_arts(sect_id, tier);
                assert_eq!(tier_arts.len(), 5, "{sect_id} {tier:?}");
                assert_eq!(
                    tier_arts
                        .iter()
                        .map(|art| art.category)
                        .collect::<BTreeSet<_>>(),
                    SkillCategory::COMBAT.into_iter().collect()
                );
            }
        }
    }

    #[test]
    fn ids_are_unique_and_knowledge_never_fights() {
        let arts = all_martial_arts();
        assert_eq!(
            arts.len(),
            arts.iter()
                .map(|art| &art.id)
                .collect::<BTreeSet<_>>()
                .len()
        );
        assert!(arts
            .iter()
            .filter(|art| art.category == SkillCategory::Knowledge)
            .all(|art| !art.is_combat && art.atk == 0 && art.def == 0 && art.spd == 0));
    }
}
