use rand::Rng;
use serde::{Deserialize, Serialize};
use crate::logic::disciple as disc;

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

/// 江湖风云事件池
fn jianghu_events() -> Vec<RandomEvent> {
    vec![
        RandomEvent { id: "jh_01".into(), text: "邻派遣使来谒，言语间颇有试探之意。掌门好言相待，使者惭而退。".into(), effect: EventEffect { prestige: Some(3), morale: Some(2), silver: None, injury: None, free_recruit: None, special: None, loyalty_loss: None, loyalty_change: None }, good: true },
        RandomEvent { id: "jh_02".into(), text: "邻派率众来犯，声称本派侵占了他们的采药之地。一场恶斗，各有损伤。".into(), effect: EventEffect { prestige: Some(-2), morale: Some(-3), silver: Some(-30), injury: Some(8), free_recruit: None, special: None, loyalty_loss: None, loyalty_change: None }, good: false },
        RandomEvent { id: "jh_03".into(), text: "朝廷特使到访，说圣上听闻本派侠名，特赐「侠义之门」匾额一面，并赏银百两。".into(), effect: EventEffect { prestige: Some(8), silver: Some(100), morale: Some(5), injury: None, free_recruit: None, special: None, loyalty_loss: None, loyalty_change: None }, good: true },
        RandomEvent { id: "jh_04".into(), text: "江湖豪杰数人慕名来投，愿意拜入本派门下。掌门大喜，设宴款待。".into(), effect: EventEffect { prestige: Some(2), morale: Some(3), silver: None, injury: None, free_recruit: Some(2), special: None, loyalty_loss: None, loyalty_change: None }, good: true },
        RandomEvent { id: "jh_05".into(), text: "山贼下山劫掠，洗了山脚的村子。掌门率弟子连夜追击，剿灭匪首。".into(), effect: EventEffect { prestige: Some(5), silver: Some(40), morale: Some(2), injury: None, free_recruit: None, special: None, loyalty_loss: None, loyalty_change: None }, good: true },
        RandomEvent { id: "jh_06".into(), text: "江湖传言本派藏有上古秘笈，各路宵小蠢蠢欲动。掌门连夜布防，一夜不得安寝。".into(), effect: EventEffect { prestige: Some(-1), morale: Some(-2), silver: None, injury: Some(5), free_recruit: None, special: None, loyalty_loss: None, loyalty_change: None }, good: false },
        RandomEvent { id: "jh_07".into(), text: "一位云游僧人在山门盘桓数日，临行前留下一卷残缺心法。".into(), effect: EventEffect { prestige: Some(1), silver: None, morale: Some(1), injury: None, free_recruit: None, special: Some("manual".into()), loyalty_loss: None, loyalty_change: None }, good: true },
        RandomEvent { id: "jh_08".into(), text: "魔教余孽在附近作乱，各大派约请本派共商讨魔大计。掌门率精锐前往会盟。".into(), effect: EventEffect { prestige: Some(6), morale: Some(3), silver: Some(-20), injury: Some(10), free_recruit: None, special: None, loyalty_loss: None, loyalty_change: None }, good: true },
        RandomEvent { id: "jh_09".into(), text: "本派弟子在镇上酒楼与人争执，失手伤了人。掌门亲赴赔礼，花费不少。".into(), effect: EventEffect { prestige: Some(-3), silver: Some(-50), morale: Some(-1), injury: None, free_recruit: None, special: None, loyalty_loss: None, loyalty_change: None }, good: false },
        RandomEvent { id: "jh_10".into(), text: "官府张榜悬赏江洋大盗，掌门遣弟子前往缉拿。苦战三日，终将贼人擒获。".into(), effect: EventEffect { prestige: Some(4), silver: Some(80), morale: Some(4), injury: Some(8), free_recruit: None, special: None, loyalty_loss: None, loyalty_change: None }, good: true },
        RandomEvent { id: "jh_11".into(), text: "邻派掌门暴毙，其门下弟子怀疑是本派所为。江湖上议论纷纷。".into(), effect: EventEffect { prestige: Some(-5), morale: Some(-3), silver: None, injury: None, free_recruit: None, special: None, loyalty_loss: None, loyalty_change: None }, good: false },
        RandomEvent { id: "jh_12".into(), text: "西域异人携奇珍异宝路过本地，掌门以礼相待，宾主尽欢。异人临别赠以珍奇药材。".into(), effect: EventEffect { prestige: Some(2), morale: Some(1), silver: Some(60), injury: None, free_recruit: None, special: None, loyalty_loss: None, loyalty_change: None }, good: true },
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

/// 触发随机事件（50% 江湖 / 50% 门中）
pub fn trigger_random_event(rng: &mut impl Rng) -> RandomEvent {
    if rng.gen_bool(0.5) {
        let pool = jianghu_events();
        pool[rng.gen_range(0..pool.len())].clone()
    } else {
        let pool = mensheng_events();
        pool[rng.gen_range(0..pool.len())].clone()
    }
}

use crate::models::{GameState};
use crate::models::martial_art::all_martial_arts;

/// 应用事件效果到游戏状态，返回额外生成的事件文本列表
pub fn apply_event_effect(
    rng: &mut impl Rng,
    state: &mut GameState,
    event: &RandomEvent,
) -> Vec<String> {
    let mut extra_events = vec![];
    let e = &event.effect;

    if let Some(v) = e.prestige { state.prestige = disc::clamp(state.prestige + v, 0, 100); }
    if let Some(v) = e.silver { state.silver = (state.silver + v).max(0); }
    if let Some(v) = e.morale { state.morale = disc::clamp(state.morale + v, 0, 100); }
    if let Some(v) = e.injury { state.injury = disc::clamp(state.injury + v, 0, 100); }

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
                if let Some(art) = unlearned.get(rng.gen_range(0..unlearned.len())) {
                    state.martial_arts_learned.push(art.id.clone());
                    extra_events.push(format!("获得武学残卷，参悟《{}》！", art.name));
                }
            }
            "epiphany" => {
                state.prestige = disc::clamp(state.prestige + 3, 0, 100);
                let alive_count = state.disciples.iter().filter(|d| d.alive).count();
                if alive_count > 0 {
                    let idx = rng.gen_range(0..alive_count);
                    if let Some(d) = state.disciples.iter_mut().filter(|d| d.alive).nth(idx) {
                        let gain = disc::rand_range(rng, 8, 20);
                        d.inner_power = disc::clamp(d.inner_power + gain, 10, 100);
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
