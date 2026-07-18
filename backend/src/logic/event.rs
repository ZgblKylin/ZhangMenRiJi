use crate::logic::disciple as disc;
use crate::models::sect::{MoralDirection, SectPolicy, SectState};
use rand::Rng;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct EventEffect {
    pub prestige: Option<i32>,
    pub silver: Option<i32>,
    pub morale: Option<i32>,
    pub injury: Option<i32>,
    pub free_recruit: Option<i32>,
    pub special: Option<String>, // "manual" | "epiphany"
    pub loyalty_loss: Option<bool>,
    pub loyalty_change: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RandomEvent {
    pub id: String,
    pub text: String,
    pub effect: EventEffect,
    pub good: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventChoice {
    pub id: String,
    pub label: String,
    pub result_text: String,
    pub effect: EventEffect,
    pub good: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PendingWorldEvent {
    pub id: String,
    pub category: String,
    pub title: String,
    pub text: String,
    pub choices: Vec<EventChoice>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum EventTag {
    /// 扶危济困、以礼服人或因侠名而来的机会。
    Righteous,
    /// 可借机获取银钱、物资或世俗利益。
    Profit,
    /// 恶名、门人失序或江湖仇怨招来的追缉与反噬。
    Backlash,
    Martial,
    Scholarly,
    Mercantile,
    Reclusive,
    Domestic,
}

/// 事件标签只属于静态模板，不写入 pending_event，避免改变旧存档中的待决事件结构。
fn event_tags(id: &str) -> &'static [EventTag] {
    use EventTag::*;
    match id {
        "choice_provocation" => &[Martial, Backlash],
        "choice_rice" => &[Profit, Mercantile, Domestic],
        "choice_deviation" => &[Martial, Reclusive],
        "choice_court" => &[Profit, Martial],
        "choice_refugees" => &[Righteous, Domestic],
        "choice_cave" => &[Martial, Scholarly],
        "choice_herbs" => &[Profit, Mercantile, Reclusive],
        "choice_alliance" => &[Righteous, Martial],
        "choice_apprentice" => &[Martial, Scholarly, Domestic],
        "choice_smuggler" => &[Profit, Backlash, Mercantile],
        "choice_plague" => &[Righteous, Martial, Domestic],

        "jh_01" => &[Righteous],
        "jh_02" => &[Martial, Backlash],
        "jh_03" | "jh_04" => &[Righteous],
        "jh_05" | "jh_10" => &[Righteous, Profit, Martial],
        "jh_06" => &[Martial, Backlash],
        "jh_07" => &[Scholarly, Reclusive],
        "jh_08" => &[Righteous, Martial],
        "jh_09" | "jh_11" => &[Backlash],
        "jh_12" => &[Profit, Mercantile, Reclusive],

        "ms_01" => &[Mercantile, Domestic],
        "ms_02" => &[Martial, Reclusive, Domestic],
        "ms_03" | "ms_05" => &[Domestic],
        "ms_04" => &[Martial, Scholarly, Reclusive, Domestic],
        "ms_06" | "ms_10" => &[Reclusive, Domestic],
        "ms_07" => &[Backlash, Domestic],
        "ms_08" | "ms_11" => &[Domestic],
        "ms_09" => &[Mercantile, Domestic],
        "ms_12" => &[Profit, Domestic],

        "ex_01" | "ex_09" => &[Reclusive, Domestic],
        "ex_02" => &[Scholarly, Domestic],
        "ex_03" => &[Profit, Mercantile, Domestic],
        "ex_04" => &[Martial, Domestic],
        "ex_05" | "ex_10" => &[Righteous, Domestic],
        "ex_06" => &[Backlash, Domestic],
        "ex_07" => &[Righteous, Martial],
        "ex_08" => &[Domestic],
        "ex_11" => &[Mercantile, Domestic],
        "ex_12" => &[Righteous, Reclusive, Domestic],
        "ex_13" => &[Domestic],
        "ex_14" => &[Backlash, Domestic],
        "ex_15" => &[Martial, Domestic],
        "ex_16" => &[Reclusive, Domestic],
        "ex_17" => &[Righteous, Martial],
        "ex_18" => &[Profit, Mercantile],
        "ex_19" => &[Backlash, Martial],
        "ex_20" => &[Righteous, Profit, Domestic],
        "ex_21" => &[Righteous, Domestic],
        "ex_22" => &[Domestic, Reclusive],
        "ex_23" => &[Profit, Mercantile, Scholarly, Domestic],
        "ex_24" => &[Righteous, Martial],
        "season_spring" => &[Domestic, Mercantile],
        "season_summer" => &[Domestic, Reclusive],
        "season_autumn" => &[Martial, Domestic, Mercantile],
        "season_winter" => &[Reclusive, Scholarly, Domestic],
        _ => &[],
    }
}

/// 门风决定正邪机会与后果，经营方针决定题材侧重。所有修正均为温和加减，
/// 最低仍保留十分权重，避免某种门风永久删去一类江湖内容。
fn event_weight(sect: &SectState, event_id: &str) -> u32 {
    use EventTag::*;
    let tags = event_tags(event_id);
    let has = |tag| tags.contains(&tag);
    let mut weight = 100_i32;

    match sect.moral_direction {
        MoralDirection::Righteous => {
            if has(Righteous) {
                weight += 60;
            }
            if has(Backlash) {
                weight -= 30;
            }
        }
        MoralDirection::Neutral => {
            if has(Domestic) {
                weight += 20;
            }
        }
        MoralDirection::Villainous => {
            if has(Righteous) {
                weight -= 35;
            }
            if has(Profit) {
                weight += 45;
            }
            if has(Backlash) {
                weight += 45;
            }
        }
    }

    let policy_match = match sect.policy {
        SectPolicy::Balanced => has(Domestic),
        SectPolicy::Martial => has(Martial),
        SectPolicy::Scholarly => has(Scholarly),
        SectPolicy::Chivalrous => has(Righteous),
        SectPolicy::Mercantile => has(Mercantile) || has(Profit),
        SectPolicy::Reclusive => has(Reclusive),
    };
    if policy_match {
        weight += if sect.policy == SectPolicy::Balanced {
            15
        } else {
            30
        };
    }

    weight.max(10) as u32
}

fn choose_weighted<T>(rng: &mut impl Rng, choices: Vec<(T, u32)>) -> T {
    let total = choices
        .iter()
        .map(|(_, weight)| u64::from(*weight))
        .sum::<u64>();
    assert!(total > 0, "事件池及权重不得为空");
    let mut roll = rng.gen_range(0..total);
    for (choice, weight) in choices {
        if roll < u64::from(weight) {
            return choice;
        }
        roll -= u64::from(weight);
    }
    unreachable!("事件权重抽取应在总权重内命中")
}

fn effect(
    prestige: i32,
    silver: i32,
    morale: i32,
    injury: i32,
    special: Option<&str>,
) -> EventEffect {
    EventEffect {
        prestige: (prestige != 0).then_some(prestige),
        silver: (silver != 0).then_some(silver),
        morale: (morale != 0).then_some(morale),
        injury: (injury != 0).then_some(injury),
        free_recruit: None,
        special: special.map(str::to_owned),
        loyalty_loss: None,
        loyalty_change: None,
    }
}

pub fn interactive_events() -> Vec<PendingWorldEvent> {
    vec![
        PendingWorldEvent {
            id: "choice_provocation".into(),
            category: "江湖".into(),
            title: "山门问剑".into(),
            text: "邻派少侠在山门外连败数名外门弟子，扬言要请掌门赐教。如何处置？".into(),
            choices: vec![
                EventChoice {
                    id: "fight".into(),
                    label: "亲自应战".into(),
                    result_text: "掌门亲自下场，以寥寥数招定胜负。来客抱拳服输，江湖为之侧目。"
                        .into(),
                    effect: effect(6, 0, 4, 8, None),
                    good: true,
                },
                EventChoice {
                    id: "talk".into(),
                    label: "以茶论武".into(),
                    result_text: "掌门不动刀兵，只在茶席间点破对方招式破绽，来客心悦诚服。".into(),
                    effect: effect(3, -20, 2, 0, Some("improve_relation")),
                    good: true,
                },
            ],
        },
        PendingWorldEvent {
            id: "choice_rice".into(),
            category: "生计".into(),
            title: "米珠薪桂".into(),
            text: "山下歉收，米行一夜三次涨价。司库问，是趁早囤粮，还是紧一紧各房用度？".into(),
            choices: vec![
                EventChoice {
                    id: "stock".into(),
                    label: "拨银囤粮".into(),
                    result_text: "数车新粮运入外门仓库，虽费银钱，众弟子心里安定。".into(),
                    effect: effect(0, -80, 4, 0, Some("stock_grain")),
                    good: true,
                },
                EventChoice {
                    id: "ration".into(),
                    label: "减省口粮".into(),
                    result_text: "厨房改作稀粥，银钱省下了，演武场上的抱怨却多了。".into(),
                    effect: effect(0, 20, -5, 0, Some("lose_loyalty")),
                    good: false,
                },
            ],
        },
        PendingWorldEvent {
            id: "choice_deviation".into(),
            category: "门内".into(),
            title: "走火入魔".into(),
            text: "一名弟子强练内功，真气岔入经脉，眼下昏迷不醒。药师称尚有两法可救。".into(),
            choices: vec![
                EventChoice {
                    id: "medicine".into(),
                    label: "重金延医".into(),
                    result_text: "名医以金针导气，终于将人从鬼门关拉了回来。".into(),
                    effect: effect(0, -70, 3, 0, Some("heal_disciple")),
                    good: true,
                },
                EventChoice {
                    id: "transfer".into(),
                    label: "掌门渡气".into(),
                    result_text: "掌门耗费真气替弟子续脉，人救回了，自己却伤了元气。".into(),
                    effect: effect(1, 0, 5, 14, Some("heal_disciple")),
                    good: true,
                },
            ],
        },
        PendingWorldEvent {
            id: "choice_court".into(),
            category: "朝廷".into(),
            title: "征召文书".into(),
            text: "朝廷下文，欲借本派高手护送一批军饷。应下此差，难免卷入庙堂是非。".into(),
            choices: vec![
                EventChoice {
                    id: "accept".into(),
                    label: "领命护送".into(),
                    result_text: "军饷平安抵达，朝廷赏赐丰厚，只是江湖同道颇有微词。".into(),
                    effect: effect(7, 140, -1, 0, Some("court_service")),
                    good: true,
                },
                EventChoice {
                    id: "decline".into(),
                    label: "称病推辞".into(),
                    result_text: "使者冷脸而去。门派保住了清静，也失了几分官面人情。".into(),
                    effect: effect(-3, 0, 2, 0, Some("uphold_morality")),
                    good: false,
                },
            ],
        },
        PendingWorldEvent {
            id: "choice_refugees".into(),
            category: "乡里".into(),
            title: "流民叩山".into(),
            text: "百余流民扶老携幼来到山下，求一处避雨与几日口粮。外门仓储并不宽裕。".into(),
            choices: vec![
                EventChoice {
                    id: "shelter".into(),
                    label: "开仓赈济".into(),
                    result_text: "山门内外架起粥棚，乡民感恩，侠义之名传遍数县。".into(),
                    effect: effect(8, -65, 5, 0, Some("charity")),
                    good: true,
                },
                EventChoice {
                    id: "refuse".into(),
                    label: "给路费劝返".into(),
                    result_text: "司库给了些散碎银钱，流民沉默着转下山去。".into(),
                    effect: effect(-2, -15, -2, 0, None),
                    good: false,
                },
            ],
        },
        PendingWorldEvent {
            id: "choice_cave".into(),
            category: "奇遇".into(),
            title: "后山石室".into(),
            text: "暴雨冲开后山崖壁，露出一扇刻满剑痕的石门。门缝里隐有寒气透出。".into(),
            choices: vec![
                EventChoice {
                    id: "explore".into(),
                    label: "入内探查".into(),
                    result_text: "石室机关重重，众人带伤而返，却从壁刻中拓下一篇失传心法。".into(),
                    effect: effect(4, 0, 3, 6, Some("manual")),
                    good: true,
                },
                EventChoice {
                    id: "seal".into(),
                    label: "封门待后".into(),
                    result_text: "掌门命人封存石门，待门中高手足备再来探查。".into(),
                    effect: effect(0, -10, 1, 0, None),
                    good: true,
                },
            ],
        },
        PendingWorldEvent {
            id: "choice_herbs".into(),
            category: "生计".into(),
            title: "西域药商".into(),
            text: "西域药商携来一批上好雪莲，索价不菲。药师连称此物可遇不可求。".into(),
            choices: vec![
                EventChoice {
                    id: "buy".into(),
                    label: "买下雪莲".into(),
                    result_text: "雪莲收入药库，药香数日不散。".into(),
                    effect: effect(0, -90, 2, 0, Some("stock_herbs")),
                    good: true,
                },
                EventChoice {
                    id: "decline".into(),
                    label: "婉言谢绝".into(),
                    result_text: "药商收起木匣，转投别派去了。".into(),
                    effect: effect(0, 0, -1, 0, None),
                    good: false,
                },
            ],
        },
        PendingWorldEvent {
            id: "choice_alliance".into(),
            category: "江湖".into(),
            title: "会盟请帖".into(),
            text: "数家正道门派邀本派共赴会盟，商议清剿盘踞官道的绿林悍匪。".into(),
            choices: vec![
                EventChoice {
                    id: "join".into(),
                    label: "赴会相助".into(),
                    result_text: "诸派合力荡平匪寨，本派弟子也在群雄面前露了脸。".into(),
                    effect: effect(7, 35, 4, 5, Some("improve_relation")),
                    good: true,
                },
                EventChoice {
                    id: "stay".into(),
                    label: "留守山门".into(),
                    result_text: "本派闭门不出，避开了伤亡，却也错过一场江湖盛会。".into(),
                    effect: effect(-2, 0, 0, 0, None),
                    good: false,
                },
            ],
        },
    ]
}

pub fn trigger_interactive_event(rng: &mut impl Rng, sect: &SectState) -> PendingWorldEvent {
    choose_weighted(
        rng,
        interactive_events()
            .into_iter()
            .map(|event| {
                let weight = event_weight(sect, &event.id);
                (event, weight)
            })
            .collect(),
    )
}

fn expanded_events() -> Vec<RandomEvent> {
    vec![
        simple(
            "ex_01",
            "药圃丰收，药师拣出不少上品草药，门中伤病有了着落。",
            0,
            20,
            3,
            -3,
            true,
        ),
        simple(
            "ex_02",
            "藏经阁返潮，数卷抄本生了霉斑，掌书弟子连夜晾晒修补。",
            0,
            -25,
            -2,
            0,
            false,
        ),
        simple(
            "ex_03",
            "一队镖师借宿山门，临行留下谢银，又将沿途见闻说与外务弟子。",
            2,
            45,
            2,
            0,
            true,
        ),
        simple(
            "ex_04",
            "练武场地砖年久松动，一名弟子踏空扭伤，众人只得暂缓切磋。",
            0,
            -20,
            -2,
            3,
            false,
        ),
        simple(
            "ex_05",
            "乡民送来新酿与腊肉，谢本派去年驱走山贼，山门里热闹了一晚。",
            3,
            15,
            5,
            0,
            true,
        ),
        simple(
            "ex_06",
            "官差误将本派弟子当作逃犯扣押，外务房奔走数日才将人领回。",
            -2,
            -35,
            -2,
            0,
            false,
        ),
        simple(
            "ex_07",
            "两派弟子在渡口相遇，先是斗嘴，后来以武会友，竟结下一段交情。",
            2,
            0,
            3,
            1,
            true,
        ),
        simple(
            "ex_08",
            "夜里一声雷劈倒山门古松，幸未伤人，内勤房忙了整整三日。",
            0,
            -30,
            -1,
            0,
            false,
        ),
        simple(
            "ex_09",
            "掌门腰疾又犯，议事时只得倚着软垫，弟子们说话也轻了三分。",
            0,
            0,
            -1,
            4,
            false,
        ),
        simple(
            "ex_10",
            "一名杂役在溪边捡到碎银，原封不动交给司库，掌门当众嘉许。",
            1,
            18,
            4,
            0,
            true,
        ),
        simple(
            "ex_11",
            "附近州县开庙会，弟子们轮流下山采买，外门仓库添了不少日用之物。",
            1,
            -20,
            4,
            0,
            true,
        ),
        simple(
            "ex_12",
            "江湖小报误传掌门闭关仙逝，山门前竟来了几拨吊客，令人哭笑不得。",
            -1,
            12,
            2,
            0,
            false,
        ),
    ]
}

fn simple(
    id: &str,
    text: &str,
    prestige: i32,
    silver: i32,
    morale: i32,
    injury: i32,
    good: bool,
) -> RandomEvent {
    RandomEvent {
        id: id.into(),
        text: text.into(),
        effect: effect(prestige, silver, morale, injury, None),
        good,
    }
}

/// 江湖风云事件池
fn jianghu_events() -> Vec<RandomEvent> {
    vec![
        RandomEvent {
            id: "jh_01".into(),
            text: "邻派遣使来谒，言语间颇有试探之意。掌门好言相待，使者惭而退。".into(),
            effect: EventEffect {
                prestige: Some(3),
                morale: Some(2),
                silver: None,
                injury: None,
                free_recruit: None,
                special: None,
                loyalty_loss: None,
                loyalty_change: None,
            },
            good: true,
        },
        RandomEvent {
            id: "jh_02".into(),
            text: "邻派率众来犯，声称本派侵占了他们的采药之地。一场恶斗，各有损伤。".into(),
            effect: EventEffect {
                prestige: Some(-2),
                morale: Some(-3),
                silver: Some(-30),
                injury: Some(8),
                free_recruit: None,
                special: None,
                loyalty_loss: None,
                loyalty_change: None,
            },
            good: false,
        },
        RandomEvent {
            id: "jh_03".into(),
            text: "朝廷特使到访，说圣上听闻本派侠名，特赐「侠义之门」匾额，并有丰厚赏赐。".into(),
            effect: EventEffect {
                prestige: Some(8),
                silver: Some(100),
                morale: Some(5),
                injury: None,
                free_recruit: None,
                special: None,
                loyalty_loss: None,
                loyalty_change: None,
            },
            good: true,
        },
        RandomEvent {
            id: "jh_04".into(),
            text: "江湖豪杰数人慕名来投，愿意拜入本派门下。掌门大喜，设宴款待。".into(),
            effect: EventEffect {
                prestige: Some(2),
                morale: Some(3),
                silver: None,
                injury: None,
                free_recruit: Some(2),
                special: None,
                loyalty_loss: None,
                loyalty_change: None,
            },
            good: true,
        },
        RandomEvent {
            id: "jh_05".into(),
            text: "山贼下山劫掠，洗了山脚的村子。掌门率弟子连夜追击，剿灭匪首。".into(),
            effect: EventEffect {
                prestige: Some(5),
                silver: Some(40),
                morale: Some(2),
                injury: None,
                free_recruit: None,
                special: None,
                loyalty_loss: None,
                loyalty_change: None,
            },
            good: true,
        },
        RandomEvent {
            id: "jh_06".into(),
            text: "江湖传言本派藏有上古秘笈，各路宵小蠢蠢欲动。掌门连夜布防，一夜不得安寝。".into(),
            effect: EventEffect {
                prestige: Some(-1),
                morale: Some(-2),
                silver: None,
                injury: Some(5),
                free_recruit: None,
                special: None,
                loyalty_loss: None,
                loyalty_change: None,
            },
            good: false,
        },
        RandomEvent {
            id: "jh_07".into(),
            text: "一位云游僧人在山门盘桓数日，临行前留下一卷残缺心法。".into(),
            effect: EventEffect {
                prestige: Some(1),
                silver: None,
                morale: Some(1),
                injury: None,
                free_recruit: None,
                special: Some("manual".into()),
                loyalty_loss: None,
                loyalty_change: None,
            },
            good: true,
        },
        RandomEvent {
            id: "jh_08".into(),
            text: "魔教余孽在附近作乱，各大派约请本派共商讨魔大计。掌门率精锐前往会盟。".into(),
            effect: EventEffect {
                prestige: Some(6),
                morale: Some(3),
                silver: Some(-20),
                injury: Some(10),
                free_recruit: None,
                special: None,
                loyalty_loss: None,
                loyalty_change: None,
            },
            good: true,
        },
        RandomEvent {
            id: "jh_09".into(),
            text: "本派弟子在镇上酒楼与人争执，失手伤了人。掌门亲赴赔礼，花费不少。".into(),
            effect: EventEffect {
                prestige: Some(-3),
                silver: Some(-50),
                morale: Some(-1),
                injury: None,
                free_recruit: None,
                special: None,
                loyalty_loss: None,
                loyalty_change: None,
            },
            good: false,
        },
        RandomEvent {
            id: "jh_10".into(),
            text: "官府张榜悬赏江洋大盗，掌门遣弟子前往缉拿。苦战数日，终将贼人擒获。".into(),
            effect: EventEffect {
                prestige: Some(4),
                silver: Some(80),
                morale: Some(4),
                injury: Some(8),
                free_recruit: None,
                special: None,
                loyalty_loss: None,
                loyalty_change: None,
            },
            good: true,
        },
        RandomEvent {
            id: "jh_11".into(),
            text: "邻派掌门暴毙，其门下弟子怀疑是本派所为。江湖上议论纷纷。".into(),
            effect: EventEffect {
                prestige: Some(-5),
                morale: Some(-3),
                silver: None,
                injury: None,
                free_recruit: None,
                special: None,
                loyalty_loss: None,
                loyalty_change: None,
            },
            good: false,
        },
        RandomEvent {
            id: "jh_12".into(),
            text: "西域异人携奇珍异宝路过本地，掌门以礼相待，宾主尽欢。异人临别赠以珍奇药材。"
                .into(),
            effect: EventEffect {
                prestige: Some(2),
                morale: Some(1),
                silver: Some(60),
                injury: None,
                free_recruit: None,
                special: None,
                loyalty_loss: None,
                loyalty_change: None,
            },
            good: true,
        },
    ]
}

/// 门中生息事件池
fn mensheng_events() -> Vec<RandomEvent> {
    vec![
        RandomEvent { id: "ms_01".into(), text: "近日米价飞涨，厨房采买耗费甚巨。管事来报，本月开销比往常多了三成。".into(), effect: EventEffect { prestige: None, morale: None, silver: Some(-40), injury: None, free_recruit: None, special: None, loyalty_loss: None, loyalty_change: None }, good: false },
        RandomEvent { id: "ms_02".into(), text: "掌门晨起练功，不慎扭了腰。敷了药膏，将养数日方见好转。".into(), effect: EventEffect { prestige: None, morale: Some(-1), silver: None, injury: Some(5), free_recruit: None, special: None, loyalty_loss: None, loyalty_change: None }, good: false },
        RandomEvent { id: "ms_03".into(), text: "连日暴雨，练功场的木桩被大水冲毁大半。弟子们只好在室内演练，颇为憋闷。".into(), effect: EventEffect { prestige: None, morale: Some(-2), silver: Some(-30), injury: None, free_recruit: None, special: None, loyalty_loss: None, loyalty_change: None }, good: false },
        RandomEvent { id: "ms_04".into(), text: "掌门半夜观星，忽有所悟，对武学之道有了新的理解。当夜挥毫写下一篇心法。".into(), effect: EventEffect { prestige: Some(1), morale: Some(3), silver: None, injury: None, free_recruit: None, special: Some("epiphany".into()), loyalty_loss: None, loyalty_change: None }, good: true },
        RandomEvent { id: "ms_05".into(), text: "厨房大娘告老还乡，新来的厨子手艺极差，弟子们怨声载道。掌门只好亲自下厨半月。".into(), effect: EventEffect { prestige: None, morale: Some(-2), silver: None, injury: Some(3), free_recruit: None, special: None, loyalty_loss: None, loyalty_change: None }, good: false },
        RandomEvent { id: "ms_06".into(), text: "山后竹林里冒出了一眼温泉，弟子们轮流泡汤，个个神清气爽。".into(), effect: EventEffect { prestige: None, morale: Some(5), silver: None, injury: Some(-5), free_recruit: None, special: None, loyalty_loss: None, loyalty_change: None }, good: true },
        RandomEvent { id: "ms_07".into(), text: "两个弟子为了一件小事大打出手，掌门罚二人挑水三月，门派上下引以为戒。".into(), effect: EventEffect { prestige: None, morale: Some(-3), silver: None, injury: None, free_recruit: None, special: None, loyalty_loss: Some(true), loyalty_change: None }, good: false },
        RandomEvent { id: "ms_08".into(), text: "掌门生日，弟子们凑银两买了一坛好酒。月下共饮，其乐融融。".into(), effect: EventEffect { prestige: None, morale: Some(4), silver: Some(-10), injury: None, free_recruit: None, special: None, loyalty_loss: None, loyalty_change: None }, good: true },
        RandomEvent { id: "ms_09".into(), text: "库房闹了鼠患，不少药材被啃坏了。管库弟子自责不已，掌门好言宽慰。".into(), effect: EventEffect { prestige: None, morale: Some(1), silver: Some(-20), injury: None, free_recruit: None, special: None, loyalty_loss: None, loyalty_change: None }, good: false },
        RandomEvent { id: "ms_10".into(), text: "山下镇子里来了个老郎中，掌门请他上山给弟子们调理身体。众人药后精神焕发。".into(), effect: EventEffect { prestige: None, morale: Some(3), silver: Some(-25), injury: Some(-8), free_recruit: None, special: None, loyalty_loss: None, loyalty_change: None }, good: true },
        RandomEvent { id: "ms_11".into(), text: "一位弟子的家人来山探望，掌门准了半月探亲假。门中上下颇有归家之思。".into(), effect: EventEffect { prestige: None, morale: Some(-1), silver: None, injury: None, free_recruit: None, special: None, loyalty_loss: None, loyalty_change: Some(true) }, good: false },
        RandomEvent { id: "ms_12".into(), text: "掌门整理旧物，从师父遗留的木箱中发现一封书信，内附银票若干。师父在天之灵，仍挂念着本派。".into(), effect: EventEffect { prestige: Some(1), morale: Some(6), silver: Some(150), injury: None, free_recruit: None, special: None, loyalty_loss: None, loyalty_change: None }, good: true },
    ]
}

/// 触发普通月闻：无方略修正时，江湖、门内生活与扩展杂闻仍保持 40:35:25
/// 的基础占比；具体事件再由门风与经营方针共同偏置。
/// 时令事件：按游戏月份返回春、夏、秋、冬四时各一条生活流事件。
fn seasonal_events() -> Vec<RandomEvent> {
    [
        // 季春 — 茶山采青
        RandomEvent {
            id: "season_spring".into(),
            text: "三月采茶时节，门中杂役背上竹篓进山采青。今年的新茶比往年肥厚，炒制后满院茶香，弟子们分饮之余，还用茶饼换了邻镇几石白米。".into(),
            effect: EventEffect {
                morale: Some(3),
                silver: Some(5),
                ..Default::default()
            },
            good: true,
        },
        // 盛夏 — 消暑练功
        RandomEvent {
            id: "season_summer".into(),
            text: "酷暑难当，演武场石板晒得烫脚。掌门命弟子寅时起身，趁晨凉练功，午间在藏经阁避暑研读。一月下来，门中拳脚比往常精进了几分。".into(),
            effect: EventEffect {
                morale: Some(2),
                prestige: Some(1),
                ..Default::default()
            },
            good: true,
        },
        // 金秋 — 社日比武
        RandomEvent {
            id: "season_autumn".into(),
            text: "秋社之日，掌门在山门外摆下石锁、箭靶与擂台，邀四邻壮士前来较技。弟子们与乡民切磋一日，虽未尽全力，却也展了本门的威风，四邻心服，送来不少秋收贺礼。".into(),
            effect: EventEffect {
                prestige: Some(4),
                silver: Some(10),
                morale: Some(3),
                ..Default::default()
            },
            good: true,
        },
        // 严冬 — 抗寒修行
        RandomEvent {
            id: "season_winter".into(),
            text: "数九寒冬，山中滴水成冰。掌门下令门中上下晨起先打熬气血一个时辰方可进食。初时众人叫苦不迭，半月后却人人都觉得筋骨比入冬前强韧了不少。".into(),
            effect: EventEffect {
                silver: Some(-3),
                morale: Some(1),
                ..Default::default()
            },
            good: true,
        },
    ].into_iter().collect()
}

pub fn trigger_random_event(rng: &mut impl Rng, sect: &SectState) -> RandomEvent {
    let weighted_pool = jianghu_events()
        .into_iter()
        .map(|event| (event, 35_u32))
        .chain(mensheng_events().into_iter().map(|event| (event, 30_u32)))
        .chain(expanded_events().into_iter().map(|event| (event, 20_u32)))
        .chain(seasonal_events().into_iter().map(|event| (event, 15_u32)))
        .map(|(event, category_weight)| {
            let weight = category_weight.saturating_mul(event_weight(sect, &event.id));
            (event, weight)
        })
        .collect();
    choose_weighted(rng, weighted_pool)
}

use crate::models::martial_art::all_martial_arts;
use crate::models::GameState;

/// 应用事件效果到游戏状态，返回额外生成的事件文本列表
pub fn apply_event_effect(
    rng: &mut impl Rng,
    state: &mut GameState,
    event: &RandomEvent,
) -> Vec<String> {
    let mut extra_events = vec![];
    let e = &event.effect;

    if let Some(v) = e.prestige {
        state.prestige = disc::clamp(state.prestige + v, 0, 1000);
    }
    if let Some(v) = e.silver {
        state.silver = (state.silver + v).max(0);
    }
    if let Some(v) = e.morale {
        state.morale = disc::clamp(state.morale + v, 0, 100);
    }
    if let Some(v) = e.injury {
        state.injury = disc::clamp(state.injury + v, 0, 100);
    }

    if let Some(count) = e.free_recruit {
        for _ in 0..count {
            state.disciples.push(disc::generate_disciple(rng, 10));
        }
        extra_events.push("数名江湖人士加入本派！".into());
    }

    if let Some(ref special) = e.special {
        match special.as_str() {
            "manual" => {
                let privately_held = state
                    .disciples
                    .iter()
                    .flat_map(|disciple| disciple.martial_progress.private_books.iter())
                    .cloned()
                    .collect::<std::collections::BTreeSet<_>>();
                let arts = all_martial_arts();
                let unlearned: Vec<_> = arts
                    .iter()
                    .filter(|a| {
                        !state.sect.public_books.contains(&a.id)
                            && !state.martial_arts_learned.contains(&a.id)
                            && !privately_held.contains(&a.id)
                            && !(a.category == crate::models::martial_art::SkillCategory::Parry
                                && a.tier != crate::models::martial_art::MartialTier::Basic)
                    })
                    .collect();
                let owners = state
                    .disciples
                    .iter()
                    .enumerate()
                    .filter(|(_, disciple)| disciple.alive)
                    .map(|(index, _)| index)
                    .collect::<Vec<_>>();
                if !unlearned.is_empty() && !owners.is_empty() {
                    let art = unlearned[rng.gen_range(0..unlearned.len())];
                    let owner_index = owners[rng.gen_range(0..owners.len())];
                    let owner = &mut state.disciples[owner_index];
                    owner.martial_progress.private_books.push(art.id.clone());
                    extra_events.push(format!(
                        "{}偶得《{}》残卷，暂收入个人行囊，可自行研读或献入藏经阁。",
                        owner.name, art.name
                    ));
                }
            }
            "stock_grain" => *state.sect.inventory.entry("粮秣".into()).or_default() += 60,
            "stock_herbs" => *state.sect.inventory.entry("草药".into()).or_default() += 18,
            "lose_loyalty" => {
                for disciple in &mut state.disciples {
                    disciple.loyalty = (disciple.loyalty - 5).max(0);
                }
            }
            "heal_disciple" => {
                if let Some(disciple) = state
                    .disciples
                    .iter_mut()
                    .min_by_key(|disciple| disciple.attributes.qi.current)
                {
                    disciple.attributes.qi.current = disciple.attributes.qi.maximum / 2;
                    disciple.attributes.spirit.current = disciple.attributes.spirit.maximum / 2;
                    disc::refresh_condition(disciple);
                }
            }
            "court_service" => {
                state.sect.attributes.morality = (state.sect.attributes.morality - 5).max(0);
            }
            "uphold_morality" => {
                state.sect.attributes.morality = (state.sect.attributes.morality + 3).min(100);
            }
            "charity" => {
                state.sect.attributes.morality = (state.sect.attributes.morality + 8).min(100);
                let grain = state
                    .sect
                    .inventory
                    .get("粮秣")
                    .copied()
                    .unwrap_or(0)
                    .saturating_sub(30);
                state.sect.inventory.insert("粮秣".into(), grain);
            }
            "improve_relation" => {
                if !state.npc_sects.is_empty() {
                    let index = rng.gen_range(0..state.npc_sects.len());
                    let other_id = state.npc_sects[index].id.clone();
                    let other_name = state.npc_sects[index].name.clone();
                    let player_id = state.sect.id.clone();
                    let player_relation = state.sect.relations.entry(other_id.clone()).or_default();
                    *player_relation = (*player_relation + 8).clamp(-100, 100);
                    let other_relation = state.npc_sects[index]
                        .relations
                        .entry(player_id)
                        .or_default();
                    *other_relation = (*other_relation + 8).clamp(-100, 100);
                    extra_events.push(format!("本派与{}互致盟书，双方交情各有进益。", other_name));
                }
            }
            "epiphany" => {
                state.prestige = disc::clamp(state.prestige + 3, 0, 1000);
                let alive_count = state.disciples.iter().filter(|d| d.alive).count();
                if alive_count > 0 {
                    let idx = rng.gen_range(0..alive_count);
                    if let Some(d) = state.disciples.iter_mut().filter(|d| d.alive).nth(idx) {
                        let gain = disc::rand_range(rng, 8, 20);
                        d.inner_power = (d.inner_power + gain).max(10);
                        extra_events.push(format!("{}听掌门讲道，若有所悟，内力精进！", d.name));
                    }
                }
            }
            _ => {}
        }
    }

    if e.loyalty_loss.unwrap_or(false) {
        for d in state.disciples.iter_mut() {
            if d.alive && rng.gen_bool(0.3) {
                d.loyalty = disc::clamp(d.loyalty - disc::rand_range(rng, 5, 15), 0, 100);
            }
        }
    }

    if e.loyalty_change.unwrap_or(false) {
        for d in state.disciples.iter_mut() {
            if d.alive {
                d.loyalty = disc::clamp(d.loyalty + disc::rand_range(rng, -5, 10), 0, 100);
            }
        }
    }

    extra_events
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::sect::{MoralDirection, SectPolicy, SectState};
    use crate::models::{Disciple, GameState};
    use rand::{rngs::StdRng, SeedableRng};

    #[test]
    fn interactive_pool_has_multiple_wuxia_categories_and_choices() {
        let events = interactive_events();
        assert!(events.len() >= 8);
        let categories: std::collections::BTreeSet<_> =
            events.iter().map(|event| event.category.as_str()).collect();
        assert!(categories.len() >= 5);
        assert!(events.iter().all(|event| event.choices.len() >= 2));
    }

    #[test]
    fn moral_direction_and_policy_have_exact_thematic_event_weights() {
        let mut righteous = SectState::default();
        righteous.moral_direction = MoralDirection::Righteous;
        righteous.policy = SectPolicy::Balanced;
        let mut neutral = righteous.clone();
        neutral.moral_direction = MoralDirection::Neutral;
        let mut villainous = righteous.clone();
        villainous.moral_direction = MoralDirection::Villainous;

        // 救济既合正派门风，又属于持中方针偏爱的门内生计。
        assert_eq!(event_weight(&righteous, "choice_refugees"), 175);
        assert_eq!(event_weight(&neutral, "choice_refugees"), 135);
        assert_eq!(event_weight(&villainous, "choice_refugees"), 80);

        // 邪派同时更容易遇到可牟利的朝廷差事，以及仇怨上门的反噬。
        assert_eq!(event_weight(&righteous, "choice_court"), 100);
        assert_eq!(event_weight(&villainous, "choice_court"), 145);
        assert_eq!(event_weight(&righteous, "choice_provocation"), 70);
        assert_eq!(event_weight(&villainous, "choice_provocation"), 145);
        assert_eq!(event_weight(&righteous, "jh_05"), 160);
        assert_eq!(event_weight(&villainous, "jh_05"), 110);
        assert_eq!(event_weight(&righteous, "jh_09"), 70);
        assert_eq!(event_weight(&villainous, "jh_09"), 145);

        let mut mercantile = SectState::default();
        mercantile.moral_direction = MoralDirection::Neutral;
        mercantile.policy = SectPolicy::Mercantile;
        let mut reclusive = mercantile.clone();
        reclusive.policy = SectPolicy::Reclusive;
        assert_eq!(event_weight(&mercantile, "choice_rice"), 150);
        assert_eq!(event_weight(&reclusive, "choice_rice"), 120);
        assert_eq!(event_weight(&mercantile, "jh_07"), 100);
        assert_eq!(event_weight(&reclusive, "jh_07"), 130);
    }

    #[test]
    fn every_event_remains_possible_under_every_governance_combination() {
        let ids = interactive_events()
            .into_iter()
            .map(|event| event.id)
            .chain(jianghu_events().into_iter().map(|event| event.id))
            .chain(mensheng_events().into_iter().map(|event| event.id))
            .chain(expanded_events().into_iter().map(|event| event.id))
            .collect::<Vec<_>>();
        let directions = [
            MoralDirection::Righteous,
            MoralDirection::Neutral,
            MoralDirection::Villainous,
        ];
        let policies = [
            SectPolicy::Balanced,
            SectPolicy::Martial,
            SectPolicy::Scholarly,
            SectPolicy::Chivalrous,
            SectPolicy::Mercantile,
            SectPolicy::Reclusive,
        ];

        for direction in directions {
            for policy in &policies {
                let mut sect = SectState::default();
                sect.moral_direction = direction.clone();
                sect.policy = policy.clone();
                assert!(
                    ids.iter().all(|id| event_weight(&sect, id) > 0),
                    "{direction:?} / {policy:?} 不得将任何事件权重降为零"
                );
            }
        }
    }

    #[test]
    fn fixed_seed_picks_more_matching_interactive_events() {
        let mut righteous = SectState::default();
        righteous.moral_direction = MoralDirection::Righteous;
        righteous.policy = SectPolicy::Chivalrous;
        let mut villainous = SectState::default();
        villainous.moral_direction = MoralDirection::Villainous;
        villainous.policy = SectPolicy::Mercantile;
        let mut righteous_rng = StdRng::seed_from_u64(20260719);
        let mut villainous_rng = StdRng::seed_from_u64(20260719);
        let mut righteous_rescues = 0;
        let mut villainous_rescues = 0;
        let mut righteous_profit_or_backlash = 0;
        let mut villainous_profit_or_backlash = 0;

        for _ in 0..10_000 {
            let righteous_event = trigger_interactive_event(&mut righteous_rng, &righteous);
            let villainous_event = trigger_interactive_event(&mut villainous_rng, &villainous);
            righteous_rescues += usize::from(righteous_event.id == "choice_refugees");
            villainous_rescues += usize::from(villainous_event.id == "choice_refugees");
            righteous_profit_or_backlash += usize::from(matches!(
                righteous_event.id.as_str(),
                "choice_rice" | "choice_court" | "choice_herbs" | "choice_provocation"
            ));
            villainous_profit_or_backlash += usize::from(matches!(
                villainous_event.id.as_str(),
                "choice_rice" | "choice_court" | "choice_herbs" | "choice_provocation"
            ));
        }

        assert!(righteous_rescues > villainous_rescues);
        assert!(villainous_profit_or_backlash > righteous_profit_or_backlash);
    }

    #[test]
    fn jianghu_chronicles_do_not_expose_numeric_deltas() {
        assert!(jianghu_events().iter().all(|event| {
            !event
                .text
                .chars()
                .any(|character| character.is_ascii_digit())
                && !event.text.contains("点")
                && !event.text.contains("两")
                && !event.text.contains('+')
        }));
    }

    #[test]
    fn manual_events_create_a_real_private_book_instead_of_a_public_unlock() {
        let mut state = GameState::default();
        state.disciples.push(Disciple {
            name: "沈归鸿".into(),
            ..Disciple::default()
        });
        let public_before = state.sect.public_books.clone();
        let learned_before = state.martial_arts_learned.clone();
        let event = RandomEvent {
            id: "private-manual-test".into(),
            text: "山中偶得残卷。".into(),
            effect: effect(0, 0, 0, 0, Some("manual")),
            good: true,
        };
        let mut rng = StdRng::seed_from_u64(20260718);

        let extra = apply_event_effect(&mut rng, &mut state, &event);

        assert_eq!(state.disciples[0].martial_progress.private_books.len(), 1);
        let book = &state.disciples[0].martial_progress.private_books[0];
        assert!(!state.sect.public_books.contains(book));
        assert_eq!(state.sect.public_books, public_before);
        assert_eq!(state.martial_arts_learned, learned_before);
        assert!(extra[0].contains("个人行囊"));
    }

    #[test]
    fn alliance_event_improves_both_factions_relations_with_bounds() {
        let mut state = GameState::default();
        let mut other = SectState {
            id: "wudang".into(),
            name: "武当派".into(),
            ..SectState::default()
        };
        state.sect.relations.insert(other.id.clone(), 96);
        other.relations.insert(state.sect.id.clone(), 95);
        state.npc_sects.push(other);
        let event = RandomEvent {
            id: "bilateral-alliance-test".into(),
            text: "两派互致盟书。".into(),
            effect: effect(0, 0, 0, 0, Some("improve_relation")),
            good: true,
        };
        let mut rng = StdRng::seed_from_u64(20260719);

        let extra = apply_event_effect(&mut rng, &mut state, &event);

        assert_eq!(state.sect.relations.get("wudang"), Some(&100));
        assert_eq!(state.npc_sects[0].relations.get(&state.sect.id), Some(&100));
        assert!(extra.iter().any(|text| text.contains("武当派")));
    }
}
