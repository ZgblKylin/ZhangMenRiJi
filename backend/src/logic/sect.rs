use crate::models::game::GameState;
use crate::models::sect::SectState;

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
    crate::logic::world::hydrate_world(state);
    sync_legacy_fields(state);
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
    let expense = (20 + disciples as i32 * 5 + sect.buildings.len() as i32 * 2)
        * (100 - frugal.clamp(0, 40))
        / 100;
    sect.attributes.silver = (sect.attributes.silver + income - expense).max(0);
    sect.attributes.morality =
        (sect.attributes.morality + order_bonus(sect, "morality") / 6).clamp(0, 100);

    for building in &mut sect.buildings {
        if building.upgrading_months > 0 {
            building.upgrading_months -= 1;
            if building.upgrading_months == 0 {
                building.level += 1;
                building.condition = 100;
            }
        } else {
            building.condition = (building.condition - 1).max(0);
        }
    }
    for order in &mut sect.active_orders {
        order.remaining_months -= 1;
    }
    sect.active_orders
        .retain(|order| order.remaining_months > 0);
    (income, expense)
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
}
