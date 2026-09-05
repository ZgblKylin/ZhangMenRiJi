use crate::logic::{disciple, sect};
use crate::models::attributes::{ActionKind, ActionPlan, DiscipleRank};
use crate::models::management::ManagementRequest;
use crate::models::martial_art::{
    all_martial_arts, canonical_skill_id, knowledge_skill_id, martial_art_by_id_with_created,
    MartialArt, MartialTier, SkillCategory,
};
use crate::models::medicine::{
    pill_recipe, recipe_silver_cost, Medicine, MedicineRate, PillRecipe,
    LEGACY_WOUND_MEDICINE_NAME, PILL_RECIPES,
};
use crate::models::sect::{BuildingKind, MoralDirection, SectOrder, SectPolicy};
use crate::models::{Disciple, GameEvent, GameState};
use rand::Rng;
use std::collections::BTreeMap;

pub fn execute_management(
    rng: &mut impl Rng,
    state: &mut GameState,
    request: ManagementRequest,
) -> Result<Vec<GameEvent>, String> {
    let spends_decision = !matches!(
        &request,
        ManagementRequest::PrepareSkill { .. }
            | ManagementRequest::SetElderDuty { .. }
            | ManagementRequest::SetAutoBrewQueue { .. }
    );
    if state.game_over {
        return Err(if state.game_won {
            "此局已经终了，诸务不可再行。"
        } else {
            "山门已散，诸事皆休。"
        }
        .into());
    }
    if state.pending_event.is_some() {
        return Err("眼前江湖事尚未处置，不宜改动门中卷宗。".into());
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
            let martial_art_id = martial_art_id.map(|id| canonical_skill_id(&id));
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
            validate_action_selection(
                state,
                candidate,
                &kind,
                target_id.as_deref(),
                martial_art_id.as_deref(),
            )?;
            let target_name = target_id.as_deref().and_then(|id| {
                state
                    .disciples
                    .iter()
                    .find(|other| other.id == id)
                    .map(|other| other.name.clone())
            });
            let created_martial_arts = state.sect.created_martial_arts.clone();
            let disciple = player_disciple_mut(state, &disciple_id)?;
            if kind == ActionKind::CultivateNeili
                && disciple.attributes.neili.maximum >= disciple::neili_training_cap(disciple)
            {
                return Err("此人现有内力已达到所准备内功的修炼上限。".into());
            }
            if kind == ActionKind::CultivateNeili
                && disciple.attributes.spirit.current.saturating_mul(10)
                    < disciple.attributes.spirit.maximum.saturating_mul(7)
            {
                return Err("打坐须心神清明，此人精神不足七成。".into());
            }
            if kind == ActionKind::Meditate
                && disciple.attributes.energy.maximum >= disciple::energy_training_cap(disciple)
            {
                return Err("此人现有精力已达到知识修为上限。".into());
            }
            if kind == ActionKind::Meditate
                && disciple.attributes.qi.current.saturating_mul(10)
                    < disciple.attributes.qi.maximum.saturating_mul(7)
            {
                return Err("冥想须气血安稳，此人气血不足七成。".into());
            }
            disciple.action = Some(ActionPlan {
                kind: kind.clone(),
                target_id,
                martial_art_id: martial_art_id.clone(),
                assigned_by: Some("掌门".into()),
                remaining_months: 1,
                ..ActionPlan::default()
            });
            let detail = match (target_name, martial_art_id.as_deref()) {
                (Some(target), Some(art)) => {
                    format!("与{}同参{}", target, art_name(art, &created_martial_arts))
                }
                (Some(target), None) => format!("与{}同修", target),
                (None, Some(art)) => art_name(art, &created_martial_arts),
                (None, None) => action_label(&kind).to_string(),
            };
            format!(
                "掌门传话，命{}本月专司{}（{}）。",
                disciple.name,
                action_label(&kind),
                detail
            )
        }
        ManagementRequest::DispatchTask {
            building_id,
            disciple_id,
            kind,
            duration_months,
        } => {
            if !(1..=3).contains(&duration_months) {
                return Err("任务须持续一至三个月。".into());
            }
            let building = state
                .sect
                .buildings
                .iter()
                .find(|building| building.id == building_id)
                .ok_or_else(|| "门中并无此处建筑。".to_string())?;
            if !dispatch_task_allowed(&building.kind, &kind) {
                return Err("这项任务不在该建筑职掌范围内。".into());
            }
            if sect::building_effectiveness(&state.sect, &building_id) <= 0 {
                return Err("此处已经损毁，须先修缮方能派发堂务。".into());
            }
            let building_name = building.name.clone();
            let disciple = player_disciple_mut(state, &disciple_id)?;
            if !disciple::can_act(disciple) {
                return Err("此人眼下不在门中，或伤重难行。".into());
            }
            disciple.action = Some(ActionPlan {
                kind,
                target_id: Some(building_id.clone()),
                assigned_by: Some(format!("building:{building_id}")),
                remaining_months: duration_months,
                ..ActionPlan::default()
            });
            format!(
                "{}向{}派下{}个月堂务，过月即开始办理。",
                building_name, disciple.name, duration_months
            )
        }
        ManagementRequest::PrepareSkill {
            disciple_id,
            basic_skill_id,
            martial_art_id,
        } => {
            let created_martial_arts = state.sect.created_martial_arts.clone();
            let disciple = player_disciple_mut(state, &disciple_id)?;
            disciple::prepare_skill_with_created(
                disciple,
                &basic_skill_id,
                &martial_art_id,
                &created_martial_arts,
            )?;
            format!(
                "{}将{}改作当前运用的武学。",
                disciple.name,
                art_name(&martial_art_id, &created_martial_arts)
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
            // 扩建等级不得超越庶务堂；庶务堂本身不在此限。
            if building.kind != crate::models::sect::BuildingKind::Logistics {
                let logistics_level = crate::logic::sect::building_level(&state.sect, "logistics");
                if building.level >= logistics_level {
                    return Err(format!(
                        "{}已是{}级，而庶务堂仅{}级。请先将庶务堂升至{}级以上方可续建。",
                        building.name,
                        building.level,
                        logistics_level,
                        building.level + 1,
                    ));
                }
            }
            let (silver_cost, iron_cost) = upgrade_building_cost(building.level);
            require_inventory(state, "精铁", iron_cost)?;
            spend(state, silver_cost)?;
            consume_inventory(state, "精铁", iron_cost);
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
                "拨库银{}两、精铁{}份扩建{}，尚需杂役投入{}点工作量。",
                silver_cost, iron_cost, building.name, building.work_required
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
            // 建筑完全损毁时不能让“缺铁/缺银 → 无法修复 → 无法生产”的
            // 资源闭环把存档锁死。掌门可以先组织一次不耗材料的临时抢修，
            // 让堂舍恢复到低效但可运转的 25%；后续再按正常费用修复。
            if building.condition <= 0 {
                let building_name = building.name.clone();
                let building = state
                    .sect
                    .buildings
                    .iter_mut()
                    .find(|building| building.id == building_id)
                    .ok_or_else(|| "门中并无此处建筑。".to_string())?;
                building.condition = 25;
                format!(
                    "暂无材料可用，掌门亲自组织临时抢修{}，堂舍恢复至25%效力；后续可再按常规费用修缮。",
                    building_name
                )
            } else {
                let (silver_cost, iron_cost) = repair_building_cost(missing);
                require_inventory(state, "精铁", iron_cost)?;
                spend(state, silver_cost)?;
                consume_inventory(state, "精铁", iron_cost);
                let building = state
                    .sect
                    .buildings
                    .iter_mut()
                    .find(|building| building.id == building_id)
                    .expect("建筑已验证存在");
                building.condition = 100;
                format!(
                    "拨库银{}两、精铁{}份修葺{}，梁柱瓦石焕然一新。",
                    silver_cost, iron_cost, building.name
                )
            }
        }
        ManagementRequest::Recruit => {
            require_building_effectiveness(state, "affairs", "执事堂")?;
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
            let current = state
                .disciples
                .iter()
                .find(|disciple| disciple.id == disciple_id)
                .ok_or_else(|| "查无此人。".to_string())?;
            let current_rank = current.rank.clone();
            let current_department = current.department.clone();
            if current_rank == rank && current_department == department {
                return Err("此人的品秩与职司均未改变，无须重复任用。".into());
            }
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
            sect::normalize_master_assignments(&mut state.disciples);
            format!("经掌门考校，{}获授新职，门中众人皆来道贺。", name)
        }
        ManagementRequest::AssignMaster {
            disciple_id,
            master_id,
        } => {
            let student_index = state
                .disciples
                .iter()
                .position(|disciple| disciple.id == disciple_id)
                .ok_or_else(|| "查无此人。".to_string())?;
            let student = &state.disciples[student_index];
            if !student.alive {
                return Err("亡故门人不可再行拜师。".into());
            }
            if student.rank == DiscipleRank::Chore {
                return Err("杂役尚未正式列入门墙，不可拜师。".into());
            }
            let student_name = student.name.clone();

            if let Some(master_id) = master_id {
                if state.disciples[student_index].master_id.as_deref() == Some(master_id.as_str()) {
                    return Err("此人已经列在所选师父门下，无须重复行礼。".into());
                }
                if master_id == disciple_id {
                    return Err("不可拜自己为师。".into());
                }
                let master_index = state
                    .disciples
                    .iter()
                    .position(|disciple| disciple.id == master_id)
                    .ok_or_else(|| "门中查无所选师父。".to_string())?;
                let master = &state.disciples[master_index];
                if !master.alive {
                    return Err("所选师父已经亡故。".into());
                }
                if master.rank != DiscipleRank::Inner {
                    return Err("师父须从同门在世内门弟子中择任。".into());
                }
                if master.sect_id != state.disciples[student_index].sect_id {
                    return Err("师徒须同属一门。".into());
                }
                if sect::master_assignment_creates_cycle(&state.disciples, &disciple_id, &master_id)
                {
                    return Err("如此任命会使师门辈分首尾相接，不可成礼。".into());
                }
                let apprentice_count = state
                    .disciples
                    .iter()
                    .filter(|disciple| {
                        disciple.id != disciple_id
                            && disciple.alive
                            && disciple.rank != DiscipleRank::Chore
                            && disciple.master_id.as_deref() == Some(master_id.as_str())
                    })
                    .count();
                if apprentice_count >= 5 {
                    return Err("此人门下已有五名在籍弟子，不宜再收徒。".into());
                }
                let master_name = master.name.clone();

                state.disciples[student_index].master_id = Some(master_id.clone());
                state.disciples[student_index].attributes.sect_loyalty =
                    (state.disciples[student_index].attributes.sect_loyalty + 3).min(100);
                state.disciples[student_index]
                    .relations
                    .entry(master_id.clone())
                    .and_modify(|relation| *relation = (*relation).max(50))
                    .or_insert(50);
                state.disciples[master_index]
                    .relations
                    .entry(disciple_id)
                    .and_modify(|relation| *relation = (*relation).max(50))
                    .or_insert(50);
                disciple::sync_legacy_attributes(&mut state.disciples[student_index]);

                format!(
                    "{}向{}行过拜师礼，自此正式列入其门下。",
                    student_name, master_name
                )
            } else {
                let former_master_id = state.disciples[student_index]
                    .master_id
                    .clone()
                    .ok_or_else(|| "此人眼下并无师承，无须解除。".to_string())?;
                let former_master = state
                    .disciples
                    .iter()
                    .find(|disciple| disciple.id == former_master_id)
                    .map(|disciple| disciple.name.clone())
                    .unwrap_or(former_master_id);
                state.disciples[student_index].master_id = None;
                format!("{}与{}禀明缘由，解除师承。", student_name, former_master)
            }
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
            duty_target,
        } => {
            if duty_id == "brew" {
                if let Some(target) = duty_target.as_deref() {
                    pill_recipe(target).ok_or_else(|| "百草堂中并无此方。".to_string())?;
                }
            }
            let building = state
                .sect
                .buildings
                .iter_mut()
                .find(|building| building.id == building_id)
                .ok_or_else(|| "门中并无此处建筑。".to_string())?;
            if !elder_duties(&building.kind).contains(&duty_id.as_str()) {
                return Err("这桩事务不在该堂职掌之内。".into());
            }
            if matches!(duty_id.as_str(), "expand" | "brew") {
                building.duty_target = duty_target;
            }
            building.selected_duty = Some(duty_id);
            building.elder_action_used = false;
            format!("{}已择定下月堂务，过月即依此办理。", building.elder_title)
        }
        ManagementRequest::SetAutoBrewQueue { recipe_ids } => {
            if recipe_ids.len() > PILL_RECIPES.len() {
                return Err(format!("常设药序至多列{}张不同药方。", PILL_RECIPES.len()));
            }
            let mut seen = std::collections::BTreeSet::new();
            for recipe_id in &recipe_ids {
                pill_recipe(recipe_id)
                    .ok_or_else(|| format!("百草堂中并无药方“{}”。", recipe_id))?;
                if !seen.insert(recipe_id.clone()) {
                    return Err("常设药序中不宜重复列入同一药方。".into());
                }
            }
            state.sect.auto_brew_queue = recipe_ids;
            state.sect.auto_brew_index = 0;
            state.sect.auto_brew_progress = 0;
            if state.sect.auto_brew_queue.is_empty() {
                "百草堂已封存常设药炉，暂不自行炼药。".into()
            } else {
                let names = state
                    .sect
                    .auto_brew_queue
                    .iter()
                    .filter_map(|id| pill_recipe(id))
                    .map(|recipe| recipe.medicine.name())
                    .collect::<Vec<_>>()
                    .join("、");
                format!("百草堂重录常设药序：{}；往后依次循环开炉。", names)
            }
        }
        ManagementRequest::Expel { disciple_id } => {
            let index = state
                .disciples
                .iter()
                .position(|disciple| disciple.id == disciple_id)
                .ok_or_else(|| "查无此人。".to_string())?;
            let disciple = state.disciples.remove(index);
            sect::normalize_elder_assignments(&mut state.sect, &state.disciples);
            sect::normalize_master_assignments(&mut state.disciples);
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
                    disciple.personal_rations += quantity;
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
            let recipe = pill_recipe(&recipe_id).ok_or_else(|| "百草堂中并无此方。".to_string())?;
            let effectiveness = require_building_effectiveness(state, "herb_hall", "百草堂")?;
            let herbs = state.sect.inventory.get("草药").copied().unwrap_or(0);
            if herbs < recipe.herb_cost {
                return Err(format!("草药不足，尚缺{}份。", recipe.herb_cost - herbs));
            }
            let silver_cost = recipe_silver_cost(recipe);
            if state.sect.attributes.silver < silver_cost {
                return Err(format!(
                    "库银不足以开炉，尚缺{}两。",
                    silver_cost - state.sect.attributes.silver
                ));
            }
            state.sect.attributes.silver -= silver_cost;
            *state.sect.inventory.entry("草药".into()).or_default() -= recipe.herb_cost;
            let quick_months = (recipe.months * 2 + 2) / 3;
            let remaining_months = (quick_months * 100 + effectiveness - 1) / effectiveness;
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
                    name: recipe.medicine.name().to_string(),
                    output_item: recipe.medicine.name().to_string(),
                    quantity: recipe.quantity,
                    remaining_months,
                });
            format!(
                "掌门命百草堂快速开炉炼制{}，耗草药{}份、库银{}两，需时{}个月。",
                recipe.medicine.name(),
                recipe.herb_cost,
                silver_cost,
                remaining_months
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
            let martial_art_id = canonical_skill_id(&martial_art_id);
            martial_art_by_id_with_created(&martial_art_id, &state.sect.created_martial_arts)
                .ok_or_else(|| "武学谱中并无这册典籍。".to_string())?;
            if state.sect.public_books.contains(&martial_art_id) {
                return Err("此典籍早已收在藏经阁中。".into());
            }
            let owner_name = {
                let owner = state
                    .disciples
                    .iter_mut()
                    .find(|disciple| {
                        disciple
                            .martial_progress
                            .private_books
                            .iter()
                            .any(|book| canonical_skill_id(book) == martial_art_id)
                    })
                    .ok_or_else(|| "门中无人持有此册私藏。".to_string())?;
                owner
                    .martial_progress
                    .private_books
                    .retain(|book| canonical_skill_id(book) != martial_art_id);
                owner.name.clone()
            };
            state.sect.public_books.push(martial_art_id.clone());
            if !state.martial_arts_learned.contains(&martial_art_id) {
                state.martial_arts_learned.push(martial_art_id.clone());
            }
            format!(
                "{}献出私藏，{}自此列入藏经阁公册。",
                owner_name,
                art_name(&martial_art_id, &state.sect.created_martial_arts)
            )
        }
        ManagementRequest::ResearchMartial { martial_art_id } => {
            let martial_art_id = canonical_skill_id(&martial_art_id);
            if !state.sect.public_books.contains(&martial_art_id) {
                return Err("藏经阁中并无此门武学典籍。".into());
            }
            let art =
                martial_art_by_id_with_created(&martial_art_id, &state.sect.created_martial_arts)
                    .ok_or_else(|| "武学谱中并无这册典籍。".to_string())?;
            if !art.is_combat {
                return Err("知识义理不设门派参研等级上限，无须耗银合参。".into());
            }
            let effectiveness = require_building_effectiveness(state, "scripture", "藏经阁")?;
            spend(state, 40)?;
            let base_gain = 20 + sect::building_level(&state.sect, "scripture") * 6;
            let gain = scaled_output(base_gain, effectiveness);
            let research = state
                .sect
                .martial_research
                .entry(martial_art_id.clone())
                .or_insert(50);
            *research += gain as i64;
            format!(
                "传功、掌书两房合参{}，门派造诣上限增了{}点。",
                art_name(&martial_art_id, &state.sect.created_martial_arts),
                gain
            )
        }
        ManagementRequest::ResearchNewMartial => research_new_martial(state)?,
        ManagementRequest::SetHeritageArt { martial_art_id } => {
            let canonical = crate::models::martial_art::canonical_skill_id(&martial_art_id);
            sect::toggle_heritage_art(&mut state.sect, &canonical)?
        }
        ManagementRequest::CreateMartialArt {
            name,
            category,
            basic_skill,
            weapon_basic,
        } => create_martial_art(
            state,
            rng,
            &name,
            category,
            &basic_skill,
            weapon_basic.as_deref(),
        )?,
        ManagementRequest::Exchange {
            sect_id,
            disciple_id,
        } => {
            let envoy_id = validate_inner_envoy(state, disciple_id.as_deref())?;
            require_building_effectiveness(state, "intelligence", "天枢阁")?;
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
            require_building_effectiveness(state, "intelligence", "天枢阁")?;
            let text = request_manual(state, &sect_id, &martial_art_id)?;
            dispatch_inner_envoy(state, &envoy_id, "天枢阁请教");
            text
        }
        ManagementRequest::JointPatrol {
            sect_id,
            disciple_id,
        } => {
            let envoy_id = validate_inner_envoy(state, disciple_id.as_deref())?;
            require_building_effectiveness(state, "intelligence", "天枢阁")?;
            joint_patrol_with_ally(state, rng, &sect_id)?;
            dispatch_inner_envoy(state, &envoy_id, "天枢阁联巡");
            format!(
                "本派与{}联手巡行边境，共御匪患。",
                sect_name(state, &sect_id)
            )
        }
        ManagementRequest::CallAid { sect_id } => {
            require_building_effectiveness(state, "intelligence", "天枢阁")?;
            call_ally_aid(state, rng, &sect_id)?
        }
        ManagementRequest::HostExchange {
            sect_id,
            disciple_id,
        } => {
            let envoy_id = validate_inner_envoy(state, disciple_id.as_deref())?;
            require_building_effectiveness(state, "intelligence", "天枢阁")?;
            host_exchange_with_ally(state, rng, &sect_id)?;
            dispatch_inner_envoy(state, &envoy_id, "天枢阁论道");
            format!(
                "邀请{}长老来门交流武学心得，双方皆有所获。",
                sect_name(state, &sect_id)
            )
        }
        ManagementRequest::TradeWithAlly {
            sect_id,
            item,
            quantity,
        } => {
            require_building_effectiveness(state, "intelligence", "天枢阁")?;
            trade_with_ally(state, rng, &sect_id, &item, quantity)?
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

/// 掌门自创一门全新武学，融汇门派造诣与个人悟性。
/// 名称由掌门亲定，武学门类、基础根基与兵器偏好亦须明确。
/// 论剑范畴与威力以藏经阁当前堂效、长老悟性与随机灵感综合决断。
pub(crate) fn create_martial_art(
    state: &mut GameState,
    rng: &mut impl rand::Rng,
    name: &str,
    category: crate::models::martial_art::SkillCategory,
    basic_skill: &str,
    weapon_basic: Option<&str>,
) -> Result<String, String> {
    use crate::logic::sect;
    use crate::models::martial_art::{tier_label_cn, MartialArt, MartialTier, SkillCategory};

    let name = name.trim().to_string();
    if name.is_empty() || name.chars().count() > 8 {
        return Err("武学名称须有一至八个汉字。".into());
    }
    if !category.is_combat_category() {
        return Err("自创武学须为战斗武学（拳脚/轻功/内功/兵器）。".into());
    }
    let basic_skill = crate::models::martial_art::canonical_skill_id(basic_skill);
    let basic_art = crate::models::martial_art::martial_art_by_id(&basic_skill)
        .ok_or_else(|| "所选根基武学谱中无载。".to_string())?;
    if !basic_art.is_combat || basic_art.tier != MartialTier::Basic {
        return Err("根基须为战斗基础技能。".into());
    }
    if category == SkillCategory::Weapon && weapon_basic.is_none() {
        return Err("兵器武学须择一剑/刀/棍/枪/鞭。".into());
    }
    let weapon_key = if category == SkillCategory::Weapon {
        weapon_basic.unwrap_or("basic_sword")
    } else {
        ""
    };
    require_building_effectiveness(state, "scripture", "藏经阁")?;
    let level = sect::building_effectiveness(&state.sect, "scripture");
    let knowledge_power = state
        .sect
        .martial_research
        .values()
        .copied()
        .max()
        .unwrap_or(0);
    let scripture_elder = state
        .sect
        .buildings
        .iter()
        .find(|building| building.id == "scripture")
        .and_then(|building| {
            building.elder_id.as_deref().and_then(|elder_id| {
                state
                    .disciples
                    .iter()
                    .find(|disciple| disciple.id == elder_id)
            })
        });
    let elder_wisdom = scripture_elder
        .map(|disciple| {
            sect::elder_competence_percent(disciple, crate::models::sect::BuildingKind::Scripture)
        })
        .unwrap_or(85);
    let tier = if knowledge_power >= 260 {
        MartialTier::Inner
    } else if knowledge_power >= 130 {
        MartialTier::Outer
    } else {
        MartialTier::Chore
    };
    let power = match tier {
        MartialTier::Basic => 1,
        MartialTier::Chore => 3,
        MartialTier::Outer => 6,
        MartialTier::Inner => 10,
    };
    let inspiration: i32 = rng.gen_range(0..=power + 7);
    let effective_power = (power * level / 100 * elder_wisdom / 100)
        .max(power / 2)
        .saturating_add(inspiration);
    let (atk, def, spd) = match category {
        SkillCategory::Unarmed => (effective_power + 2, effective_power, effective_power),
        SkillCategory::Dodge => (effective_power, effective_power, effective_power + 2),
        SkillCategory::Force => (effective_power + 1, effective_power + 2, effective_power),
        SkillCategory::Weapon => (
            effective_power + 2,
            effective_power + 1,
            effective_power + 1,
        ),
        _ => unreachable!(),
    };
    let difficulty = match tier {
        MartialTier::Chore => 16,
        MartialTier::Outer => 30,
        MartialTier::Inner => 48,
        _ => 16,
    };
    let art_type = if category == SkillCategory::Weapon {
        let type_name = crate::models::martial_art::martial_art_by_id(weapon_key)
            .map(|art| art.name)
            .unwrap_or_else(|| "兵器".to_string());
        category.display(&type_name)
    } else {
        category.display("")
    };
    let art_id = format!(
        "player_created_{}_{}",
        state.sect.created_martial_arts.len(),
        name
    );
    let art = MartialArt {
        id: art_id.clone(),
        name: name.clone(),
        art_type,
        category,
        tier,
        is_combat: true,
        desc: format!(
            "{}自创的{}武学，融汇门中造诣与一己悟性。",
            state.sect.name, name
        ),
        atk,
        def,
        spd,
        req_talent: match tier {
            MartialTier::Chore => 15,
            MartialTier::Outer => 25,
            MartialTier::Inner => 35,
            _ => 15,
        },
        sect_id: Some("player".into()),
        basic_skill,
        difficulty,
        usable_for_parry: false,
    };
    let silver_cost = match tier {
        MartialTier::Chore => 80,
        MartialTier::Outer => 160,
        MartialTier::Inner => 280,
        _ => 80,
    };
    let elder_note = scripture_elder
        .map(|elder| format!("，{}参详提点尤多", elder.name))
        .unwrap_or_default();
    spend(state, silver_cost)?;
    state.sect.created_martial_arts.push(art);
    state.martial_arts_learned.push(art_id.clone());
    state.sect.public_books.push(art_id.clone());
    let research_cap = 50
        + scaled_output(
            5 + level / 5,
            sect::building_effectiveness(&state.sect, "scripture"),
        );
    state
        .sect
        .martial_research
        .insert(art_id, i64::from(research_cap));
    state.sect.attributes.prestige = (state.sect.attributes.prestige + 5 + power as i32).min(1000);
    Ok(format!(
        "掌门悟通{}，亲创{}《{}》，本派武学又开新天{}。",
        category.display(weapon_key),
        tier_label_cn(tier),
        name,
        elder_note
    ))
}

pub(crate) fn research_new_martial(state: &mut GameState) -> Result<String, String> {
    let effectiveness = require_building_effectiveness(state, "scripture", "藏经阁")?;
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
    state.sect.martial_research.insert(
        candidate.id.clone(),
        i64::from(50 + scaled_output(10, effectiveness)),
    );
    state.sect.attributes.prestige = (state.sect.attributes.prestige + 5).min(1000);
    Ok(format!(
        "群策群力，终于创成{}，本派武学又开一脉。",
        art_name(&candidate.id, &state.sect.created_martial_arts)
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
        BuildingKind::Logistics => &["maintain", "supervise", "expand"],
    }
}

fn elder_brew_weight(recipe: PillRecipe, herbs: i32) -> i32 {
    let rate_weight = match recipe.rate {
        MedicineRate::Regular => 9,
        MedicineRate::Slow => 3,
        MedicineRate::VerySlow => 1,
    };
    if herbs < recipe.herb_cost {
        rate_weight
    } else {
        rate_weight * 3
    }
}

fn random_elder_brew_recipe(rng: &mut impl Rng, herbs: i32) -> PillRecipe {
    let total_weight = PILL_RECIPES
        .iter()
        .map(|recipe| elder_brew_weight(*recipe, herbs))
        .sum::<i32>();
    let mut roll = rng.gen_range(0..total_weight);
    PILL_RECIPES
        .iter()
        .copied()
        .find(|recipe| {
            roll -= elder_brew_weight(*recipe, herbs);
            roll < 0
        })
        .expect("丹药配方权重应为正数")
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
    let competence = sect::elder_competence_percent(elder, kind.clone());
    let duty_target = building.duty_target.clone();
    let building_name = building.name.clone();
    if !elder_duties(&kind).contains(&duty_id) {
        return Err("这桩事务不在该堂职掌之内。".into());
    }
    let building_effectiveness =
        require_building_effectiveness(state, building_id, &building_name)?;
    // 堂效与主事能力各只折算一次，以下所有堂务产出及营造进度共用这一最终比例。
    let effectiveness = building_effectiveness.saturating_mul(competence) / 100;

    let result = match duty_id {
        "instruct" => {
            let gain = scaled_output(3, effectiveness);
            state.sect.attributes.morale = (state.sect.attributes.morale + gain).min(100);
            return finish_elder_duty(
                state,
                building_id,
                &elder_name,
                format!("整饬教习，门人志气提升{}点", gain),
            );
        }
        "drill" => {
            let gain = scaled_output(2, effectiveness) as i64;
            for disciple in state.disciples.iter_mut().filter(|disciple| disciple.alive) {
                disciple.merit += gain;
            }
            return finish_elder_duty(
                state,
                building_id,
                &elder_name,
                format!("主持月考，门人各添{}点功绩", gain),
            );
        }
        "curate" => {
            let gain = i64::from(scaled_output(4, effectiveness));
            let books = state
                .sect
                .public_books
                .iter()
                .filter(|book| {
                    martial_art_by_id_with_created(book, &state.sect.created_martial_arts)
                        .is_some_and(|art| art.is_combat)
                })
                .cloned()
                .collect::<Vec<_>>();
            for book in books {
                *state.sect.martial_research.entry(book).or_insert(50) += gain;
            }
            return finish_elder_duty(
                state,
                building_id,
                &elder_name,
                format!("校勘武学群籍，各册门派可授上限提升{}级", gain),
            );
        }
        "comprehend" => {
            let gain = i64::from(scaled_output(18, effectiveness));
            if let Some(book) = state
                .sect
                .public_books
                .iter()
                .find(|book| {
                    martial_art_by_id_with_created(book, &state.sect.created_martial_arts)
                        .is_some_and(|art| art.is_combat)
                })
                .cloned()
            {
                *state.sect.martial_research.entry(book).or_insert(50) += gain;
            }
            return finish_elder_duty(
                state,
                building_id,
                &elder_name,
                format!("邀集门人合参一册武学，其门派可授上限提升{}级", gain),
            );
        }
        "audit" => {
            let gain = scaled_output(12 + rng.gen_range(0..=12), effectiveness);
            state.sect.attributes.silver += gain;
            return finish_elder_duty(
                state,
                building_id,
                &elder_name,
                format!("清点旧账，追回库银{}两", gain),
            );
        }
        "purchase" => {
            if state.sect.attributes.silver < 15 {
                return Err("库银不足以采买物资。".into());
            }
            let quantity = scaled_output(5, effectiveness);
            state.sect.attributes.silver -= 15;
            *state.sect.inventory.entry("草药".into()).or_default() += quantity;
            return finish_elder_duty(
                state,
                building_id,
                &elder_name,
                format!("下山采买，耗库银十五两，为库中添了{}份草药", quantity),
            );
        }
        "treat" => {
            let recovery = scaled_output(8, effectiveness);
            state.injury = (state.injury - recovery).max(0);
            return finish_elder_duty(
                state,
                building_id,
                &elder_name,
                format!("亲自诊治，掌门伤势减轻{}点", recovery),
            );
        }
        "brew" => {
            let herbs = state.sect.inventory.get("草药").copied().unwrap_or(0);
            let recipe = match duty_target.as_deref() {
                Some(target) => {
                    pill_recipe(target).ok_or_else(|| "百草堂中并无此方。".to_string())?
                }
                None => random_elder_brew_recipe(rng, herbs),
            };
            if herbs < recipe.herb_cost {
                return Err(format!(
                    "炼制{}尚缺{}份草药。",
                    recipe.medicine.name(),
                    recipe.herb_cost - herbs
                ));
            }
            let silver_cost = recipe_silver_cost(recipe);
            if state.sect.attributes.silver < silver_cost {
                return Err(format!(
                    "炼制{}尚缺{}两库银。",
                    recipe.medicine.name(),
                    silver_cost - state.sect.attributes.silver
                ));
            }
            state.sect.attributes.silver -= silver_cost;
            *state.sect.inventory.entry("草药".into()).or_default() -= recipe.herb_cost;
            let quantity = scaled_output(recipe.quantity, effectiveness);
            *state
                .sect
                .inventory
                .entry(recipe.medicine.name().into())
                .or_default() += quantity;
            let result = format!(
                "试炼一炉{}，耗草药{}份、库银{}两，得药{}份",
                recipe.medicine.name(),
                recipe.herb_cost,
                silver_cost,
                quantity
            );
            return finish_elder_duty(state, building_id, &elder_name, result);
        }
        "correspond" => {
            let gain = scaled_output(2, effectiveness);
            for relation in state.sect.relations.values_mut() {
                *relation = (*relation + gain).min(100);
            }
            return finish_elder_duty(
                state,
                building_id,
                &elder_name,
                format!("修书诸派，各派交情提升{}点", gain),
            );
        }
        "scout" => {
            let gain = scaled_output(3, effectiveness);
            state.sect.attributes.prestige = (state.sect.attributes.prestige + gain).min(1000);
            return finish_elder_duty(
                state,
                building_id,
                &elder_name,
                format!("遣人查探江湖消息，本派声望提升{}点", gain),
            );
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
                MoralDirection::Righteous => {
                    state.sect.attributes.morality += scaled_output(3, effectiveness)
                }
                MoralDirection::Neutral => {
                    state.sect.attributes.morale += scaled_output(2, effectiveness)
                }
                MoralDirection::Villainous => {
                    state.sect.attributes.morality -= scaled_output(3, effectiveness);
                    state.sect.attributes.silver += scaled_output(18, effectiveness);
                }
            }
            state.sect.attributes.morality = state.sect.attributes.morality.clamp(0, 100);
            state.sect.attributes.morale = state.sect.attributes.morale.clamp(0, 100);
            "依本派门风处置一桩江湖事务"
        }
        "maintain" => {
            let fully_supplied = state.sect.attributes.silver >= 8
                && state.sect.inventory.get("精铁").copied().unwrap_or(0) >= 2;
            let base_recovery = if fully_supplied {
                state.sect.attributes.silver -= 8;
                *state.sect.inventory.entry("精铁".into()).or_default() -= 2;
                4
            } else {
                2
            };
            let recovery = scaled_output(base_recovery, effectiveness);
            for building in &mut state.sect.buildings {
                building.condition = (building.condition + recovery).min(100);
            }
            return finish_elder_duty(
                state,
                building_id,
                &elder_name,
                if fully_supplied {
                    format!(
                        "耗库银八两、精铁两份巡检诸堂，所有建筑完好度恢复{}点",
                        recovery
                    )
                } else {
                    format!(
                        "库银或精铁不足，仅作最低限度维护，所有建筑完好度恢复{}点",
                        recovery
                    )
                },
            );
        }
        "supervise" => {
            let progress = scaled_output(6, effectiveness);
            for building in &mut state.sect.buildings {
                if building.work_required > 0 {
                    building.work_invested =
                        (building.work_invested + progress).min(building.work_required);
                }
            }
            return finish_elder_duty(
                state,
                building_id,
                &elder_name,
                format!("亲临工地督造，各处营造进度增加{}点", progress),
            );
        }
        "expand" => {
            let logistics_level = crate::logic::sect::building_level(&state.sect, "logistics");
            let candidates = state
                .sect
                .buildings
                .iter()
                .filter(|building| {
                    building.work_required == 0
                        && (building.kind == crate::models::sect::BuildingKind::Logistics
                            || building.level < logistics_level)
                })
                .map(|building| (building.id.clone(), building.level))
                .collect::<Vec<_>>();
            if candidates.is_empty() {
                return Err("眼下各处皆在施工，无处可再立项扩建。".into());
            }

            let selected_id = duty_target
                .filter(|target| candidates.iter().any(|(id, _)| id == target))
                .unwrap_or_else(|| {
                    let max_level = candidates
                        .iter()
                        .map(|(_, level)| *level)
                        .max()
                        .unwrap_or(1);
                    let total_weight = candidates
                        .iter()
                        .map(|(_, level)| max_level - level + 1)
                        .sum::<i32>();
                    let mut roll = rng.gen_range(0..total_weight);
                    candidates
                        .iter()
                        .find_map(|(id, level)| {
                            roll -= max_level - level + 1;
                            (roll < 0).then(|| id.clone())
                        })
                        .expect("扩建候选建筑权重应为正数")
                });
            let (target_name, target_level) = state
                .sect
                .buildings
                .iter()
                .find(|building| building.id == selected_id)
                .map(|building| (building.name.clone(), building.level))
                .expect("扩建候选建筑应存在");
            let (silver_cost, iron_cost) = upgrade_building_cost(target_level);
            require_inventory(state, "精铁", iron_cost)?;
            spend(state, silver_cost)?;
            consume_inventory(state, "精铁", iron_cost);
            let work_required = (target_level + 1) * 20;
            if let Some(target) = state
                .sect
                .buildings
                .iter_mut()
                .find(|building| building.id == selected_id)
            {
                target.work_required = work_required;
                target.work_invested = 0;
                target.upgrading_months = (work_required + 9) / 10;
            }
            let result = format!(
                "拨库银{}两、精铁{}份督率杂役扩建{}，立项{}点工作量",
                silver_cost, iron_cost, target_name, work_required
            );
            return finish_elder_duty(state, building_id, &elder_name, result);
        }
        _ => unreachable!("堂务已校验"),
    };
    finish_elder_duty(state, building_id, &elder_name, result.to_owned())
}

fn finish_elder_duty(
    state: &mut GameState,
    building_id: &str,
    elder_name: &str,
    result: String,
) -> Result<String, String> {
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

fn require_building_effectiveness(
    state: &GameState,
    building_id: &str,
    building_name: &str,
) -> Result<i32, String> {
    let effectiveness = sect::building_effectiveness(&state.sect, building_id);
    if effectiveness <= 0 {
        return Err(format!("{}已经损毁，须先修缮方能理事。", building_name));
    }
    Ok(effectiveness)
}

fn scaled_output(base: i32, effectiveness: i32) -> i32 {
    base.saturating_mul(effectiveness.max(0))
        .saturating_div(100)
        .max(1)
}

fn upgrade_building_cost(level: i32) -> (i32, i32) {
    (80 + level * 60, 3 + level * 2)
}

fn repair_building_cost(missing_condition: i32) -> (i32, i32) {
    (
        (missing_condition * 2).max(10),
        ((missing_condition + 24) / 25).max(1),
    )
}

fn require_inventory(state: &GameState, item: &str, amount: i32) -> Result<(), String> {
    let stock = state.sect.inventory.get(item).copied().unwrap_or(0);
    if stock < amount {
        return Err(format!("{}不足，尚缺{}份。", item, amount - stock));
    }
    Ok(())
}

fn consume_inventory(state: &mut GameState, item: &str, amount: i32) {
    *state.sect.inventory.entry(item.to_owned()).or_default() -= amount;
}

fn validate_action_selection(
    state: &GameState,
    actor: &Disciple,
    kind: &ActionKind,
    target_id: Option<&str>,
    martial_art_id: Option<&str>,
) -> Result<(), String> {
    if state.disciples.iter().any(|other| {
        other.id != actor.id
            && other.action.as_ref().is_some_and(|plan| {
                matches!(plan.kind, ActionKind::Teach | ActionKind::Spar)
                    && plan.target_id.as_deref() == Some(actor.id.as_str())
            })
    }) {
        return Err("此人已列入另一桩同修安排，本月不可另派差事。".into());
    }

    if matches!(kind, ActionKind::Maintain | ActionKind::Construct) {
        if !state
            .sect
            .buildings
            .iter()
            .any(|building| Some(building.id.as_str()) == target_id)
        {
            return Err("须指定一处门派建筑。".into());
        }
    }
    let required_building = match kind {
        ActionKind::Read | ActionKind::Meditate => Some(("scripture", "藏经阁")),
        ActionKind::Practice
        | ActionKind::Teach
        | ActionKind::Spar
        | ActionKind::TemperBody
        | ActionKind::CultivateNeili => Some(("practice", "传功堂")),
        ActionKind::Produce | ActionKind::Business => Some(("warehouse", "司库房")),
        ActionKind::Gather => Some(("herb_hall", "百草堂")),
        ActionKind::Maintain | ActionKind::Construct => Some(("logistics", "庶务堂")),
        ActionKind::SectMission | ActionKind::Wander | ActionKind::Recover => None,
    };
    if let Some((building_id, building_name)) = required_building {
        require_building_effectiveness(state, building_id, building_name)?;
    }

    let partner = if matches!(kind, ActionKind::Teach | ActionKind::Spar) {
        Some(
            target_id
                .ok_or_else(|| "须指定一名同门共同修习。".to_string())
                .and_then(|id| {
                    if id == actor.id {
                        return Err("不可与自己同修。".to_string());
                    }
                    let target = state
                        .disciples
                        .iter()
                        .find(|disciple| disciple.id == id)
                        .ok_or_else(|| "门中查无所选同修。".to_string())?;
                    if !disciple::can_act(target) {
                        return Err("所选同修眼下不在门中，或伤重难行。".into());
                    }
                    if target.action.is_some()
                        || state.disciples.iter().any(|other| {
                            other.id != actor.id
                                && other.id != target.id
                                && other.action.as_ref().is_some_and(|plan| {
                                    matches!(plan.kind, ActionKind::Teach | ActionKind::Spar)
                                        && plan.target_id.as_deref() == Some(target.id.as_str())
                                })
                        })
                    {
                        return Err("所选同修本月已有安排，不可兼顾两事。".into());
                    }
                    Ok(target)
                })?,
        )
    } else {
        None
    };

    if kind == &ActionKind::Teach
        && partner.is_some_and(|student| !sect::teaching_relationship_eligible(actor, student))
    {
        return Err("传武只可授予门下弟子，或交情达到三十五分的亲近同门。".into());
    }

    let Some(art_id) = martial_art_id else {
        return Ok(());
    };
    let art = martial_art_by_id_with_created(art_id, &state.sect.created_martial_arts)
        .ok_or_else(|| "武学谱中并无这门功夫。".to_string())?;
    match kind {
        ActionKind::Read => {
            let available = state.sect.public_books.iter().any(|book| book == &art.id)
                || actor
                    .martial_progress
                    .private_books
                    .iter()
                    .any(|book| canonical_skill_id(book) == art.id);
            if !available {
                return Err("此册既非藏经阁公卷，也不在此人私藏之中。".into());
            }
            if !rank_can_receive_tier(&actor.rank, art.tier) {
                return Err("此人身份未到，不可研读这层武学。".into());
            }
            validate_learning_foundations(actor, &art, &state.sect.created_martial_arts)?;
            let current_level = actor
                .martial_progress
                .proficiencies
                .get(&art.id)
                .map(|progress| progress.level)
                .unwrap_or(0);
            if learning_level_cap(state, actor, &art).is_some_and(|cap| current_level >= cap) {
                return Err("此门武学已触及个人修为或门派参研上限。".into());
            }
        }
        ActionKind::Practice => {
            if !actor.martial_progress.proficiencies.contains_key(&art.id) {
                return Err("此人尚未学会所选武学，不能径自练习。".into());
            }
            if art.category == SkillCategory::Knowledge {
                return Err("知识心法须以研读或传授参悟，不可在演武场练习。".into());
            }
            if learning_level_cap(state, actor, &art).is_some_and(|cap| {
                actor
                    .martial_progress
                    .proficiencies
                    .get(&art.id)
                    .is_some_and(|progress| progress.level >= cap)
            }) {
                return Err("此门武学已触及造诣、门派参研、基础或知识瓶颈。".into());
            }
        }
        ActionKind::Teach => {
            let teacher_level = actor
                .martial_progress
                .proficiencies
                .get(&art.id)
                .map(|progress| progress.level)
                .ok_or_else(|| "授业之人尚未学会所选武学。".to_string())?;
            if let Some(student) = partner {
                validate_learning_foundations(student, &art, &state.sect.created_martial_arts)?;
                if !rank_can_receive_tier(&student.rank, art.tier) {
                    return Err(format!("{}身份未到，不可得授这层武学。", student.name));
                }
                let student_level = student
                    .martial_progress
                    .proficiencies
                    .get(&art.id)
                    .map(|progress| progress.level)
                    .unwrap_or(0);
                if teacher_level <= student_level {
                    return Err("授业之人在这门武学上的造诣须高于受教者。".into());
                }
                if learning_level_cap(state, student, &art).is_some_and(|cap| student_level >= cap)
                {
                    return Err("受教者已触及造诣、基础、知识或门派参研瓶颈，须先另求突破。".into());
                }
            }
        }
        _ => return Err("此项行止无需另择武学。".into()),
    }
    Ok(())
}

fn validate_learning_foundations(
    disciple: &Disciple,
    art: &MartialArt,
    created_martial_arts: &[MartialArt],
) -> Result<(), String> {
    if !art.is_combat || art.tier == MartialTier::Basic {
        return Ok(());
    }
    if !art.basic_skill.is_empty()
        && disciple
            .martial_progress
            .proficiencies
            .get(&art.basic_skill)
            .is_none_or(|progress| progress.level <= 0)
    {
        return Err(format!(
            "须先习得{}，方能参悟此门武学。",
            art_name(&art.basic_skill, created_martial_arts)
        ));
    }
    if let Some(sect_id) = art.sect_id.as_deref() {
        let knowledge_id = knowledge_skill_id(sect_id);
        if disciple
            .martial_progress
            .proficiencies
            .get(&knowledge_id)
            .is_none_or(|progress| progress.level <= 0)
        {
            return Err(format!(
                "须先研读{}，方能领会此派武学义理。",
                art_name(&knowledge_id, created_martial_arts)
            ));
        }
    }
    Ok(())
}

fn learning_level_cap(state: &GameState, disciple: &Disciple, art: &MartialArt) -> Option<i32> {
    disciple::skill_level_cap_with_created(disciple, &art.id, &state.sect.created_martial_arts)
        .into_iter()
        .chain(sect::martial_research_level_cap(&state.sect, &art.id))
        .min()
}

fn rank_can_receive_tier(rank: &DiscipleRank, tier: MartialTier) -> bool {
    match tier {
        MartialTier::Basic | MartialTier::Chore => true,
        MartialTier::Outer => matches!(rank, DiscipleRank::Outer | DiscipleRank::Inner),
        MartialTier::Inner => matches!(rank, DiscipleRank::Inner),
    }
}

fn action_label(kind: &ActionKind) -> &'static str {
    match kind {
        ActionKind::Read => "研读典籍",
        ActionKind::Practice => "演练武学",
        ActionKind::Teach => "传武授艺",
        ActionKind::Spar => "同门切磋",
        ActionKind::TemperBody => "打熬气血",
        ActionKind::CultivateNeili => "打坐炼气",
        ActionKind::Meditate => "澄心冥想",
        ActionKind::SectMission => "外派办事",
        ActionKind::Wander => "江湖历练",
        ActionKind::Recover => "静养调息",
        ActionKind::Maintain => "修缮堂舍",
        ActionKind::Construct => "督造营建",
        ActionKind::Produce => "操持生产",
        ActionKind::Business => "经营采买",
        ActionKind::Gather => "入山采集",
    }
}

fn rank_merit(rank: &DiscipleRank) -> i64 {
    match rank {
        DiscipleRank::Chore => 0,
        DiscipleRank::Outer => 10,
        DiscipleRank::Inner => 40,
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
            ActionKind::Read
                | ActionKind::Practice
                | ActionKind::Spar
                | ActionKind::TemperBody
                | ActionKind::CultivateNeili
                | ActionKind::Meditate
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

fn dispatch_task_allowed(kind: &BuildingKind, action: &ActionKind) -> bool {
    match kind {
        BuildingKind::Practice => matches!(action, ActionKind::Practice),
        BuildingKind::Scripture => matches!(action, ActionKind::Read),
        BuildingKind::Warehouse => matches!(action, ActionKind::Business | ActionKind::Produce),
        BuildingKind::HerbHall => matches!(action, ActionKind::Gather),
        _ => false,
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
    let art_id = canonical_skill_id(art_id);
    if state.sect.public_books.contains(&art_id) {
        return Err("此册门中已有，无须再求。".into());
    }
    let other = state
        .npc_sects
        .iter()
        .find(|sect| sect.id == sect_id)
        .ok_or_else(|| "江湖中查无此派。".to_string())?;
    if !other.public_books.contains(&art_id) {
        return Err("对方并无这册典籍。".into());
    }
    let art = all_martial_arts()
        .into_iter()
        .find(|art| art.id == art_id)
        .ok_or_else(|| "此武学谱录无考。".to_string())?;
    if art.category == crate::models::martial_art::SkillCategory::Parry
        && art.tier != crate::models::martial_art::MartialTier::Basic
    {
        return Err("这册旧制招架谱已不再传授。".into());
    }
    let relation = state.sect.relations.get(sect_id).copied().unwrap_or(0);
    let (required_relation, required_prestige) = manual_requirements(&art);
    if relation < required_relation {
        return Err(format!(
            "两派交情尚浅，须达{}分方可开口。",
            required_relation
        ));
    }
    if state.sect.attributes.prestige < required_prestige {
        return Err(format!(
            "本派江湖声望尚浅，须达{}方有资格请教此册。",
            required_prestige
        ));
    }
    let cost = 60 + art.difficulty * 5;
    spend(state, cost)?;
    state.sect.attributes.prestige = (state.sect.attributes.prestige - 3).max(0);
    state.sect.public_books.push(art.id.clone());
    state.martial_arts_learned.push(art.id.clone());
    Ok(format!(
        "耗费人情与库银，请得{}抄本一册。",
        art_name(&art.id, &[])
    ))
}

/// 请教层次由交情与本门声望共同约束。先求门派义理，再逐层问艺，
/// 才能让异派武学的知识瓶颈形成真实的经营—养成闭环。
fn manual_requirements(art: &MartialArt) -> (i32, i32) {
    if art.category == SkillCategory::Knowledge || art.tier == MartialTier::Basic {
        return (10, 0);
    }
    match art.tier {
        MartialTier::Basic => (10, 0),
        MartialTier::Chore => (20, 40),
        MartialTier::Outer => (45, 90),
        MartialTier::Inner => (75, 180),
    }
}

fn art_name(id: &str, created_martial_arts: &[crate::models::martial_art::MartialArt]) -> String {
    martial_art_by_id_with_created(id, created_martial_arts)
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
    use crate::models::attributes::{Aptitudes, Department, SkillProgress};
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
    fn pending_event_blocks_even_free_management_changes() {
        let mut state = GameState::default();
        state.pending_event = Some(serde_json::json!({ "id": "pending" }));
        let original_queue = state.sect.auto_brew_queue.clone();
        let mut rng = StdRng::seed_from_u64(2);

        let result = execute_management(
            &mut rng,
            &mut state,
            ManagementRequest::SetAutoBrewQueue { recipe_ids: vec![] },
        );

        assert!(result.is_err());
        assert_eq!(state.sect.auto_brew_queue, original_queue);
        assert_eq!(state.decisions_used, 0);
    }

    #[test]
    fn assigning_and_releasing_a_master_updates_both_relations_and_loyalty() {
        let mut state = GameState {
            max_decisions: 4,
            ..GameState::default()
        };
        let master = Disciple {
            id: "master".into(),
            name: "谢师父".into(),
            rank: DiscipleRank::Inner,
            ..Disciple::default()
        };
        let student = Disciple {
            id: "student".into(),
            name: "沈徒弟".into(),
            rank: DiscipleRank::Outer,
            ..Disciple::default()
        };
        let loyalty_before = student.attributes.sect_loyalty;
        state.disciples.extend([master, student]);
        let mut rng = StdRng::seed_from_u64(91);

        execute_management(
            &mut rng,
            &mut state,
            ManagementRequest::AssignMaster {
                disciple_id: "student".into(),
                master_id: Some("master".into()),
            },
        )
        .unwrap();

        assert_eq!(state.decisions_used, 1);
        assert_eq!(state.disciples[1].master_id.as_deref(), Some("master"));
        assert_eq!(state.disciples[1].relations["master"], 50);
        assert_eq!(state.disciples[0].relations["student"], 50);
        assert_eq!(
            state.disciples[1].attributes.sect_loyalty,
            loyalty_before + 3
        );

        let loyalty_after_assignment = state.disciples[1].attributes.sect_loyalty;
        let repeated = execute_management(
            &mut rng,
            &mut state,
            ManagementRequest::AssignMaster {
                disciple_id: "student".into(),
                master_id: Some("master".into()),
            },
        );
        assert!(repeated.is_err());
        assert_eq!(state.decisions_used, 1);
        assert_eq!(
            state.disciples[1].attributes.sect_loyalty,
            loyalty_after_assignment
        );

        execute_management(
            &mut rng,
            &mut state,
            ManagementRequest::AssignMaster {
                disciple_id: "student".into(),
                master_id: None,
            },
        )
        .unwrap();
        assert_eq!(state.decisions_used, 2);
        assert!(state.disciples[1].master_id.is_none());

        let repeated_release = execute_management(
            &mut rng,
            &mut state,
            ManagementRequest::AssignMaster {
                disciple_id: "student".into(),
                master_id: None,
            },
        );
        assert!(repeated_release.is_err());
        assert_eq!(state.decisions_used, 2);
    }

    #[test]
    fn master_assignment_enforces_capacity_and_rejects_lineage_cycles() {
        let mut state = GameState {
            max_decisions: 10,
            ..GameState::default()
        };
        state.disciples.push(Disciple {
            id: "master".into(),
            rank: DiscipleRank::Inner,
            ..Disciple::default()
        });
        state.disciples.extend((0..5).map(|index| Disciple {
            id: format!("existing-{index}"),
            master_id: Some("master".into()),
            ..Disciple::default()
        }));
        state.disciples.push(Disciple {
            id: "sixth".into(),
            ..Disciple::default()
        });
        let mut rng = StdRng::seed_from_u64(92);

        let full = execute_management(
            &mut rng,
            &mut state,
            ManagementRequest::AssignMaster {
                disciple_id: "sixth".into(),
                master_id: Some("master".into()),
            },
        )
        .unwrap_err();
        assert!(full.contains("五名在籍弟子"));
        assert_eq!(state.decisions_used, 0);

        state.disciples.truncate(1);
        state.disciples.extend([
            Disciple {
                id: "senior".into(),
                rank: DiscipleRank::Inner,
                ..Disciple::default()
            },
            Disciple {
                id: "junior".into(),
                rank: DiscipleRank::Inner,
                ..Disciple::default()
            },
        ]);
        execute_management(
            &mut rng,
            &mut state,
            ManagementRequest::AssignMaster {
                disciple_id: "junior".into(),
                master_id: Some("senior".into()),
            },
        )
        .unwrap();
        let cycle = execute_management(
            &mut rng,
            &mut state,
            ManagementRequest::AssignMaster {
                disciple_id: "senior".into(),
                master_id: Some("junior".into()),
            },
        )
        .unwrap_err();
        assert!(cycle.contains("辈分首尾相接"));
    }

    #[test]
    fn master_assignment_authoritatively_validates_student_and_master_status() {
        let mut state = GameState {
            max_decisions: 10,
            ..GameState::default()
        };
        state.disciples.extend([
            Disciple {
                id: "student".into(),
                rank: DiscipleRank::Chore,
                ..Disciple::default()
            },
            Disciple {
                id: "master".into(),
                rank: DiscipleRank::Outer,
                ..Disciple::default()
            },
        ]);
        let mut rng = StdRng::seed_from_u64(921);

        let chore = execute_management(
            &mut rng,
            &mut state,
            ManagementRequest::AssignMaster {
                disciple_id: "student".into(),
                master_id: Some("master".into()),
            },
        )
        .unwrap_err();
        assert!(chore.contains("杂役"));

        state.disciples[0].rank = DiscipleRank::Outer;
        let outer_master = execute_management(
            &mut rng,
            &mut state,
            ManagementRequest::AssignMaster {
                disciple_id: "student".into(),
                master_id: Some("master".into()),
            },
        )
        .unwrap_err();
        assert!(outer_master.contains("内门"));

        state.disciples[1].rank = DiscipleRank::Inner;
        state.disciples[1].sect_id = Some("wudang".into());
        let foreign = execute_management(
            &mut rng,
            &mut state,
            ManagementRequest::AssignMaster {
                disciple_id: "student".into(),
                master_id: Some("master".into()),
            },
        )
        .unwrap_err();
        assert!(foreign.contains("同属一门"));

        let self_master = execute_management(
            &mut rng,
            &mut state,
            ManagementRequest::AssignMaster {
                disciple_id: "student".into(),
                master_id: Some("student".into()),
            },
        )
        .unwrap_err();
        assert!(self_master.contains("自己"));
        assert_eq!(state.decisions_used, 0);
    }

    #[test]
    fn demoting_a_student_to_chore_clears_master_but_keeps_department_change() {
        let mut state = GameState::default();
        state.disciples.extend([
            Disciple {
                id: "master".into(),
                rank: DiscipleRank::Inner,
                ..Disciple::default()
            },
            Disciple {
                id: "student".into(),
                master_id: Some("master".into()),
                ..Disciple::default()
            },
        ]);
        let mut rng = StdRng::seed_from_u64(93);

        execute_management(
            &mut rng,
            &mut state,
            ManagementRequest::SetPersonnel {
                disciple_id: "student".into(),
                rank: DiscipleRank::Chore,
                department: Some(crate::models::attributes::Department::Treasury),
            },
        )
        .unwrap();

        assert_eq!(state.disciples[1].rank, DiscipleRank::Chore);
        assert_eq!(
            state.disciples[1].department,
            Some(crate::models::attributes::Department::Treasury)
        );
        assert!(state.disciples[1].master_id.is_none());

        let repeated = execute_management(
            &mut rng,
            &mut state,
            ManagementRequest::SetPersonnel {
                disciple_id: "student".into(),
                rank: DiscipleRank::Chore,
                department: Some(crate::models::attributes::Department::Treasury),
            },
        );
        assert!(repeated.is_err());
        assert_eq!(state.decisions_used, 1);
    }

    #[test]
    fn directed_teaching_validates_partner_and_selected_martial_art() {
        let mut state = GameState::default();
        let mut rng = StdRng::seed_from_u64(101);
        let mut teacher = disciple::generate_disciple(&mut rng, 10);
        let mut student = disciple::generate_disciple(&mut rng, 0);
        teacher.rank = DiscipleRank::Inner;
        student.rank = DiscipleRank::Outer;
        teacher.martial_progress.proficiencies.insert(
            "basic_unarmed".into(),
            crate::models::attributes::SkillProgress::new(70, 0),
        );
        student.martial_progress.proficiencies.insert(
            "basic_unarmed".into(),
            crate::models::attributes::SkillProgress::new(20, 0),
        );
        let teacher_id = teacher.id.clone();
        let student_id = student.id.clone();
        student.master_id = Some(teacher_id.clone());
        state.disciples.extend([teacher, student]);

        let missing_target = execute_management(
            &mut rng,
            &mut state,
            ManagementRequest::AssignAction {
                disciple_id: teacher_id.clone(),
                kind: ActionKind::Teach,
                target_id: Some("missing".into()),
                martial_art_id: Some("basic_unarmed".into()),
            },
        )
        .unwrap_err();
        assert!(missing_target.contains("查无所选同修"));
        assert_eq!(state.decisions_used, 0);

        let unknown_art = execute_management(
            &mut rng,
            &mut state,
            ManagementRequest::AssignAction {
                disciple_id: teacher_id.clone(),
                kind: ActionKind::Teach,
                target_id: Some(student_id.clone()),
                martial_art_id: Some("not_a_skill".into()),
            },
        )
        .unwrap_err();
        assert!(unknown_art.contains("并无这门功夫"));

        execute_management(
            &mut rng,
            &mut state,
            ManagementRequest::AssignAction {
                disciple_id: teacher_id.clone(),
                kind: ActionKind::Teach,
                target_id: Some(student_id.clone()),
                martial_art_id: Some("basic_unarmed".into()),
            },
        )
        .unwrap();
        let plan = state.disciples[0].action.as_ref().unwrap();
        assert_eq!(plan.target_id.as_deref(), Some(student_id.as_str()));
        assert_eq!(plan.martial_art_id.as_deref(), Some("basic_unarmed"));

        let double_booking = execute_management(
            &mut rng,
            &mut state,
            ManagementRequest::AssignAction {
                disciple_id: student_id,
                kind: ActionKind::Practice,
                target_id: None,
                martial_art_id: Some("basic_unarmed".into()),
            },
        )
        .unwrap_err();
        assert!(double_booking.contains("同修安排"));
    }

    #[test]
    fn directed_teaching_requires_mastery_or_a_close_personal_relation() {
        let mut state = GameState::default();
        let mut teacher = Disciple {
            id: "teacher".into(),
            rank: DiscipleRank::Inner,
            ..Disciple::default()
        };
        teacher.martial_progress.proficiencies.insert(
            "basic_unarmed".into(),
            crate::models::attributes::SkillProgress::new(70, 0),
        );
        let mut student = Disciple {
            id: "student".into(),
            ..Disciple::default()
        };
        student.martial_progress.proficiencies.insert(
            "basic_unarmed".into(),
            crate::models::attributes::SkillProgress::new(20, 0),
        );
        student.attributes.attainment = disciple::attainment_required_for_level(100);
        state.disciples.extend([teacher, student]);
        let mut rng = StdRng::seed_from_u64(94);

        let distant = execute_management(
            &mut rng,
            &mut state,
            ManagementRequest::AssignAction {
                disciple_id: "teacher".into(),
                kind: ActionKind::Teach,
                target_id: Some("student".into()),
                martial_art_id: Some("basic_unarmed".into()),
            },
        )
        .unwrap_err();
        assert!(distant.contains("三十五分"));

        state.disciples[1].relations.insert("teacher".into(), 35);
        execute_management(
            &mut rng,
            &mut state,
            ManagementRequest::AssignAction {
                disciple_id: "teacher".into(),
                kind: ActionKind::Teach,
                target_id: Some("student".into()),
                martial_art_id: Some("basic_unarmed".into()),
            },
        )
        .unwrap();
    }

    #[test]
    fn requesting_foreign_manuals_starts_with_knowledge_and_scales_by_tier() {
        let mut state = GameState::default();
        let (sects, disciples) = world::generate_npc_world(state.world_seed);
        state.npc_sects = sects;
        state.npc_disciples = disciples;
        state.sect.relations.insert("wudang".into(), 10);

        let text = request_manual(&mut state, "wudang", "wudang_knowledge").unwrap();
        assert!(text.contains("道家心法"));
        assert!(state.sect.public_books.contains(&"wudang_knowledge".into()));

        let relation_error = request_manual(&mut state, "wudang", "wudang_foundation").unwrap_err();
        assert!(relation_error.contains("20分"));

        state.sect.relations.insert("wudang".into(), 20);
        request_manual(&mut state, "wudang", "wudang_foundation").unwrap();

        state.sect.relations.insert("wudang".into(), 45);
        let prestige_error =
            request_manual(&mut state, "wudang", "wudang_outer_unarmed").unwrap_err();
        assert!(prestige_error.contains("声望"));
        assert!(prestige_error.contains("90"));
    }

    #[test]
    fn a_private_manual_can_be_donated_into_the_public_library() {
        let mut state = GameState::default();
        let mut rng = StdRng::seed_from_u64(102);
        let mut owner = disciple::generate_disciple(&mut rng, 0);
        owner.name = "沈归鸿".into();
        owner
            .martial_progress
            .private_books
            .push("wudang_knowledge".into());
        state.disciples.push(owner);

        let events = execute_management(
            &mut rng,
            &mut state,
            ManagementRequest::LibraryAdd {
                martial_art_id: "wudang_knowledge".into(),
            },
        )
        .unwrap();

        assert!(events[0].text.contains("沈归鸿献出私藏"));
        assert!(state.sect.public_books.contains(&"wudang_knowledge".into()));
        assert!(state
            .martial_arts_learned
            .contains(&"wudang_knowledge".into()));
        assert!(state.disciples[0].martial_progress.private_books.is_empty());
    }

    #[test]
    fn researching_a_public_martial_art_raises_its_real_training_cap() {
        let mut state = GameState::default();
        let mut rng = StdRng::seed_from_u64(103);
        let mut student = Disciple {
            id: "research-student".into(),
            name: "顾听澜".into(),
            rank: DiscipleRank::Inner,
            ..Disciple::default()
        };
        student.attributes.attainment = disciple::attainment_required_for_level(100);
        student.martial_progress.proficiencies = BTreeMap::from([
            (
                "basic_force".into(),
                crate::models::attributes::SkillProgress::new(80, 0),
            ),
            (
                "player_knowledge".into(),
                crate::models::attributes::SkillProgress::new(80, 0),
            ),
            (
                "hunyuan".into(),
                crate::models::attributes::SkillProgress::new(50, 0),
            ),
        ]);
        state.disciples.push(student);

        let capped = execute_management(
            &mut rng,
            &mut state,
            ManagementRequest::AssignAction {
                disciple_id: "research-student".into(),
                kind: ActionKind::Practice,
                target_id: None,
                martial_art_id: Some("hunyuan".into()),
            },
        )
        .unwrap_err();
        assert!(capped.contains("门派参研"));

        execute_management(
            &mut rng,
            &mut state,
            ManagementRequest::ResearchMartial {
                martial_art_id: "hunyuan".into(),
            },
        )
        .unwrap();
        assert_eq!(state.sect.martial_research["hunyuan"], 76);

        execute_management(
            &mut rng,
            &mut state,
            ManagementRequest::AssignAction {
                disciple_id: "research-student".into(),
                kind: ActionKind::Practice,
                target_id: None,
                martial_art_id: Some("hunyuan".into()),
            },
        )
        .unwrap();
    }

    #[test]
    fn player_created_martial_art_survives_prepare_and_action_validation() {
        let mut state = GameState::default();
        let mut rng = StdRng::seed_from_u64(104);

        execute_management(
            &mut rng,
            &mut state,
            ManagementRequest::CreateMartialArt {
                name: "归元拳".into(),
                category: crate::models::martial_art::SkillCategory::Unarmed,
                basic_skill: "basic_unarmed".into(),
                weapon_basic: None,
            },
        )
        .unwrap();
        let created_id = state.sect.created_martial_arts[0].id.clone();

        let mut student = disciple::generate_disciple(&mut rng, 0);
        student.id = "created-art-student".into();
        student.sect_id = Some("player".into());
        student.rank = DiscipleRank::Inner;
        student.martial_progress.proficiencies.insert(
            "basic_unarmed".into(),
            crate::models::attributes::SkillProgress::new(40, 0),
        );
        student.martial_progress.proficiencies.insert(
            "player_knowledge".into(),
            crate::models::attributes::SkillProgress::new(40, 0),
        );
        student.martial_progress.proficiencies.insert(
            created_id.clone(),
            crate::models::attributes::SkillProgress::new(1, 0),
        );
        disciple::normalize_prepared_skills_with_created(
            &mut student,
            &state.sect.created_martial_arts,
        );
        disciple::recalculate_attribute_maxima(&mut student);
        let student_id = student.id.clone();
        state.disciples.push(student);

        execute_management(
            &mut rng,
            &mut state,
            ManagementRequest::PrepareSkill {
                disciple_id: student_id.clone(),
                basic_skill_id: "basic_unarmed".into(),
                martial_art_id: created_id.clone(),
            },
        )
        .unwrap();
        assert_eq!(
            state.disciples[0].prepared_skills.get("basic_unarmed"),
            Some(&created_id)
        );

        execute_management(
            &mut rng,
            &mut state,
            ManagementRequest::AssignAction {
                disciple_id: student_id,
                kind: ActionKind::Practice,
                target_id: None,
                martial_art_id: Some(created_id.clone()),
            },
        )
        .unwrap();
        assert_eq!(
            state.disciples[0]
                .action
                .as_ref()
                .and_then(|action| action.martial_art_id.as_deref()),
            Some(created_id.as_str())
        );
    }

    #[test]
    fn changing_preparation_is_free_and_preserves_actual_neili() {
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
            ManagementRequest::PrepareSkill {
                disciple_id: id,
                basic_skill_id: "basic_force".into(),
                martial_art_id: "wudang_foundation".into(),
            },
        )
        .unwrap();

        assert_eq!(state.decisions_used, 0);
        assert_eq!(state.disciples[0].attributes.neili.maximum, actual);
        assert_eq!(
            state.disciples[0].prepared_skills.get("basic_force"),
            Some(&"wudang_foundation".into())
        );
    }

    #[test]
    fn a_two_person_sect_can_promote_its_first_inner_disciple_at_forty_merit() {
        let mut state = GameState::default();
        let mut rng = StdRng::seed_from_u64(111);
        let mut candidate = disciple::generate_disciple(&mut rng, 0);
        candidate.rank = DiscipleRank::Outer;
        candidate.merit = 40;
        let candidate_id = candidate.id.clone();
        let mut fellow = disciple::generate_disciple(&mut rng, 0);
        fellow.rank = DiscipleRank::Outer;
        state.disciples.extend([candidate, fellow]);

        execute_management(
            &mut rng,
            &mut state,
            ManagementRequest::SetPersonnel {
                disciple_id: candidate_id,
                rank: DiscipleRank::Inner,
                department: None,
            },
        )
        .unwrap();

        assert_eq!(state.disciples[0].rank, DiscipleRank::Inner);
    }

    #[test]
    fn building_upgrade_requires_and_consumes_iron_atomically() {
        let mut state = GameState::default();
        let mut rng = StdRng::seed_from_u64(12);
        state.sect.attributes.silver = 300;
        // 扩建庶务堂本身不受庶务堂等级约束
        state.sect.inventory.insert("精铁".into(), 4);

        let error = execute_management(
            &mut rng,
            &mut state,
            ManagementRequest::UpgradeBuilding {
                building_id: "logistics".into(),
            },
        )
        .unwrap_err();
        assert!(error.contains("精铁不足"));
        assert_eq!(state.sect.attributes.silver, 300);
        assert_eq!(state.sect.buildings[6].work_required, 0);
        assert_eq!(state.decisions_used, 0);

        state.sect.inventory.insert("精铁".into(), 5);
        execute_management(
            &mut rng,
            &mut state,
            ManagementRequest::UpgradeBuilding {
                building_id: "logistics".into(),
            },
        )
        .unwrap();
        assert_eq!(state.sect.attributes.silver, 160);
        assert_eq!(state.sect.inventory["精铁"], 0);
        assert_eq!(state.sect.buildings[6].work_required, 40);
        assert_eq!(state.sect.buildings[6].upgrading_months, 4);
    }

    #[test]
    fn building_repair_scales_iron_with_damage() {
        let mut state = GameState::default();
        let mut rng = StdRng::seed_from_u64(13);
        state.sect.attributes.silver = 300;
        state.sect.inventory.insert("精铁".into(), 0);
        state.sect.buildings[0].condition = 51;

        let error = execute_management(
            &mut rng,
            &mut state,
            ManagementRequest::RepairBuilding {
                building_id: "practice".into(),
            },
        )
        .unwrap_err();
        assert!(error.contains("尚缺2份"));
        assert_eq!(state.sect.attributes.silver, 300);
        assert_eq!(state.sect.buildings[0].condition, 51);

        state.sect.inventory.insert("精铁".into(), 2);
        execute_management(
            &mut rng,
            &mut state,
            ManagementRequest::RepairBuilding {
                building_id: "practice".into(),
            },
        )
        .unwrap();
        assert_eq!(state.sect.attributes.silver, 202);
        assert_eq!(state.sect.inventory["精铁"], 0);
        assert_eq!(state.sect.buildings[0].condition, 100);
    }

    #[test]
    fn completely_destroyed_building_has_a_material_free_recovery_path() {
        let mut state = GameState::default();
        let mut rng = StdRng::seed_from_u64(14);
        state.sect.attributes.silver = 0;
        state.sect.inventory.insert("精铁".into(), 0);
        for building in &mut state.sect.buildings {
            building.condition = 0;
        }

        let events = execute_management(
            &mut rng,
            &mut state,
            ManagementRequest::RepairBuilding {
                building_id: "herb_hall".into(),
            },
        )
        .unwrap();

        assert_eq!(
            state
                .sect
                .buildings
                .iter()
                .find(|building| building.id == "herb_hall")
                .map(|building| building.condition),
            Some(25)
        );
        assert_eq!(state.sect.attributes.silver, 0);
        assert_eq!(state.sect.inventory["精铁"], 0);
        assert!(events[0].text.contains("临时抢修"));
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
            duty_target: None,
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
    fn configuring_the_background_brew_queue_is_free_and_validated() {
        let mut state = GameState::default();
        let mut rng = StdRng::seed_from_u64(212);

        execute_management(
            &mut rng,
            &mut state,
            ManagementRequest::SetAutoBrewQueue {
                recipe_ids: vec!["foundation".into(), "wound".into()],
            },
        )
        .unwrap();

        assert_eq!(state.decisions_used, 0);
        assert_eq!(
            state.sect.auto_brew_queue,
            vec!["foundation".to_string(), "wound".to_string()]
        );
        assert_eq!(state.sect.auto_brew_index, 0);
        assert_eq!(state.sect.auto_brew_progress, 0);

        let error = execute_management(
            &mut rng,
            &mut state,
            ManagementRequest::SetAutoBrewQueue {
                recipe_ids: vec!["wound".into(), "wound".into()],
            },
        )
        .unwrap_err();
        assert!(error.contains("重复"));
        assert_eq!(
            state.sect.auto_brew_queue,
            vec!["foundation".to_string(), "wound".to_string()]
        );
    }

    #[test]
    fn elder_duty_scales_with_building_level_and_stops_when_destroyed() {
        let mut state = GameState::default();
        let mut rng = StdRng::seed_from_u64(211);
        let mut elder = Disciple {
            id: "baseline_elder".into(),
            ..Disciple::default()
        };
        elder.rank = DiscipleRank::Inner;
        let elder_id = elder.id.clone();
        state.disciples.push(elder);
        let scripture = state
            .sect
            .buildings
            .iter_mut()
            .find(|building| building.id == "scripture")
            .unwrap();
        scripture.elder_id = Some(elder_id);
        scripture.level = 2;

        execute_elder_duty(&mut rng, &mut state, "scripture", "comprehend").unwrap();
        assert_eq!(state.sect.martial_research["hunyuan"], 71);

        {
            let scripture = state
                .sect
                .buildings
                .iter_mut()
                .find(|building| building.id == "scripture")
                .unwrap();
            scripture.condition = 0;
            scripture.elder_action_used = false;
        }
        let error =
            execute_elder_duty(&mut rng, &mut state, "scripture", "comprehend").unwrap_err();
        assert!(error.contains("已经损毁"));
        assert_eq!(state.sect.martial_research["hunyuan"], 71);
        assert!(
            !state
                .sect
                .buildings
                .iter()
                .find(|building| building.id == "scripture")
                .unwrap()
                .elder_action_used
        );
    }

    #[test]
    fn monthly_elder_duty_output_distinguishes_high_and_low_competence() {
        fn state_with_elder(mut elder: Disciple) -> GameState {
            elder.rank = DiscipleRank::Inner;
            let elder_id = elder.id.clone();
            let mut state = GameState::default();
            state.disciples.push(elder);
            let scripture = state
                .sect
                .buildings
                .iter_mut()
                .find(|building| building.id == "scripture")
                .unwrap();
            scripture.elder_id = Some(elder_id);
            scripture.selected_duty = Some("comprehend".into());
            state
        }

        let mut high = Disciple {
            id: "high_elder".into(),
            department: Some(Department::Library),
            aptitudes: Aptitudes {
                intelligence: 60,
                ..Aptitudes::default()
            },
            merit: 500,
            ..Disciple::default()
        };
        high.martial_progress
            .proficiencies
            .insert("player_knowledge".into(), SkillProgress::new(100, 0));
        let low = Disciple {
            id: "low_elder".into(),
            department: Some(Department::Transmission),
            aptitudes: Aptitudes {
                intelligence: 0,
                ..Aptitudes::default()
            },
            merit: -500,
            ..Disciple::default()
        };
        let mut high_state = state_with_elder(high);
        let mut low_state = state_with_elder(low);
        let mut high_rng = StdRng::seed_from_u64(214);
        let mut low_rng = StdRng::seed_from_u64(214);

        execute_monthly_elder_duties(&mut high_rng, &mut high_state);
        execute_monthly_elder_duties(&mut low_rng, &mut low_state);

        assert_eq!(high_state.sect.martial_research["hunyuan"], 73);
        assert_eq!(low_state.sect.martial_research["hunyuan"], 65);
    }

    #[test]
    fn a_destroyed_hall_blocks_its_corresponding_disciple_assignment() {
        let mut state = GameState::default();
        let mut rng = StdRng::seed_from_u64(213);
        let mut reader = disciple::generate_disciple(&mut rng, 0);
        reader.rank = DiscipleRank::Outer;
        let reader_id = reader.id.clone();
        state.disciples.push(reader);
        state
            .sect
            .buildings
            .iter_mut()
            .find(|building| building.id == "scripture")
            .unwrap()
            .condition = 0;

        let error = execute_management(
            &mut rng,
            &mut state,
            ManagementRequest::AssignAction {
                disciple_id: reader_id,
                kind: ActionKind::Read,
                target_id: None,
                martial_art_id: Some("player_knowledge".into()),
            },
        )
        .unwrap_err();

        assert!(error.contains("藏经阁已经损毁"));
        assert_eq!(state.decisions_used, 0);
        assert!(state.disciples[0].action.is_none());
    }

    #[test]
    fn expand_duty_persists_and_uses_the_selected_building() {
        let mut state = GameState::default();
        let mut rng = StdRng::seed_from_u64(23);
        let mut elder = disciple::generate_disciple(&mut rng, 0);
        elder.rank = DiscipleRank::Inner;
        let elder_id = elder.id.clone();
        state.disciples.push(elder);
        // 扩建其他建筑须庶务堂至少高于目标等级；先升一级庶务堂以通过约束。
        state
            .sect
            .buildings
            .iter_mut()
            .find(|building| building.id == "logistics")
            .unwrap()
            .level = 2;
        let logistics = state
            .sect
            .buildings
            .iter_mut()
            .find(|building| building.id == "logistics")
            .unwrap();
        logistics.elder_id = Some(elder_id);

        execute_management(
            &mut rng,
            &mut state,
            ManagementRequest::SetElderDuty {
                building_id: "logistics".into(),
                duty_id: "expand".into(),
                duty_target: Some("scripture".into()),
            },
        )
        .unwrap();
        let logistics = state
            .sect
            .buildings
            .iter()
            .find(|building| building.id == "logistics")
            .unwrap();
        assert_eq!(logistics.duty_target.as_deref(), Some("scripture"));

        let result = execute_elder_duty(&mut rng, &mut state, "logistics", "expand").unwrap();
        let scripture = state
            .sect
            .buildings
            .iter()
            .find(|building| building.id == "scripture")
            .unwrap();
        assert_eq!(scripture.work_required, 40);
        assert_eq!(scripture.work_invested, 0);
        assert!(result.contains("扩建藏经阁，立项40点工作量"));
    }

    #[test]
    fn expand_duty_falls_back_to_an_available_weighted_target() {
        let mut state = GameState::default();
        let mut rng = StdRng::seed_from_u64(24);
        let mut elder = disciple::generate_disciple(&mut rng, 0);
        elder.rank = DiscipleRank::Inner;
        let elder_id = elder.id.clone();
        state.disciples.push(elder);
        for building in &mut state.sect.buildings {
            building.level = 3;
            building.work_required = 10;
        }
        let practice = state
            .sect
            .buildings
            .iter_mut()
            .find(|building| building.id == "practice")
            .unwrap();
        practice.level = 1;
        practice.work_required = 0;
        let logistics = state
            .sect
            .buildings
            .iter_mut()
            .find(|building| building.id == "logistics")
            .unwrap();
        logistics.elder_id = Some(elder_id);
        logistics.duty_target = Some("missing".into());

        execute_elder_duty(&mut rng, &mut state, "logistics", "expand").unwrap();
        let practice = state
            .sect
            .buildings
            .iter()
            .find(|building| building.id == "practice")
            .unwrap();
        assert_eq!(practice.work_required, 40);
    }

    #[test]
    fn every_pill_recipe_consumes_herbs_and_finishes_into_inventory() {
        let mut rng = StdRng::seed_from_u64(22);

        for recipe in PILL_RECIPES {
            let mut state = GameState::default();
            state.sect.inventory.insert("草药".into(), 100);
            execute_management(
                &mut rng,
                &mut state,
                ManagementRequest::BrewPill {
                    recipe_id: recipe.id.into(),
                },
            )
            .unwrap();
            let fast_months = (recipe.months * 2 + 2) / 3;
            assert_eq!(state.sect.inventory["草药"], 100 - recipe.herb_cost);
            assert_eq!(state.sect.productions[0].remaining_months, fast_months);
            for _ in 0..fast_months {
                crate::logic::sect::apply_monthly_upkeep(&mut state.sect, 0);
            }
            assert_eq!(
                state.sect.inventory[recipe.medicine.name()],
                recipe.quantity
            );
            assert!(state.sect.productions.is_empty());
        }
    }

    #[test]
    fn elder_brew_uses_selected_recipe_and_rate_weights() {
        let mut state = GameState::default();
        let mut rng = StdRng::seed_from_u64(24);
        let mut elder = disciple::generate_disciple(&mut rng, 0);
        elder.rank = DiscipleRank::Inner;
        let elder_id = elder.id.clone();
        state.disciples.push(elder);
        let herb_hall = state
            .sect
            .buildings
            .iter_mut()
            .find(|building| building.id == "herb_hall")
            .unwrap();
        herb_hall.elder_id = Some(elder_id);

        execute_management(
            &mut rng,
            &mut state,
            ManagementRequest::SetElderDuty {
                building_id: "herb_hall".into(),
                duty_id: "brew".into(),
                duty_target: Some("foundation".into()),
            },
        )
        .unwrap();
        let result = execute_elder_duty(&mut rng, &mut state, "herb_hall", "brew").unwrap();

        assert_eq!(state.sect.inventory["草药"], 18);
        assert_eq!(state.sect.inventory[Medicine::Foundation.name()], 1);
        assert!(result.contains("培元丹"));
        assert_eq!(
            state
                .sect
                .buildings
                .iter()
                .find(|building| building.id == "herb_hall")
                .unwrap()
                .duty_target
                .as_deref(),
            Some("foundation")
        );

        let regular = pill_recipe("wound").unwrap();
        let slow = pill_recipe("foundation").unwrap();
        let very_slow = pill_recipe("marrow").unwrap();
        assert!(
            elder_brew_weight(regular, 100) > elder_brew_weight(slow, 100)
                && elder_brew_weight(slow, 100) > elder_brew_weight(very_slow, 100)
        );
        assert_eq!(
            elder_brew_weight(regular, regular.herb_cost - 1) * 3,
            elder_brew_weight(regular, regular.herb_cost)
        );
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

fn sect_name(state: &GameState, sect_id: &str) -> String {
    state
        .npc_sects
        .iter()
        .find(|s| s.id == sect_id)
        .map(|s| s.name.clone())
        .unwrap_or_else(|| sect_id.to_string())
}

fn is_ally(state: &GameState, sect_id: &str) -> bool {
    let player_to_them = state.sect.relations.get(sect_id).copied().unwrap_or(0);
    let them_to_player = state
        .npc_sects
        .iter()
        .find(|s| s.id == sect_id)
        .and_then(|s| s.relations.get("player").copied())
        .unwrap_or(0);
    player_to_them.min(them_to_player) >= 70
}

fn joint_patrol_with_ally(
    state: &mut GameState,
    rng: &mut impl rand::Rng,
    sect_id: &str,
) -> Result<(), String> {
    if !is_ally(state, sect_id) {
        return Err("此派与本门尚未结盟。".into());
    }
    spend(state, 60)?;
    let gain = rng.gen_range(4..=7);
    *state.sect.relations.entry(sect_id.to_string()).or_default() += gain;
    let ally = state
        .npc_sects
        .iter_mut()
        .find(|s| s.id == sect_id)
        .ok_or_else(|| "江湖中查无此派。".to_string())?;
    *ally.relations.entry("player".into()).or_default() += gain;
    state.sect.attributes.prestige = (state.sect.attributes.prestige + 2).min(1000);
    // 选择一个在门弟子获得历练经验
    let created_martial_arts = state.sect.created_martial_arts.clone();
    if let Some(fighter) = state
        .disciples
        .iter_mut()
        .filter(|d| d.alive && d.rank != DiscipleRank::Chore && crate::logic::disciple::can_act(d))
        .max_by_key(|d| d.merit)
    {
        let xp = rng.gen_range(30..=60);
        // 给该弟子首项战斗武学加经验
        for (art_id, progress) in &mut fighter.martial_progress.proficiencies {
            let art = martial_art_by_id_with_created(art_id, &created_martial_arts);
            if art.map_or(false, |a| a.is_combat) {
                progress.experience += xp;
                break;
            }
        }
    }
    Ok(())
}

fn call_ally_aid(
    state: &mut GameState,
    rng: &mut impl rand::Rng,
    sect_id: &str,
) -> Result<String, String> {
    if !is_ally(state, sect_id) {
        return Err("此派与本门尚未结盟。".into());
    }
    spend(state, 80)?;
    let gain = rng.gen_range(5..=9);
    *state.sect.relations.entry(sect_id.to_string()).or_default() += gain;
    let ally = state
        .npc_sects
        .iter_mut()
        .find(|s| s.id == sect_id)
        .ok_or_else(|| "江湖中查无此派。".to_string())?;
    *ally.relations.entry("player".into()).or_default() += gain;
    // 盟友遣人援助：招募一名随机弟子
    let prestige = state.sect.attributes.prestige;
    let mut recruit = crate::logic::disciple::generate_disciple(rng, prestige / 20);
    recruit.sect_id = Some("player".into());
    recruit.rank = DiscipleRank::Chore;
    recruit.attributes.sect_loyalty = 70;
    let name = recruit.name.clone();
    state.disciples.push(recruit);
    state.total_disciples_recruited += 1;
    Ok(format!(
        "{}遣精锐弟子{}{}来援，自此列入本派门墙。",
        ally.name,
        name,
        if gain >= 8 {
            "，两派情谊愈加深厚"
        } else {
            ""
        }
    ))
}

fn host_exchange_with_ally(
    state: &mut GameState,
    rng: &mut impl rand::Rng,
    sect_id: &str,
) -> Result<(), String> {
    if !is_ally(state, sect_id) {
        return Err("此派与本门尚未结盟。".into());
    }
    spend(state, 40)?;
    let gain = rng.gen_range(3..=5);
    *state.sect.relations.entry(sect_id.to_string()).or_default() += gain;
    let ally = state
        .npc_sects
        .iter_mut()
        .find(|s| s.id == sect_id)
        .ok_or_else(|| "江湖中查无此派。".to_string())?;
    *ally.relations.entry("player".into()).or_default() += gain;
    // 提升本门首部战斗武学的研究上限
    let research_gain = rng.gen_range(5..=12) as i64;
    if let Some(first_combat) = state
        .sect
        .public_books
        .iter()
        .filter_map(|id| {
            let art = martial_art_by_id_with_created(id, &state.sect.created_martial_arts)?;
            art.is_combat.then_some(id.clone())
        })
        .next()
    {
        *state
            .sect
            .martial_research
            .entry(first_combat)
            .or_insert(50) += research_gain;
    }
    Ok(())
}

fn trade_with_ally(
    state: &mut GameState,
    rng: &mut impl rand::Rng,
    sect_id: &str,
    item: &str,
    quantity: i32,
) -> Result<String, String> {
    if !is_ally(state, sect_id) {
        return Err("此派与本门尚未结盟。".into());
    }
    if quantity <= 0 || quantity > 50 {
        return Err("交易数量须在一至五十份之间。".into());
    }
    let allowed = ["草药", "精铁", "粮秣"];
    if !allowed.contains(&item) {
        return Err("只可交易草药、精铁或粮秣。".into());
    }
    let available = *state.sect.inventory.get(item).unwrap_or(&0);
    if available < quantity {
        return Err(format!("门中{}不足{}份。", item, quantity));
    }
    // 交易：消耗物资，换取银两与关系
    *state.sect.inventory.entry(item.to_string()).or_default() -= quantity;
    let silver_per_unit = match item {
        "草药" => 3,
        "精铁" => 5,
        "粮秣" => 2,
        _ => 2,
    };
    let silver_gain = quantity * silver_per_unit;
    state.sect.attributes.silver += silver_gain;
    let relation_gain = rng.gen_range(2..=4);
    *state.sect.relations.entry(sect_id.to_string()).or_default() += relation_gain;
    let ally = state
        .npc_sects
        .iter_mut()
        .find(|s| s.id == sect_id)
        .ok_or_else(|| "江湖中查无此派。".to_string())?;
    *ally.relations.entry("player".into()).or_default() += relation_gain;
    let ally_name = ally.name.clone();
    Ok(format!(
        "以{}份{}易得{}两库银，{}商人甚感满意，双方交情添了{}分。",
        quantity, item, silver_gain, ally_name, relation_gain
    ))
}
