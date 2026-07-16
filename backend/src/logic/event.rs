use crate::logic::disciple as disc;
use rand::Rng;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
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
            text: "邻派少侠在山门外连败三名外门弟子，扬言要请掌门赐教。如何处置？".into(),
            choices: vec![
                EventChoice {
                    id: "fight".into(),
                    label: "亲自应战".into(),
                    result_text: "掌门亲自下场，以三招定胜负。来客抱拳服输，江湖为之侧目。".into(),
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

pub fn trigger_interactive_event(rng: &mut impl Rng) -> PendingWorldEvent {
    let pool = interactive_events();
    pool[rng.gen_range(0..pool.len())].clone()
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
            text: "朝廷特使到访，说圣上听闻本派侠名，特赐「侠义之门」匾额一面，并赏银百两。".into(),
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
            text: "官府张榜悬赏江洋大盗，掌门遣弟子前往缉拿。苦战三日，终将贼人擒获。".into(),
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

/// 触发普通月闻：江湖、门内与生活杂闻各占一部分。
pub fn trigger_random_event(rng: &mut impl Rng) -> RandomEvent {
    let roll = rng.gen_range(0..100);
    if roll < 40 {
        let pool = jianghu_events();
        pool[rng.gen_range(0..pool.len())].clone()
    } else if roll < 75 {
        let pool = mensheng_events();
        pool[rng.gen_range(0..pool.len())].clone()
    } else {
        let pool = expanded_events();
        pool[rng.gen_range(0..pool.len())].clone()
    }
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
        extra_events.push(format!("{}名江湖人士加入本派！", count));
    }

    if let Some(ref special) = e.special {
        match special.as_str() {
            "manual" => {
                let learned = &state.martial_arts_learned;
                let arts = all_martial_arts();
                let unlearned: Vec<_> = arts.iter().filter(|a| !learned.contains(&a.id)).collect();
                if !unlearned.is_empty() {
                    let art = unlearned[rng.gen_range(0..unlearned.len())];
                    state.martial_arts_learned.push(art.id.clone());
                    state.sect.public_books.push(art.id.clone());
                    extra_events.push(format!("获得武学残卷，参悟《{}》！", art.name));
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
                    let other = &state.npc_sects[rng.gen_range(0..state.npc_sects.len())];
                    *state.sect.relations.entry(other.id.clone()).or_default() += 8;
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

    #[test]
    fn interactive_pool_has_multiple_wuxia_categories_and_choices() {
        let events = interactive_events();
        assert!(events.len() >= 8);
        let categories: std::collections::BTreeSet<_> =
            events.iter().map(|event| event.category.as_str()).collect();
        assert!(categories.len() >= 5);
        assert!(events.iter().all(|event| event.choices.len() >= 2));
    }
}
