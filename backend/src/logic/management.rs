use crate::logic::{disciple, sect};
use crate::models::attributes::{ActionPlan, DiscipleRank};
use crate::models::management::ManagementRequest;
use crate::models::martial_art::all_martial_arts;
use crate::models::sect::{SectOrder, SectPolicy};
use crate::models::{GameEvent, GameState};
use rand::Rng;
use std::collections::BTreeMap;

pub fn execute_management(
    rng: &mut impl Rng,
    state: &mut GameState,
    request: ManagementRequest,
) -> Result<Vec<GameEvent>, String> {
    if state.game_over {
        return Err("山门已散，诸事皆休。".into());
    }
    if state.pending_event.is_some() {
        return Err("眼前江湖事尚未处置，不宜另发掌门令。".into());
    }
    if state.decisions_used >= state.max_decisions {
        return Err("本月可议之事已尽，请推进月份。".into());
    }

    let text = match request {
        ManagementRequest::AssignAction {
            disciple_id,
            kind,
            target_id,
            martial_art_id,
        } => {
            let disciple = player_disciple_mut(state, &disciple_id)?;
            if !disciple::can_act(disciple) {
                return Err("此人眼下不在门中，或伤重难行。".into());
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
        ManagementRequest::SetPolicy { policy } => {
            state.sect.policy = policy;
            format!(
                "门中上下奉行“{}”之策，自本月起各有侧重。",
                policy_name(&state.sect.policy)
            )
        }
        ManagementRequest::UpgradeBuilding { building_id } => {
            let building = state
                .sect
                .buildings
                .iter()
                .find(|building| building.id == building_id)
                .ok_or_else(|| "门中并无此处建筑。".to_string())?;
            if building.upgrading_months > 0 {
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
            building.upgrading_months = building.level.max(1);
            format!(
                "拨库银{}两扩建{}，约需{}个月。",
                cost, building.name, building.upgrading_months
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
            let count = if rng.gen_bool(0.25) { 2 } else { 1 };
            let mut names = Vec::new();
            for _ in 0..count {
                let mut recruit =
                    disciple::generate_disciple(rng, state.sect.attributes.prestige / 20);
                recruit.sect_id = Some("player".into());
                names.push(recruit.name.clone());
                state.disciples.push(recruit);
            }
            state.total_disciples_recruited += count;
            format!("招贤榜下新收{}，共{}人拜入山门。", names.join("、"), count)
        }
        ManagementRequest::SetPersonnel {
            disciple_id,
            rank,
            department,
        } => {
            let disciple = player_disciple_mut(state, &disciple_id)?;
            let required = rank_merit(&rank);
            if disciple.merit < required {
                return Err(format!("此人功绩尚浅，须有{}点功绩方可任用。", required));
            }
            disciple.rank = rank;
            disciple.department = department;
            disciple.attributes.sect_loyalty = (disciple.attributes.sect_loyalty + 4).min(100);
            disciple::sync_legacy_attributes(disciple);
            format!("经掌门考校，{}获授新职，门中众人皆来道贺。", disciple.name)
        }
        ManagementRequest::Expel { disciple_id } => {
            let index = state
                .disciples
                .iter()
                .position(|disciple| disciple.id == disciple_id)
                .ok_or_else(|| "查无此人。".to_string())?;
            let disciple = state.disciples.remove(index);
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
            let stock = state.sect.inventory.get(&item).copied().unwrap_or(0);
            if stock < quantity {
                return Err(format!("{}存量不足。", item));
            }
            *state.sect.inventory.entry(item.clone()).or_default() -= quantity;
            let disciple = player_disciple_mut(state, &disciple_id)?;
            match item.as_str() {
                "草药" => {
                    disciple.attributes.qi.current = (disciple.attributes.qi.current
                        + quantity * 8)
                        .min(disciple.attributes.qi.maximum);
                    disciple.attributes.spirit.current = (disciple.attributes.spirit.current
                        + quantity * 5)
                        .min(disciple.attributes.spirit.maximum);
                    disciple::refresh_condition(disciple);
                }
                "粮秣" => {
                    disciple.attributes.sect_loyalty =
                        (disciple.attributes.sect_loyalty + quantity / 2).min(100);
                }
                _ => {
                    disciple.merit += quantity as i64;
                }
            }
            disciple::sync_legacy_attributes(disciple);
            format!("司库奉命，将{}{}份发予{}。", item, quantity, disciple.name)
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
        ManagementRequest::ResearchNewMartial => {
            spend(state, 120)?;
            let candidate = all_martial_arts()
                .into_iter()
                .find(|art| {
                    art.sect_id.as_deref() == Some("player")
                        && art.is_combat
                        && !state.martial_arts_learned.contains(&art.id)
                })
                .ok_or_else(|| "本门自创武学已尽数参明。".to_string())?;
            state.martial_arts_learned.push(candidate.id.clone());
            state.sect.public_books.push(candidate.id.clone());
            state.sect.martial_research.insert(candidate.id.clone(), 60);
            state.sect.attributes.prestige = (state.sect.attributes.prestige + 5).min(1000);
            format!(
                "群策群力，终于创成{}，本派武学又开一脉。",
                art_name(&candidate.id)
            )
        }
        ManagementRequest::Exchange { sect_id } => {
            spend(state, 45)?;
            let other = state
                .npc_sects
                .iter_mut()
                .find(|sect| sect.id == sect_id)
                .ok_or_else(|| "江湖中查无此派。".to_string())?;
            let gain = rng.gen_range(10..=18);
            *state.sect.relations.entry(sect_id.clone()).or_default() += gain;
            *other.relations.entry("player".into()).or_default() += gain;
            state.sect.attributes.prestige = (state.sect.attributes.prestige + 2).min(1000);
            format!("本派携礼拜会{}，宾主论武，交情添了{}分。", other.name, gain)
        }
        ManagementRequest::RequestManual {
            sect_id,
            martial_art_id,
        } => request_manual(state, &sect_id, &martial_art_id)?,
    };

    state.decisions_used += 1;
    sect::sync_legacy_fields(state);
    let event = GameEvent {
        text,
        mood: "good".into(),
        year: state.year,
        month: state.month,
    };
    state.event_log.push(event.clone());
    trim_log(state);
    Ok(vec![event])
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
        DiscipleRank::Elder => 250,
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
                kind: crate::models::attributes::ActionKind::Read,
                target_id: None,
                martial_art_id: None,
            },
        )
        .unwrap();
        assert_eq!(state.decisions_used, 1);
        assert!(state.disciples[0].action.is_some());
    }

    #[test]
    fn exchange_unlocks_requesting_a_foundation_manual() {
        let mut state = GameState::default();
        let (sects, disciples) = world::generate_npc_world(state.world_seed);
        state.npc_sects = sects;
        state.npc_disciples = disciples;
        state.sect.relations.insert("wudang".into(), 50);
        let mut rng = StdRng::seed_from_u64(2);
        execute_management(
            &mut rng,
            &mut state,
            ManagementRequest::RequestManual {
                sect_id: "wudang".into(),
                martial_art_id: "wudang_foundation".into(),
            },
        )
        .unwrap();
        assert!(state
            .sect
            .public_books
            .contains(&"wudang_foundation".into()));
    }
}
