use crate::logic::{disciple, sect};
use crate::models::attributes::{ActionKind, ActionPlan, DiscipleRank};
use crate::models::management::ManagementRequest;
use crate::models::martial_art::all_martial_arts;
use crate::models::medicine::{Medicine, LEGACY_WOUND_MEDICINE_NAME};
use crate::models::sect::{BuildingKind, MoralDirection, SectOrder, SectPolicy};
use crate::models::{GameEvent, GameState};
use rand::Rng;
use std::collections::BTreeMap;

pub fn execute_management(
    rng: &mut impl Rng,
    state: &mut GameState,
    request: ManagementRequest,
) -> Result<Vec<GameEvent>, String> {
    let spends_decision = !matches!(
        &request,
        ManagementRequest::EquipSkill { .. } | ManagementRequest::SetElderDuty { .. }
    );
    if state.game_over {
        return Err("山门已散，诸事皆休。".into());
    }
    if spends_decision && state.pending_event.is_some() {
        return Err("眼前江湖事尚未处置，不宜另发掌门令。".into());
    }
    if spends_decision && state.decisions_used >= state.max_decisions {
        return Err("本月可议之事已尽，请推进月份。".into());
    }

    let text = match request {
        ManagementRequest::AssignAction {
            disciple_id,
            kind,
            target_id,
            martial_art_id,
        } => {
            let candidate = state
                .disciples
                .iter()
                .find(|disciple| disciple.id == disciple_id)
                .ok_or_else(|| "查无此人。".to_string())?;
            if !disciple::can_act(candidate) {
                return Err("此人眼下不在门中，或伤重难行。".into());
            }
            if !rank_allows_action(&candidate.rank, &kind) {
                return Err("此项差事不合此人的门中身份。".into());
            }
            if matches!(kind, ActionKind::Maintain | ActionKind::Construct)
                && !state
                    .sect
                    .buildings
                    .iter()
                    .any(|building| Some(building.id.as_str()) == target_id.as_deref())
            {
                return Err("须指定一处门派建筑。".into());
            }
            let disciple = player_disciple_mut(state, &disciple_id)?;
            if kind == ActionKind::CultivateNeili
                && disciple.attributes.neili.maximum >= disciple::neili_training_cap(disciple)
            {
                return Err("此人现有内力已达到所装备内功的修炼上限。".into());
            }
            if kind == ActionKind::Meditate
                && disciple.attributes.energy.maximum >= disciple::energy_training_cap(disciple)
            {
                return Err("此人现有精力已达到知识修为上限。".into());
            }
            disciple.action = Some(ActionPlan {
                kind,
                target_id,
                martial_art_id,
                assigned_by: Some("掌门".into()),
                remaining_months: 1,
            });
            format!("掌门传话，命{}依令安排本月行止。", disciple.name)
        }
        ManagementRequest::EquipSkill {
            disciple_id,
            basic_skill_id,
            martial_art_id,
        } => {
            let disciple = player_disciple_mut(state, &disciple_id)?;
            disciple::equip_skill(disciple, &basic_skill_id, &martial_art_id)?;
            format!(
                "{}将{}改作当前运用的武学。",
                disciple.name,
                art_name(&martial_art_id)
            )
        }
        ManagementRequest::SetPolicy { policy } => {
            state.sect.policy = policy;
            format!(
                "门中上下奉行“{}”之策，自本月起各有侧重。",
                policy_name(&state.sect.policy)
            )
        }
        ManagementRequest::SetMoralDirection { direction } => {
            state.sect.moral_direction = direction;
            format!(
                "执事堂颁下门风新训，本派自此以“{}”为行事准绳。",
                moral_direction_name(&state.sect.moral_direction)
            )
        }
        ManagementRequest::UpgradeBuilding { building_id } => {
            let building = state
                .sect
                .buildings
                .iter()
                .find(|building| building.id == building_id)
                .ok_or_else(|| "门中并无此处建筑。".to_string())?;
            if building.work_required > 0 {
                return Err("此处尚在施工，不可再兴土木。".into());
            }
            let cost = 80 + building.level * 60;
            spend(state, cost)?;
            let building = state
                .sect
                .buildings
                .iter_mut()
                .find(|building| building.id == building_id)
                .expect("建筑已验证存在");
            building.work_required = (building.level + 1) * 20;
            building.work_invested = 0;
            building.upgrading_months = (building.work_required + 9) / 10;
            format!(
                "拨库银{}两扩建{}，尚需杂役投入{}点工作量。",
                cost, building.name, building.work_required
            )
        }
        ManagementRequest::RepairBuilding { building_id } => {
            let building = state
                .sect
                .buildings
                .iter()
                .find(|building| building.id == building_id)
                .ok_or_else(|| "门中并无此处建筑。".to_string())?;
            let missing = 100 - building.condition;
            if missing <= 0 {
                return Err("此处完好，无须修缮。".into());
            }
            let cost = (missing * 2).max(10);
            spend(state, cost)?;
            let building = state
                .sect
                .buildings
                .iter_mut()
                .find(|building| building.id == building_id)
                .expect("建筑已验证存在");
            building.condition = 100;
            format!("拨库银{}两修葺{}，梁柱瓦石焕然一新。", cost, building.name)
        }
        ManagementRequest::Recruit => {
            spend(state, 50)?;
            let count: i32 = if rng.gen_bool(0.25) { 2 } else { 1 };
            let mut recruits = Vec::with_capacity(count as usize);
            for _ in 0..count {
                let mut recruit =
                    disciple::generate_disciple(rng, state.sect.attributes.prestige / 20);
                recruit.sect_id = Some("player".into());
                recruits.push(recruit);
            }
            sect::assign_recruit_ranks(&state.sect, &state.disciples, &mut recruits);
            let names = recruits
                .iter()
                .map(|recruit| format!("{}（{}）", recruit.name, rank_label(&recruit.rank)))
                .collect::<Vec<_>>();
            state.disciples.extend(recruits);
            state.total_disciples_recruited += count;
            format!("招贤榜下新收{}，共{}人拜入山门。", names.join("、"), count)
        }
        ManagementRequest::SetPersonnel {
            disciple_id,
            rank,
            department,
        } => {
            let current_rank = state
                .disciples
                .iter()
                .find(|disciple| disciple.id == disciple_id)
                .ok_or_else(|| "查无此人。".to_string())?
                .rank
                .clone();
            if current_rank != rank {
                let (outer_limit, inner_limit) = sect::rank_limits(&state.sect, &state.disciples);
                let target_count = state
                    .disciples
                    .iter()
                    .filter(|disciple| disciple.alive && disciple.rank == rank)
                    .count();
                let limit = match rank {
                    DiscipleRank::Chore => None,
                    DiscipleRank::Outer => Some((outer_limit, "外门")),
                    DiscipleRank::Inner => Some((inner_limit, "内门")),
                };
                if let Some((limit, name)) = limit {
                    if target_count >= limit {
                        return Err(format!(
                            "{}名额已满（现有{}/上限{}）。",
                            name, target_count, limit
                        ));
                    }
                }
            }
            let disciple = player_disciple_mut(state, &disciple_id)?;
            let required = rank_merit(&rank);
            if disciple.merit < required {
                return Err(format!("此人功绩尚浅，须有{}点功绩方可任用。", required));
            }
            disciple.rank = rank;
            disciple.department = department;
            disciple.attributes.sect_loyalty = (disciple.attributes.sect_loyalty + 4).min(100);
            disciple::sync_legacy_attributes(disciple);
            let name = disciple.name.clone();
            sect::normalize_elder_assignments(&mut state.sect, &state.disciples);
            format!("经掌门考校，{}获授新职，门中众人皆来道贺。", name)
        }
        ManagementRequest::AssignElder {
            building_id,
            disciple_id,
        } => {
            let building = state
                .sect
                .buildings
                .iter()
                .find(|building| building.id == building_id)
                .ok_or_else(|| "门中并无此处建筑。".to_string())?;
            let building_name = building.name.clone();
            let elder_title = building.elder_title.clone();
            let elder_name =
                if let Some(id) = disciple_id.as_deref() {
                    let candidate = state
                        .disciples
                        .iter()
                        .find(|disciple| disciple.id == id && disciple.alive)
                        .ok_or_else(|| "查无此人，或此人已不在世。".to_string())?;
                    if candidate.rank != DiscipleRank::Inner {
                        return Err("长老须从内门弟子中择任。".into());
                    }
                    if state.sect.buildings.iter().any(|other| {
                        other.id != building_id && other.elder_id.as_deref() == Some(id)
                    }) {
                        return Err("此人已主持别处事务，不可兼任两席长老。".into());
                    }
                    Some(candidate.name.clone())
                } else {
                    None
                };
            let building = state
                .sect
                .buildings
                .iter_mut()
                .find(|building| building.id == building_id)
                .expect("建筑已验证存在");
            building.elder_id = disciple_id;
            if building.selected_duty.is_none() {
                building.selected_duty = Some(building.kind.default_elder_duty().into());
            }
            building.elder_action_used = false;
            match elder_name {
                Some(name) => format!("擢任{}为{}，自此主持{}。", name, elder_title, building_name),
                None => format!(
                    "{}暂行空缺，{}事务仍由掌门兼领。",
                    elder_title, building_name
                ),
            }
        }
        ManagementRequest::SetElderDuty {
            building_id,
            duty_id,
        } => {
            let building = state
                .sect
                .buildings
                .iter_mut()
                .find(|building| building.id == building_id)
                .ok_or_else(|| "门中并无此处建筑。".to_string())?;
            if !elder_duties(&building.kind).contains(&duty_id.as_str()) {
                return Err("这桩事务不在该堂职掌之内。".into());
            }
            building.selected_duty = Some(duty_id);
            building.elder_action_used = false;
            format!("{}已择定下月堂务，过月即依此办理。", building.elder_title)
        }
        ManagementRequest::Expel { disciple_id } => {
            let index = state
                .disciples
                .iter()
                .position(|disciple| disciple.id == disciple_id)
                .ok_or_else(|| "查无此人。".to_string())?;
            let disciple = state.disciples.remove(index);
            sect::normalize_elder_assignments(&mut state.sect, &state.disciples);
            state.sect.attributes.morale = (state.sect.attributes.morale - 4).max(0);
            format!("{}被逐出山门，自此恩义两断。", disciple.name)
        }
        ManagementRequest::IssueItem {
            disciple_id,
            item,
            quantity,
        } => {
            if quantity <= 0 {
                return Err("发放数目须为正数。".into());
            }
            let item = canonical_item_name(&item).to_owned();
            let medicine = Medicine::from_name(&item);
            if !is_issuable_item(&item) {
                return Err("此物不可直接赐予弟子使用。".into());
            }
            let stock = state.sect.inventory.get(&item).copied().unwrap_or(0);
            if stock < quantity {
                return Err(format!("{}存量不足。", item));
            }
            *state.sect.inventory.entry(item.clone()).or_default() -= quantity;
            let disciple = player_disciple_mut(state, &disciple_id)?;
            match (item.as_str(), medicine) {
                ("草药", _) => {
                    disciple.attributes.qi.current = (disciple.attributes.qi.current
                        + quantity * 8)
                        .min(disciple.attributes.qi.maximum);
                    disciple.attributes.spirit.current = (disciple.attributes.spirit.current
                        + quantity * 5)
                        .min(disciple.attributes.spirit.maximum);
                    disciple::refresh_condition(disciple);
                }
                ("粮秣", _) => {
                    disciple.attributes.sect_loyalty =
                        (disciple.attributes.sect_loyalty + quantity / 2).min(100);
                }
                (_, Some(Medicine::Wound)) => {
                    disciple.attributes.qi.current = (disciple.attributes.qi.current
                        + quantity * 30)
                        .min(disciple.attributes.qi.maximum);
                    disciple::refresh_condition(disciple);
                }
                (_, Some(Medicine::Qi)) => {
                    disciple.attributes.neili.current = (disciple.attributes.neili.current
                        + quantity * 25)
                        .min(disciple.attributes.neili.maximum);
                }
                (_, Some(Medicine::Spirit)) => {
                    disciple.attributes.spirit.current = (disciple.attributes.spirit.current
                        + quantity * 30)
                        .min(disciple.attributes.spirit.maximum);
                    disciple::refresh_condition(disciple);
                }
                (_, Some(Medicine::Energy)) => {
                    disciple.attributes.energy.current = (disciple.attributes.energy.current
                        + quantity * 25)
                        .min(disciple.attributes.energy.maximum);
                }
                (_, Some(Medicine::Foundation)) => {
                    add_permanent_qi(disciple, quantity * 10);
                }
                (_, Some(Medicine::GatherQi)) => {
                    let amount = quantity * 5;
                    disciple::add_permanent_neili(disciple, amount);
                    disciple.attributes.neili.current = (disciple.attributes.neili.current
                        + amount)
                        .min(disciple.attributes.neili.maximum);
                }
                (_, Some(Medicine::CalmSpirit)) => {
                    add_permanent_spirit(disciple, quantity * 10);
                }
                (_, Some(Medicine::RestoreOrigin)) => {
                    let amount = quantity * 5;
                    disciple::add_permanent_energy(disciple, amount);
                    disciple.attributes.energy.current = (disciple.attributes.energy.current
                        + amount)
                        .min(disciple.attributes.energy.maximum);
                }
                (_, Some(Medicine::Marrow)) => {
                    disciple.aptitudes.constitution =
                        disciple.aptitudes.constitution.saturating_add(quantity);
                    disciple::recalculate_attribute_maxima(disciple);
                }
                (_, Some(Medicine::Sinew)) => {
                    disciple.aptitudes.strength =
                        disciple.aptitudes.strength.saturating_add(quantity);
                    disciple::recalculate_attribute_maxima(disciple);
                }
                (_, Some(Medicine::Awaken)) => {
                    disciple.aptitudes.intelligence =
                        disciple.aptitudes.intelligence.saturating_add(quantity);
                    disciple::recalculate_attribute_maxima(disciple);
                }
                (_, Some(Medicine::Lightness)) => {
                    disciple.aptitudes.agility =
                        disciple.aptitudes.agility.saturating_add(quantity);
                    disciple::recalculate_attribute_maxima(disciple);
                }
                (_, Some(Medicine::Longevity)) => {
                    disciple.age = disciple.age.saturating_sub(quantity).max(15);
                    disciple::recalculate_attribute_maxima(disciple);
                }
                _ => unreachable!("可赐物品均应有明确效果"),
            }
            disciple::sync_legacy_attributes(disciple);
            format!("司库奉命，将{}{}份发予{}。", item, quantity, disciple.name)
        }
        ManagementRequest::BrewPill { recipe_id } => {
            let (name, herb_cost, quantity, months) =
                pill_recipe(&recipe_id).ok_or_else(|| "百草堂中并无此方。".to_string())?;
            let herbs = state.sect.inventory.get("草药").copied().unwrap_or(0);
            if herbs < herb_cost {
                return Err(format!("草药不足，尚缺{}份。", herb_cost - herbs));
            }
            *state.sect.inventory.entry("草药".into()).or_default() -= herb_cost;
            state
                .sect
                .productions
                .push(crate::models::sect::ProductionTask {
                    id: format!(
                        "{}_{}_{}_{}",
                        recipe_id,
                        state.year,
                        state.month,
                        state.sect.productions.len()
                    ),
                    name: name.to_string(),
                    output_item: name.to_string(),
                    quantity,
                    remaining_months: months,
                });
            format!(
                "百草堂开炉炼制{}，耗草药{}份，需时{}个月。",
                name, herb_cost, months
            )
        }
        ManagementRequest::IssueOrder { order_id } => {
            if state
                .sect
                .active_orders
                .iter()
                .any(|order| order.id == order_id)
            {
                return Err("此令尚在施行，不宜重复颁布。".into());
            }
            let order =
                order_definition(&order_id).ok_or_else(|| "并无这道门派令。".to_string())?;
            spend(state, order.silver_cost)?;
            let name = order.name.clone();
            let months = order.remaining_months;
            state.sect.active_orders.push(order);
            format!("掌门颁下“{}”，门中奉行{}个月。", name, months)
        }
        ManagementRequest::LibraryAdd { martial_art_id } => {
            if state.sect.public_books.contains(&martial_art_id) {
                return Err("此典籍早已收在藏经阁中。".into());
            }
            let owner = state
                .disciples
                .iter_mut()
                .find(|d| d.martial_progress.private_books.contains(&martial_art_id))
                .ok_or_else(|| "门中无人持有此册私藏。".to_string())?;
            owner
                .martial_progress
                .private_books
                .retain(|book| book != &martial_art_id);
            state.sect.public_books.push(martial_art_id.clone());
            format!(
                "{}献出私藏，{}自此列入藏经阁公册。",
                owner.name,
                art_name(&martial_art_id)
            )
        }
        ManagementRequest::ResearchMartial { martial_art_id } => {
            if !state.sect.public_books.contains(&martial_art_id) {
                return Err("藏经阁中并无此门武学典籍。".into());
            }
            spend(state, 40)?;
            let gain = 20 + sect::building_level(&state.sect, "scripture") * 6;
            *state
                .sect
                .martial_research
                .entry(martial_art_id.clone())
                .or_default() += gain as i64;
            format!(
                "传功、掌书两房合参{}，门派造诣上限增了{}点。",
                art_name(&martial_art_id),
                gain
            )
        }
        ManagementRequest::ResearchNewMartial => research_new_martial(state)?,
        ManagementRequest::Exchange {
            sect_id,
            disciple_id,
        } => {
            let envoy_id = validate_inner_envoy(state, disciple_id.as_deref())?;
            spend(state, 45)?;
            let gain = rng.gen_range(10..=18);
            *state.sect.relations.entry(sect_id.clone()).or_default() += gain;
            let other_name = {
                let other = state
                    .npc_sects
                    .iter_mut()
                    .find(|sect| sect.id == sect_id)
                    .ok_or_else(|| "江湖中查无此派。".to_string())?;
                *other.relations.entry("player".into()).or_default() += gain;
                other.name.clone()
            };
            state.sect.attributes.prestige = (state.sect.attributes.prestige + 2).min(1000);
            dispatch_inner_envoy(state, &envoy_id, "天枢阁通问");
            format!("本派携礼拜会{}，宾主论武，交情添了{}分。", other_name, gain)
        }
        ManagementRequest::RequestManual {
            sect_id,
            martial_art_id,
            disciple_id,
        } => {
            let envoy_id = validate_inner_envoy(state, disciple_id.as_deref())?;
            let text = request_manual(state, &sect_id, &martial_art_id)?;
            dispatch_inner_envoy(state, &envoy_id, "天枢阁请教");
            text
        }
    };

    if spends_decision {
        state.decisions_used += 1;
    }
    sect::sync_legacy_fields(state);
    let event = GameEvent {
        text,
        mood: "good".into(),
        year: state.year,
        month: state.month,
        category: "sect".into(),
    };
    state.event_log.push(event.clone());
    trim_log(state);
    Ok(vec![event])
}

/// 研发议事与旧存档管理指令共用同一套结算，确保新武学同时归入藏经阁公册。
pub(crate) fn research_new_martial(state: &mut GameState) -> Result<String, String> {
    let candidate = all_martial_arts()
        .into_iter()
        .find(|art| {
            art.sect_id.as_deref() == Some("player")
                && art.is_combat
                && !state.martial_arts_learned.contains(&art.id)
        })
        .ok_or_else(|| "本门自创武学已尽数参明。".to_string())?;
    spend(state, 120)?;
    state.martial_arts_learned.push(candidate.id.clone());
    if !state.sect.public_books.contains(&candidate.id) {
        state.sect.public_books.push(candidate.id.clone());
    }
    state.sect.martial_research.insert(candidate.id.clone(), 60);
    state.sect.attributes.prestige = (state.sect.attributes.prestige + 5).min(1000);
    Ok(format!(
        "群策群力，终于创成{}，本派武学又开一脉。",
        art_name(&candidate.id)
    ))
}

fn player_disciple_mut<'a>(
    state: &'a mut GameState,
    id: &str,
) -> Result<&'a mut crate::models::Disciple, String> {
    state
        .disciples
        .iter_mut()
        .find(|disciple| disciple.id == id)
        .ok_or_else(|| "查无此人。".to_string())
}

fn canonical_item_name(item: &str) -> &str {
    match item {
        LEGACY_WOUND_MEDICINE_NAME => Medicine::Wound.name(),
        _ => item,
    }
}

fn is_issuable_item(item: &str) -> bool {
    matches!(item, "草药" | "粮秣") || Medicine::from_name(item).is_some()
}

fn pill_recipe(id: &str) -> Option<(Medicine, i32, i32, i32)> {
    Some(match id {
        "wound" => (Medicine::Wound, 4, 2, 1),
        "qi" => (Medicine::Qi, 6, 1, 2),
        "spirit" => (Medicine::Spirit, 5, 2, 1),
        "energy" => (Medicine::Energy, 6, 1, 2),
        "foundation" => (Medicine::Foundation, 12, 1, 6),
        "gather_qi" => (Medicine::GatherQi, 12, 1, 6),
        "calm_spirit" => (Medicine::CalmSpirit, 12, 1, 6),
        "restore_origin" => (Medicine::RestoreOrigin, 12, 1, 6),
        "marrow" => (Medicine::Marrow, 20, 1, 12),
        "sinew" => (Medicine::Sinew, 20, 1, 12),
        "awaken" => (Medicine::Awaken, 20, 1, 12),
        "lightness" => (Medicine::Lightness, 20, 1, 12),
        "longevity" => (Medicine::Longevity, 24, 1, 12),
        _ => return None,
    })
}

fn add_permanent_qi(disciple: &mut crate::models::Disciple, amount: i32) {
    disciple.attribute_bonuses.qi = disciple.attribute_bonuses.qi.saturating_add(amount);
    disciple::recalculate_attribute_maxima(disciple);
    disciple.attributes.qi.current =
        (disciple.attributes.qi.current + amount).min(disciple.attributes.qi.maximum);
}

fn add_permanent_spirit(disciple: &mut crate::models::Disciple, amount: i32) {
    disciple.attribute_bonuses.spirit = disciple.attribute_bonuses.spirit.saturating_add(amount);
    disciple::recalculate_attribute_maxima(disciple);
    disciple.attributes.spirit.current =
        (disciple.attributes.spirit.current + amount).min(disciple.attributes.spirit.maximum);
}

fn validate_inner_envoy(state: &GameState, id: Option<&str>) -> Result<String, String> {
    let id = id.ok_or_else(|| "须择一名内门弟子前往。".to_string())?;
    let disciple = state
        .disciples
        .iter()
        .find(|disciple| disciple.id == id)
        .ok_or_else(|| "查无这名使者。".to_string())?;
    if disciple.rank != DiscipleRank::Inner || !disciple::can_act(disciple) {
        return Err("使者须为眼下可行动的内门弟子。".into());
    }
    Ok(id.to_owned())
}

pub fn execute_monthly_elder_duties(
    rng: &mut impl Rng,
    state: &mut GameState,
) -> Vec<(String, Result<String, String>)> {
    for building in &mut state.sect.buildings {
        building.elder_action_used = false;
    }
    let duties = state
        .sect
        .buildings
        .iter()
        .filter(|building| building.elder_id.is_some())
        .filter_map(|building| {
            building
                .selected_duty
                .as_ref()
                .map(|duty| (building.id.clone(), building.name.clone(), duty.clone()))
        })
        .collect::<Vec<_>>();

    duties
        .into_iter()
        .map(|(building_id, building_name, duty_id)| {
            let result = execute_elder_duty(rng, state, &building_id, &duty_id);
            if result.is_err() {
                if let Some(building) = state
                    .sect
                    .buildings
                    .iter_mut()
                    .find(|building| building.id == building_id)
                {
                    building.elder_action_used = true;
                }
            }
            (building_name, result)
        })
        .collect()
}

fn elder_duties(kind: &BuildingKind) -> &'static [&'static str] {
    match kind {
        BuildingKind::Practice => &["instruct", "drill"],
        BuildingKind::Scripture => &["curate", "comprehend"],
        BuildingKind::Warehouse => &["audit", "purchase"],
        BuildingKind::HerbHall => &["treat", "brew"],
        BuildingKind::Intelligence => &["correspond", "scout"],
        BuildingKind::Affairs => &["recruit", "arbitrate"],
        BuildingKind::Logistics => &["maintain", "supervise"],
    }
}

fn execute_elder_duty(
    rng: &mut impl Rng,
    state: &mut GameState,
    building_id: &str,
    duty_id: &str,
) -> Result<String, String> {
    let building = state
        .sect
        .buildings
        .iter()
        .find(|building| building.id == building_id)
        .ok_or_else(|| "门中并无此处建筑。".to_string())?;
    if building.elder_action_used {
        return Err("这位长老本月已办过一桩堂务。".into());
    }
    let elder_id = building
        .elder_id
        .clone()
        .ok_or_else(|| "此处长老席位尚缺，无人主持堂务。".to_string())?;
    let elder = state
        .disciples
        .iter()
        .find(|disciple| {
            disciple.id == elder_id && disciple.alive && disciple.rank == DiscipleRank::Inner
        })
        .ok_or_else(|| "现任长老已无法理事，请重新择任。".to_string())?;
    if !disciple::can_act(elder) {
        return Err("这位长老眼下伤病或外出，无法主持堂务。".into());
    }
    let elder_name = elder.name.clone();
    let kind = building.kind.clone();
    if !elder_duties(&kind).contains(&duty_id) {
        return Err("这桩事务不在该堂职掌之内。".into());
    }

    let result = match duty_id {
        "instruct" => {
            state.sect.attributes.morale = (state.sect.attributes.morale + 3).min(100);
            "整饬教习，门人习武之心更盛"
        }
        "drill" => {
            for disciple in state.disciples.iter_mut().filter(|disciple| disciple.alive) {
                disciple.merit += 2;
            }
            "主持月考，门人各添功绩"
        }
        "curate" => {
            let books = state.sect.public_books.clone();
            for book in books {
                *state.sect.martial_research.entry(book).or_default() += 4;
            }
            "校勘群籍，各册参研皆有所得"
        }
        "comprehend" => {
            if let Some(book) = state.sect.public_books.first().cloned() {
                *state.sect.martial_research.entry(book).or_default() += 18;
            }
            "邀集门人合参一册，武理渐明"
        }
        "audit" => {
            state.sect.attributes.silver += 12 + rng.gen_range(0..=12);
            "清点旧账，追回一笔散碎库银"
        }
        "purchase" => {
            if state.sect.attributes.silver < 15 {
                return Err("库银不足以采买物资。".into());
            }
            state.sect.attributes.silver -= 15;
            *state.sect.inventory.entry("草药".into()).or_default() += 5;
            "下山采买，为库中添了五份草药"
        }
        "treat" => {
            state.injury = (state.injury - 8).max(0);
            "亲自诊治，掌门伤势稍减"
        }
        "brew" => {
            let herbs = state.sect.inventory.get("草药").copied().unwrap_or(0);
            if herbs < 2 {
                return Err("草药不足两份，难以开炉。".into());
            }
            *state.sect.inventory.entry("草药".into()).or_default() -= 2;
            *state
                .sect
                .inventory
                .entry(Medicine::Wound.name().into())
                .or_default() += 1;
            Medicine::Wound.name()
        }
        "correspond" => {
            for relation in state.sect.relations.values_mut() {
                *relation = (*relation + 2).min(100);
            }
            "修书诸派，江湖交情稍有增益"
        }
        "scout" => {
            state.sect.attributes.prestige = (state.sect.attributes.prestige + 3).min(1000);
            "遣人查探江湖消息，本派声名渐著"
        }
        "recruit" => {
            if state.sect.attributes.silver < 25 {
                return Err("库银不足以张罗纳徒。".into());
            }
            state.sect.attributes.silver -= 25;
            let mut recruit = disciple::generate_disciple(rng, state.sect.attributes.prestige / 20);
            recruit.rank = DiscipleRank::Chore;
            recruit.sect_id = Some("player".into());
            state.disciples.push(recruit);
            "代掌门访得一名新人，先收入杂役名册"
        }
        "arbitrate" => {
            match state.sect.moral_direction {
                MoralDirection::Righteous => state.sect.attributes.morality += 3,
                MoralDirection::Neutral => state.sect.attributes.morale += 2,
                MoralDirection::Villainous => {
                    state.sect.attributes.morality -= 3;
                    state.sect.attributes.silver += 18;
                }
            }
            state.sect.attributes.morality = state.sect.attributes.morality.clamp(0, 100);
            state.sect.attributes.morale = state.sect.attributes.morale.clamp(0, 100);
            "依本派门风处置一桩江湖事务"
        }
        "maintain" => {
            for building in &mut state.sect.buildings {
                building.condition = (building.condition + 4).min(100);
            }
            "督率杂役巡检诸堂，建筑损耗得以修复"
        }
        "supervise" => {
            for building in &mut state.sect.buildings {
                if building.work_required > 0 {
                    building.work_invested =
                        (building.work_invested + 6).min(building.work_required);
                }
            }
            "亲临工地督造，各处营造进度俱增"
        }
        _ => unreachable!("堂务已校验"),
    };
    let result = if duty_id == "brew" {
        format!("试炼一炉，得{}一份", result)
    } else {
        result.to_owned()
    };
    if let Some(building) = state
        .sect
        .buildings
        .iter_mut()
        .find(|building| building.id == building_id)
    {
        building.elder_action_used = true;
    }
    Ok(format!("{}本月{}。", elder_name, result))
}

fn dispatch_inner_envoy(state: &mut GameState, id: &str, assigned_by: &str) {
    if let Some(disciple) = state
        .disciples
        .iter_mut()
        .find(|disciple| disciple.id == id)
    {
        disciple.away_months = 2;
        disciple.action = Some(ActionPlan {
            kind: ActionKind::SectMission,
            assigned_by: Some(assigned_by.into()),
            remaining_months: 2,
            ..ActionPlan::default()
        });
    }
}

fn spend(state: &mut GameState, amount: i32) -> Result<(), String> {
    if state.sect.attributes.silver < amount {
        return Err(format!(
            "库银不足，尚缺{}两。",
            amount - state.sect.attributes.silver
        ));
    }
    state.sect.attributes.silver -= amount;
    Ok(())
}

fn rank_merit(rank: &DiscipleRank) -> i64 {
    match rank {
        DiscipleRank::Chore => 0,
        DiscipleRank::Outer => 10,
        DiscipleRank::Inner => 80,
    }
}

fn rank_label(rank: &DiscipleRank) -> &'static str {
    match rank {
        DiscipleRank::Chore => "杂役",
        DiscipleRank::Outer => "外门",
        DiscipleRank::Inner => "内门",
    }
}

fn rank_allows_action(rank: &DiscipleRank, kind: &ActionKind) -> bool {
    match rank {
        DiscipleRank::Chore => matches!(
            kind,
            ActionKind::Maintain
                | ActionKind::Construct
                | ActionKind::Produce
                | ActionKind::Business
                | ActionKind::Gather
                | ActionKind::Recover
        ),
        DiscipleRank::Outer => matches!(
            kind,
            ActionKind::Practice
                | ActionKind::Spar
                | ActionKind::SectMission
                | ActionKind::Wander
                | ActionKind::Recover
        ),
        DiscipleRank::Inner => matches!(
            kind,
            ActionKind::Read
                | ActionKind::Practice
                | ActionKind::Teach
                | ActionKind::Spar
                | ActionKind::TemperBody
                | ActionKind::CultivateNeili
                | ActionKind::Meditate
                | ActionKind::SectMission
                | ActionKind::Wander
                | ActionKind::Recover
        ),
    }
}

fn policy_name(policy: &SectPolicy) -> &'static str {
    match policy {
        SectPolicy::Balanced => "持中守成",
        SectPolicy::Martial => "崇武精进",
        SectPolicy::Scholarly => "研经明理",
        SectPolicy::Chivalrous => "行侠尚义",
        SectPolicy::Mercantile => "通商裕库",
        SectPolicy::Reclusive => "闭门清修",
    }
}

fn moral_direction_name(direction: &MoralDirection) -> &'static str {
    match direction {
        MoralDirection::Righteous => "行侠仗义",
        MoralDirection::Neutral => "独善其身",
        MoralDirection::Villainous => "为非作歹",
    }
}

fn order_definition(id: &str) -> Option<SectOrder> {
    let (name, months, cost, effect) = match id {
        "diligent" => ("勤修令", 3, 60, BTreeMap::from([("martial".into(), 15)])),
        "righteous" => ("尚义令", 4, 80, BTreeMap::from([("morality".into(), 12)])),
        "frugal" => ("节用令", 4, 50, BTreeMap::from([("income".into(), 12)])),
        "rest" => ("调息令", 2, 45, BTreeMap::from([("recovery".into(), 18)])),
        _ => return None,
    };
    Some(SectOrder {
        id: id.into(),
        name: name.into(),
        remaining_months: months,
        silver_cost: cost,
        effect,
    })
}

fn request_manual(state: &mut GameState, sect_id: &str, art_id: &str) -> Result<String, String> {
    if state.sect.public_books.contains(&art_id.to_string()) {
        return Err("此册门中已有，无须再求。".into());
    }
    let other = state
        .npc_sects
        .iter()
        .find(|sect| sect.id == sect_id)
        .ok_or_else(|| "江湖中查无此派。".to_string())?;
    if !other.public_books.contains(&art_id.to_string()) {
        return Err("对方并无这册典籍。".into());
    }
    let art = all_martial_arts()
        .into_iter()
        .find(|art| art.id == art_id)
        .ok_or_else(|| "此武学谱录无考。".to_string())?;
    let relation = state.sect.relations.get(sect_id).copied().unwrap_or(0);
    let required_relation = if art.id.ends_with("_foundation") {
        10
    } else {
        45
    };
    if relation < required_relation {
        return Err(format!(
            "两派交情尚浅，须达{}分方可开口。",
            required_relation
        ));
    }
    let cost = 60 + art.difficulty * 5;
    spend(state, cost)?;
    state.sect.attributes.prestige = (state.sect.attributes.prestige - 3).max(0);
    state.sect.public_books.push(art.id.clone());
    state.martial_arts_learned.push(art.id.clone());
    Ok(format!(
        "耗费人情与库银，请得{}抄本一册。",
        art_name(&art.id)
    ))
}

fn art_name(id: &str) -> String {
    all_martial_arts()
        .into_iter()
        .find(|art| art.id == id)
        .map(|art| format!("《{}》", art.name))
        .unwrap_or_else(|| format!("《{}》", id))
}

fn trim_log(state: &mut GameState) {
    if state.event_log.len() > 80 {
        let excess = state.event_log.len() - 80;
        state.event_log.drain(0..excess);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::logic::world;
    use rand::{rngs::StdRng, SeedableRng};

    #[test]
    fn management_spends_one_decision_and_assigns_action() {
        let mut state = GameState::default();
        let mut rng = StdRng::seed_from_u64(1);
        state
            .disciples
            .push(disciple::generate_disciple(&mut rng, 0));
        let id = state.disciples[0].id.clone();
        execute_management(
            &mut rng,
            &mut state,
            ManagementRequest::AssignAction {
                disciple_id: id,
                kind: crate::models::attributes::ActionKind::Practice,
                target_id: None,
                martial_art_id: None,
            },
        )
        .unwrap();
        assert_eq!(state.decisions_used, 1);
        assert!(state.disciples[0].action.is_some());
    }

    #[test]
    fn changing_equipment_is_free_and_preserves_actual_neili() {
        let mut state = GameState::default();
        let mut rng = StdRng::seed_from_u64(11);
        let mut d = disciple::generate_disciple(&mut rng, 0);
        d.martial_progress.proficiencies.insert(
            "basic_force".into(),
            crate::models::attributes::SkillProgress::new(50, 0),
        );
        d.martial_progress.proficiencies.insert(
            "wudang_foundation".into(),
            crate::models::attributes::SkillProgress::new(25, 0),
        );
        let id = d.id.clone();
        let actual = d.attributes.neili.maximum;
        state.disciples.push(d);

        execute_management(
            &mut rng,
            &mut state,
            ManagementRequest::EquipSkill {
                disciple_id: id,
                basic_skill_id: "basic_force".into(),
                martial_art_id: "wudang_foundation".into(),
            },
        )
        .unwrap();

        assert_eq!(state.decisions_used, 0);
        assert_eq!(state.disciples[0].attributes.neili.maximum, actual);
        assert_eq!(
            state.disciples[0].equipped_skills.get("basic_force"),
            Some(&"wudang_foundation".into())
        );
    }

    #[test]
    fn exchange_unlocks_requesting_a_foundation_manual() {
        let mut state = GameState::default();
        let (sects, disciples) = world::generate_npc_world(state.world_seed);
        state.npc_sects = sects;
        state.npc_disciples = disciples;
        state.sect.relations.insert("wudang".into(), 50);
        let mut rng = StdRng::seed_from_u64(2);
        let mut envoy = disciple::generate_disciple(&mut rng, 0);
        envoy.rank = DiscipleRank::Inner;
        let envoy_id = envoy.id.clone();
        state.disciples.push(envoy);
        execute_management(
            &mut rng,
            &mut state,
            ManagementRequest::RequestManual {
                sect_id: "wudang".into(),
                martial_art_id: "wudang_foundation".into(),
                disciple_id: Some(envoy_id),
            },
        )
        .unwrap();
        assert!(state
            .sect
            .public_books
            .contains(&"wudang_foundation".into()));
        assert_eq!(state.disciples[0].away_months, 2);
    }

    #[test]
    fn selecting_elder_duty_is_free_and_persistent() {
        let mut state = GameState::default();
        let mut rng = StdRng::seed_from_u64(21);
        let mut elder = disciple::generate_disciple(&mut rng, 0);
        elder.rank = DiscipleRank::Inner;
        let elder_id = elder.id.clone();
        state.disciples.push(elder);
        state.sect.buildings[0].elder_id = Some(elder_id);
        let request = ManagementRequest::SetElderDuty {
            building_id: "practice".into(),
            duty_id: "drill".into(),
        };
        execute_management(&mut rng, &mut state, request).unwrap();
        assert_eq!(state.decisions_used, 0);
        assert_eq!(
            state.sect.buildings[0].selected_duty.as_deref(),
            Some("drill")
        );
        assert!(!state.sect.buildings[0].elder_action_used);
    }

    #[test]
    fn every_pill_recipe_consumes_herbs_and_finishes_into_inventory() {
        let mut rng = StdRng::seed_from_u64(22);
        let recipe_ids = [
            "wound",
            "qi",
            "spirit",
            "energy",
            "foundation",
            "gather_qi",
            "calm_spirit",
            "restore_origin",
            "marrow",
            "sinew",
            "awaken",
            "lightness",
            "longevity",
        ];

        for recipe_id in recipe_ids {
            let mut state = GameState::default();
            state.sect.inventory.insert("草药".into(), 100);
            let (name, herb_cost, quantity, months) = pill_recipe(recipe_id).unwrap();
            execute_management(
                &mut rng,
                &mut state,
                ManagementRequest::BrewPill {
                    recipe_id: recipe_id.into(),
                },
            )
            .unwrap();
            assert_eq!(state.sect.inventory["草药"], 100 - herb_cost);
            assert_eq!(state.sect.productions[0].remaining_months, months);
            for _ in 0..months {
                crate::logic::sect::apply_monthly_upkeep(&mut state.sect, 0);
            }
            assert_eq!(state.sect.inventory[name.name()], quantity);
            assert!(state.sect.productions.is_empty());
        }
    }

    #[test]
    fn issuing_medicines_applies_each_restorative_and_permanent_effect() {
        let mut state = GameState::default();
        state.max_decisions = 20;
        let mut disciple = crate::models::Disciple {
            id: "medicine_target".into(),
            age: 30,
            ..crate::models::Disciple::default()
        };
        disciple.attributes.qi.current = 1;
        disciple.attributes.neili.current = 1;
        disciple.attributes.spirit.current = 1;
        disciple.attributes.energy.current = 1;
        state.disciples.push(disciple);
        let items = Medicine::ALL;
        for item in items {
            state.sect.inventory.insert(item.name().into(), 1);
        }
        let before = state.disciples[0].clone();
        let mut rng = StdRng::seed_from_u64(23);
        for item in items {
            execute_management(
                &mut rng,
                &mut state,
                ManagementRequest::IssueItem {
                    disciple_id: "medicine_target".into(),
                    item: item.name().into(),
                    quantity: 1,
                },
            )
            .unwrap();
            assert_eq!(state.sect.inventory[item.name()], 0);
        }

        let after = &state.disciples[0];
        assert_eq!(after.attributes.qi.current, 41);
        assert_eq!(after.attributes.neili.current, 31);
        assert_eq!(after.attributes.spirit.current, 41);
        assert_eq!(after.attributes.energy.current, 31);
        assert_eq!(after.attribute_bonuses.qi, before.attribute_bonuses.qi + 10);
        assert_eq!(
            after.attribute_bonuses.neili,
            before.attribute_bonuses.neili + 5
        );
        assert_eq!(
            after.attribute_bonuses.spirit,
            before.attribute_bonuses.spirit + 10
        );
        assert_eq!(
            after.attribute_bonuses.energy,
            before.attribute_bonuses.energy + 5
        );
        assert_eq!(
            after.aptitudes.constitution,
            before.aptitudes.constitution + 1
        );
        assert_eq!(after.aptitudes.strength, before.aptitudes.strength + 1);
        assert_eq!(
            after.aptitudes.intelligence,
            before.aptitudes.intelligence + 1
        );
        assert_eq!(after.aptitudes.agility, before.aptitudes.agility + 1);
        assert_eq!(after.age, 29);
    }
}
