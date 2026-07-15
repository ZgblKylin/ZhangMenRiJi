use crate::logic::disciple as disc;
use crate::logic::event::{self, RandomEvent};
use crate::logic::tournament;
use crate::models::{GameEvent, GameState};
use rand::Rng;

/// 推进月份
pub fn advance_month(
    rng: &mut impl Rng,
    state: &mut GameState,
) -> (
    Vec<GameEvent>,
    Option<crate::models::tournament::TournamentResult>,
    bool,
) {
    let mut events = vec![];

    // 1. 所有人物基于月初快照并行行动，结果统一归并。
    events.extend(crate::logic::action::run_auto_actions(state));

    // 2. 触发随机事件
    let random_event: RandomEvent = event::trigger_random_event(rng);
    let extra_events = event::apply_event_effect(rng, state, &random_event);
    let mood = if random_event.good { "good" } else { "bad" };
    events.push(GameEvent {
        text: random_event.text.clone(),
        mood: mood.into(),
        year: state.year,
        month: state.month,
    });
    for t in extra_events {
        events.push(GameEvent {
            text: t,
            mood: "good".into(),
            year: state.year,
            month: state.month,
        });
    }

    // 3. 弟子月度恢复、年龄与门忠变化
    disc::monthly_growth(rng, &mut state.disciples, state.morale);
    for sect in &state.npc_sects {
        let morale = sect.attributes.morale;
        let mut members: Vec<&mut crate::models::Disciple> = state
            .npc_disciples
            .iter_mut()
            .filter(|d| d.sect_id.as_deref() == Some(sect.id.as_str()))
            .collect();
        for member in &mut members {
            disc::monthly_growth(rng, std::slice::from_mut(*member), morale);
        }
    }

    // 3. 被动收支
    let alive_count = state.disciples.iter().filter(|d| d.alive).count() as i32;
    let income = state.prestige * 3 / 10 + alive_count * 3;
    let expense = alive_count * 5 + 20;
    state.silver = (state.silver + income - expense).max(0);

    // 4. 志气自然浮动
    state.morale = disc::clamp(state.morale + disc::rand_range(rng, -3, 3), 0, 100);

    // 5. 掌门伤势恢复
    state.injury = disc::clamp(state.injury - disc::rand_range(rng, 2, 5), 0, 100);

    // 6. 叛逃检查
    let deserters = disc::check_desertion(rng, &mut state.disciples);
    for name in &deserters {
        events.push(GameEvent {
            text: format!("{}叛出师门，不知所踪！", name),
            mood: "bad".into(),
            year: state.year,
            month: state.month,
        });
        state.morale = disc::clamp(state.morale - 5, 0, 100);
    }

    // 7. 库银枯竭
    if state.silver <= 0 {
        let alive = state.disciples.iter().filter(|d| d.alive).count();
        if alive > 2 {
            state.morale = disc::clamp(state.morale - 10, 0, 100);
            let leave_count = (alive / 3).max(1);
            let mut left = 0;
            state.disciples.retain(|d| {
                if !d.alive {
                    return false;
                }
                if left < leave_count {
                    left += 1;
                    false
                } else {
                    true
                }
            });
            events.push(GameEvent {
                text: "库银见底，数名弟子辞别而去……".into(),
                mood: "bad".into(),
                year: state.year,
                month: state.month,
            });
        }
    }

    // 8. 游戏结束检查
    let alive = state.disciples.iter().filter(|d| d.alive).count();
    if alive == 0 && state.silver < 20 {
        state.game_over = true;
        state.game_over_reason = "门中无弟子，库银枯竭。山门冷落，掌门黯然隐退……".into();
    }
    if state.prestige <= 0 && state.morale <= 0 {
        state.game_over = true;
        state.game_over_reason = "江湖声望尽失，门人志气消沉。本派终究未能撑过难关。".into();
    }

    // 9. 论剑（12月推进时触发）
    let tournament_result = if state.month == 12 {
        let result = tournament::run_tournament(rng, state);
        events.push(GameEvent {
            text: format!(
                "年终论剑！本派在{}派中位列第{}名。{}",
                result.total_sects, result.rank, result.desc_text
            ),
            mood: "good".into(),
            year: state.year,
            month: 12,
        });
        Some(result)
    } else {
        None
    };

    // 10. 推进时间
    state.month += 1;
    if state.month > 12 {
        state.month = 1;
        state.year += 1;
        events.push(GameEvent {
            text: format!("———— 第{}年 ————", state.year),
            mood: "neutral".into(),
            year: state.year,
            month: state.month,
        });
    }

    // ★ 将本轮事件追加到 state.event_log（保留最近 50 条）
    for ev in &events {
        state.event_log.push(ev.clone());
    }
    if state.event_log.len() > 50 {
        let excess = state.event_log.len() - 50;
        state.event_log.drain(0..excess);
    }

    // 11. 重置决策
    state.decisions_used = 0;
    state.pending_event = None;

    (events, tournament_result, state.game_over)
}
