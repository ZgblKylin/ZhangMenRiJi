use rand::Rng;
use crate::models::{GameState, GameEvent};
use crate::models::martial_art::all_martial_arts;
use crate::logic::disciple as disc;

fn clamp(v: i32, lo: i32, hi: i32) -> i32 { v.max(lo).min(hi) }

/// 执行决策，返回产生的事件
pub fn execute_decision(rng: &mut impl Rng, state: &mut GameState, decision_id: &str) -> Vec<GameEvent> {
    let mut events = vec![];

    match decision_id {
        "recruit" => {
            if state.silver < 50 { return events; }
            state.silver -= 50;
            let count = disc::rand_range(rng, 1, 2);
            for _ in 0..count {
                let bonus = if state.prestige > 50 { 10 } else { 0 };
                state.disciples.push(disc::generate_disciple(rng, bonus));
            }
            state.total_disciples_recruited += count;
            events.push(GameEvent { text: format!("招贤榜贴出，{}人前来拜山投师。", count), mood: "good".into(), year: state.year, month: state.month });
        }
        "train" => {
            if state.injury >= 30 { return events; }
            for d in state.disciples.iter_mut().filter(|d| d.alive) {
                let gain = (disc::rand_range(rng, 2, 6) - state.injury / 20).max(1);
                d.inner_power = clamp(d.inner_power + gain, 10, 100);
                d.loyalty = clamp(d.loyalty + disc::rand_range(rng, 1, 4), 0, 100);
            }
            state.injury = clamp(state.injury + disc::rand_range(rng, 3, 8), 0, 100);
            state.morale = clamp(state.morale + 2, 0, 100);
            events.push(GameEvent { text: "掌门率众苦练一月，弟子内力均有所长。掌门略感疲惫。".into(), mood: "good".into(), year: state.year, month: state.month });
        }
        "mission" => {
            let count = state.disciples.iter().filter(|d| d.alive).count();
            if count == 0 { return events; }
            let idx = disc::rand_range(rng, 0, count as i32 - 1) as usize;
            if let Some(d) = state.disciples.iter_mut().filter(|d| d.alive).nth(idx) {
                let score = disc::get_combat_score(d);
                if score > 25 {
                    let sg = disc::rand_range(rng, 20, 80) + score / 2;
                    let pg = disc::rand_range(rng, 1, 4);
                    state.silver += sg;
                    state.prestige = clamp(state.prestige + pg, 0, 100);
                    d.loyalty = clamp(d.loyalty + disc::rand_range(rng, 1, 5), 0, 100);
                    events.push(GameEvent { text: format!("{}下山行侠，仗剑惩恶，带回银{}两，乡民称颂。", d.name, sg), mood: "good".into(), year: state.year, month: state.month });
                } else {
                    d.inner_power = clamp(d.inner_power - 5, 5, 100);
                    d.loyalty = clamp(d.loyalty - disc::rand_range(rng, 3, 8), 0, 100);
                    state.injury = clamp(state.injury + disc::rand_range(rng, 3, 8), 0, 100);
                    events.push(GameEvent { text: format!("{}下山行侠，不料遭遇强敌！弟子受伤，掌门亲赴接应。", d.name), mood: "bad".into(), year: state.year, month: state.month });
                }
            }
        }
        "repair" => {
            if state.silver < 60 { return events; }
            state.silver -= 60;
            state.morale = clamp(state.morale + disc::rand_range(rng, 5, 10), 0, 100);
            events.push(GameEvent { text: "山门修缮一新，弟子们很是高兴，练功也格外卖力。".into(), mood: "good".into(), year: state.year, month: state.month });
        }
        "diplomacy" => {
            if state.silver < 40 { return events; }
            state.silver -= 40;
            let pg = disc::rand_range(rng, 2, 6);
            let sg = disc::rand_range(rng, 0, 30);
            state.prestige = clamp(state.prestige + pg, 0, 100);
            state.silver += sg;
            state.morale = clamp(state.morale + 1, 0, 100);
            let extra = if sg > 0 { "对方回赠了薄礼。" } else { "" };
            events.push(GameEvent { text: format!("掌门携礼拜访邻派，相谈甚欢。{}", extra), mood: "good".into(), year: state.year, month: state.month });
        }
        "rest" => {
            let heal = disc::rand_range(rng, 15, 30);
            state.injury = clamp(state.injury - heal, 0, 100);
            state.morale = clamp(state.morale - 1, 0, 100);
            events.push(GameEvent { text: "掌门闭门静养月余，伤势大有好转。".into(), mood: "good".into(), year: state.year, month: state.month });
        }
        "study" => {
            if state.silver < 30 { return events; }
            state.silver -= 30;
            state.injury = clamp(state.injury + disc::rand_range(rng, 5, 12), 0, 100);
            let arts = all_martial_arts();
            let unlearned: Vec<_> = arts.iter().filter(|a| !state.martial_arts_learned.contains(&a.id)).collect();
            if !unlearned.is_empty() && rng.gen_bool(0.5) {
                let art = &unlearned[disc::rand_range(rng, 0, unlearned.len() as i32 - 1) as usize];
                state.martial_arts_learned.push(art.id.clone());
                state.prestige = clamp(state.prestige + 3, 0, 100);
                events.push(GameEvent { text: format!("苦心孤诣，终有所成！掌门创出《{}》，本派武学再添一门绝技！", art.name), mood: "good".into(), year: state.year, month: state.month });
            } else {
                events.push(GameEvent { text: "掌门闭关苦思，虽未创出新招，但对武学之理解更深了一层。".into(), mood: "neutral".into(), year: state.year, month: state.month });
            }
        }
        "teach" => {
            if state.silver < 20 { return events; }
            let alive_count = state.disciples.iter().filter(|d| d.alive).count();
            if alive_count == 0 { return events; }
            state.silver -= 20;
            state.injury = clamp(state.injury + disc::rand_range(rng, 3, 7), 0, 100);
            let count = alive_count.min(3);
            let mut names = vec![];
            for d in state.disciples.iter_mut().filter(|d| d.alive).take(count) {
                d.inner_power = clamp(d.inner_power + disc::rand_range(rng, 3, 10), 10, 100);
                d.loyalty = clamp(d.loyalty + disc::rand_range(rng, 2, 6), 0, 100);
                if rng.gen_bool(0.3) && !state.martial_arts_learned.is_empty() {
                    let idx = disc::rand_range(rng, 0, state.martial_arts_learned.len() as i32 - 1) as usize;
                    d.martial_art = state.martial_arts_learned[idx].clone();
                }
                names.push(d.name.clone());
            }
            events.push(GameEvent { text: format!("掌门亲自为{}传功，诸弟子获益良多。", names.join("、")), mood: "good".into(), year: state.year, month: state.month });
        }
        _ => {}
    }

    state.decisions_used += 1;

    // ★ 将决策事件写入 state.event_log（保留最近 50 条）
    for ev in &events {
        state.event_log.push(ev.clone());
    }
    if state.event_log.len() > 50 {
        let excess = state.event_log.len() - 50;
        state.event_log.drain(0..excess);
    }

    events
}
