use serde::{Deserialize, Serialize};

/// 武学静态定义（5门）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MartialArt {
    pub id: String,
    pub name: String,
    #[serde(rename = "type")]
    pub art_type: String,
    pub desc: String,
    pub atk: i32,
    pub def: i32,
    pub spd: i32,
    #[serde(rename = "req_talent")]
    pub req_talent: i32,
}

/// 返回全部 5 门武学静态数据
pub fn all_martial_arts() -> Vec<MartialArt> {
    vec![
        MartialArt {
            id: "taixu".into(),
            name: "太虚剑法".into(),
            art_type: "剑法".into(),
            desc: "攻守兼备，虚实相生，乃本派镇山之绝学。".into(),
            atk: 3, def: 3, spd: 2, req_talent: 30,
        },
        MartialArt {
            id: "jiuyang".into(),
            name: "九阳烈掌".into(),
            art_type: "掌法".into(),
            desc: "至刚至猛，掌风所至，金石俱裂。".into(),
            atk: 5, def: 1, spd: 2, req_talent: 40,
        },
        MartialArt {
            id: "xuanbing".into(),
            name: "玄冰心经".into(),
            art_type: "内功".into(),
            desc: "以柔克刚，心如寒冰。内力深厚者修之，可御万般攻势。".into(),
            atk: 1, def: 5, spd: 2, req_talent: 35,
        },
        MartialArt {
            id: "zhuifeng".into(),
            name: "追风步".into(),
            art_type: "轻功".into(),
            desc: "踏雪无痕，追风逐电。临敌之际，先机在握。".into(),
            atk: 2, def: 2, spd: 5, req_talent: 25,
        },
        MartialArt {
            id: "hunyuan".into(),
            name: "混元功".into(),
            art_type: "内功".into(),
            desc: "浑厚绵长，根基扎实。修至大成，内力如长江大河，绵绵不绝。".into(),
            atk: 3, def: 3, spd: 2, req_talent: 20,
        },
    ]
}
