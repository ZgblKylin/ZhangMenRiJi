use crate::logic::disciple;
use crate::models::attributes::{Department, DiscipleRank};
use crate::models::game::GameState;
use crate::models::martial_art::martial_art_by_id_with_created;
use crate::models::medicine::{Medicine, LEGACY_WOUND_MEDICINE_NAME};
use crate::models::sect::{sect_buildings, Building, BuildingKind, SectState};
use crate::models::Disciple;
use std::collections::{BTreeMap, BTreeSet, HashMap};

/// 载入 v2 存档时，以旧顶层字段补齐 v3 门派状态。
pub fn hydrate_player_sect(state: &mut GameState, sect_name: &str) {
    if state.sect.name == "无名派" || state.sect.name.is_empty() {
        state.sect.name = sect_name.to_owned();
        state.sect.attributes.prestige = state.prestige;
        state.sect.attributes.silver = state.silver;
        state.sect.attributes.morale = state.morale;
    }
    let created_martial_arts = state.sect.created_martial_arts.clone();
    for disciple in &mut state.disciples {
        crate::logic::disciple::hydrate_v2_disciple_with_created(disciple, &created_martial_arts);
        // 玩家名册本身就是权威归属。早期存档与开局生成的人物没有
        // `sect_id`，若不在这里补齐，长老有效性、师承与自动药炉会把
        // 同一名册中的旧门人误判为外派人物。
        disciple.sect_id = Some(state.sect.id.clone());
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
    normalize_master_assignments(&mut state.disciples);
    normalize_master_assignments(&mut state.npc_disciples);
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
    let mut normalized = sect_buildings(&sect.id);
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

/// 计算一名弟子主持指定堂口时的能力修正。
///
/// 一般人选保持在旧有的百分之百附近；部门经历、相关有效天赋、知识或
/// 内功修为及功绩只作温和修正，避免长老人选盖过建筑等级与完好度本身。
pub fn elder_competence_percent(disciple: &Disciple, kind: BuildingKind) -> i32 {
    let expected_department = match &kind {
        BuildingKind::Practice => Department::Transmission,
        BuildingKind::Scripture => Department::Library,
        BuildingKind::Warehouse => Department::Treasury,
        BuildingKind::HerbHall => Department::Apothecary,
        BuildingKind::Intelligence => Department::ExternalAffairs,
        BuildingKind::Affairs | BuildingKind::Logistics => Department::Stewardship,
    };
    let department_adjustment = match disciple.department.as_ref() {
        Some(department) if department == &expected_department => 8,
        Some(_) => -4,
        None => 0,
    };

    let effective = disciple::effective_aptitudes(disciple);
    let (relevant_aptitude, expertise) = match kind {
        BuildingKind::Practice => (
            (effective.strength + effective.constitution + effective.agility) / 3,
            disciple::effective_force_level(disciple),
        ),
        BuildingKind::Scripture => (effective.intelligence, disciple::knowledge_level(disciple)),
        BuildingKind::Warehouse => (
            (effective.intelligence + effective.fortune) / 2,
            disciple::knowledge_level(disciple),
        ),
        BuildingKind::HerbHall => (
            (effective.intelligence + effective.constitution) / 2,
            disciple::knowledge_level(disciple),
        ),
        BuildingKind::Intelligence => (
            (effective.intelligence + effective.agility + effective.fortune) / 3,
            disciple::knowledge_level(disciple),
        ),
        BuildingKind::Affairs => (
            (effective.intelligence + effective.fortune) / 2,
            disciple::knowledge_level(disciple),
        ),
        BuildingKind::Logistics => (
            (effective.strength + effective.constitution) / 2,
            disciple::effective_force_level(disciple),
        ),
    };
    let aptitude_adjustment = ((relevant_aptitude - 20) / 4).clamp(-5, 8);
    // 二十级视作能胜任日常堂务的基础，不因旧存档缺少技能明细而倒扣。
    let expertise_adjustment = ((expertise.max(20) - 20) / 10).clamp(0, 8);
    let merit_adjustment = (disciple.merit.clamp(-150, 150) / 25) as i32;

    (100 + department_adjustment + aptitude_adjustment + expertise_adjustment + merit_adjustment)
        .clamp(85, 130)
}

/// 清理失效师承，并以名册顺序保留每位师父最早的五名在籍弟子。
///
/// 人物离派会从对应名册中移除；因此不存在于同一名册、跨门派、亡故、
/// 不再具备内门身份的师父都会自然失去授业资格。亡故或降为杂役的学生
/// 也不再占用师门名额。
pub fn normalize_master_assignments(disciples: &mut [Disciple]) {
    let status = disciples
        .iter()
        .map(|disciple| {
            (
                disciple.id.clone(),
                (
                    disciple.alive,
                    disciple.rank.clone(),
                    disciple.sect_id.clone(),
                    disciple.master_id.clone(),
                ),
            )
        })
        .collect::<BTreeMap<_, _>>();
    let mut apprentice_counts: BTreeMap<String, usize> = BTreeMap::new();

    for student in disciples {
        let valid = student.alive
            && student.rank != DiscipleRank::Chore
            && student.master_id.as_deref().is_some_and(|master_id| {
                master_id != student.id
                    && master_chain_is_acyclic(&status, &student.id, master_id)
                    && status
                        .get(master_id)
                        .is_some_and(|(alive, rank, sect_id, _)| {
                            *alive
                                && *rank == DiscipleRank::Inner
                                && *sect_id == student.sect_id
                                && apprentice_counts.get(master_id).copied().unwrap_or(0) < 5
                        })
            });
        if valid {
            let master_id = student.master_id.as_ref().expect("师承已验证");
            *apprentice_counts.entry(master_id.clone()).or_default() += 1;
        } else {
            student.master_id = None;
        }
    }
}

fn master_chain_is_acyclic(
    status: &BTreeMap<String, (bool, DiscipleRank, Option<String>, Option<String>)>,
    student_id: &str,
    master_id: &str,
) -> bool {
    let mut cursor = Some(master_id.to_owned());
    let mut visited = BTreeSet::new();
    while let Some(id) = cursor {
        if id == student_id || !visited.insert(id.clone()) {
            return false;
        }
        cursor = status
            .get(&id)
            .and_then(|(_, _, _, next_master)| next_master.clone());
    }
    true
}

/// 判断一项新任命是否会令学生成为自己的师祖，或接入已有闭环。
pub fn master_assignment_creates_cycle(
    disciples: &[Disciple],
    student_id: &str,
    master_id: &str,
) -> bool {
    let status = disciples
        .iter()
        .map(|disciple| {
            (
                disciple.id.clone(),
                (
                    disciple.alive,
                    disciple.rank.clone(),
                    disciple.sect_id.clone(),
                    disciple.master_id.clone(),
                ),
            )
        })
        .collect::<BTreeMap<_, _>>();
    !master_chain_is_acyclic(&status, student_id, master_id)
}

/// 师父可向正式弟子授业：已有明确师承，或至少一方将另一方视为亲近同门。
pub fn teaching_relationship_eligible(teacher: &Disciple, student: &Disciple) -> bool {
    teacher.id != student.id
        && teacher.alive
        && student.alive
        && teacher.sect_id == student.sect_id
        && (student.master_id.as_deref() == Some(teacher.id.as_str())
            || teacher.relations.get(&student.id).copied().unwrap_or(0) >= 35
            || student.relations.get(&teacher.id).copied().unwrap_or(0) >= 35)
}

pub fn rank_limits(sect: &SectState, disciples: &[Disciple]) -> (usize, usize) {
    let alive = disciples.iter().filter(|disciple| disciple.alive).count();
    let outer_limit =
        ((alive as f32) * sect.rank_rules.outer_ratio.clamp(0.0, 1.0)).floor() as usize;
    let outer_count = disciples
        .iter()
        .filter(|disciple| disciple.alive && disciple.rank == DiscipleRank::Outer)
        .count();
    let proportional_inner_limit =
        ((outer_count as f32) * sect.rank_rules.inner_ratio.clamp(0.0, 1.0)).floor() as usize;
    // 新立山门只得两三名外门时也须能擢任首名内门，否则研读、传授、长老与外交会整串锁死。
    let inner_limit = if outer_count > 0 {
        proportional_inner_limit.max(1)
    } else {
        0
    };
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

/// 藏经阁未曾专门参研的战斗武学默认可修至五十级；参研值本身即门派可授上限。
/// 知识类不受此限，已有旧人物的更高修为也不会被倒扣。
pub fn martial_research_level_cap(sect: &SectState, art_id: &str) -> Option<i32> {
    let art = martial_art_by_id_with_created(art_id, &sect.created_martial_arts)?;
    art.is_combat.then(|| {
        sect.martial_research
            .get(&art.id)
            .copied()
            .unwrap_or(0)
            .clamp(50, i64::from(i32::MAX)) as i32
    })
}

/// 一级且完好时为 100%；每升一级增益 20%，再按完好度折算。
/// 完好度归零即暂停该堂职能，迫使扩建、修缮与日常经营真正相互牵动。
pub fn building_effectiveness(sect: &SectState, building_id: &str) -> i32 {
    sect.buildings
        .iter()
        .find(|building| building.id == building_id)
        .map(|building| {
            let level_factor = 80_i32.saturating_add(building.level.max(1).saturating_mul(20));
            level_factor.saturating_mul(building.condition.clamp(0, 100)) / 100
        })
        .unwrap_or(0)
}

#[allow(dead_code)]
pub fn apply_monthly_upkeep(sect: &mut SectState, disciples: usize) -> (i32, i32) {
    apply_monthly_upkeep_with_market(sect, disciples, 100)
}

pub fn apply_monthly_upkeep_with_market(
    sect: &mut SectState,
    disciples: usize,
    market_percent: i32,
) -> (i32, i32) {
    let prosperity = sect.attributes.prestige / 10;
    let (warehouse_level, warehouse_condition) = sect
        .buildings
        .iter()
        .find(|building| building.id == "warehouse")
        .map(|building| (building.level.max(0), building.condition.clamp(0, 100)))
        .unwrap_or((0, 0));
    // 仓库每重可贡献四两基础进项，破损时按完好度折算，避免升级只停留在卷宗数字上。
    let warehouse_income = warehouse_level
        .saturating_mul(4)
        .saturating_mul(warehouse_condition)
        / 100;
    let base_income = prosperity + disciples as i32 * 3 + warehouse_income;
    let income_bonus = policy_bonus(sect, "income") + order_bonus(sect, "income");
    let policy_income = base_income * (100 + income_bonus) / 100;
    // 市况只在被动进项的最终结果上折算一次；用度与恶名方略的额外所得
    // 各循原规则，避免繁荣同时放大收入链上的多个环节。
    let income = crate::logic::country::scale_positive(policy_income, market_percent);
    let frugal = order_bonus(sect, "income") / 2;
    // 弟子月俸已按身份直接发到个人名下；此处只保留山门杂项与建筑用度。
    // 七堂每两重额外规模、每百点累计破损各添一两用度，影响温和且只在此统一结算一次。
    let building_count = sect.buildings.len() as i32;
    let total_levels = sect.buildings.iter().fold(0_i32, |total, building| {
        total.saturating_add(building.level.max(0))
    });
    let total_condition_deficit = sect.buildings.iter().fold(0_i32, |total, building| {
        total.saturating_add(100 - building.condition.clamp(0, 100))
    });
    let scale_expense = total_levels.saturating_sub(building_count) / 2;
    let wear_expense = total_condition_deficit / 100;
    let gross_expense = 20_i32
        .saturating_add(building_count.saturating_mul(2))
        .saturating_add(scale_expense)
        .saturating_add(wear_expense);
    let expense = gross_expense.saturating_mul(100 - frugal.clamp(0, 40)) / 100;
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
            sect.attributes.prestige = (sect.attributes.prestige - 1).max(0);
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
    use crate::models::attributes::{Aptitudes, DiscipleRank, SkillProgress};

    #[test]
    fn hydration_marks_every_player_roster_member_as_belonging_to_the_player_sect() {
        let mut state = GameState::default();
        state.disciples = vec![
            Disciple {
                id: "legacy-none".into(),
                sect_id: None,
                ..Disciple::default()
            },
            Disciple {
                id: "stale-foreign".into(),
                sect_id: Some("wudang".into()),
                ..Disciple::default()
            },
        ];

        hydrate_player_sect(&mut state, "归卷门");

        assert!(state
            .disciples
            .iter()
            .all(|disciple| disciple.sect_id.as_deref() == Some(state.sect.id.as_str())));
    }

    #[test]
    fn elder_competence_distinguishes_fit_and_preserves_the_old_baseline() {
        let baseline = Disciple::default();
        for kind in [
            BuildingKind::Practice,
            BuildingKind::Scripture,
            BuildingKind::Warehouse,
            BuildingKind::HerbHall,
            BuildingKind::Intelligence,
            BuildingKind::Affairs,
            BuildingKind::Logistics,
        ] {
            assert_eq!(elder_competence_percent(&baseline, kind), 100);
        }

        let mut high = Disciple {
            department: Some(Department::Transmission),
            aptitudes: Aptitudes {
                strength: 60,
                intelligence: 60,
                constitution: 60,
                agility: 60,
                fortune: 60,
            },
            merit: 500,
            ..Disciple::default()
        };
        high.martial_progress
            .proficiencies
            .insert("basic_force".into(), SkillProgress::new(100, 0));
        high.prepared_skills
            .insert("basic_force".into(), "basic_force".into());

        let mut low = Disciple {
            department: Some(Department::Library),
            aptitudes: Aptitudes {
                strength: 0,
                intelligence: 0,
                constitution: 0,
                agility: 0,
                fortune: 0,
            },
            merit: -500,
            ..Disciple::default()
        };
        low.martial_progress.proficiencies.clear();

        assert_eq!(elder_competence_percent(&high, BuildingKind::Practice), 130);
        assert_eq!(elder_competence_percent(&low, BuildingKind::Practice), 85);
    }

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
    fn warehouse_level_increases_monthly_income() {
        let mut level_one = SectState::default();
        level_one.attributes.prestige = 0;
        level_one.policy = crate::models::sect::SectPolicy::Reclusive;
        let mut level_three = level_one.clone();
        level_three
            .buildings
            .iter_mut()
            .find(|building| building.id == "warehouse")
            .unwrap()
            .level = 3;

        let (level_one_income, _) = apply_monthly_upkeep(&mut level_one, 0);
        let (level_three_income, _) = apply_monthly_upkeep(&mut level_three, 0);

        assert_eq!(level_one_income, 4);
        assert_eq!(level_three_income, 12);
    }

    #[test]
    fn damaged_warehouse_reduces_monthly_income() {
        let mut healthy = SectState::default();
        healthy.attributes.prestige = 0;
        healthy.policy = crate::models::sect::SectPolicy::Reclusive;
        let warehouse = healthy
            .buildings
            .iter_mut()
            .find(|building| building.id == "warehouse")
            .unwrap();
        warehouse.level = 3;
        warehouse.condition = 100;
        let mut damaged = healthy.clone();
        damaged
            .buildings
            .iter_mut()
            .find(|building| building.id == "warehouse")
            .unwrap()
            .condition = 25;

        let (healthy_income, _) = apply_monthly_upkeep(&mut healthy, 0);
        let (damaged_income, _) = apply_monthly_upkeep(&mut damaged, 0);

        assert_eq!(healthy_income, 12);
        assert_eq!(damaged_income, 3);
    }

    #[test]
    fn upkeep_is_aggregated_once_and_never_negative() {
        let mut sect = SectState::default();
        sect.attributes.prestige = 0;
        sect.attributes.silver = 100;
        sect.policy = crate::models::sect::SectPolicy::Reclusive;

        let (income, expense) = apply_monthly_upkeep(&mut sect, 0);

        assert_eq!(income, 4);
        assert_eq!(expense, 34);
        assert_eq!(sect.attributes.silver, 70);
        assert!(expense >= 0);

        let mut impoverished = SectState::default();
        impoverished.attributes.prestige = 0;
        impoverished.attributes.silver = 0;
        impoverished.policy = crate::models::sect::SectPolicy::Reclusive;
        let (_, impoverished_expense) = apply_monthly_upkeep(&mut impoverished, 0);
        assert_eq!(impoverished_expense, 34);
        assert_eq!(impoverished.attributes.silver, 0);
    }

    #[test]
    fn seven_hall_scale_and_wear_have_a_mild_exact_upkeep_cost() {
        let mut sect = SectState::default();
        sect.policy = crate::models::sect::SectPolicy::Reclusive;
        for building in &mut sect.buildings {
            building.level = 3;
            building.condition = 50;
        }

        let (_, expense) = apply_monthly_upkeep(&mut sect, 0);

        // 基础 34 + 十四重额外规模 / 2 + 三百五十点累计破损 / 100。
        assert_eq!(expense, 44);
    }

    #[test]
    fn country_market_scales_only_passive_income_not_expense() {
        let mut low = SectState::default();
        low.attributes.prestige = 0;
        low.attributes.silver = 100;
        low.policy = crate::models::sect::SectPolicy::Reclusive;
        low.buildings
            .iter_mut()
            .find(|building| building.id == "warehouse")
            .unwrap()
            .level = 3;
        let mut neutral = low.clone();
        let mut high = low.clone();

        let (low_income, low_expense) = apply_monthly_upkeep_with_market(&mut low, 0, 85);
        let (neutral_income, neutral_expense) =
            apply_monthly_upkeep_with_market(&mut neutral, 0, 100);
        let (high_income, high_expense) = apply_monthly_upkeep_with_market(&mut high, 0, 115);

        assert_eq!((low_income, neutral_income, high_income), (10, 12, 13));
        assert_eq!(low_expense, neutral_expense);
        assert_eq!(neutral_expense, high_expense);
        assert_eq!(low.attributes.silver, 100 + low_income - low_expense);
        assert_eq!(high.attributes.silver, 100 + high_income - high_expense);
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
    fn building_migration_keeps_sect_specific_titles() {
        let mut sect = SectState {
            id: "shaolin".into(),
            ..SectState::default()
        };

        normalize_buildings(&mut sect);

        assert_eq!(sect.buildings[0].elder_title, "罗汉堂首座");
        assert_eq!(sect.buildings[6].elder_title, "都寺");
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
    fn a_small_new_sect_always_has_one_inner_disciple_slot() {
        let sect = SectState::default();
        let mut disciples = vec![Disciple::default(), Disciple::default()];
        for (index, disciple) in disciples.iter_mut().enumerate() {
            disciple.id = index.to_string();
            disciple.rank = DiscipleRank::Outer;
        }

        assert_eq!(rank_limits(&sect, &disciples), (0, 1));
    }

    #[test]
    fn invalid_and_cyclic_master_assignments_are_normalized() {
        let mut valid_master = Disciple {
            id: "valid-master".into(),
            rank: DiscipleRank::Inner,
            ..Disciple::default()
        };
        valid_master.master_id = Some("cycle-student".into());
        let cycle_student = Disciple {
            id: "cycle-student".into(),
            rank: DiscipleRank::Inner,
            master_id: Some("valid-master".into()),
            ..Disciple::default()
        };
        let dead_master = Disciple {
            id: "dead-master".into(),
            rank: DiscipleRank::Inner,
            alive: false,
            ..Disciple::default()
        };
        let dead_master_student = Disciple {
            id: "dead-master-student".into(),
            master_id: Some("dead-master".into()),
            ..Disciple::default()
        };
        let foreign_master = Disciple {
            id: "foreign-master".into(),
            sect_id: Some("wudang".into()),
            rank: DiscipleRank::Inner,
            ..Disciple::default()
        };
        let foreign_student = Disciple {
            id: "foreign-student".into(),
            master_id: Some("foreign-master".into()),
            ..Disciple::default()
        };
        let chore_student = Disciple {
            id: "chore-student".into(),
            rank: DiscipleRank::Chore,
            master_id: Some("valid-master".into()),
            ..Disciple::default()
        };
        let mut disciples = vec![
            valid_master,
            cycle_student,
            dead_master,
            dead_master_student,
            foreign_master,
            foreign_student,
            chore_student,
        ];

        normalize_master_assignments(&mut disciples);

        assert!(disciples
            .iter()
            .filter(|disciple| {
                matches!(
                    disciple.id.as_str(),
                    "valid-master"
                        | "cycle-student"
                        | "dead-master-student"
                        | "foreign-student"
                        | "chore-student"
                )
            })
            .all(|disciple| disciple.master_id.is_none()));
    }

    #[test]
    fn master_normalization_keeps_at_most_five_active_apprentices() {
        let master = Disciple {
            id: "master".into(),
            rank: DiscipleRank::Inner,
            ..Disciple::default()
        };
        let mut disciples = vec![master];
        disciples.extend((0..6).map(|index| Disciple {
            id: format!("student-{index}"),
            master_id: Some("master".into()),
            ..Disciple::default()
        }));

        normalize_master_assignments(&mut disciples);

        assert_eq!(
            disciples
                .iter()
                .filter(|disciple| disciple.master_id.as_deref() == Some("master"))
                .count(),
            5
        );
        assert!(disciples.last().unwrap().master_id.is_none());
    }

    #[test]
    fn martial_research_is_a_real_combat_skill_level_cap() {
        let mut sect = SectState::default();
        assert_eq!(martial_research_level_cap(&sect, "hunyuan"), Some(50));
        assert_eq!(martial_research_level_cap(&sect, "player_knowledge"), None);

        sect.martial_research.insert("hunyuan".into(), 86);
        assert_eq!(martial_research_level_cap(&sect, "hunyuan"), Some(86));
    }

    #[test]
    fn building_levels_help_while_damage_reduces_or_stops_the_function() {
        let mut sect = SectState::default();
        assert_eq!(building_effectiveness(&sect, "practice"), 100);

        {
            let practice = sect
                .buildings
                .iter_mut()
                .find(|building| building.id == "practice")
                .unwrap();
            practice.level = 3;
            practice.condition = 50;
        }
        assert_eq!(building_effectiveness(&sect, "practice"), 70);

        sect.buildings
            .iter_mut()
            .find(|building| building.id == "practice")
            .unwrap()
            .condition = 0;
        assert_eq!(building_effectiveness(&sect, "practice"), 0);
    }

    #[test]
    fn moral_directions_have_distinct_monthly_effects() {
        let mut righteous = SectState::default();
        righteous.attributes.morality = 50;
        righteous.attributes.prestige = 100;
        let mut neutral = righteous.clone();
        neutral.moral_direction = crate::models::sect::MoralDirection::Neutral;
        let mut villainous = righteous.clone();
        villainous.moral_direction = crate::models::sect::MoralDirection::Villainous;
        apply_monthly_upkeep(&mut righteous, 0);
        apply_monthly_upkeep(&mut neutral, 0);
        apply_monthly_upkeep(&mut villainous, 0);
        assert_eq!(righteous.attributes.morality, 51);
        assert_eq!(righteous.attributes.prestige, 101);
        assert_eq!(neutral.attributes.morality, 50);
        assert_eq!(neutral.attributes.prestige, 100);
        assert_eq!(villainous.attributes.morality, 48);
        assert_eq!(villainous.attributes.prestige, 99);
        assert_eq!(villainous.attributes.silver, neutral.attributes.silver + 10);
    }

    #[test]
    fn deep_lineage_chain_has_expected_generations() {
        let depth = 1_000;
        let mut disciples = Vec::with_capacity(depth);
        for index in (0..depth).rev() {
            let master_id = if index == 0 {
                None
            } else {
                Some(format!("disciple-{}", index - 1))
            };
            disciples.push(Disciple {
                id: format!("disciple-{index}"),
                master_id,
                ..Disciple::default()
            });
        }

        compute_lineage_generations(&mut disciples);

        for (position, disciple) in disciples.iter().enumerate() {
            assert_eq!(disciple.lineage_generation, (depth - position - 1) as i32);
        }
    }

    #[test]
    fn invalid_lineage_paths_are_zeroed_but_valid_paths_are_preserved() {
        let make_disciple = |id: &str, master_id: Option<&str>| Disciple {
            id: id.into(),
            master_id: master_id.map(str::to_owned),
            ..Disciple::default()
        };
        let mut disciples = vec![
            make_disciple("root", None),
            make_disciple("valid-child", Some("root")),
            make_disciple("missing-master", Some("does-not-exist")),
            make_disciple("missing-descendant", Some("missing-master")),
            make_disciple("self-cycle", Some("self-cycle")),
            make_disciple("cycle-a", Some("cycle-b")),
            make_disciple("cycle-b", Some("cycle-a")),
            make_disciple("cycle-descendant", Some("cycle-a")),
        ];

        compute_lineage_generations(&mut disciples);

        let generation = |id: &str| {
            disciples
                .iter()
                .find(|disciple| disciple.id == id)
                .map(|disciple| disciple.lineage_generation)
        };
        assert_eq!(generation("root"), Some(0));
        assert_eq!(generation("valid-child"), Some(1));
        for id in [
            "missing-master",
            "missing-descendant",
            "self-cycle",
            "cycle-a",
            "cycle-b",
            "cycle-descendant",
        ] {
            assert_eq!(generation(id), Some(0), "{id} should be zeroed");
        }
    }
}

/// 从弟子名录中计算掌门的辈分链深度。
/// 无师承的直系同门为初代（generation 0），逐代递增。
pub fn compute_lineage_generations(disciples: &mut [Disciple]) {
    let mut index_by_id = HashMap::with_capacity(disciples.len());
    for (index, disciple) in disciples.iter().enumerate() {
        // Preserve the old linear `find` behavior when malformed data contains
        // duplicate IDs: the first matching disciple remains authoritative.
        index_by_id.entry(disciple.id.clone()).or_insert(index);
    }
    let has_master = disciples
        .iter()
        .map(|disciple| disciple.master_id.is_some())
        .collect::<Vec<_>>();
    let master_indices = disciples
        .iter()
        .map(|disciple| {
            disciple
                .master_id
                .as_deref()
                .and_then(|master_id| index_by_id.get(master_id).copied())
        })
        .collect::<Vec<_>>();

    const UNVISITED: u8 = 0;
    const VISITING: u8 = 1;
    const RESOLVED: u8 = 2;
    let mut state = vec![UNVISITED; disciples.len()];
    let mut generations = vec![0_i32; disciples.len()];
    let mut valid = vec![false; disciples.len()];
    let mut path = Vec::new();

    // The master relation is a functional graph. Resolve each path once and
    // memoize both valid generations and invalid paths, so chains, breaks, and
    // cycles all take linear time after the ID index is built.
    for start in 0..disciples.len() {
        if state[start] != UNVISITED {
            continue;
        }

        path.clear();
        let mut current = start;
        let first_generation = loop {
            if state[current] == UNVISITED {
                state[current] = VISITING;
                path.push(current);

                if !has_master[current] {
                    break Some(0);
                }
                if let Some(master_index) = master_indices[current] {
                    current = master_index;
                } else {
                    break None;
                }
            } else if state[current] == VISITING {
                // The current path reaches a cycle. Its descendants are
                // invalid too, matching the old final zeroing behavior.
                break None;
            } else if valid[current] {
                break Some(generations[current].saturating_add(1));
            } else {
                break None;
            }
        };

        if let Some(mut generation) = first_generation {
            for &index in path.iter().rev() {
                generations[index] = generation;
                valid[index] = true;
                state[index] = RESOLVED;
                generation = generation.saturating_add(1);
            }
        } else {
            for &index in &path {
                generations[index] = 0;
                valid[index] = false;
                state[index] = RESOLVED;
            }
        }
    }

    for (index, disciple) in disciples.iter_mut().enumerate() {
        disciple.lineage_generation = if valid[index] { generations[index] } else { 0 };
    }
}

/// 指定武学是否已被纳入门中传承核心。
pub fn is_heritage_art(sect: &SectState, art_id: &str) -> bool {
    sect.heritage_arts.contains(&art_id.to_string())
}

/// 将武学纳为传承核心，或从核心中移除。
pub fn toggle_heritage_art(sect: &mut SectState, art_id: &str) -> Result<String, String> {
    let exists = sect.public_books.iter().any(|id| id == art_id)
        || sect.created_martial_arts.iter().any(|art| art.id == art_id);
    if !exists {
        return Err("藏经阁公册中无此武学。".into());
    }
    if sect.heritage_arts.contains(&art_id.to_string()) {
        sect.heritage_arts.retain(|id| id != art_id);
        Ok("此武学已从传承核心中移出。".into())
    } else {
        sect.heritage_arts.push(art_id.to_string());
        Ok("此武学已纳为本门传承核心。".into())
    }
}

/// 弟子研习传承武学时获得的额外经验效率。
#[allow(dead_code)]
pub fn heritage_bonus(sect: &SectState, disciple: &Disciple, art_id: &str) -> i32 {
    if !is_heritage_art(sect, art_id) {
        return 0;
    }
    // 辈分越浅、忠诚越高，传承认同越深
    let gen_bonus = 3_i32.saturating_sub(disciple.lineage_generation).max(0);
    let loyalty_bonus = (disciple.attributes.sect_loyalty - 60).max(0) / 10;
    gen_bonus + loyalty_bonus
}
