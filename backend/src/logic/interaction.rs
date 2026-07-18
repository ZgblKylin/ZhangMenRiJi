use crate::logic::{country, disciple, sect};
use crate::models::attributes::{Department, DiscipleRank};
use crate::models::martial_art::{canonical_skill_id, martial_art_by_id};
use crate::models::named_npc::NpcPosition;
use crate::models::sect::{SectPolicy, SectState};
use crate::models::{Disciple, GameEvent, GameState};
use rand::Rng;
use std::collections::BTreeSet;

#[derive(Clone)]
struct Faction {
    slot: usize,
    id: String,
    name: String,
    country: String,
    policy: SectPolicy,
    prestige: i32,
    silver: i32,
    morality: i32,
    #[cfg(test)]
    members: usize,
    defectors: usize,
    conscriptable: usize,
}

#[derive(Clone, Copy)]
enum InteractionKind {
    Spar,
    Trade,
    BorderConflict,
    JointBandits,
    Alliance,
    AlliedJointPatrol,
    AlliedRescue,
    AllianceBetrayal,
    Defection,
    CourtConscription,
    YuanSongWar,
    DaliExchange,
    XiaTrade,
    XiaAssassination,
    YuanCourt,
    SongCourt,
    CourtClash,
}

struct Candidate {
    a: Faction,
    b: Faction,
    kind: InteractionKind,
    weight: u32,
}

/// 每个完成结算的月份生成三至五组不重复的门派交互。
pub fn run_monthly_interactions(rng: &mut impl Rng, state: &mut GameState) -> Vec<GameEvent> {
    let target = rng.gen_range(3..=5);
    let mut used_pairs = BTreeSet::new();
    let mut national_war_resolved = false;
    let mut events = Vec::with_capacity(target);

    // 掌门更替是单门派事件，优先占用本月一条世界纪事，再由门派交互补足总数。
    if let Some(event) = resolve_leader_succession(state) {
        events.push(event);
    }

    while events.len() < target {
        let factions = faction_snapshot(state);
        let mut candidates = Vec::new();
        for left in 0..factions.len() {
            for right in left + 1..factions.len() {
                let a = &factions[left];
                let b = &factions[right];
                let key = ordered_pair(&a.id, &b.id);
                if used_pairs.contains(&key) {
                    continue;
                }
                candidates.extend(
                    pair_candidates(state, a, b)
                        .into_iter()
                        .filter(|(kind, _)| !national_war_resolved || !is_national_war(*kind))
                        .map(|(kind, weight)| Candidate {
                            a: a.clone(),
                            b: b.clone(),
                            kind,
                            weight,
                        }),
                );
            }
        }
        if candidates.is_empty() {
            break;
        }

        let total: u32 = candidates.iter().map(|candidate| candidate.weight).sum();
        let mut draw = rng.gen_range(0..total);
        let selected = candidates
            .into_iter()
            .find(|candidate| {
                if draw < candidate.weight {
                    true
                } else {
                    draw -= candidate.weight;
                    false
                }
            })
            .expect("weighted interaction candidate");
        used_pairs.insert(ordered_pair(&selected.a.id, &selected.b.id));
        national_war_resolved |= is_national_war(selected.kind);
        if let Some((text, mood)) =
            apply_interaction(rng, state, &selected.a, &selected.b, selected.kind)
        {
            events.push(GameEvent {
                text,
                mood: mood.into(),
                year: state.year,
                month: state.month,
                category: "world".into(),
            });
        }
    }

    crate::logic::sect::sync_legacy_fields(state);
    events
}

fn faction_snapshot(state: &GameState) -> Vec<Faction> {
    std::iter::once((0, &state.sect))
        .chain(
            state
                .npc_sects
                .iter()
                .enumerate()
                .map(|(index, sect)| (index + 1, sect)),
        )
        .map(|(slot, sect)| {
            let members = faction_members(state, slot);
            // 任何门派至少保留一名在籍弟子，避免交互事件直接掏空山门。
            let defectors = if members.len() > 1 {
                members
                    .iter()
                    .filter(|disciple| defection_eligible(disciple))
                    .count()
            } else {
                0
            };
            let conscriptable = if members.len() > 1 {
                members
                    .iter()
                    .filter(|disciple| !is_npc_leader(disciple))
                    .count()
            } else {
                0
            };
            Faction {
                slot,
                id: sect.id.clone(),
                name: sect.name.clone(),
                country: sect.country_id.clone(),
                policy: sect.policy.clone(),
                prestige: sect.attributes.prestige,
                silver: sect.attributes.silver,
                morality: sect.attributes.morality,
                #[cfg(test)]
                members: members.len(),
                defectors,
                conscriptable,
            }
        })
        .collect()
}

fn pair_candidates(state: &GameState, a: &Faction, b: &Faction) -> Vec<(InteractionKind, u32)> {
    if is_court(a) && is_court(b) {
        return vec![(InteractionKind::CourtClash, 180)];
    }

    // 双向关系达到七十即视为会盟。盟友走独立候选池，不能在同一份盟约仍
    // 有效时被普通国别、边境或叛逃候选绕过保护；低道德背盟会先真实撕毁
    // 双向关系，后续月份才重新进入普通互动池。
    if !is_court(a) && !is_court(b) && mutual_relation(state, a, b) >= 70 {
        return allied_pair_candidates(state, a, b);
    }

    let mut kinds = Vec::new();
    if a.id == "court" || b.id == "court" {
        kinds.push((InteractionKind::YuanCourt, 60));
    }
    if a.id == "song_court" || b.id == "song_court" {
        kinds.push((InteractionKind::SongCourt, 60));
    }
    if (is_court(a) || is_court(b))
        && (if is_court(a) {
            b.conscriptable
        } else {
            a.conscriptable
        }) > 0
    {
        kinds.push((InteractionKind::CourtConscription, 38));
    }
    if countries_are(a, b, "yuan", "song") {
        kinds.push((InteractionKind::YuanSongWar, 34));
    }
    if a.country != b.country && (a.country == "dali" || b.country == "dali") {
        kinds.push((InteractionKind::DaliExchange, 24));
    }
    if a.country != b.country && (a.country == "xia" || b.country == "xia") {
        kinds.push((InteractionKind::XiaTrade, 18));
        let xia_morality = if a.country == "xia" {
            a.morality
        } else {
            b.morality
        };
        kinds.push((
            InteractionKind::XiaAssassination,
            (8 + (50 - xia_morality).max(0) / 3) as u32,
        ));
    }

    if has_sparring_member(state, a.slot) && has_sparring_member(state, b.slot) {
        let gap = (a.morality - b.morality).abs();
        kinds.push((InteractionKind::Spar, (8 + gap / 3) as u32));
    }
    if is_trade_pair(a, b) {
        kinds.push((
            InteractionKind::Trade,
            (18 + (a.prestige + b.prestige).max(0) / 30) as u32,
        ));
    }
    if a.country == b.country {
        let low_morality = a.morality.min(b.morality);
        kinds.push((
            InteractionKind::BorderConflict,
            (8 + (60 - low_morality).max(0) / 2) as u32,
        ));
        if (a.morality - b.morality).abs() < 30 {
            kinds.push((InteractionKind::JointBandits, 20));
        }
    }
    let relation = mutual_relation(state, a, b);
    if !is_court(a) && !is_court(b) && (45..70).contains(&relation) {
        kinds.push((InteractionKind::Alliance, (20 + relation - 45) as u32));
    }
    if a.defectors > 0 || b.defectors > 0 {
        let attraction = defection_attraction(a, b).max(defection_attraction(b, a));
        kinds.push((InteractionKind::Defection, (6 + attraction / 4) as u32));
    }
    kinds
}

fn allied_pair_candidates(
    state: &GameState,
    a: &Faction,
    b: &Faction,
) -> Vec<(InteractionKind, u32)> {
    let mut kinds = vec![(InteractionKind::Trade, 28)];
    if has_sparring_member(state, a.slot) && has_sparring_member(state, b.slot) {
        kinds.push((InteractionKind::Spar, 14));
        kinds.push((InteractionKind::AlliedJointPatrol, 36));
    }
    if has_allied_rescue(state, a, b) {
        kinds.push((InteractionKind::AlliedRescue, 24));
    }
    if alliance_betrayal_possible(a, b) {
        let bad_morality = a.morality.min(b.morality);
        kinds.push((
            InteractionKind::AllianceBetrayal,
            (8 + (35 - bad_morality).max(0)) as u32,
        ));
    }
    kinds
}

fn apply_interaction(
    rng: &mut impl Rng,
    state: &mut GameState,
    a: &Faction,
    b: &Faction,
    kind: InteractionKind,
) -> Option<(String, &'static str)> {
    match kind {
        InteractionKind::Spar => spar(rng, state, a, b),
        InteractionKind::Trade => Some(trade(state, a, b)),
        InteractionKind::BorderConflict => Some(border_conflict(state, a, b)),
        InteractionKind::JointBandits => Some(joint_bandits(state, a, b)),
        InteractionKind::Alliance => Some(alliance(state, a, b)),
        InteractionKind::AlliedJointPatrol => allied_joint_patrol(rng, state, a, b),
        InteractionKind::AlliedRescue => allied_rescue(rng, state, a, b),
        InteractionKind::AllianceBetrayal => alliance_betrayal(state, a, b),
        InteractionKind::Defection => defection(rng, state, a, b),
        InteractionKind::CourtConscription => court_conscription(rng, state, a, b),
        InteractionKind::YuanSongWar => Some(yuan_song_war(rng, state, a, b)),
        InteractionKind::DaliExchange => Some(dali_exchange(state, a, b)),
        InteractionKind::XiaTrade => Some(xia_trade(state, a, b)),
        InteractionKind::XiaAssassination => Some(xia_assassination(state, a, b)),
        InteractionKind::YuanCourt => Some(yuan_court(rng, state, a, b)),
        InteractionKind::SongCourt => Some(song_court(rng, state, a, b)),
        InteractionKind::CourtClash => Some(court_clash(rng, state, a, b)),
    }
}

fn spar(
    rng: &mut impl Rng,
    state: &mut GameState,
    a: &Faction,
    b: &Faction,
) -> Option<(String, &'static str)> {
    let left = choose_sparring_member(rng, state, a.slot)?;
    let right = choose_sparring_member(rng, state, b.slot)?;
    let left_art = spar_art(&left)?;
    let right_art = spar_art(&right)?;
    let left_score = combat_score(&left) + rng.gen_range(0..=80);
    let right_score = combat_score(&right) + rng.gen_range(0..=80);
    let left_wins = left_score >= right_score;
    let (winner, loser) = if left_wins { (a, b) } else { (b, a) };
    let left_research_cap = sect::martial_research_level_cap(sect_ref(state, a.slot), &left_art);
    let right_research_cap = sect::martial_research_level_cap(sect_ref(state, b.slot), &right_art);
    let (left_experience, right_experience, left_qi_cost, right_qi_cost) = if left_wins {
        (12, 7, 5, 8)
    } else {
        (7, 12, 8, 5)
    };
    let applied_left = apply_spar_result(
        state,
        a.slot,
        &left.id,
        &left_art,
        left_experience,
        left_qi_cost,
        left_wins,
        left_research_cap,
    )?;
    let applied_right = apply_spar_result(
        state,
        b.slot,
        &right.id,
        &right_art,
        right_experience,
        right_qi_cost,
        !left_wins,
        right_research_cap,
    )?;
    change_prestige(state, winner.slot, 2);
    change_prestige(state, loser.slot, -1);
    change_relation(state, a, b, 1);
    let (winner_name, winner_art, winner_experience, loser_name, loser_art, loser_experience) =
        if left_wins {
            (
                left.name.as_str(),
                left_art.as_str(),
                applied_left,
                right.name.as_str(),
                right_art.as_str(),
                applied_right,
            )
        } else {
            (
                right.name.as_str(),
                right_art.as_str(),
                applied_right,
                left.name.as_str(),
                left_art.as_str(),
                applied_left,
            )
        };
    Some((
        format!(
            "{}弟子{}与{}弟子{}切磋，{}以{}胜过{}，双方点到为止；{}{}经验 +{}，{}{}经验 +{}，{}个人声望 +1。",
            a.name,
            left.name,
            b.name,
            right.name,
            winner_name,
            martial_name(winner_art),
            loser_name,
            winner_name,
            martial_name(winner_art),
            winner_experience,
            loser_name,
            martial_name(loser_art),
            loser_experience,
            winner_name,
        ),
        "neutral",
    ))
}

pub(crate) fn spar_art(disciple: &Disciple) -> Option<String> {
    let is_known_combat = |id: &str| {
        disciple.martial_progress.proficiencies.contains_key(id)
            && martial_art_by_id(id).is_some_and(|art| art.is_combat)
    };
    if is_known_combat(&disciple.martial_art) {
        return Some(canonical_skill_id(&disciple.martial_art));
    }
    disciple
        .prepared_skills
        .values()
        .filter(|id| is_known_combat(id))
        .chain(
            disciple
                .martial_progress
                .proficiencies
                .keys()
                .filter(|id| is_known_combat(id)),
        )
        .max_by_key(|id| {
            disciple
                .martial_progress
                .proficiencies
                .get(*id)
                .map(|progress| progress.level)
                .unwrap_or(0)
        })
        .map(|id| canonical_skill_id(id))
}

fn apply_spar_result(
    state: &mut GameState,
    slot: usize,
    disciple_id: &str,
    art_id: &str,
    experience: i64,
    qi_cost: i32,
    won: bool,
    research_cap: Option<i32>,
) -> Option<i64> {
    let disciple = member_mut(state, slot, disciple_id)?;
    if disciple.attributes.qi.current > 0 {
        disciple.attributes.qi.current = disciple
            .attributes
            .qi
            .current
            .saturating_sub(qi_cost)
            .max(1);
    }
    let applied_experience = gain_spar_experience(disciple, art_id, experience, research_cap);
    if won {
        disciple.attributes.reputation = disciple
            .attributes
            .reputation
            .saturating_add(1)
            .clamp(0, 1000);
    }
    disciple::sync_legacy_attributes(disciple);
    Some(applied_experience)
}

pub(crate) fn gain_spar_experience(
    disciple: &mut Disciple,
    art_id: &str,
    experience: i64,
    research_cap: Option<i32>,
) -> i64 {
    let art_id = canonical_skill_id(art_id);
    let before = total_skill_experience(disciple, &art_id);
    disciple::gain_skill_experience_with_cap(disciple, &art_id, experience, research_cap);
    let after = total_skill_experience(disciple, &art_id);
    (after - before).clamp(0, i128::from(i64::MAX)) as i64
}

fn total_skill_experience(disciple: &Disciple, art_id: &str) -> i128 {
    let Some(progress) = disciple
        .martial_progress
        .proficiencies
        .get(&canonical_skill_id(art_id))
    else {
        return 0;
    };
    let level = i128::from(progress.level.max(0));
    level
        .saturating_mul(level + 1)
        .saturating_mul(level.saturating_mul(2) + 1)
        / 6
        + i128::from(progress.experience.max(0))
}

fn martial_name(art_id: &str) -> String {
    martial_art_by_id(art_id)
        .map(|art| format!("《{}》", art.name))
        .unwrap_or_else(|| format!("《{}》", art_id))
}

fn trade(state: &mut GameState, a: &Faction, b: &Faction) -> (String, &'static str) {
    let income = 15 + (a.prestige + b.prestige).max(0) / 25;
    change_silver(state, a.slot, income);
    change_silver(state, b.slot, income);
    change_relation(state, a, b, 3);
    adjust_countries_once(state, [a.country.as_str(), b.country.as_str()], 1, 0);
    (
        format!(
            "{}商队与{}互通货殖，双方各获利{}两。",
            a.name, b.name, income
        ),
        "good",
    )
}

fn border_conflict(state: &mut GameState, a: &Faction, b: &Faction) -> (String, &'static str) {
    let (strong, weak) = if (a.prestige, a.morality) >= (b.prestige, b.morality) {
        (a, b)
    } else {
        (b, a)
    };
    change_prestige(state, strong.slot, 1);
    change_prestige(state, weak.slot, -2);
    change_relation(state, a, b, -5);
    adjust_countries_once(state, [a.country.as_str(), b.country.as_str()], 0, -1);
    (
        format!("{}仗势侵占{}地界，后者被迫退让。", strong.name, weak.name),
        "bad",
    )
}

fn joint_bandits(state: &mut GameState, a: &Faction, b: &Faction) -> (String, &'static str) {
    for faction in [a, b] {
        change_prestige(state, faction.slot, 1);
        change_morality(state, faction.slot, 1);
    }
    change_relation(state, a, b, 4);
    adjust_countries_once(state, [a.country.as_str(), b.country.as_str()], 0, 1);
    (
        format!("{}与{}联手清剿邻境悍匪，两派侠名俱增。", a.name, b.name),
        "good",
    )
}

fn alliance(state: &mut GameState, a: &Faction, b: &Faction) -> (String, &'static str) {
    // 现有模型没有独立盟约表；双向关系达到 70 即作为持久联盟语义，
    // 后续冲突仍可真实破坏这份关系，而非只留下一条日志。
    set_relation_minimum(state, a, b, 70);
    change_prestige(state, a.slot, 2);
    change_prestige(state, b.slot, 2);
    (
        format!("{}与{}于山门会盟立誓，自此互为援手。", a.name, b.name),
        "good",
    )
}

const ALLIED_RESCUE_COST: i32 = 25;

/// 盟友联合巡行使用双方真实在门战力，并在本次判定结束后才把经验落回人物。
/// 气血最低保留一点，既体现实战消耗，也保证这类友军行动本身不会杀死人。
fn allied_joint_patrol(
    rng: &mut impl Rng,
    state: &mut GameState,
    a: &Faction,
    b: &Faction,
) -> Option<(String, &'static str)> {
    let left = choose_sparring_member(rng, state, a.slot)?;
    let right = choose_sparring_member(rng, state, b.slot)?;
    let left_art = spar_art(&left)?;
    let right_art = spar_art(&right)?;
    let allied_score = combat_score(&left)
        .saturating_add(combat_score(&right))
        .saturating_add(rng.gen_range(0..=120));
    let threat = 240_i32
        .saturating_add((a.prestige + b.prestige).clamp(0, 400) / 2)
        .saturating_add(rng.gen_range(0..=160));
    let succeeded = allied_score >= threat;
    let (experience, qi_cost, personal_reputation) =
        if succeeded { (14, 7, 2) } else { (8, 14, 1) };
    let left_cap = sect::martial_research_level_cap(sect_ref(state, a.slot), &left_art);
    let right_cap = sect::martial_research_level_cap(sect_ref(state, b.slot), &right_art);
    let applied_left = apply_allied_field_result(
        state,
        a.slot,
        &left.id,
        &left_art,
        experience,
        qi_cost,
        personal_reputation,
        left_cap,
    )?;
    let applied_right = apply_allied_field_result(
        state,
        b.slot,
        &right.id,
        &right_art,
        experience,
        qi_cost,
        personal_reputation,
        right_cap,
    )?;

    let (silver_delta, prestige_delta, relation_delta, prosperity_delta, order_delta) = if succeeded
    {
        (35, 3, 5, 1, 2)
    } else {
        (-12, -1, 1, -1, -1)
    };
    let left_silver_before = sect_ref(state, a.slot).attributes.silver;
    let right_silver_before = sect_ref(state, b.slot).attributes.silver;
    for faction in [a, b] {
        change_silver(state, faction.slot, silver_delta);
        change_prestige(state, faction.slot, prestige_delta);
    }
    let left_silver_spent = (left_silver_before - sect_ref(state, a.slot).attributes.silver).max(0);
    let right_silver_spent =
        (right_silver_before - sect_ref(state, b.slot).attributes.silver).max(0);
    change_relation(state, a, b, relation_delta);
    adjust_countries_once(
        state,
        [a.country.as_str(), b.country.as_str()],
        prosperity_delta,
        order_delta,
    );

    let text = if succeeded {
        format!(
            "{}与{}依盟约联合巡行，{}弟子{}携{}、{}弟子{}施{}合力剿灭悍匪；{}实际经验 +{}，{}实际经验 +{}，二人个人声名各 +{}，两派各获银35两、声望 +3。",
            a.name,
            b.name,
            a.name,
            left.name,
            martial_name(&left_art),
            b.name,
            right.name,
            martial_name(&right_art),
            martial_name(&left_art),
            applied_left,
            martial_name(&right_art),
            applied_right,
            personal_reputation,
        )
    } else {
        format!(
            "{}与{}依盟约联合巡行，{}弟子{}和{}弟子{}遭悍匪伏击，二人气血各耗{}点但并无性命之忧；{}实际经验 +{}，{}实际经验 +{}，{}实耗银{}两、{}实耗银{}两。",
            a.name,
            b.name,
            a.name,
            left.name,
            b.name,
            right.name,
            qi_cost,
            martial_name(&left_art),
            applied_left,
            martial_name(&right_art),
            applied_right,
            a.name,
            left_silver_spent,
            b.name,
            right_silver_spent,
        )
    };
    Some((text, if succeeded { "good" } else { "bad" }))
}

#[allow(clippy::too_many_arguments)]
fn apply_allied_field_result(
    state: &mut GameState,
    slot: usize,
    disciple_id: &str,
    art_id: &str,
    experience: i64,
    qi_cost: i32,
    reputation_gain: i32,
    research_cap: Option<i32>,
) -> Option<i64> {
    let disciple = member_mut(state, slot, disciple_id)?;
    disciple.attributes.qi.current = disciple
        .attributes
        .qi
        .current
        .saturating_sub(qi_cost)
        .max(1);
    let applied_experience = gain_spar_experience(disciple, art_id, experience, research_cap);
    disciple.attributes.reputation = disciple
        .attributes
        .reputation
        .saturating_add(reputation_gain)
        .clamp(0, 1000);
    disciple::sync_legacy_attributes(disciple);
    Some(applied_experience)
}

fn allied_rescue(
    rng: &mut impl Rng,
    state: &mut GameState,
    a: &Faction,
    b: &Faction,
) -> Option<(String, &'static str)> {
    let directions = [(a, b), (b, a)]
        .into_iter()
        .filter(|(rescuer, target)| can_rescue(state, rescuer, target))
        .collect::<Vec<_>>();
    if directions.is_empty() {
        return None;
    }
    let (rescuer_faction, target_faction) = directions[rng.gen_range(0..directions.len())];
    let rescuer = choose_sparring_member(rng, state, rescuer_faction.slot)?;
    let target = choose_in_transit_member(rng, state, target_faction.slot)?;
    let target_member = member_mut(state, target_faction.slot, &target.id)?;
    let new_remaining = target_member.away_months.saturating_sub(1).max(1);
    target_member.away_months = new_remaining;
    let plan = target_member.action.as_mut()?;
    plan.remaining_months = new_remaining;

    change_silver(state, rescuer_faction.slot, -ALLIED_RESCUE_COST);
    change_prestige(state, rescuer_faction.slot, 1);
    change_relation(state, rescuer_faction, target_faction, 4);
    Some((
        format!(
            "{}闻盟友途中有难，遣弟子{}驰援{}在途弟子{}，耗银{}两；{}的行程与任务同步缩短一月，尚余{}月归山。",
            rescuer_faction.name,
            rescuer.name,
            target_faction.name,
            target.name,
            ALLIED_RESCUE_COST,
            target.name,
            new_remaining,
        ),
        "good",
    ))
}

fn alliance_betrayal(
    state: &mut GameState,
    a: &Faction,
    b: &Faction,
) -> Option<(String, &'static str)> {
    let (betrayer, victim) = betrayal_parties(a, b);
    if betrayer.morality >= 35 {
        return None;
    }
    let stolen = sect_ref(state, victim.slot).attributes.silver.clamp(0, 30);
    if stolen == 0 {
        return None;
    }
    change_silver(state, victim.slot, -stolen);
    change_silver(state, betrayer.slot, stolen);
    change_relation(state, betrayer, victim, -35);
    change_prestige(state, betrayer.slot, -6);
    change_prestige(state, victim.slot, 1);
    change_morale(state, betrayer.slot, -4);
    change_morale(state, victim.slot, -6);
    adjust_countries_once(
        state,
        [betrayer.country.as_str(), victim.country.as_str()],
        -1,
        -2,
    );
    Some((
        format!(
            "{}利令智昏公然背盟，劫走{}库银{}两；两派盟誓破裂，{}声望受损、双方士气大跌。",
            betrayer.name, victim.name, stolen, betrayer.name
        ),
        "bad",
    ))
}

fn defection(
    rng: &mut impl Rng,
    state: &mut GameState,
    a: &Faction,
    b: &Faction,
) -> Option<(String, &'static str)> {
    let toward_b = defection_attraction(a, b);
    let toward_a = defection_attraction(b, a);
    let (source, target) = if a.defectors == 0 {
        (b, a)
    } else if b.defectors == 0 {
        (a, b)
    } else if rng.gen_range(0..toward_a + toward_b + 2) < toward_b + 1 {
        (a, b)
    } else {
        (b, a)
    };
    let member = choose_defector(rng, state, source.slot)?;
    let mut disciple = take_member(state, source.slot, &member.id)?;
    disciple.sect_id = Some(target.id.clone());
    disciple.loyalty = 50;
    disciple.attributes.sect_loyalty = 50;
    disciple.department = None;
    disciple.master_id = None;
    disciple.action = None;
    assign_incoming_rank(state, target, &mut disciple);
    put_member(state, target.slot, disciple);
    change_prestige(state, source.slot, -2);
    change_prestige(state, target.slot, 1);
    change_relation(state, source, target, -8);
    Some((
        format!(
            "{}弟子{}叛逃，转投{}门下。",
            source.name, member.name, target.name
        ),
        "bad",
    ))
}

fn court_conscription(
    rng: &mut impl Rng,
    state: &mut GameState,
    a: &Faction,
    b: &Faction,
) -> Option<(String, &'static str)> {
    let (court, source) = if is_court(a) { (a, b) } else { (b, a) };
    let selected = choose_member(rng, state, source.slot, true)?;
    let mut disciple = take_member(state, source.slot, &selected.id)?;
    disciple.sect_id = Some(court.id.clone());
    disciple.loyalty = if court.id == "court" { 55 } else { 70 };
    disciple.attributes.sect_loyalty = disciple.loyalty;
    disciple.department = Some(Department::ExternalAffairs);
    disciple.master_id = None;
    disciple.action = None;
    disciple.away_months = 0;
    put_member(state, court.slot, disciple);

    change_prestige(state, court.slot, 1);
    if court.id == "court" {
        change_prestige(state, source.slot, -1);
        change_relation(state, court, source, -7);
        Some((
            format!(
                "金帐汗国强征{}弟子{}入怯薛军，{}敢怒不敢言。",
                source.name, selected.name, source.name
            ),
            "bad",
        ))
    } else {
        change_prestige(state, source.slot, 1);
        change_relation(state, court, source, 5);
        Some((
            format!(
                "枢密院征辟{}弟子{}入朝听用，{}以忠义相送。",
                source.name, selected.name, source.name
            ),
            "good",
        ))
    }
}

fn yuan_song_war(
    rng: &mut impl Rng,
    state: &mut GameState,
    a: &Faction,
    b: &Faction,
) -> (String, &'static str) {
    let (yuan, song) = if a.country == "yuan" { (a, b) } else { (b, a) };
    let yuan_score = war_score(state, yuan, rng.gen_range(0..=70));
    let song_score = war_score(state, song, rng.gen_range(0..=70));
    let (winner, loser, battle_text) = if yuan_score >= song_score {
        (
            yuan,
            song,
            format!("金帐汗国边骑劫掠{}辖境，{}应战失利。", song.name, song.name),
        )
    } else {
        (
            song,
            yuan,
            format!("大宋北伐军邀{}为前锋，击退{}边骑。", song.name, yuan.name),
        )
    };
    change_prestige(state, winner.slot, 2);
    change_prestige(state, loser.slot, -2);
    change_relation(state, a, b, -6);
    let aftermath = apply_war_aftermath(state, &winner.country, &loser.country);
    (format!("{battle_text}{aftermath}"), "bad")
}

fn dali_exchange(state: &mut GameState, a: &Faction, b: &Faction) -> (String, &'static str) {
    let (dali, guest) = if a.country == "dali" { (a, b) } else { (b, a) };
    change_morality(state, dali.slot, 1);
    change_morality(state, guest.slot, 1);
    change_relation(state, a, b, 4);
    adjust_countries_once(state, [a.country.as_str(), b.country.as_str()], 0, 1);
    (
        format!("大理高僧至{}讲经论武，{}设斋相迎。", guest.name, guest.name),
        "good",
    )
}

fn xia_trade(state: &mut GameState, a: &Faction, b: &Faction) -> (String, &'static str) {
    let (xia, guest) = if a.country == "xia" { (a, b) } else { (b, a) };
    change_silver(state, xia.slot, 25);
    change_silver(state, guest.slot, 20);
    change_relation(state, a, b, 2);
    adjust_countries_once(state, [a.country.as_str(), b.country.as_str()], 1, 0);
    (
        format!("大夏驼队与{}开市通商，奇货沿丝路往来不绝。", guest.name),
        "good",
    )
}

fn xia_assassination(state: &mut GameState, a: &Faction, b: &Faction) -> (String, &'static str) {
    let target = if a.country == "xia" { b } else { a };
    change_prestige(state, target.slot, -2);
    change_relation(state, a, b, -7);
    country::adjust_country(state, &target.country, 0, -2);
    (
        format!(
            "大夏刺客潜入{}驿馆行刺，虽未得手，却令江湖震动。",
            target.name
        ),
        "bad",
    )
}

fn yuan_court(
    rng: &mut impl Rng,
    state: &mut GameState,
    a: &Faction,
    b: &Faction,
) -> (String, &'static str) {
    let other = if a.id == "court" { b } else { a };
    change_relation(state, a, b, -4);
    if rng.gen_bool(0.5) {
        change_prestige(state, other.slot, -5);
        (
            format!("怯薛军向{}颁下征召令，拒命之后声望受损。", other.name),
            "bad",
        )
    } else {
        change_silver(state, other.slot, -30);
        (
            format!("金帐汗国遣使至{}，索要岁币三千两。", other.name),
            "bad",
        )
    }
}

fn song_court(
    rng: &mut impl Rng,
    state: &mut GameState,
    a: &Faction,
    b: &Faction,
) -> (String, &'static str) {
    let other = if a.id == "song_court" { b } else { a };
    change_relation(state, a, b, 5);
    if rng.gen_bool(0.5) {
        change_prestige(state, other.slot, 3);
        (
            format!("枢密院传檄天下，褒奖{}忠义之举。", other.name),
            "good",
        )
    } else {
        change_morality(state, other.slot, 2);
        (
            format!("枢密院招安赐封{}，勉其护境安民。", other.name),
            "good",
        )
    }
}

fn court_clash(
    rng: &mut impl Rng,
    state: &mut GameState,
    a: &Faction,
    b: &Faction,
) -> (String, &'static str) {
    let (yuan, song) = if a.id == "court" { (a, b) } else { (b, a) };
    let yuan_score = war_score_for_country(state, yuan, "yuan", rng.gen_range(0..=80));
    let song_score = war_score_for_country(state, song, "song", rng.gen_range(0..=80));
    let (winner, loser, winner_country, loser_country) = if yuan_score >= song_score {
        (yuan, song, "yuan", "song")
    } else {
        (song, yuan, "song", "yuan")
    };
    change_prestige(state, winner.slot, 3);
    change_prestige(state, loser.slot, -3);
    change_relation(state, a, b, -10);
    let aftermath = apply_war_aftermath(state, winner_country, loser_country);
    (
        format!(
            "怯薛军与枢密院在边关正面交锋，{}得胜，{}败退。{}",
            winner.name, loser.name, aftermath
        ),
        "bad",
    )
}

fn resolve_leader_succession(state: &mut GameState) -> Option<GameEvent> {
    for sect_index in 0..state.npc_sects.len() {
        let sect_id = state.npc_sects[sect_index].id.clone();
        let sect_name = state.npc_sects[sect_index].name.clone();
        let has_living_leader = state.npc_disciples.iter().any(|disciple| {
            disciple.alive
                && disciple.sect_id.as_deref() == Some(sect_id.as_str())
                && is_npc_leader(disciple)
        });
        if has_living_leader {
            continue;
        }

        let elder_ids = state.npc_sects[sect_index]
            .buildings
            .iter()
            .filter_map(|building| building.elder_id.clone())
            .collect::<BTreeSet<_>>();
        // 正常存档严格从在任长老中选拔；仅旧档长老字段残缺时才以现存内门兜底，
        // 避免一个门派永久失去掌门而无法恢复身份。
        let successor_index = succession_candidate_index(state, &sect_id, &elder_ids, true)
            .or_else(|| succession_candidate_index(state, &sect_id, &elder_ids, false));
        let Some(successor_index) = successor_index else {
            continue;
        };

        let predecessor = state
            .npc_disciples
            .iter()
            .find(|disciple| {
                disciple.sect_id.as_deref() == Some(sect_id.as_str()) && is_npc_leader(disciple)
            })
            .map(|disciple| disciple.name.clone());
        for disciple in &mut state.npc_disciples {
            if disciple.sect_id.as_deref() == Some(sect_id.as_str()) && is_npc_leader(disciple) {
                disciple.npc_position = Some("故掌门".into());
            }
        }

        let successor_name = {
            let successor = &mut state.npc_disciples[successor_index];
            successor.npc_position = Some(NpcPosition::SectLeader.display().into());
            successor.rank = DiscipleRank::Inner;
            successor.department = Some(Department::Transmission);
            successor.loyalty = successor.loyalty.max(85);
            successor.attributes.sect_loyalty = successor.attributes.sect_loyalty.max(85);
            successor.master_id = None;
            successor.action = None;
            successor.away_months = 0;
            successor.name.clone()
        };
        let attributes = &mut state.npc_sects[sect_index].attributes;
        attributes.prestige = (attributes.prestige - 1).max(0);
        attributes.morale = (attributes.morale - 2).max(0);

        let text = predecessor.map_or_else(
            || {
                format!(
                    "{}掌门之位久悬，众长老共推{}继任。",
                    sect_name, successor_name
                )
            },
            |name| {
                format!(
                    "{}掌门{}身故，长老{}继任掌门。",
                    sect_name, name, successor_name
                )
            },
        );
        return Some(GameEvent {
            text,
            mood: "neutral".into(),
            year: state.year,
            month: state.month,
            category: "world".into(),
        });
    }
    None
}

fn succession_candidate_index(
    state: &GameState,
    sect_id: &str,
    elder_ids: &BTreeSet<String>,
    elders_only: bool,
) -> Option<usize> {
    state
        .npc_disciples
        .iter()
        .enumerate()
        .filter(|(_, disciple)| {
            disciple.alive
                && disciple.sect_id.as_deref() == Some(sect_id)
                && disciple.rank == DiscipleRank::Inner
                && (!elders_only
                    || elder_ids.contains(&disciple.id)
                    || disciple.npc_position.as_deref() == Some(NpcPosition::Elder.display()))
        })
        .max_by_key(|(_, disciple)| {
            (
                disciple.merit,
                combat_score(disciple),
                disciple.attributes.reputation,
            )
        })
        .map(|(index, _)| index)
}

fn faction_members(state: &GameState, slot: usize) -> Vec<&Disciple> {
    if slot == 0 {
        state
            .disciples
            .iter()
            .filter(|disciple| disciple.alive)
            .collect()
    } else {
        let sect_id = &state.npc_sects[slot - 1].id;
        state
            .npc_disciples
            .iter()
            .filter(|disciple| {
                disciple.alive && disciple.sect_id.as_deref() == Some(sect_id.as_str())
            })
            .collect()
    }
}

fn choose_member(
    rng: &mut impl Rng,
    state: &GameState,
    slot: usize,
    defecting: bool,
) -> Option<Disciple> {
    let members: Vec<&Disciple> = faction_members(state, slot)
        .into_iter()
        .filter(|disciple| !defecting || !is_npc_leader(disciple))
        .collect();
    (!members.is_empty()).then(|| members[rng.gen_range(0..members.len())].clone())
}

fn defection_eligible(disciple: &Disciple) -> bool {
    !is_npc_leader(disciple) && disciple.attributes.sect_loyalty < 60
}

fn choose_defector(rng: &mut impl Rng, state: &GameState, slot: usize) -> Option<Disciple> {
    let members = faction_members(state, slot)
        .into_iter()
        .filter(|candidate| defection_eligible(candidate))
        .collect::<Vec<_>>();
    (!members.is_empty()).then(|| members[rng.gen_range(0..members.len())].clone())
}

fn has_sparring_member(state: &GameState, slot: usize) -> bool {
    faction_members(state, slot)
        .into_iter()
        .any(|candidate| disciple::can_act(candidate) && spar_art(candidate).is_some())
}

fn choose_sparring_member(rng: &mut impl Rng, state: &GameState, slot: usize) -> Option<Disciple> {
    let members = faction_members(state, slot)
        .into_iter()
        .filter(|candidate| disciple::can_act(candidate) && spar_art(candidate).is_some())
        .collect::<Vec<_>>();
    (!members.is_empty()).then(|| members[rng.gen_range(0..members.len())].clone())
}

fn has_in_transit_member(state: &GameState, slot: usize) -> bool {
    faction_members(state, slot)
        .into_iter()
        .any(|candidate| candidate.away_months > 1 && candidate.action.is_some())
}

fn choose_in_transit_member(
    rng: &mut impl Rng,
    state: &GameState,
    slot: usize,
) -> Option<Disciple> {
    let members = faction_members(state, slot)
        .into_iter()
        .filter(|candidate| candidate.away_months > 1 && candidate.action.is_some())
        .collect::<Vec<_>>();
    (!members.is_empty()).then(|| members[rng.gen_range(0..members.len())].clone())
}

fn can_rescue(state: &GameState, rescuer: &Faction, target: &Faction) -> bool {
    sect_ref(state, rescuer.slot).attributes.silver >= ALLIED_RESCUE_COST
        && has_sparring_member(state, rescuer.slot)
        && has_in_transit_member(state, target.slot)
}

fn has_allied_rescue(state: &GameState, a: &Faction, b: &Faction) -> bool {
    can_rescue(state, a, b) || can_rescue(state, b, a)
}

fn betrayal_parties<'a>(a: &'a Faction, b: &'a Faction) -> (&'a Faction, &'a Faction) {
    if a.morality < b.morality || (a.morality == b.morality && a.id <= b.id) {
        (a, b)
    } else {
        (b, a)
    }
}

fn alliance_betrayal_possible(a: &Faction, b: &Faction) -> bool {
    let (betrayer, victim) = betrayal_parties(a, b);
    betrayer.morality < 35 && victim.silver > 0
}

fn member_mut<'a>(
    state: &'a mut GameState,
    slot: usize,
    disciple_id: &str,
) -> Option<&'a mut Disciple> {
    let members = if slot == 0 {
        &mut state.disciples
    } else {
        &mut state.npc_disciples
    };
    members
        .iter_mut()
        .find(|disciple| disciple.id == disciple_id)
}

fn is_npc_leader(disciple: &Disciple) -> bool {
    disciple.npc_position.as_deref() == Some(NpcPosition::SectLeader.display())
        || disciple
            .sect_id
            .as_deref()
            .is_some_and(|sect_id| disciple.id == format!("npc_{sect_id}_1"))
}

fn take_member(state: &mut GameState, slot: usize, id: &str) -> Option<Disciple> {
    let members = if slot == 0 {
        &mut state.disciples
    } else {
        &mut state.npc_disciples
    };
    members
        .iter()
        .position(|disciple| disciple.id == id)
        .map(|index| members.remove(index))
}

fn put_member(state: &mut GameState, slot: usize, disciple: Disciple) {
    if slot == 0 {
        state.disciples.push(disciple);
    } else {
        state.npc_disciples.push(disciple);
    }
}

/// 叛投者虽保留武学与功绩，却不能把原门派品秩一并带入新山门。
/// 依目标门派现有人数重新补入外门名额，名额已满时从杂役做起。
fn assign_incoming_rank(state: &GameState, target: &Faction, disciple: &mut Disciple) {
    let existing = if target.slot == 0 {
        state
            .disciples
            .iter()
            .filter(|member| member.sect_id.as_deref() == Some(target.id.as_str()))
            .cloned()
            .collect::<Vec<_>>()
    } else {
        state
            .npc_disciples
            .iter()
            .filter(|member| member.sect_id.as_deref() == Some(target.id.as_str()))
            .cloned()
            .collect::<Vec<_>>()
    };
    crate::logic::sect::assign_recruit_ranks(
        sect_ref(state, target.slot),
        &existing,
        std::slice::from_mut(disciple),
    );
}

fn combat_score(disciple: &Disciple) -> i32 {
    (disciple.attributes.attainment / 20).clamp(0, i64::from(i32::MAX)) as i32
        + disciple.talent
        + disciple.attributes.neili.maximum
        + disciple.attributes.reputation
}

fn defection_attraction(source: &Faction, target: &Faction) -> i32 {
    ((target.prestige - source.prestige).max(0) / 2
        + (target.morality - source.morality).max(0)
        + (50 - source.morality).max(0))
    .max(1)
}

fn is_trade_pair(a: &Faction, b: &Faction) -> bool {
    (a.policy == SectPolicy::Mercantile && is_producer(b))
        || (b.policy == SectPolicy::Mercantile && is_producer(a))
}

fn is_producer(faction: &Faction) -> bool {
    matches!(
        faction.policy,
        SectPolicy::Scholarly | SectPolicy::Reclusive | SectPolicy::Balanced
    ) || faction.silver < 500
}

fn is_court(faction: &Faction) -> bool {
    matches!(faction.id.as_str(), "court" | "song_court")
}

fn countries_are(a: &Faction, b: &Faction, left: &str, right: &str) -> bool {
    (a.country == left && b.country == right) || (a.country == right && b.country == left)
}

fn is_national_war(kind: InteractionKind) -> bool {
    matches!(
        kind,
        InteractionKind::YuanSongWar | InteractionKind::CourtClash
    )
}

fn adjust_countries_once<'a>(
    state: &mut GameState,
    country_ids: impl IntoIterator<Item = &'a str>,
    prosperity_delta: i32,
    order_delta: i32,
) {
    let country_ids = country_ids
        .into_iter()
        .map(str::to_owned)
        .collect::<BTreeSet<_>>();
    for country_id in country_ids {
        country::adjust_country(state, &country_id, prosperity_delta, order_delta);
    }
}

fn war_score(state: &GameState, faction: &Faction, random_roll: i32) -> i32 {
    war_score_for_country(state, faction, &faction.country, random_roll)
}

fn war_score_for_country(
    state: &GameState,
    faction: &Faction,
    country_id: &str,
    random_roll: i32,
) -> i32 {
    faction.prestige + country::war_support(state, country_id) + random_roll
}

fn apply_war_aftermath(
    state: &mut GameState,
    winner_country_id: &str,
    loser_country_id: &str,
) -> String {
    let winner_name = country_name(state, winner_country_id);
    let loser_name = country_name(state, loser_country_id);
    let winner_delta = country::adjust_country(state, winner_country_id, -1, 1);
    let loser_delta = country::adjust_country(state, loser_country_id, -2, -2);
    format!(
        "兵燹波及四境：{}；{}。",
        describe_country_change(&winner_name, winner_delta),
        describe_country_change(&loser_name, loser_delta)
    )
}

fn country_name(state: &GameState, country_id: &str) -> String {
    country::find_country(&state.countries, country_id)
        .map(|country| country.name.clone())
        .unwrap_or_else(|| country_id.to_owned())
}

fn describe_country_change(name: &str, delta: (i32, i32)) -> String {
    format!(
        "{}繁荣{}、治安{}",
        name,
        signed_delta(delta.0),
        signed_delta(delta.1)
    )
}

fn signed_delta(delta: i32) -> String {
    match delta.cmp(&0) {
        std::cmp::Ordering::Greater => format!("＋{delta}"),
        std::cmp::Ordering::Less => format!("－{}", delta.saturating_abs()),
        std::cmp::Ordering::Equal => "不变".into(),
    }
}

fn ordered_pair(a: &str, b: &str) -> (String, String) {
    if a <= b {
        (a.into(), b.into())
    } else {
        (b.into(), a.into())
    }
}

fn sect_ref(state: &GameState, slot: usize) -> &SectState {
    if slot == 0 {
        &state.sect
    } else {
        &state.npc_sects[slot - 1]
    }
}

fn sect_mut(state: &mut GameState, slot: usize) -> &mut SectState {
    if slot == 0 {
        &mut state.sect
    } else {
        &mut state.npc_sects[slot - 1]
    }
}

fn mutual_relation(state: &GameState, a: &Faction, b: &Faction) -> i32 {
    let left = sect_ref(state, a.slot)
        .relations
        .get(&b.id)
        .copied()
        .unwrap_or(0);
    let right = sect_ref(state, b.slot)
        .relations
        .get(&a.id)
        .copied()
        .unwrap_or(0);
    left.min(right)
}

fn change_prestige(state: &mut GameState, slot: usize, delta: i32) {
    let attributes = &mut sect_mut(state, slot).attributes;
    attributes.prestige = (attributes.prestige + delta).clamp(0, 1000);
}

fn change_silver(state: &mut GameState, slot: usize, delta: i32) {
    let attributes = &mut sect_mut(state, slot).attributes;
    attributes.silver = (attributes.silver + delta).max(0);
}

fn change_morality(state: &mut GameState, slot: usize, delta: i32) {
    let attributes = &mut sect_mut(state, slot).attributes;
    attributes.morality = (attributes.morality + delta).clamp(0, 100);
}

fn change_morale(state: &mut GameState, slot: usize, delta: i32) {
    let attributes = &mut sect_mut(state, slot).attributes;
    attributes.morale = (attributes.morale + delta).clamp(0, 100);
}

fn change_relation(state: &mut GameState, a: &Faction, b: &Faction, delta: i32) {
    let left = sect_mut(state, a.slot)
        .relations
        .entry(b.id.clone())
        .or_default();
    *left = (*left + delta).clamp(-100, 100);
    let right = sect_mut(state, b.slot)
        .relations
        .entry(a.id.clone())
        .or_default();
    *right = (*right + delta).clamp(-100, 100);
}

fn set_relation_minimum(state: &mut GameState, a: &Faction, b: &Faction, minimum: i32) {
    let left = sect_mut(state, a.slot)
        .relations
        .entry(b.id.clone())
        .or_default();
    *left = (*left).max(minimum).clamp(-100, 100);
    let right = sect_mut(state, b.slot)
        .relations
        .entry(a.id.clone())
        .or_default();
    *right = (*right).max(minimum).clamp(-100, 100);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::logic::world::generate_npc_world;
    use rand::{rngs::StdRng, SeedableRng};

    fn world() -> GameState {
        let mut state = GameState::default();
        let (sects, disciples) = generate_npc_world(41);
        state.npc_sects = sects;
        state.npc_disciples = disciples;
        state
    }

    fn faction(state: &GameState, id: &str) -> Faction {
        faction_snapshot(state)
            .into_iter()
            .find(|faction| faction.id == id)
            .unwrap()
    }

    fn make_allies(state: &mut GameState, left_id: &str, right_id: &str, relation: i32) {
        let left = faction(state, left_id);
        let right = faction(state, right_id);
        sect_mut(state, left.slot)
            .relations
            .insert(right.id.clone(), relation);
        sect_mut(state, right.slot)
            .relations
            .insert(left.id.clone(), relation);
    }

    #[test]
    fn monthly_interactions_always_emit_three_to_five_world_events() {
        let mut state = world();
        let mut rng = StdRng::seed_from_u64(7);
        for _ in 0..24 {
            let events = run_monthly_interactions(&mut rng, &mut state);
            assert!((3..=5).contains(&events.len()));
            assert!(events.iter().all(|event| event.category == "world"));
            assert!(
                events
                    .iter()
                    .filter(|event| event.text.contains("兵燹波及四境"))
                    .count()
                    <= 1
            );
        }
    }

    #[test]
    fn monthly_interactions_limit_national_war_without_reducing_event_count() {
        let mut observed_war = false;
        for seed in 0..64 {
            let mut state = world();
            let mut rng = StdRng::seed_from_u64(seed);
            let events = run_monthly_interactions(&mut rng, &mut state);
            let war_count = events
                .iter()
                .filter(|event| event.text.contains("兵燹波及四境"))
                .count();
            observed_war |= war_count == 1;
            assert!(war_count <= 1, "seed {seed} 结算了多次全国战事");
            assert!(
                (3..=5).contains(&events.len()),
                "seed {seed} 未由普通互动补足纪事"
            );
        }
        assert!(observed_war);
    }

    #[test]
    fn away_or_incapacitated_factions_do_not_offer_a_sparring_event() {
        let mut state = world();
        for disciple in &mut state.npc_disciples {
            if disciple.sect_id.as_deref() == Some("wudang") {
                disciple.away_months = 1;
            }
        }
        let factions = faction_snapshot(&state);
        let wudang = factions
            .iter()
            .find(|faction| faction.id == "wudang")
            .unwrap();
        let huashan = factions
            .iter()
            .find(|faction| faction.id == "huashan")
            .unwrap();
        let candidates = pair_candidates(&state, wudang, huashan);
        let mut rng = StdRng::seed_from_u64(702);

        assert!(candidates
            .iter()
            .all(|(kind, _)| !matches!(kind, InteractionKind::Spar)));
        assert!(choose_sparring_member(&mut rng, &state, wudang.slot).is_none());
        assert!(choose_sparring_member(&mut rng, &state, huashan.slot).is_some());
    }

    #[test]
    fn national_wars_include_country_support_and_apply_exact_aftermath() {
        use rand::rngs::mock::StepRng;

        let mut state = world();
        for country in &mut state.countries {
            match country.id.as_str() {
                "song" => {
                    country.prosperity = 100;
                    country.order = 90;
                }
                "yuan" => {
                    country.prosperity = 10;
                    country.order = 10;
                }
                _ => {}
            }
        }
        for sect_id in ["wudang", "mingjiao", "court", "song_court"] {
            state
                .npc_sects
                .iter_mut()
                .find(|sect| sect.id == sect_id)
                .unwrap()
                .attributes
                .prestige = 100;
        }
        let factions = faction_snapshot(&state);
        let song = factions
            .iter()
            .find(|faction| faction.id == "wudang")
            .unwrap()
            .clone();
        let yuan = factions
            .iter()
            .find(|faction| faction.id == "mingjiao")
            .unwrap()
            .clone();
        assert!(war_score(&state, &song, 0) > war_score(&state, &yuan, 0));

        let mut rng = StepRng::new(0, 0);
        let (text, _) = yuan_song_war(&mut rng, &mut state, &yuan, &song);
        assert!(text.contains("大宋繁荣－1、治安＋1"));
        assert!(text.contains("大元繁荣－2、治安－2"));
        assert_eq!(country::country_values(&state, "song"), (99, 91));
        assert_eq!(country::country_values(&state, "yuan"), (8, 8));

        for country in &mut state.countries {
            match country.id.as_str() {
                "song" => {
                    country.prosperity = 90;
                    country.order = 80;
                }
                "yuan" => {
                    country.prosperity = 10;
                    country.order = 10;
                }
                _ => {}
            }
        }
        let factions = faction_snapshot(&state);
        let court = factions
            .iter()
            .find(|faction| faction.id == "court")
            .unwrap()
            .clone();
        let song_court = factions
            .iter()
            .find(|faction| faction.id == "song_court")
            .unwrap()
            .clone();
        let mut rng = StepRng::new(0, 0);
        let (text, _) = court_clash(&mut rng, &mut state, &court, &song_court);
        assert!(text.contains("大宋繁荣－1、治安＋1"));
        assert!(text.contains("大元繁荣－2、治安－2"));
        assert_eq!(country::country_values(&state, "song"), (89, 81));
        assert_eq!(country::country_values(&state, "yuan"), (8, 8));
    }

    #[test]
    fn ordinary_interactions_feed_back_into_each_country_once() {
        let mut state = world();
        for country in &mut state.countries {
            country.prosperity = 50;
            country.order = 50;
        }
        let factions = faction_snapshot(&state);
        let faction = |id: &str| {
            factions
                .iter()
                .find(|faction| faction.id == id)
                .unwrap()
                .clone()
        };
        let wudang = faction("wudang");
        let huashan = faction("huashan");
        let mingjiao = faction("mingjiao");
        let tianlong = faction("tianlong");
        let xingxiu = faction("xingxiu");

        trade(&mut state, &wudang, &mingjiao);
        assert_eq!(country::country_values(&state, "song").0, 51);
        assert_eq!(country::country_values(&state, "yuan").0, 51);
        trade(&mut state, &wudang, &huashan);
        assert_eq!(
            country::country_values(&state, "song").0,
            52,
            "同国贸易只应增加一次繁荣"
        );

        border_conflict(&mut state, &wudang, &huashan);
        assert_eq!(country::country_values(&state, "song").1, 49);
        joint_bandits(&mut state, &wudang, &huashan);
        assert_eq!(country::country_values(&state, "song").1, 50);

        dali_exchange(&mut state, &tianlong, &wudang);
        assert_eq!(country::country_values(&state, "dali").1, 51);
        assert_eq!(country::country_values(&state, "song").1, 51);

        xia_trade(&mut state, &xingxiu, &wudang);
        assert_eq!(country::country_values(&state, "xia").0, 51);
        assert_eq!(country::country_values(&state, "song").0, 53);

        xia_assassination(&mut state, &xingxiu, &wudang);
        assert_eq!(country::country_values(&state, "song").1, 49);
    }

    #[test]
    fn spar_and_court_events_apply_their_declared_effects() {
        let mut state = world();
        let factions = faction_snapshot(&state);
        let wudang = factions
            .iter()
            .find(|faction| faction.id == "wudang")
            .unwrap();
        let huashan = factions
            .iter()
            .find(|faction| faction.id == "huashan")
            .unwrap();
        let before_total = wudang.prestige + huashan.prestige;
        let mut rng = StdRng::seed_from_u64(13);
        let (text, _) = spar(&mut rng, &mut state, wudang, huashan).unwrap();
        assert!(text.contains("切磋"));
        assert_eq!(
            state.npc_sects[wudang.slot - 1].attributes.prestige
                + state.npc_sects[huashan.slot - 1].attributes.prestige,
            before_total + 1
        );

        let factions = faction_snapshot(&state);
        let court = factions
            .iter()
            .find(|faction| faction.id == "court")
            .unwrap();
        let target = factions
            .iter()
            .find(|faction| faction.id == "wudang")
            .unwrap();
        let before = state.npc_sects[target.slot - 1].attributes.silver;
        let mut levy_rng = StdRng::seed_from_u64(2);
        let (text, _) = yuan_court(&mut levy_rng, &mut state, court, target);
        assert!(text.contains("怯薛军") || text.contains("金帐汗国"));
        let after = state.npc_sects[target.slot - 1].attributes.silver;
        assert!(after == before || after == (before - 30).max(0));
    }

    #[test]
    fn cross_sect_spar_updates_the_real_disciples_and_names_their_growth() {
        use rand::rngs::mock::StepRng;

        let mut state = world();
        let wudang_id = state
            .npc_disciples
            .iter()
            .find(|disciple| disciple.sect_id.as_deref() == Some("wudang") && disciple.alive)
            .unwrap()
            .id
            .clone();
        let huashan_id = state
            .npc_disciples
            .iter()
            .find(|disciple| disciple.sect_id.as_deref() == Some("huashan") && disciple.alive)
            .unwrap()
            .id
            .clone();
        for disciple in &mut state.npc_disciples {
            if matches!(disciple.sect_id.as_deref(), Some("wudang" | "huashan")) {
                disciple.alive = disciple.id == wudang_id || disciple.id == huashan_id;
            }
        }

        let wudang_index = state
            .npc_disciples
            .iter()
            .position(|disciple| disciple.id == wudang_id)
            .unwrap();
        let huashan_index = state
            .npc_disciples
            .iter()
            .position(|disciple| disciple.id == huashan_id)
            .unwrap();
        let wudang_art = spar_art(&state.npc_disciples[wudang_index]).unwrap();
        let huashan_art = spar_art(&state.npc_disciples[huashan_index]).unwrap();
        for (index, neili) in [(wudang_index, 600), (huashan_index, 1)] {
            let disciple = &mut state.npc_disciples[index];
            disciple.attributes.attainment = disciple::attainment_required_for_level(100);
            disciple.attributes.neili.maximum = neili;
            disciple.attributes.neili.current = neili;
            disciple.attributes.qi.current = 4;
            disciple.attributes.reputation = 20;
        }
        {
            let progress = state.npc_disciples[wudang_index]
                .martial_progress
                .proficiencies
                .get_mut(&wudang_art)
                .unwrap();
            progress.level = 1;
            progress.experience = 0;
        }
        {
            let progress = state.npc_disciples[huashan_index]
                .martial_progress
                .proficiencies
                .get_mut(&huashan_art)
                .unwrap();
            progress.level = 1;
            progress.experience = 0;
        }

        let left_experience_before =
            total_skill_experience(&state.npc_disciples[wudang_index], &wudang_art);
        let right_experience_before =
            total_skill_experience(&state.npc_disciples[huashan_index], &huashan_art);
        let factions = faction_snapshot(&state);
        let wudang = factions
            .iter()
            .find(|faction| faction.id == "wudang")
            .unwrap()
            .clone();
        let huashan = factions
            .iter()
            .find(|faction| faction.id == "huashan")
            .unwrap()
            .clone();
        let left_relation_before = sect_ref(&state, wudang.slot)
            .relations
            .get("huashan")
            .copied()
            .unwrap_or(0);
        let right_relation_before = sect_ref(&state, huashan.slot)
            .relations
            .get("wudang")
            .copied()
            .unwrap_or(0);
        let mut rng = StepRng::new(0, 0);

        let (text, mood) = spar(&mut rng, &mut state, &wudang, &huashan).unwrap();

        let left = state
            .npc_disciples
            .iter()
            .find(|disciple| disciple.id == wudang_id)
            .unwrap();
        let right = state
            .npc_disciples
            .iter()
            .find(|disciple| disciple.id == huashan_id)
            .unwrap();
        assert_eq!(mood, "neutral");
        assert!(text.contains(&format!("{}以{}胜过", left.name, martial_name(&wudang_art))));
        assert!(text.contains(&format!("{}经验 +12", martial_name(&wudang_art))));
        assert!(text.contains(&format!("{}经验 +7", martial_name(&huashan_art))));
        assert_eq!(
            total_skill_experience(left, &wudang_art) - left_experience_before,
            12
        );
        assert_eq!(
            total_skill_experience(right, &huashan_art) - right_experience_before,
            7
        );
        assert_eq!(left.attributes.reputation, 21);
        assert_eq!(right.attributes.reputation, 20);
        assert_eq!(left.attributes.qi.current, 1);
        assert_eq!(right.attributes.qi.current, 1);
        assert_eq!(
            state.npc_sects[wudang.slot - 1].attributes.prestige,
            wudang.prestige + 2
        );
        assert_eq!(
            state.npc_sects[huashan.slot - 1].attributes.prestige,
            (huashan.prestige - 1).max(0)
        );
        assert_eq!(
            sect_ref(&state, wudang.slot)
                .relations
                .get("huashan")
                .copied(),
            Some((left_relation_before + 1).clamp(-100, 100))
        );
        assert_eq!(
            sect_ref(&state, huashan.slot)
                .relations
                .get("wudang")
                .copied(),
            Some((right_relation_before + 1).clamp(-100, 100))
        );
    }

    #[test]
    fn spar_growth_respects_research_attainment_and_basic_skill_caps() {
        use crate::models::attributes::SkillProgress;
        use crate::models::martial_art::{MartialTier, SkillCategory};

        let state = world();
        let base = state
            .npc_disciples
            .iter()
            .find(|disciple| disciple.sect_id.as_deref() == Some("wudang") && disciple.alive)
            .unwrap()
            .clone();
        let art_id = spar_art(&base).unwrap();
        let art = martial_art_by_id(&art_id).unwrap();
        assert_ne!(art.tier, MartialTier::Basic);
        assert!(!art.basic_skill.is_empty());

        let prepare_support = |disciple: &mut Disciple, basic_level: i32, attainment_level: i32| {
            for (id, progress) in &mut disciple.martial_progress.proficiencies {
                if martial_art_by_id(id)
                    .is_some_and(|candidate| candidate.category == SkillCategory::Knowledge)
                {
                    *progress = SkillProgress::new(100, 0);
                }
            }
            disciple
                .martial_progress
                .proficiencies
                .insert(art.basic_skill.clone(), SkillProgress::new(basic_level, 0));
            disciple.attributes.attainment =
                disciple::attainment_required_for_level(attainment_level);
        };

        let mut research_limited = base.clone();
        prepare_support(&mut research_limited, 100, 100);
        research_limited
            .martial_progress
            .proficiencies
            .insert(art_id.clone(), SkillProgress::new(50, 2_600));
        assert_eq!(
            gain_spar_experience(&mut research_limited, &art_id, 12, Some(50)),
            0
        );
        assert_eq!(
            research_limited.martial_progress.proficiencies[&art_id].level,
            50
        );

        let mut attainment_limited = base.clone();
        prepare_support(&mut attainment_limited, 100, 10);
        attainment_limited
            .martial_progress
            .proficiencies
            .insert(art_id.clone(), SkillProgress::new(10, 120));
        assert_eq!(
            gain_spar_experience(&mut attainment_limited, &art_id, 12, Some(100)),
            0
        );
        assert_eq!(
            attainment_limited.martial_progress.proficiencies[&art_id].level,
            10
        );

        let mut basic_limited = base;
        prepare_support(&mut basic_limited, 10, 100);
        basic_limited
            .martial_progress
            .proficiencies
            .insert(art_id.clone(), SkillProgress::new(10, 120));
        assert_eq!(
            gain_spar_experience(&mut basic_limited, &art_id, 12, Some(100)),
            0
        );
        assert_eq!(
            basic_limited.martial_progress.proficiencies[&art_id].level,
            10
        );
    }

    #[test]
    fn defection_moves_a_real_disciple_between_rosters() {
        let mut state = world();
        for disciple in &mut state.npc_disciples {
            if matches!(disciple.sect_id.as_deref(), Some("wudang" | "huashan")) {
                disciple.attributes.sect_loyalty = 100;
                disciple.loyalty = 100;
            }
        }
        let defector_id = state
            .npc_disciples
            .iter()
            .find(|disciple| {
                disciple.sect_id.as_deref() == Some("wudang") && !is_npc_leader(disciple)
            })
            .unwrap()
            .id
            .clone();
        let defector = state
            .npc_disciples
            .iter_mut()
            .find(|disciple| disciple.id == defector_id)
            .unwrap();
        defector.attributes.sect_loyalty = 20;
        defector.loyalty = 20;

        let factions = faction_snapshot(&state);
        let wudang = factions
            .iter()
            .find(|faction| faction.id == "wudang")
            .unwrap();
        let huashan = factions
            .iter()
            .find(|faction| faction.id == "huashan")
            .unwrap();
        let before_wudang = wudang.members;
        let before_huashan = huashan.members;
        let mut rng = StdRng::seed_from_u64(19);
        let (text, _) = defection(&mut rng, &mut state, wudang, huashan).unwrap();
        assert!(text.contains("叛逃"));
        assert!(state.npc_disciples.iter().any(|disciple| {
            disciple.id == defector_id && disciple.sect_id.as_deref() == Some("huashan")
        }));
        let after = faction_snapshot(&state);
        let after_wudang = after.iter().find(|faction| faction.id == "wudang").unwrap();
        let after_huashan = after
            .iter()
            .find(|faction| faction.id == "huashan")
            .unwrap();
        assert_eq!(after_wudang.members + 1, before_wudang);
        assert_eq!(after_huashan.members, before_huashan + 1);
    }

    #[test]
    fn loyal_factions_do_not_offer_a_defection_event() {
        let mut state = world();
        for disciple in &mut state.npc_disciples {
            if matches!(disciple.sect_id.as_deref(), Some("wudang" | "huashan")) {
                disciple.attributes.sect_loyalty = 100;
                disciple.loyalty = 100;
            }
        }
        let factions = faction_snapshot(&state);
        let wudang = factions
            .iter()
            .find(|faction| faction.id == "wudang")
            .unwrap();
        let huashan = factions
            .iter()
            .find(|faction| faction.id == "huashan")
            .unwrap();

        assert_eq!((wudang.defectors, huashan.defectors), (0, 0));
        assert!(pair_candidates(&state, wudang, huashan)
            .iter()
            .all(|(kind, _)| !matches!(kind, InteractionKind::Defection)));
    }

    #[test]
    fn incoming_defectors_receive_the_target_sects_available_rank() {
        let mut state = world();
        state.disciples = (0..2)
            .map(|index| Disciple {
                id: format!("player-{index}"),
                rank: DiscipleRank::Chore,
                ..Disciple::default()
            })
            .collect();
        let target = faction_snapshot(&state)
            .into_iter()
            .find(|faction| faction.slot == 0)
            .unwrap();
        let mut incoming = Disciple {
            id: "incoming".into(),
            rank: DiscipleRank::Inner,
            ..Disciple::default()
        };

        assign_incoming_rank(&state, &target, &mut incoming);
        assert_eq!(incoming.rank, DiscipleRank::Outer);

        for member in &mut state.disciples {
            member.rank = DiscipleRank::Outer;
        }
        incoming.rank = DiscipleRank::Inner;
        assign_incoming_rank(&state, &target, &mut incoming);
        assert_eq!(incoming.rank, DiscipleRank::Chore);
    }

    #[test]
    fn alliance_persists_as_a_bilateral_relation_and_changes_prestige() {
        let mut state = world();
        let wudang_index = state
            .npc_sects
            .iter()
            .position(|sect| sect.id == "wudang")
            .unwrap();
        let huashan_index = state
            .npc_sects
            .iter()
            .position(|sect| sect.id == "huashan")
            .unwrap();
        state.npc_sects[wudang_index]
            .relations
            .insert("huashan".into(), 55);
        state.npc_sects[huashan_index]
            .relations
            .insert("wudang".into(), 55);
        let factions = faction_snapshot(&state);
        let wudang = factions
            .iter()
            .find(|faction| faction.id == "wudang")
            .unwrap()
            .clone();
        let huashan = factions
            .iter()
            .find(|faction| faction.id == "huashan")
            .unwrap()
            .clone();
        let before = wudang.prestige + huashan.prestige;
        assert!(pair_candidates(&state, &wudang, &huashan)
            .iter()
            .any(|(kind, _)| matches!(*kind, InteractionKind::Alliance)));

        let (text, mood) = alliance(&mut state, &wudang, &huashan);

        assert!(text.contains("会盟"));
        assert_eq!(mood, "good");
        assert_eq!(
            state.npc_sects[wudang_index].relations.get("huashan"),
            Some(&70)
        );
        assert_eq!(
            state.npc_sects[huashan_index].relations.get("wudang"),
            Some(&70)
        );
        assert_eq!(
            state.npc_sects[wudang_index].attributes.prestige
                + state.npc_sects[huashan_index].attributes.prestige,
            before + 4
        );
    }

    #[test]
    fn allied_pairs_use_the_dedicated_pool_and_suppress_hostile_interactions() {
        let mut state = world();
        for (left_id, right_id) in [
            ("wudang", "huashan"),
            ("wudang", "mingjiao"),
            ("wudang", "xingxiu"),
        ] {
            make_allies(&mut state, left_id, right_id, 70);
            let left = faction(&state, left_id);
            let right = faction(&state, right_id);
            let candidates = pair_candidates(&state, &left, &right);

            assert!(candidates
                .iter()
                .any(|(kind, _)| matches!(kind, InteractionKind::Trade)));
            assert!(candidates.iter().all(|(kind, _)| matches!(
                kind,
                InteractionKind::Trade
                    | InteractionKind::Spar
                    | InteractionKind::AlliedJointPatrol
                    | InteractionKind::AlliedRescue
                    | InteractionKind::AllianceBetrayal
            )));
            assert!(candidates.iter().all(|(kind, _)| !matches!(
                kind,
                InteractionKind::BorderConflict
                    | InteractionKind::Defection
                    | InteractionKind::YuanSongWar
                    | InteractionKind::XiaAssassination
            )));
        }
    }

    #[test]
    fn allied_joint_patrol_updates_real_members_skills_resources_and_country() {
        use rand::rngs::mock::StepRng;

        let mut state = world();
        make_allies(&mut state, "wudang", "huashan", 80);
        let wudang_id = state
            .npc_disciples
            .iter()
            .find(|disciple| {
                disciple.sect_id.as_deref() == Some("wudang")
                    && disciple::can_act(disciple)
                    && spar_art(disciple).is_some()
            })
            .unwrap()
            .id
            .clone();
        let huashan_id = state
            .npc_disciples
            .iter()
            .find(|disciple| {
                disciple.sect_id.as_deref() == Some("huashan")
                    && disciple::can_act(disciple)
                    && spar_art(disciple).is_some()
            })
            .unwrap()
            .id
            .clone();
        for disciple in &mut state.npc_disciples {
            if disciple.sect_id.as_deref() == Some("wudang") {
                disciple.alive = disciple.id == wudang_id;
            } else if disciple.sect_id.as_deref() == Some("huashan") {
                disciple.alive = disciple.id == huashan_id;
            }
        }
        let wudang_index = state
            .npc_disciples
            .iter()
            .position(|disciple| disciple.id == wudang_id)
            .unwrap();
        let huashan_index = state
            .npc_disciples
            .iter()
            .position(|disciple| disciple.id == huashan_id)
            .unwrap();
        let wudang_art = spar_art(&state.npc_disciples[wudang_index]).unwrap();
        let huashan_art = spar_art(&state.npc_disciples[huashan_index]).unwrap();
        for (index, art_id) in [
            (wudang_index, wudang_art.as_str()),
            (huashan_index, huashan_art.as_str()),
        ] {
            let disciple = &mut state.npc_disciples[index];
            disciple.attributes.attainment = disciple::attainment_required_for_level(100);
            disciple.attributes.neili.maximum = 1_200;
            disciple.attributes.neili.current = 1_200;
            disciple.attributes.qi.maximum = 100;
            disciple.attributes.qi.current = 100;
            disciple.attributes.reputation = 20;
            let progress = disciple
                .martial_progress
                .proficiencies
                .get_mut(art_id)
                .unwrap();
            progress.level = 1;
            progress.experience = 0;
        }
        let left_before = total_skill_experience(&state.npc_disciples[wudang_index], &wudang_art);
        let right_before =
            total_skill_experience(&state.npc_disciples[huashan_index], &huashan_art);
        let wudang = faction(&state, "wudang");
        let huashan = faction(&state, "huashan");
        let silver_before = wudang.silver + huashan.silver;
        let prestige_before = wudang.prestige + huashan.prestige;
        let country_before = country::country_values(&state, "song");
        let mut rng = StepRng::new(0, 0);

        let (text, mood) = allied_joint_patrol(&mut rng, &mut state, &wudang, &huashan).unwrap();

        let left = &state.npc_disciples[wudang_index];
        let right = &state.npc_disciples[huashan_index];
        assert_eq!(mood, "good");
        assert!(text.contains(&left.name));
        assert!(text.contains(&right.name));
        assert!(text.contains("实际经验 +14"));
        assert_eq!(total_skill_experience(left, &wudang_art) - left_before, 14);
        assert_eq!(
            total_skill_experience(right, &huashan_art) - right_before,
            14
        );
        assert_eq!(
            (left.attributes.qi.current, right.attributes.qi.current),
            (93, 93)
        );
        assert_eq!(
            (left.attributes.reputation, right.attributes.reputation),
            (22, 22)
        );
        let after_wudang = faction(&state, "wudang");
        let after_huashan = faction(&state, "huashan");
        assert_eq!(
            after_wudang.silver + after_huashan.silver,
            silver_before + 70
        );
        assert_eq!(
            after_wudang.prestige + after_huashan.prestige,
            prestige_before + 6
        );
        assert_eq!(mutual_relation(&state, &after_wudang, &after_huashan), 85);
        assert_eq!(
            country::country_values(&state, "song"),
            (country_before.0 + 1, country_before.1 + 2)
        );
    }

    #[test]
    fn allied_rescue_synchronizes_the_real_journey_and_charges_the_helper() {
        use crate::models::attributes::{ActionKind, ActionPlan};
        use rand::rngs::mock::StepRng;

        let mut state = world();
        make_allies(&mut state, "wudang", "huashan", 80);
        let target_id = state
            .npc_disciples
            .iter()
            .find(|disciple| disciple.sect_id.as_deref() == Some("wudang") && disciple.alive)
            .unwrap()
            .id
            .clone();
        let target = state
            .npc_disciples
            .iter_mut()
            .find(|disciple| disciple.id == target_id)
            .unwrap();
        target.away_months = 3;
        target.action = Some(ActionPlan {
            kind: ActionKind::Wander,
            remaining_months: 7,
            ..ActionPlan::default()
        });
        let rescuer_id = state
            .npc_disciples
            .iter()
            .find(|disciple| {
                disciple.sect_id.as_deref() == Some("huashan")
                    && disciple::can_act(disciple)
                    && spar_art(disciple).is_some()
            })
            .unwrap()
            .id
            .clone();
        for disciple in &mut state.npc_disciples {
            if disciple.sect_id.as_deref() == Some("huashan") {
                disciple.alive = disciple.id == rescuer_id;
            }
        }
        let wudang = faction(&state, "wudang");
        let huashan = faction(&state, "huashan");
        let helper_silver_before = huashan.silver;
        let helper_prestige_before = huashan.prestige;
        let mut rng = StepRng::new(0, 0);

        let (text, mood) = allied_rescue(&mut rng, &mut state, &wudang, &huashan).unwrap();

        let target = state
            .npc_disciples
            .iter()
            .find(|disciple| disciple.id == target_id)
            .unwrap();
        assert_eq!(mood, "good");
        assert!(text.contains(&target.name));
        assert!(text.contains(
            &state
                .npc_disciples
                .iter()
                .find(|disciple| disciple.id == rescuer_id)
                .unwrap()
                .name
        ));
        assert_eq!(target.away_months, 2);
        assert_eq!(target.action.as_ref().unwrap().remaining_months, 2);
        let after_wudang = faction(&state, "wudang");
        let after_huashan = faction(&state, "huashan");
        assert_eq!(
            after_huashan.silver,
            helper_silver_before - ALLIED_RESCUE_COST
        );
        assert_eq!(after_huashan.prestige, helper_prestige_before + 1);
        assert_eq!(mutual_relation(&state, &after_wudang, &after_huashan), 84);
    }

    #[test]
    fn low_morality_betrayal_breaks_the_alliance_and_conserves_silver() {
        let mut state = world();
        let xingxiu_slot = faction(&state, "xingxiu").slot;
        let wudang_slot = faction(&state, "wudang").slot;
        sect_mut(&mut state, xingxiu_slot).attributes.morality = 20;
        sect_mut(&mut state, xingxiu_slot).attributes.silver = 10;
        sect_mut(&mut state, xingxiu_slot).attributes.morale = 2;
        sect_mut(&mut state, wudang_slot).attributes.morality = 75;
        sect_mut(&mut state, wudang_slot).attributes.silver = 20;
        sect_mut(&mut state, wudang_slot).attributes.morale = 3;
        make_allies(&mut state, "xingxiu", "wudang", 80);
        let xingxiu = faction(&state, "xingxiu");
        let wudang = faction(&state, "wudang");
        let silver_before = xingxiu.silver + wudang.silver;

        let (text, mood) = alliance_betrayal(&mut state, &xingxiu, &wudang).unwrap();

        let after_xingxiu = faction(&state, "xingxiu");
        let after_wudang = faction(&state, "wudang");
        assert_eq!(mood, "bad");
        assert!(text.contains("背盟"));
        assert!(text.contains("20两"));
        assert_eq!(after_xingxiu.silver, 30);
        assert_eq!(after_wudang.silver, 0);
        assert_eq!(after_xingxiu.silver + after_wudang.silver, silver_before);
        assert_eq!(mutual_relation(&state, &after_xingxiu, &after_wudang), 45);
        assert!(mutual_relation(&state, &after_xingxiu, &after_wudang) < 70);
        assert_eq!(
            (
                sect_ref(&state, after_xingxiu.slot).attributes.morale,
                sect_ref(&state, after_wudang.slot).attributes.morale,
            ),
            (0, 0)
        );
        assert!(state.countries.iter().all(|country| {
            (0..=100).contains(&country.prosperity) && (0..=100).contains(&country.order)
        }));
    }

    #[test]
    fn fixed_seed_replays_alliance_interactions_exactly() {
        let mut base = world();
        make_allies(&mut base, "wudang", "huashan", 80);
        make_allies(&mut base, "xingxiu", "wudang", 80);
        let mut left = base.clone();
        let mut right = base;
        let mut left_rng = StdRng::seed_from_u64(20_260_719);
        let mut right_rng = StdRng::seed_from_u64(20_260_719);

        let left_events = run_monthly_interactions(&mut left_rng, &mut left);
        let right_events = run_monthly_interactions(&mut right_rng, &mut right);

        assert_eq!(
            serde_json::to_value(left_events).unwrap(),
            serde_json::to_value(right_events).unwrap()
        );
        assert_eq!(
            serde_json::to_value(left).unwrap(),
            serde_json::to_value(right).unwrap()
        );
    }

    #[test]
    fn court_conscription_moves_a_real_disciple_into_the_court_roster() {
        let mut state = world();
        let factions = faction_snapshot(&state);
        let source = factions
            .iter()
            .find(|faction| faction.id == "wudang")
            .unwrap()
            .clone();
        let court = factions
            .iter()
            .find(|faction| faction.id == "court")
            .unwrap()
            .clone();
        assert!(pair_candidates(&state, &source, &court)
            .iter()
            .any(|(kind, _)| matches!(*kind, InteractionKind::CourtConscription)));
        let source_before = faction_members(&state, source.slot)
            .into_iter()
            .map(|disciple| disciple.id.clone())
            .collect::<BTreeSet<_>>();
        let court_before = faction_members(&state, court.slot).len();
        let mut rng = StdRng::seed_from_u64(29);

        let (text, mood) = court_conscription(&mut rng, &mut state, &source, &court).unwrap();

        let source_after = faction_members(&state, source.slot)
            .into_iter()
            .map(|disciple| disciple.id.clone())
            .collect::<BTreeSet<_>>();
        let moved_id = source_before.difference(&source_after).next().unwrap();
        let moved = state
            .npc_disciples
            .iter()
            .find(|disciple| &disciple.id == moved_id)
            .unwrap();
        assert!(text.contains("强征"));
        assert_eq!(mood, "bad");
        assert_eq!(source_after.len() + 1, source_before.len());
        assert_eq!(faction_members(&state, court.slot).len(), court_before + 1);
        assert_eq!(moved.sect_id.as_deref(), Some("court"));
        assert_eq!(moved.department, Some(Department::ExternalAffairs));
        assert!(!is_npc_leader(moved));
    }

    #[test]
    fn song_court_conscription_moves_a_disciple_into_official_service() {
        let mut state = world();
        let factions = faction_snapshot(&state);
        let source = factions
            .iter()
            .find(|faction| faction.id == "huashan")
            .unwrap()
            .clone();
        let court = factions
            .iter()
            .find(|faction| faction.id == "song_court")
            .unwrap()
            .clone();
        let source_before = faction_members(&state, source.slot)
            .into_iter()
            .map(|disciple| disciple.id.clone())
            .collect::<BTreeSet<_>>();
        let mut rng = StdRng::seed_from_u64(37);

        let (text, mood) = court_conscription(&mut rng, &mut state, &source, &court).unwrap();

        let source_after = faction_members(&state, source.slot)
            .into_iter()
            .map(|disciple| disciple.id.clone())
            .collect::<BTreeSet<_>>();
        let moved_id = source_before.difference(&source_after).next().unwrap();
        let moved = state
            .npc_disciples
            .iter()
            .find(|disciple| &disciple.id == moved_id)
            .unwrap();
        assert!(text.contains("征辟"));
        assert_eq!(mood, "good");
        assert_eq!(moved.sect_id.as_deref(), Some("song_court"));
        assert_eq!(moved.department, Some(Department::ExternalAffairs));
        assert!(moved.loyalty >= 70);
    }

    #[test]
    fn dead_npc_leader_is_replaced_by_a_real_elder() {
        let mut state = world();
        let sect_index = state
            .npc_sects
            .iter()
            .position(|sect| sect.id == "wudang")
            .unwrap();
        let sect_id = state.npc_sects[sect_index].id.clone();
        let leader_indices = state
            .npc_disciples
            .iter()
            .enumerate()
            .filter(|(_, disciple)| {
                disciple.sect_id.as_deref() == Some(sect_id.as_str()) && is_npc_leader(disciple)
            })
            .map(|(index, _)| index)
            .collect::<Vec<_>>();
        assert!(!leader_indices.is_empty());
        let predecessor_id = state.npc_disciples[leader_indices[0]].id.clone();
        for index in leader_indices {
            state.npc_disciples[index].alive = false;
        }
        let successor_index = state
            .npc_disciples
            .iter()
            .position(|disciple| {
                disciple.alive
                    && disciple.sect_id.as_deref() == Some(sect_id.as_str())
                    && !is_npc_leader(disciple)
            })
            .unwrap();
        let successor_id = state.npc_disciples[successor_index].id.clone();
        for building in &mut state.npc_sects[sect_index].buildings {
            building.elder_id = None;
        }
        for disciple in &mut state.npc_disciples {
            if disciple.sect_id.as_deref() == Some(sect_id.as_str())
                && disciple.id != successor_id
                && disciple.npc_position.as_deref() == Some(NpcPosition::Elder.display())
            {
                disciple.npc_position = Some(NpcPosition::InnerDisciple.display().into());
            }
        }
        state.npc_disciples[successor_index].rank = DiscipleRank::Inner;
        state.npc_disciples[successor_index].npc_position =
            Some(NpcPosition::Elder.display().into());
        state.npc_sects[sect_index].buildings[0].elder_id = Some(successor_id.clone());
        let prestige_before = state.npc_sects[sect_index].attributes.prestige;
        let morale_before = state.npc_sects[sect_index].attributes.morale;

        let event = resolve_leader_succession(&mut state).unwrap();

        let predecessor = state
            .npc_disciples
            .iter()
            .find(|disciple| disciple.id == predecessor_id)
            .unwrap();
        let successor = state
            .npc_disciples
            .iter()
            .find(|disciple| disciple.id == successor_id)
            .unwrap();
        assert!(event.text.contains("继任掌门"));
        assert_eq!(event.category, "world");
        assert_eq!(predecessor.npc_position.as_deref(), Some("故掌门"));
        assert_eq!(
            successor.npc_position.as_deref(),
            Some(NpcPosition::SectLeader.display())
        );
        assert_eq!(successor.department, Some(Department::Transmission));
        assert!(successor.loyalty >= 85);
        assert_eq!(
            state.npc_sects[sect_index].attributes.prestige,
            (prestige_before - 1).max(0)
        );
        assert_eq!(
            state.npc_sects[sect_index].attributes.morale,
            (morale_before - 2).max(0)
        );
    }

    #[test]
    fn monthly_interactions_are_deterministic_for_a_fixed_seed_and_snapshot() {
        let mut left = world();
        let mut right = left.clone();
        let mut left_rng = StdRng::seed_from_u64(20260719);
        let mut right_rng = StdRng::seed_from_u64(20260719);

        let left_events = run_monthly_interactions(&mut left_rng, &mut left);
        let right_events = run_monthly_interactions(&mut right_rng, &mut right);

        assert_eq!(
            left_events
                .iter()
                .map(|event| (&event.text, &event.mood, &event.category))
                .collect::<Vec<_>>(),
            right_events
                .iter()
                .map(|event| (&event.text, &event.mood, &event.category))
                .collect::<Vec<_>>()
        );
        assert_eq!(
            serde_json::to_value(left).unwrap(),
            serde_json::to_value(right).unwrap()
        );
    }
}
