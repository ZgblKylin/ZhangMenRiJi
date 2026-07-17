use crate::logic::disciple as disc;
use crate::logic::event::{self, RandomEvent};
use crate::logic::tournament;
use crate::models::attributes::{ActionKind, DiscipleCondition, DiscipleRank};
use crate::models::sect::SectState;
use crate::models::{Disciple, GameEvent, GameState};
use rand::Rng;

pub type AdvanceResult = (
    Vec<GameEvent>,
    Option<crate::models::tournament::TournamentResult>,
    bool,
);

/// 推进月份
pub fn advance_month(rng: &mut impl Rng, state: &mut GameState) -> AdvanceResult {
    let mut events = vec![];

    if state.pending_event.is_some() {
        return (events, None, state.game_over);
    }

    // 约三成月份先遇到须由掌门定夺的大事，定夺前不改动月初快照。
    if rng.gen_bool(0.3) {
        let pending = event::trigger_interactive_event(rng);
        let announcement = GameEvent {
            text: format!(
                "【{}】{}：{}",
                pending.category, pending.title, pending.text
            ),
            mood: "neutral".into(),
            year: state.year,
            month: state.month,
            category: interactive_category(&pending.category).into(),
        };
        state.pending_event = serde_json::to_value(&pending).ok();
        state.event_log.push(announcement.clone());
        trim_log(state);
        events.push(announcement);
        return (events, None, state.game_over);
    }

    // 无须抉择的普通月闻立即结算，而后继续推演本月行动。
    let random_event: RandomEvent = event::trigger_random_event(rng);
    let extra_events = event::apply_event_effect(rng, state, &random_event);
    for disciple in &mut state.disciples {
        disc::absorb_legacy_attributes(disciple);
    }
    let mood = if random_event.good { "good" } else { "bad" };
    let category = if random_event.id.starts_with("jh_") {
        "world"
    } else {
        "sect"
    };
    events.push(GameEvent {
        text: random_event.text.clone(),
        mood: mood.into(),
        year: state.year,
        month: state.month,
        category: category.into(),
    });
    for t in extra_events {
        events.push(GameEvent {
            text: t,
            mood: "good".into(),
            year: state.year,
            month: state.month,
            category: category.into(),
        });
    }
    crate::logic::sect::absorb_legacy_fields(state);

    finish_month(rng, state, events)
}

pub fn resolve_pending_event(
    rng: &mut impl Rng,
    state: &mut GameState,
    option_id: &str,
) -> Result<AdvanceResult, String> {
    let value = state
        .pending_event
        .clone()
        .ok_or_else(|| "眼下并无待决之事。".to_string())?;
    let pending: event::PendingWorldEvent =
        serde_json::from_value(value).map_err(|_| "此事卷宗已有残缺，无法处置。".to_string())?;
    let category = interactive_category(&pending.category);
    let choice = pending
        .choices
        .iter()
        .find(|choice| choice.id == option_id)
        .cloned()
        .ok_or_else(|| "掌门所选并非卷宗所列之策。".to_string())?;
    let random_event = RandomEvent {
        id: format!("{}:{}", pending.id, choice.id),
        text: choice.result_text.clone(),
        effect: choice.effect,
        good: choice.good,
    };
    let extra = event::apply_event_effect(rng, state, &random_event);
    for disciple in &mut state.disciples {
        disc::absorb_legacy_attributes(disciple);
    }
    crate::logic::sect::absorb_legacy_fields(state);
    state.pending_event = None;
    let mut events = vec![GameEvent {
        text: random_event.text,
        mood: if random_event.good { "good" } else { "bad" }.into(),
        year: state.year,
        month: state.month,
        category: category.into(),
    }];
    events.extend(extra.into_iter().map(|text| GameEvent {
        text,
        mood: "good".into(),
        year: state.year,
        month: state.month,
        category: category.into(),
    }));
    Ok(finish_month(rng, state, events))
}

fn finish_month(
    rng: &mut impl Rng,
    state: &mut GameState,
    mut events: Vec<GameEvent>,
) -> AdvanceResult {
    let payroll = pay_monthly_stipends(&mut state.sect, &mut state.disciples, None);
    events.push(GameEvent {
        text: format!(
            "门中月俸：发私银{}两、口粮{}份{}。",
            payroll.silver_paid,
            payroll.rations_paid,
            if payroll.silver_paid < payroll.silver_due
                || payroll.rations_paid < payroll.rations_due
            {
                "，因库存不足按身份次序折发"
            } else {
                ""
            }
        ),
        mood: if payroll.silver_paid < payroll.silver_due
            || payroll.rations_paid < payroll.rations_due
        {
            "neutral"
        } else {
            "good"
        }
        .into(),
        year: state.year,
        month: state.month,
        category: "sect".into(),
    });
    for npc_sect in &mut state.npc_sects {
        let sect_id = npc_sect.id.clone();
        pay_monthly_stipends(npc_sect, &mut state.npc_disciples, Some(sect_id.as_str()));
    }

    advance_auto_brew(state);
    events.extend(claim_travel_rations(state));
    for (building_name, result) in
        crate::logic::management::execute_monthly_elder_duties(rng, state)
    {
        let (text, mood) = match result {
            Ok(text) => (text, "good"),
            Err(error) => (format!("{}堂务未成：{}", building_name, error), "neutral"),
        };
        events.push(GameEvent {
            text,
            mood: mood.into(),
            year: state.year,
            month: state.month,
            category: "sect".into(),
        });
    }
    events.extend(crate::logic::world::run_npc_ai(rng, state));
    // 所有人物仍基于定夺完成后的同一份月初快照并行行动，结果统一归并。
    events.extend(crate::logic::action::run_auto_actions(state));
    // 自主决定本月外出的弟子会在行动结算后得到实际行期，此时补办一次申领。
    events.extend(claim_travel_rations(state));
    events.extend(treat_injured_disciples(state));

    // 3. 结算年龄、门忠和月末自然恢复；修为仍由每人的实际行动独立结算。
    disc::settle_month(&mut state.disciples, state.morale);
    for sect in &state.npc_sects {
        let morale = sect.attributes.morale;
        for member in state
            .npc_disciples
            .iter_mut()
            .filter(|d| d.sect_id.as_deref() == Some(sect.id.as_str()))
        {
            disc::settle_month(std::slice::from_mut(member), morale);
        }
    }

    // 4. 各派建筑、门人用度与门派令统一结算
    let alive_count = state.disciples.iter().filter(|d| d.alive).count();
    let (income, expense) = crate::logic::sect::apply_monthly_upkeep(&mut state.sect, alive_count);
    apply_building_wear(rng, &mut state.sect);
    events.push(GameEvent {
        text: format!("司库月结：进项{}两，用度{}两。", income, expense),
        mood: if income >= expense { "good" } else { "neutral" }.into(),
        year: state.year,
        month: state.month,
        category: "sect".into(),
    });
    for npc_sect in &mut state.npc_sects {
        let members = state
            .npc_disciples
            .iter()
            .filter(|d| d.alive && d.sect_id.as_deref() == Some(npc_sect.id.as_str()))
            .count();
        crate::logic::sect::apply_monthly_upkeep(npc_sect, members);
        apply_building_wear(rng, npc_sect);
    }
    crate::logic::sect::sync_legacy_fields(state);

    // 5. 志气自然浮动
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
            category: "sect".into(),
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
                category: "sect".into(),
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
    crate::logic::sect::normalize_elder_assignments(&mut state.sect, &state.disciples);
    for npc_sect in &mut state.npc_sects {
        crate::logic::sect::normalize_elder_assignments(npc_sect, &state.npc_disciples);
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
            category: "sect".into(),
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
            category: "sect".into(),
        });
    }

    // ★ 将本轮事件追加到 state.event_log（保留最近 50 条）
    for ev in &events {
        state.event_log.push(ev.clone());
    }
    trim_log(state);

    // 11. 重置决策
    state.decisions_used = 0;
    state.pending_event = None;

    (events, tournament_result, state.game_over)
}

/// 百草堂常设药炉后台循环炼制；无长老时以每两个月一月进度折算半速。
fn advance_auto_brew(state: &mut GameState) {
    use crate::models::medicine::{recipe_silver_cost, PILL_RECIPES};
    use crate::models::sect::BuildingKind;

    state.sect.auto_brew_index %= PILL_RECIPES.len();
    let recipe = PILL_RECIPES[state.sect.auto_brew_index];
    let herbs = state.sect.inventory.get("草药").copied().unwrap_or(0);
    if herbs < recipe.herb_cost {
        return;
    }

    let has_elder = state
        .sect
        .buildings
        .iter()
        .any(|building| building.kind == BuildingKind::HerbHall && building.elder_id.is_some());
    let elapsed_month = (state.year - 1) * 12 + state.month;
    if !has_elder && elapsed_month % 2 != 0 {
        return;
    }

    let next_progress = state.sect.auto_brew_progress + 1;
    if next_progress < recipe.months {
        state.sect.auto_brew_progress = next_progress;
        return;
    }

    let silver_cost = recipe_silver_cost(recipe);
    if state.sect.attributes.silver < silver_cost {
        return;
    }

    state.sect.attributes.silver -= silver_cost;
    *state.sect.inventory.entry("草药".into()).or_default() -= recipe.herb_cost;
    *state
        .sect
        .inventory
        .entry(recipe.medicine.name().into())
        .or_default() += recipe.quantity;
    state.sect.auto_brew_progress = 0;
    state.sect.auto_brew_index = (state.sect.auto_brew_index + 1) % PILL_RECIPES.len();
}

#[derive(Default)]
struct PayrollSummary {
    silver_due: i32,
    silver_paid: i32,
    rations_due: i32,
    rations_paid: i32,
}

fn rank_allowance(rank: &DiscipleRank) -> (i32, i32) {
    match rank {
        DiscipleRank::Chore => (2, 3),
        DiscipleRank::Outer => (5, 5),
        DiscipleRank::Inner => (10, 8),
    }
}

/// 先内门、再外门、后杂役；同一身份库存不足时等比例折发，余数按名册顺序补齐。
fn ranked_allocations(
    disciples: &[Disciple],
    eligible: &[usize],
    available: i32,
    allowance: impl Fn(&DiscipleRank) -> i32,
) -> Vec<(usize, i32)> {
    let mut left = available.max(0);
    let mut allocations = Vec::new();
    for rank in [
        DiscipleRank::Inner,
        DiscipleRank::Outer,
        DiscipleRank::Chore,
    ] {
        let members = eligible
            .iter()
            .copied()
            .filter(|index| disciples[*index].rank == rank)
            .collect::<Vec<_>>();
        if members.is_empty() || left <= 0 {
            continue;
        }
        let each_due = allowance(&rank);
        let group_due = each_due * members.len() as i32;
        if left >= group_due {
            allocations.extend(members.into_iter().map(|index| (index, each_due)));
            left -= group_due;
            continue;
        }
        let base = (left / members.len() as i32).min(each_due);
        let mut remainder = left - base * members.len() as i32;
        for index in members {
            let extra = i32::from(remainder > 0 && base < each_due);
            remainder -= extra;
            allocations.push((index, base + extra));
        }
        break;
    }
    allocations
}

fn pay_monthly_stipends(
    sect: &mut SectState,
    disciples: &mut [Disciple],
    sect_id: Option<&str>,
) -> PayrollSummary {
    let eligible = disciples
        .iter()
        .enumerate()
        .filter(|(_, disciple)| {
            disciple.alive && sect_id.is_none_or(|id| disciple.sect_id.as_deref() == Some(id))
        })
        .map(|(index, _)| index)
        .collect::<Vec<_>>();
    let silver_due = eligible
        .iter()
        .map(|index| rank_allowance(&disciples[*index].rank).0)
        .sum();
    let rations_due = eligible
        .iter()
        .map(|index| rank_allowance(&disciples[*index].rank).1)
        .sum();
    let silver_allocations =
        ranked_allocations(disciples, &eligible, sect.attributes.silver, |rank| {
            rank_allowance(rank).0
        });
    let ration_stock = sect.inventory.get("粮秣").copied().unwrap_or(0);
    let ration_allocations = ranked_allocations(disciples, &eligible, ration_stock, |rank| {
        rank_allowance(rank).1
    });
    let silver_paid = silver_allocations.iter().map(|(_, amount)| amount).sum();
    let rations_paid = ration_allocations.iter().map(|(_, amount)| amount).sum();
    for (index, amount) in silver_allocations {
        disciples[index].personal_silver += amount;
    }
    for (index, amount) in ration_allocations {
        disciples[index].personal_rations += amount;
    }
    sect.attributes.silver -= silver_paid;
    *sect.inventory.entry("粮秣".into()).or_default() -= rations_paid;
    PayrollSummary {
        silver_due,
        silver_paid,
        rations_due,
        rations_paid,
    }
}

fn claim_travel_rations(state: &mut GameState) -> Vec<GameEvent> {
    let mut events = Vec::new();
    for disciple in state.disciples.iter_mut().filter(|disciple| disciple.alive) {
        let Some(plan) = disciple.action.as_mut() else {
            continue;
        };
        if plan.remaining_months <= 0
            || plan.rations_claimed
            || disciple.away_months <= 0
            || !matches!(plan.kind, ActionKind::SectMission | ActionKind::Wander)
        {
            continue;
        }
        let requested = plan.remaining_months * 3;
        let stock = state.sect.inventory.get("粮秣").copied().unwrap_or(0);
        let issued = requested.min(stock.max(0));
        *state.sect.inventory.entry("粮秣".into()).or_default() -= issued;
        disciple.personal_rations += issued;
        plan.rations_claimed = true;
        events.push(GameEvent {
            text: format!(
                "{}外出前申领口粮{}份{}。",
                disciple.name,
                issued,
                if issued < requested {
                    "（库存不足）"
                } else {
                    ""
                }
            ),
            mood: if issued < requested {
                "neutral"
            } else {
                "good"
            }
            .into(),
            year: state.year,
            month: state.month,
            category: "sect".into(),
        });
    }
    events
}

fn treat_injured_disciples(state: &mut GameState) -> Vec<GameEvent> {
    use crate::models::medicine::Medicine;
    use crate::models::sect::BuildingKind;

    let staffed = state
        .sect
        .buildings
        .iter()
        .any(|building| building.kind == BuildingKind::HerbHall && building.elder_id.is_some());
    if !staffed {
        return Vec::new();
    }
    let medicine = Medicine::Wound.name();
    let mut treated = Vec::new();
    for disciple in state.disciples.iter_mut().filter(|disciple| {
        disciple.alive
            && disciple.away_months <= 0
            && disciple.condition != DiscipleCondition::Healthy
    }) {
        let stock = state.sect.inventory.get(medicine).copied().unwrap_or(0);
        if stock <= 0 {
            break;
        }
        *state.sect.inventory.entry(medicine.into()).or_default() -= 1;
        disciple.attributes.qi.current =
            (disciple.attributes.qi.current + 5).min(disciple.attributes.qi.maximum);
        disc::refresh_condition(disciple);
        treated.push(GameEvent {
            text: format!("百草堂为{}用去一份金疮药，伤势减轻5点。", disciple.name),
            mood: "good".into(),
            year: state.year,
            month: state.month,
            category: "sect".into(),
        });
    }
    treated
}

fn apply_building_wear(rng: &mut impl Rng, sect: &mut SectState) {
    for building in &mut sect.buildings {
        let mut wear = rng.gen_range(1..=3);
        if building.condition < 30 {
            wear += rng.gen_range(1..=3);
        }
        building.condition = (building.condition - wear).max(0);
    }
}

fn trim_log(state: &mut GameState) {
    if state.event_log.len() > 80 {
        let excess = state.event_log.len() - 80;
        state.event_log.drain(0..excess);
    }
}

fn interactive_category(category: &str) -> &'static str {
    match category {
        "江湖" | "朝廷" | "乡里" => "world",
        _ => "sect",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::logic::world;
    use rand::{rngs::StdRng, SeedableRng};

    #[test]
    fn auto_brew_with_elder_finishes_recipe_and_advances_index() {
        let mut state = GameState::default();
        state.sect.inventory.insert("草药".into(), 10);
        state
            .sect
            .buildings
            .iter_mut()
            .find(|building| building.id == "herb_hall")
            .unwrap()
            .elder_id = Some("elder".into());

        advance_auto_brew(&mut state);

        assert_eq!(state.sect.inventory["草药"], 6);
        assert_eq!(state.sect.inventory["金疮药"], 2);
        assert_eq!(state.sect.auto_brew_progress, 0);
        assert_eq!(state.sect.auto_brew_index, 1);
    }

    #[test]
    fn auto_brew_without_elder_runs_at_half_speed_and_waits_for_herbs() {
        let mut state = GameState::default();
        state.sect.auto_brew_index = 1;
        state.sect.inventory.insert("草药".into(), 5);

        for month in 1..=2 {
            state.month = month;
            advance_auto_brew(&mut state);
        }
        assert_eq!(state.sect.auto_brew_progress, 0, "草药不足时不应推进");

        state.sect.inventory.insert("草药".into(), 6);
        for month in 3..=6 {
            state.month = month;
            advance_auto_brew(&mut state);
        }
        assert_eq!(state.sect.inventory["草药"], 0);
        assert_eq!(state.sect.inventory["养气丹"], 1);
        assert_eq!(state.sect.auto_brew_progress, 0);
        assert_eq!(state.sect.auto_brew_index, 2);
    }

    #[test]
    fn resolving_pending_event_continues_the_paused_month() {
        let mut state = GameState {
            world_seed: 77,
            ..GameState::default()
        };
        let (sects, npc_disciples) = world::generate_npc_world(state.world_seed);
        state.npc_sects = sects;
        state.npc_disciples = npc_disciples;
        state.disciples.push(state.npc_disciples[0].clone());
        state.disciples[0].sect_id = Some("player".into());
        let pending = event::interactive_events().remove(0);
        let choice = pending.choices[0].id.clone();
        state.pending_event = Some(serde_json::to_value(pending).unwrap());
        let mut rng = StdRng::seed_from_u64(4);
        let result = resolve_pending_event(&mut rng, &mut state, &choice).unwrap();
        assert_eq!(state.month, 2);
        assert!(state.pending_event.is_none());
        assert!(!result.0.is_empty());
    }

    #[test]
    fn advancing_month_executes_selected_elder_duty() {
        let mut state = GameState::default();
        let mut rng = StdRng::seed_from_u64(31);
        let mut elder = disc::generate_disciple(&mut rng, 0);
        elder.rank = crate::models::attributes::DiscipleRank::Inner;
        let elder_id = elder.id.clone();
        state.disciples.push(elder);
        state.sect.buildings[0].elder_id = Some(elder_id);
        state.sect.buildings[0].selected_duty = Some("drill".into());

        let merit = state.disciples[0].merit;
        let (events, _, _) = finish_month(&mut rng, &mut state, vec![]);

        assert_eq!(state.disciples[0].merit, merit + 2);
        assert!(state.sect.buildings[0].elder_action_used);
        assert!(events.iter().any(|event| event.text.contains("主持月考")));
    }

    #[test]
    fn complete_playthrough_reaches_two_tournaments_in_24_months() {
        let mut rng = StdRng::seed_from_u64(20260716);
        let mut state = GameState {
            world_seed: 20260716,
            ..GameState::default()
        };
        let (sects, npc_disciples) = world::generate_npc_world(state.world_seed);
        state.npc_sects = sects;
        state.npc_disciples = npc_disciples;
        state.disciples = disc::generate_starting_disciples(&mut rng);

        let mut elapsed = 0;
        while elapsed < 24 {
            let before = (state.year, state.month);
            advance_month(&mut rng, &mut state);
            if let Some(value) = state.pending_event.clone() {
                let pending: event::PendingWorldEvent = serde_json::from_value(value).unwrap();
                resolve_pending_event(&mut rng, &mut state, &pending.choices[0].id).unwrap();
            }
            if (state.year, state.month) != before {
                elapsed += 1;
            }
            assert!(!state.game_over, "第{elapsed}个月意外散伙");
        }

        assert_eq!((state.year, state.month), (3, 1));
        assert_eq!(state.tournament_history.len(), 2);
        assert!(state
            .event_log
            .iter()
            .any(|event| event.text.contains("年终论剑")));
    }
}
