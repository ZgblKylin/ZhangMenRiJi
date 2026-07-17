use crate::models::attributes::DiscipleRank;
use crate::models::game::GameState;
use crate::models::medicine::{Medicine, LEGACY_WOUND_MEDICINE_NAME};
use crate::models::sect::{default_buildings, Building, SectState};
use crate::models::Disciple;
use std::collections::BTreeSet;

/// 载入 v2 存档时，以旧顶层字段补齐 v3 门派状态。
pub fn hydrate_player_sect(state: &mut GameState, sect_name: &str) {
    if state.sect.name == "无名派" || state.sect.name.is_empty() {
        state.sect.name = sect_name.to_owned();
        state.sect.attributes.prestige = state.prestige;
        state.sect.attributes.silver = state.silver;
        state.sect.attributes.morale = state.morale;
    }
    for disciple in &mut state.disciples {
        crate::logic::disciple::hydrate_v2_disciple(disciple);
    }
    normalize_buildings(&mut state.sect);
    normalize_inventory(&mut state.sect);
    normalize_elder_assignments(&mut state.sect, &state.disciples);
    if !state
        .sect
        .public_books
        .iter()
        .any(|id| id == "player_knowledge")
    {
        state.sect.public_books.insert(0, "player_knowledge".into());
    }
    if !state
        .martial_arts_learned
        .iter()
        .any(|id| id == "player_knowledge")
    {
        state
            .martial_arts_learned
            .insert(0, "player_knowledge".into());
    }
    crate::logic::world::hydrate_world(state);
    sync_legacy_fields(state);
}

/// 兼容早期存档中的同音药名，并同步修正在炼任务的产物名。
fn normalize_inventory(sect: &mut SectState) {
    if let Some(quantity) = sect.inventory.remove(LEGACY_WOUND_MEDICINE_NAME) {
        *sect
            .inventory
            .entry(Medicine::Wound.name().into())
            .or_default() += quantity;
    }
    for task in &mut sect.productions {
        if task.output_item == LEGACY_WOUND_MEDICINE_NAME {
            task.output_item = Medicine::Wound.name().into();
            task.name = Medicine::Wound.name().into();
        }
    }
    // 旧存档没有这两类物资时补零，不凭空赠送初始库存。
    sect.inventory.entry("粮秣".into()).or_default();
    sect.inventory.entry("精铁".into()).or_default();
}

/// 将早期 v3 的五库与药库布局迁移为七座职能建筑。
pub fn normalize_buildings(sect: &mut SectState) {
    let stored = std::mem::take(&mut sect.buildings);
    let mut normalized = default_buildings();
    for building in &mut normalized {
        let aliases: &[&str] = match building.id.as_str() {
            "warehouse" => &["warehouse", "treasury", "inner_store", "outer_store"],
            "herb_hall" => &["herb_hall", "pharmacy"],
            _ => &[building.id.as_str()],
        };
        let matches: Vec<&Building> = stored
            .iter()
            .filter(|old| aliases.contains(&old.id.as_str()))
            .collect();
        if matches.is_empty() {
            continue;
        }
        building.level = matches.iter().map(|old| old.level).max().unwrap_or(1);
        building.condition = matches.iter().map(|old| old.condition).min().unwrap_or(100);
        building.upgrading_months = matches
            .iter()
            .map(|old| old.upgrading_months)
            .max()
            .unwrap_or(0);
        building.work_required = matches
            .iter()
            .map(|old| old.work_required)
            .max()
            .unwrap_or(0);
        building.work_invested = matches
            .iter()
            .map(|old| old.work_invested)
            .max()
            .unwrap_or(0);
        if let Some(current) = matches.iter().find(|old| old.id == building.id) {
            building.elder_id = current.elder_id.clone();
            if current.selected_duty.is_some() {
                building.selected_duty = current.selected_duty.clone();
            }
            building.duty_target = current.duty_target.clone();
            building.elder_action_used = current.elder_action_used;
        }
    }
    sect.buildings = normalized;
}

pub fn normalize_elder_assignments(sect: &mut SectState, disciples: &[Disciple]) {
    let eligible: BTreeSet<&str> = disciples
        .iter()
        .filter(|disciple| disciple.alive && disciple.rank == DiscipleRank::Inner)
        .map(|disciple| disciple.id.as_str())
        .collect();
    let mut assigned = BTreeSet::new();
    for building in &mut sect.buildings {
        if building
            .elder_id
            .as_deref()
            .is_some_and(|id| !eligible.contains(id) || !assigned.insert(id.to_owned()))
        {
            building.elder_id = None;
            building.elder_action_used = false;
        }
    }
}

pub fn rank_limits(sect: &SectState, disciples: &[Disciple]) -> (usize, usize) {
    let alive = disciples.iter().filter(|disciple| disciple.alive).count();
    let outer_limit =
        ((alive as f32) * sect.rank_rules.outer_ratio.clamp(0.0, 1.0)).floor() as usize;
    let outer_count = disciples
        .iter()
        .filter(|disciple| disciple.alive && disciple.rank == DiscipleRank::Outer)
        .count();
    let inner_limit =
        ((outer_count as f32) * sect.rank_rules.inner_ratio.clamp(0.0, 1.0)).floor() as usize;
    (outer_limit, inner_limit)
}

/// 招贤榜所得新人按资质由高到低补入外门；外门名额用尽后仍从杂役起步。
pub fn assign_recruit_ranks(sect: &SectState, disciples: &[Disciple], recruits: &mut [Disciple]) {
    let projected_alive = disciples.iter().filter(|disciple| disciple.alive).count()
        + recruits.iter().filter(|disciple| disciple.alive).count();
    let outer_limit =
        ((projected_alive as f32) * sect.rank_rules.outer_ratio.clamp(0.0, 1.0)).floor() as usize;
    let current_outer = disciples
        .iter()
        .filter(|disciple| disciple.alive && disciple.rank == DiscipleRank::Outer)
        .count();
    let available = outer_limit.saturating_sub(current_outer);
    let mut order: Vec<usize> = (0..recruits.len()).collect();
    order.sort_by(|left, right| {
        recruits[*right]
            .talent
            .cmp(&recruits[*left].talent)
            .then_with(|| recruits[*left].name.cmp(&recruits[*right].name))
    });
    for (position, index) in order.into_iter().enumerate() {
        recruits[index].rank = if position < available {
            DiscipleRank::Outer
        } else {
            DiscipleRank::Chore
        };
    }
}

/// v3 以内嵌 SectState 为准，同时维护旧 API 字段以兼容已有前端。
pub fn sync_legacy_fields(state: &mut GameState) {
    state.prestige = state.sect.attributes.prestige.clamp(0, 1000);
    state.silver = state.sect.attributes.silver.max(0);
    state.morale = state.sect.attributes.morale.clamp(0, 100);
}

/// 兼容仍使用 v2 顶层字段的旧逻辑，并将其变更汇入 v3 门派状态。
pub fn absorb_legacy_fields(state: &mut GameState) {
    state.sect.attributes.prestige = state.prestige.clamp(0, 1000);
    state.sect.attributes.silver = state.silver.max(0);
    state.sect.attributes.morale = state.morale.clamp(0, 100);
}

pub fn policy_bonus(sect: &SectState, key: &str) -> i32 {
    use crate::models::sect::SectPolicy;
    match (&sect.policy, key) {
        (SectPolicy::Martial, "martial") => 20,
        (SectPolicy::Scholarly, "study") => 20,
        (SectPolicy::Chivalrous, "morality") => 15,
        (SectPolicy::Mercantile, "income") => 20,
        (SectPolicy::Reclusive, "recovery") => 15,
        (SectPolicy::Balanced, _) => 5,
        _ => 0,
    }
}

pub fn order_bonus(sect: &SectState, key: &str) -> i32 {
    sect.active_orders
        .iter()
        .filter(|order| order.remaining_months > 0)
        .filter_map(|order| order.effect.get(key))
        .sum()
}

pub fn apply_monthly_upkeep(sect: &mut SectState, disciples: usize) -> (i32, i32) {
    let prosperity = sect.attributes.prestige / 10;
    let treasury = building_level(sect, "treasury");
    let base_income = prosperity + disciples as i32 * 3 + treasury * 4;
    let income_bonus = policy_bonus(sect, "income") + order_bonus(sect, "income");
    let income = base_income * (100 + income_bonus) / 100;
    let frugal = order_bonus(sect, "income") / 2;
    // 弟子月俸已按身份直接发到个人名下；此处只保留山门杂项与建筑用度。
    let expense = (20 + sect.buildings.len() as i32 * 2) * (100 - frugal.clamp(0, 40)) / 100;
    sect.attributes.silver = (sect.attributes.silver + income - expense).max(0);
    sect.attributes.morality =
        (sect.attributes.morality + order_bonus(sect, "morality") / 6).clamp(0, 100);
    match sect.moral_direction {
        crate::models::sect::MoralDirection::Righteous => {
            sect.attributes.morality = (sect.attributes.morality + 1).min(100);
            sect.attributes.prestige = (sect.attributes.prestige + 1).min(1000);
        }
        crate::models::sect::MoralDirection::Neutral => {
            sect.attributes.morale = (sect.attributes.morale + 1).min(100);
        }
        crate::models::sect::MoralDirection::Villainous => {
            sect.attributes.morality = (sect.attributes.morality - 2).max(0);
            sect.attributes.prestige = (sect.attributes.prestige + 1).min(1000);
            sect.attributes.silver += 10;
        }
    }

    for building in &mut sect.buildings {
        if building.work_required > 0 {
            let remaining = (building.work_required - building.work_invested).max(0);
            building.upgrading_months = (remaining + 9) / 10;
            if remaining == 0 {
                building.level += 1;
                building.condition = 100;
                building.work_required = 0;
                building.work_invested = 0;
                building.upgrading_months = 0;
            }
        }
    }
    for order in &mut sect.active_orders {
        order.remaining_months -= 1;
    }
    sect.active_orders
        .retain(|order| order.remaining_months > 0);
    settle_productions(sect);
    (income, expense)
}

fn settle_productions(sect: &mut SectState) {
    for task in &mut sect.productions {
        task.remaining_months -= 1;
    }
    let completed: Vec<(String, i32)> = sect
        .productions
        .iter()
        .filter(|task| task.remaining_months <= 0)
        .map(|task| (task.output_item.clone(), task.quantity))
        .collect();
    sect.productions.retain(|task| task.remaining_months > 0);
    for (item, quantity) in completed {
        *sect.inventory.entry(item).or_default() += quantity;
    }
}

pub fn building_level(sect: &SectState, id: &str) -> i32 {
    sect.buildings
        .iter()
        .find(|b| b.id == id)
        .map(|b| b.level)
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::attributes::DiscipleRank;

    #[test]
    fn recruit_rank_assignment_prefers_higher_talent_within_outer_limit() {
        let mut sect = SectState::default();
        sect.rank_rules.outer_ratio = 0.5;
        let mut existing = vec![Disciple::default(), Disciple::default()];
        existing[0].rank = DiscipleRank::Outer;
        existing[1].rank = DiscipleRank::Chore;
        let mut recruits = vec![Disciple::default(), Disciple::default()];
        recruits[0].name = "乙".into();
        recruits[0].talent = 18;
        recruits[1].name = "甲".into();
        recruits[1].talent = 30;

        assign_recruit_ranks(&sect, &existing, &mut recruits);

        assert_eq!(recruits[0].rank, DiscipleRank::Chore);
        assert_eq!(recruits[1].rank, DiscipleRank::Outer);
    }

    #[test]
    fn legacy_wound_medicine_name_is_migrated_in_stock_and_production() {
        let mut sect = SectState::default();
        sect.inventory.insert(LEGACY_WOUND_MEDICINE_NAME.into(), 3);
        sect.productions.push(crate::models::sect::ProductionTask {
            name: LEGACY_WOUND_MEDICINE_NAME.into(),
            output_item: LEGACY_WOUND_MEDICINE_NAME.into(),
            ..crate::models::sect::ProductionTask::default()
        });

        normalize_inventory(&mut sect);

        assert!(!sect.inventory.contains_key(LEGACY_WOUND_MEDICINE_NAME));
        assert_eq!(sect.inventory[Medicine::Wound.name()], 3);
        assert_eq!(sect.productions[0].name, Medicine::Wound.name());
        assert_eq!(sect.productions[0].output_item, Medicine::Wound.name());
    }
    use crate::models::sect::Building;

    #[test]
    fn upkeep_is_aggregated_once() {
        let mut sect = SectState::default();
        let before = sect.attributes.silver;
        let (income, expense) = apply_monthly_upkeep(&mut sect, 2);
        assert_eq!(sect.attributes.silver, before + income - expense);
        assert!(income > 0 && expense > 0);
    }

    #[test]
    fn legacy_and_v3_sect_attributes_stay_in_step() {
        let mut state = GameState {
            prestige: 88,
            silver: 321,
            morale: 67,
            ..GameState::default()
        };
        absorb_legacy_fields(&mut state);
        assert_eq!(state.sect.attributes.prestige, 88);
        assert_eq!(state.sect.attributes.silver, 321);
        assert_eq!(state.sect.attributes.morale, 67);

        state.sect.attributes.silver += 10;
        sync_legacy_fields(&mut state);
        assert_eq!(state.silver, 331);
    }

    #[test]
    fn legacy_stores_migrate_to_seven_functional_buildings() {
        let mut sect = SectState::default();
        sect.buildings = vec![
            Building {
                id: "treasury".into(),
                name: "银库".into(),
                level: 3,
                ..Building::default()
            },
            Building {
                id: "pharmacy".into(),
                name: "药库".into(),
                level: 2,
                ..Building::default()
            },
        ];
        normalize_buildings(&mut sect);
        assert_eq!(sect.buildings.len(), 7);
        assert_eq!(building_level(&sect, "warehouse"), 3);
        assert_eq!(building_level(&sect, "herb_hall"), 2);
        let titles: BTreeSet<&str> = sect
            .buildings
            .iter()
            .map(|building| building.elder_title.as_str())
            .collect();
        assert_eq!(titles.len(), 7);
    }

    #[test]
    fn rank_limits_do_not_mutate_grandfathered_disciples() {
        let sect = SectState::default();
        let mut disciples = vec![Disciple::default(); 10];
        for (index, disciple) in disciples.iter_mut().enumerate() {
            disciple.id = index.to_string();
            disciple.rank = if index < 6 {
                DiscipleRank::Outer
            } else {
                DiscipleRank::Inner
            };
        }
        assert_eq!(rank_limits(&sect, &disciples), (4, 1));
        assert_eq!(
            disciples
                .iter()
                .filter(|disciple| disciple.rank == DiscipleRank::Inner)
                .count(),
            4
        );
    }

    #[test]
    fn moral_directions_have_distinct_monthly_effects() {
        let mut righteous = SectState::default();
        righteous.attributes.morality = 50;
        let mut neutral = righteous.clone();
        neutral.moral_direction = crate::models::sect::MoralDirection::Neutral;
        let mut villainous = righteous.clone();
        villainous.moral_direction = crate::models::sect::MoralDirection::Villainous;
        apply_monthly_upkeep(&mut righteous, 0);
        apply_monthly_upkeep(&mut neutral, 0);
        apply_monthly_upkeep(&mut villainous, 0);
        assert!(righteous.attributes.morality > neutral.attributes.morality);
        assert!(villainous.attributes.morality < neutral.attributes.morality);
        assert!(villainous.attributes.silver > neutral.attributes.silver);
    }
}
