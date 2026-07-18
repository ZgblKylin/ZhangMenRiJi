use crate::logic::{country, disciple, sect};
use crate::models::attributes::{
    ActionKind, ActionPlan, Department, DiscipleRank, JourneyOutcome, JourneyProgress,
};
use crate::models::sect::MoralDirection;
use crate::models::{Disciple, GameEvent, GameState};
use rand::{rngs::StdRng, Rng, SeedableRng};
use rayon::prelude::*;
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone)]
struct Actor {
    disciple: Disciple,
    sect_id: String,
    player: bool,
    public_books: Vec<String>,
    recovery_bonus: i32,
    practice_effectiveness: i32,
    scripture_effectiveness: i32,
    warehouse_effectiveness: i32,
    herb_hall_effectiveness: i32,
    logistics_effectiveness: i32,
    prosperity: i32,
    order: i32,
    market_percent: i32,
    safety_percent: i32,
    external_percent: i32,
    moral_direction: MoralDirection,
}

#[derive(Clone)]
struct PlannedAction {
    actor: usize,
    kind: ActionKind,
    target: Option<usize>,
    /// 掌门明确指定的互动优先合并，避免受教者先被独行任务占用。
    directed: bool,
}

#[derive(Clone)]
struct ActionJob {
    id: String,
    kind: ActionKind,
    actors: Vec<Actor>,
}

#[derive(Default)]
struct DiscipleDelta {
    id: String,
    sect_id: String,
    qi: i32,
    qi_max: i32,
    spirit: i32,
    spirit_max: i32,
    neili: i32,
    neili_max: i32,
    energy: i32,
    energy_max: i32,
    attainment: i64,
    reputation: i32,
    loyalty: i32,
    merit: i64,
    personal_silver: i32,
    skill_experience: BTreeMap<String, i64>,
    private_books: Vec<String>,
    away_months: Option<i32>,
    action: Option<Option<ActionPlan>>,
}

#[derive(Default)]
struct SectDelta {
    silver: i32,
    prestige: i32,
    morality: i32,
    morale: i32,
    inventory: BTreeMap<String, i32>,
    building_work: BTreeMap<String, i32>,
    building_maintenance: BTreeMap<String, i32>,
}

#[derive(Default)]
struct JobResult {
    disciples: Vec<DiscipleDelta>,
    sects: BTreeMap<String, SectDelta>,
    logs: Vec<(bool, String)>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum TravelEncounterKind {
    QuietRoad,
    FriendlySpar,
    BanditAmbush,
    FoundSupplies,
    HermitGuidance,
    LostManual,
}

/// 基于月初快照并行推演全部人物。计算期间不触碰实时状态，结果最后统一归并。
pub fn run_auto_actions(state: &mut GameState) -> Vec<GameEvent> {
    let snapshot = state.clone();
    let actors = collect_actors(&snapshot);
    let plans = plan_actions(&snapshot, &actors);
    let jobs = build_jobs(&actors, &plans);
    let turn = (snapshot.year as u64).saturating_mul(12) + snapshot.month as u64;
    let results: Vec<JobResult> = jobs
        .par_iter()
        .map(|job| {
            let seed = snapshot.world_seed ^ turn.rotate_left(17) ^ stable_hash(&job.id);
            execute_job(job, seed)
        })
        .collect();

    apply_results(state, results)
}

fn collect_actors(state: &GameState) -> Vec<Actor> {
    let player_recovery =
        sect::policy_bonus(&state.sect, "recovery") + sect::order_bonus(&state.sect, "recovery");
    let (player_prosperity, player_order) = country::country_values(state, &state.sect.country_id);
    state
        .disciples
        .iter()
        .map(|d| Actor {
            disciple: d.clone(),
            sect_id: "player".into(),
            player: true,
            public_books: state.sect.public_books.clone(),
            recovery_bonus: player_recovery,
            practice_effectiveness: sect::building_effectiveness(&state.sect, "practice"),
            scripture_effectiveness: sect::building_effectiveness(&state.sect, "scripture"),
            warehouse_effectiveness: sect::building_effectiveness(&state.sect, "warehouse"),
            herb_hall_effectiveness: sect::building_effectiveness(&state.sect, "herb_hall"),
            logistics_effectiveness: sect::building_effectiveness(&state.sect, "logistics"),
            prosperity: player_prosperity,
            order: player_order,
            market_percent: country::market_percent(player_prosperity),
            safety_percent: country::safety_percent_for(state, &state.sect.country_id),
            external_percent: country::external_percent_for(state, &state.sect.country_id),
            moral_direction: state.sect.moral_direction.clone(),
        })
        .chain(state.npc_disciples.iter().map(|d| {
            let npc_sect = d
                .sect_id
                .as_ref()
                .and_then(|id| state.npc_sects.iter().find(|sect| &sect.id == id));
            let (prosperity, order) = npc_sect
                .map(|sect| country::country_values(state, &sect.country_id))
                .unwrap_or((70, 65));
            Actor {
                disciple: d.clone(),
                sect_id: d.sect_id.clone().unwrap_or_else(|| "wanderer".into()),
                player: false,
                public_books: npc_sect
                    .map(|sect| sect.public_books.clone())
                    .unwrap_or_default(),
                recovery_bonus: npc_sect
                    .map(|sect| {
                        sect::policy_bonus(sect, "recovery") + sect::order_bonus(sect, "recovery")
                    })
                    .unwrap_or(0),
                practice_effectiveness: npc_sect
                    .map(|sect| sect::building_effectiveness(sect, "practice"))
                    .unwrap_or(100),
                scripture_effectiveness: npc_sect
                    .map(|sect| sect::building_effectiveness(sect, "scripture"))
                    .unwrap_or(100),
                warehouse_effectiveness: npc_sect
                    .map(|sect| sect::building_effectiveness(sect, "warehouse"))
                    .unwrap_or(100),
                herb_hall_effectiveness: npc_sect
                    .map(|sect| sect::building_effectiveness(sect, "herb_hall"))
                    .unwrap_or(100),
                logistics_effectiveness: npc_sect
                    .map(|sect| sect::building_effectiveness(sect, "logistics"))
                    .unwrap_or(100),
                prosperity,
                order,
                market_percent: country::market_percent(prosperity),
                safety_percent: country::safety_percent(order),
                external_percent: country::external_percent(prosperity, order),
                moral_direction: npc_sect
                    .map(|sect| sect.moral_direction.clone())
                    .unwrap_or(MoralDirection::Neutral),
            }
        }))
        .collect()
}

fn plan_actions(state: &GameState, actors: &[Actor]) -> Vec<PlannedAction> {
    actors
        .iter()
        .enumerate()
        .map(|(index, actor)| {
            let mut rng = StdRng::seed_from_u64(
                state.world_seed
                    ^ stable_hash(&actor.disciple.id)
                    ^ ((state.year * 12 + state.month) as u64).rotate_left(11),
            );
            let kind = choose_action(state, actor, &mut rng);
            let selected_target = actor
                .disciple
                .action
                .as_ref()
                .filter(|plan| matches!(plan.kind, ActionKind::Teach | ActionKind::Spar))
                .and_then(|plan| plan.target_id.as_ref())
                .and_then(|id| actors.iter().position(|other| &other.disciple.id == id))
                .filter(|target| {
                    actors[*target].sect_id == actor.sect_id
                        && disciple::can_act(&actors[*target].disciple)
                        && (kind != ActionKind::Teach
                            || (sect::teaching_relationship_eligible(
                                &actor.disciple,
                                &actors[*target].disciple,
                            ) && actor
                                .disciple
                                .action
                                .as_ref()
                                .and_then(|plan| plan.martial_art_id.as_deref())
                                .is_some_and(|art| {
                                    can_teach_art(&actor.disciple, &actors[*target].disciple, art)
                                })))
                });
            let target = match kind {
                ActionKind::Teach => selected_target
                    .or_else(|| choose_teaching_partner(index, &actor.sect_id, actors, &mut rng)),
                ActionKind::Spar => selected_target
                    .or_else(|| choose_partner(index, &actor.sect_id, actors, &mut rng)),
                _ => None,
            };
            PlannedAction {
                actor: index,
                kind,
                target,
                directed: selected_target.is_some(),
            }
        })
        .collect()
}

fn choose_action(state: &GameState, actor: &Actor, rng: &mut StdRng) -> ActionKind {
    let d = &actor.disciple;
    if !d.alive {
        return ActionKind::Recover;
    }
    if d.away_months > 0 {
        return d
            .action
            .as_ref()
            .map(|plan| plan.kind.clone())
            .unwrap_or(ActionKind::Wander);
    }
    if !disciple::can_act(d) {
        return ActionKind::Recover;
    }
    if let Some(plan) = &d.action {
        return plan.kind.clone();
    }

    let policy = if actor.player {
        &state.sect
    } else {
        state
            .npc_sects
            .iter()
            .find(|candidate| candidate.id == actor.sect_id)
            .unwrap_or(&state.sect)
    };
    let mut choices = match d.rank.clone() {
        DiscipleRank::Chore => vec![
            (ActionKind::Produce, 18),
            (
                ActionKind::Business,
                16 + sect::policy_bonus(policy, "income"),
            ),
            (ActionKind::Gather, 18),
            (ActionKind::Recover, 8),
        ],
        DiscipleRank::Outer => vec![
            (
                ActionKind::Read,
                8 + sect::policy_bonus(policy, "study") + sect::order_bonus(policy, "study"),
            ),
            (
                ActionKind::Practice,
                24 + sect::policy_bonus(policy, "martial"),
            ),
            (ActionKind::Spar, 18 + sect::policy_bonus(policy, "martial")),
            (ActionKind::TemperBody, 8),
            (ActionKind::CultivateNeili, 10),
            (ActionKind::Meditate, 6),
            (ActionKind::SectMission, 12),
            (ActionKind::Wander, 10 + d.aptitudes.fortune / 5),
        ],
        DiscipleRank::Inner => vec![
            (
                ActionKind::Read,
                12 + sect::policy_bonus(policy, "study") + sect::order_bonus(policy, "study"),
            ),
            (
                ActionKind::Practice,
                14 + sect::policy_bonus(policy, "martial") + sect::order_bonus(policy, "martial"),
            ),
            (ActionKind::Teach, 8),
            (ActionKind::Spar, 12 + sect::policy_bonus(policy, "martial")),
            (
                ActionKind::TemperBody,
                13 + sect::policy_bonus(policy, "martial") + sect::order_bonus(policy, "martial"),
            ),
            (
                ActionKind::CultivateNeili,
                15 + sect::policy_bonus(policy, "martial") + sect::order_bonus(policy, "martial"),
            ),
            (
                ActionKind::Meditate,
                9 + sect::policy_bonus(policy, "study") + sect::order_bonus(policy, "study"),
            ),
            (
                ActionKind::SectMission,
                9 + sect::policy_bonus(policy, "income") + sect::order_bonus(policy, "income"),
            ),
            (ActionKind::Wander, 8 + d.aptitudes.fortune / 5),
        ],
    };
    if d.attributes.qi.current < d.attributes.qi.maximum / 3
        || d.attributes.spirit.current < d.attributes.spirit.maximum / 3
    {
        choices.push((ActionKind::Recover, 45));
    }
    for (kind, weight) in &mut choices {
        *weight = weight
            .saturating_add(country_action_weight_adjustment(
                kind,
                actor.prosperity,
                actor.order,
            ))
            .saturating_add(moral_action_weight_adjustment(kind, &actor.moral_direction))
            .saturating_add(policy_action_weight_adjustment(policy, kind))
            .max(1);
    }
    choices.retain(|(kind, _)| match kind {
        ActionKind::Read | ActionKind::Meditate => actor.scripture_effectiveness > 0,
        ActionKind::Practice
        | ActionKind::Teach
        | ActionKind::Spar
        | ActionKind::TemperBody
        | ActionKind::CultivateNeili => actor.practice_effectiveness > 0,
        ActionKind::Produce | ActionKind::Business => actor.warehouse_effectiveness > 0,
        ActionKind::Gather => actor.herb_hall_effectiveness > 0,
        ActionKind::Maintain | ActionKind::Construct => actor.logistics_effectiveness > 0,
        _ => true,
    });
    let total: i32 = choices.iter().map(|(_, weight)| *weight).sum();
    let mut roll = rng.gen_range(0..total.max(1));
    for (kind, weight) in choices {
        if roll < weight {
            return kind;
        }
        roll -= weight;
    }
    ActionKind::CultivateNeili
}

fn policy_action_weight_adjustment(
    policy: &crate::models::sect::SectState,
    kind: &ActionKind,
) -> i32 {
    match kind {
        ActionKind::SectMission | ActionKind::Wander => sect::policy_bonus(policy, "morality") / 2,
        _ => 0,
    }
}

fn moral_action_weight_adjustment(kind: &ActionKind, direction: &MoralDirection) -> i32 {
    match (direction, kind) {
        (MoralDirection::Righteous, ActionKind::SectMission) => 12,
        (MoralDirection::Righteous, ActionKind::Wander) => 8,
        (MoralDirection::Righteous, ActionKind::Business) => -5,
        (MoralDirection::Villainous, ActionKind::Business) => 10,
        (MoralDirection::Villainous, ActionKind::SectMission) => 12,
        (MoralDirection::Villainous, ActionKind::Wander) => -4,
        _ => 0,
    }
}

fn country_action_weight_adjustment(kind: &ActionKind, prosperity: i32, order: i32) -> i32 {
    let prosperity = country::clamp_value(prosperity);
    let order = country::clamp_value(order);
    match kind {
        ActionKind::Business => ((prosperity - 70) / 5).clamp(-8, 6),
        ActionKind::Produce => ((70 - prosperity) / 5).clamp(-6, 8),
        ActionKind::SectMission => ((65 - order) / 5).clamp(-5, 8),
        ActionKind::Wander => ((order - 65) / 5).clamp(-5, 7),
        _ => 0,
    }
}

fn choose_partner(
    actor: usize,
    sect_id: &str,
    actors: &[Actor],
    rng: &mut StdRng,
) -> Option<usize> {
    let candidates: Vec<usize> = actors
        .iter()
        .enumerate()
        .filter(|(index, other)| {
            *index != actor && other.sect_id == sect_id && disciple::can_act(&other.disciple)
        })
        .map(|(index, _)| index)
        .collect();
    if candidates.is_empty() {
        None
    } else {
        Some(candidates[rng.gen_range(0..candidates.len())])
    }
}

fn choose_teaching_partner(
    actor: usize,
    sect_id: &str,
    actors: &[Actor],
    rng: &mut StdRng,
) -> Option<usize> {
    let teacher = &actors[actor].disciple;
    let eligible = actors
        .iter()
        .enumerate()
        .filter(|(index, other)| {
            *index != actor
                && other.sect_id == sect_id
                && disciple::can_act(&other.disciple)
                && sect::teaching_relationship_eligible(teacher, &other.disciple)
                && teaching_art(teacher, &other.disciple).is_some()
        })
        .map(|(index, _)| index)
        .collect::<Vec<_>>();
    let direct_apprentices = eligible
        .iter()
        .copied()
        .filter(|index| actors[*index].disciple.master_id.as_deref() == Some(teacher.id.as_str()))
        .collect::<Vec<_>>();
    let candidates = if direct_apprentices.is_empty() {
        &eligible
    } else {
        &direct_apprentices
    };
    (!candidates.is_empty()).then(|| candidates[rng.gen_range(0..candidates.len())])
}

fn build_jobs(actors: &[Actor], plans: &[PlannedAction]) -> Vec<ActionJob> {
    let mut consumed = BTreeSet::new();
    let mut jobs = Vec::with_capacity(plans.len());
    // 先锁定掌门明确安排的师生或切磋组合。若按名册顺序先结算独行任务，
    // 排在后面的教师会因学生已被占用而静默退回研读。
    for plan in plans.iter().filter(|plan| plan.directed) {
        let Some(target) = plan.target else {
            continue;
        };
        if consumed.contains(&plan.actor) || consumed.contains(&target) {
            continue;
        }
        consumed.insert(plan.actor);
        consumed.insert(target);
        let mut ids = [
            actors[plan.actor].disciple.id.clone(),
            actors[target].disciple.id.clone(),
        ];
        ids.sort();
        jobs.push(ActionJob {
            id: format!("{}:{}:{}", ids[0], ids[1], action_name(&plan.kind)),
            kind: plan.kind.clone(),
            actors: vec![actors[plan.actor].clone(), actors[target].clone()],
        });
    }
    for plan in plans {
        if consumed.contains(&plan.actor) {
            continue;
        }
        let interaction = matches!(plan.kind, ActionKind::Teach | ActionKind::Spar);
        if interaction {
            if let Some(target) = plan.target.filter(|target| !consumed.contains(target)) {
                consumed.insert(plan.actor);
                consumed.insert(target);
                let mut ids = [
                    actors[plan.actor].disciple.id.clone(),
                    actors[target].disciple.id.clone(),
                ];
                ids.sort();
                jobs.push(ActionJob {
                    id: format!("{}:{}:{}", ids[0], ids[1], action_name(&plan.kind)),
                    kind: plan.kind.clone(),
                    actors: vec![actors[plan.actor].clone(), actors[target].clone()],
                });
                continue;
            }
        }
        consumed.insert(plan.actor);
        jobs.push(ActionJob {
            id: format!(
                "{}:{}",
                actors[plan.actor].disciple.id,
                action_name(&plan.kind)
            ),
            kind: if interaction {
                ActionKind::Read
            } else {
                plan.kind.clone()
            },
            actors: vec![actors[plan.actor].clone()],
        });
    }
    jobs
}

fn execute_job(job: &ActionJob, seed: u64) -> JobResult {
    let mut rng = StdRng::seed_from_u64(seed);
    if job.actors.len() == 2 {
        execute_pair(job, &mut rng)
    } else {
        execute_solo(&job.actors[0], &job.kind, &mut rng)
    }
}

fn execute_pair(job: &ActionJob, rng: &mut StdRng) -> JobResult {
    let first = &job.actors[0];
    let second = &job.actors[1];
    let mut result = JobResult::default();
    match job.kind {
        ActionKind::Teach => {
            let directed = first.disciple.action.as_ref().is_some_and(|plan| {
                plan.kind == ActionKind::Teach
                    && plan.target_id.as_deref() == Some(second.disciple.id.as_str())
            });
            // 传授配对在规划阶段已经按“教师 -> 合格受教者”选定；不再依据
            // 名册或战力反转角色，以免师父最终被自己的徒弟当作受教者。
            let (teacher, student) = (first, second);
            let art = directed
                .then(|| {
                    first
                        .disciple
                        .action
                        .as_ref()
                        .and_then(|plan| plan.martial_art_id.clone())
                })
                .flatten()
                .filter(|art| can_teach_art(&teacher.disciple, &student.disciple, art))
                .or_else(|| teaching_art(&teacher.disciple, &student.disciple))
                .unwrap_or_else(|| teacher.disciple.martial_art.clone());
            let teacher_level = skill_level(&teacher.disciple, &art);
            let student_level = skill_level(&student.disciple, &art);
            let gap_bonus = (teacher_level - student_level).max(0) as i64;
            let gain = scale_department_experience(
                scale_experience(
                    skill_experience(
                        &student.disciple,
                        &art,
                        student.disciple.aptitudes.intelligence,
                        105 + gap_bonus.min(80) as i32,
                    ),
                    student.practice_effectiveness,
                ),
                &teacher.disciple,
                &ActionKind::Teach,
            );
            let teacher_gain = scale_department_experience(
                scale_experience(
                    skill_experience(
                        &teacher.disciple,
                        &art,
                        teacher.disciple.aptitudes.intelligence,
                        25,
                    ),
                    teacher.practice_effectiveness,
                ),
                &teacher.disciple,
                &ActionKind::Teach,
            );
            let mut teacher_delta = base_delta(teacher);
            teacher_delta.spirit -= 7;
            teacher_delta
                .skill_experience
                .insert(art.clone(), teacher_gain);
            let mut student_delta = base_delta(student);
            student_delta.spirit -= 9;
            student_delta.skill_experience.insert(art.clone(), gain);
            settle_local_plan(&teacher.disciple, &job.kind, &mut teacher_delta);
            settle_local_plan(&student.disciple, &job.kind, &mut student_delta);
            result.disciples.extend([teacher_delta, student_delta]);
            let player_action = teacher.player || student.player;
            result.logs.push((
                player_action,
                if player_action {
                    format!(
                        "{}向{}传授{}，彼此印证；{}经验 +{}，{}经验 +{}。",
                        teacher.disciple.name,
                        student.disciple.name,
                        art_display(&art),
                        art_display(&art),
                        gain,
                        art_display(&art),
                        teacher_gain
                    )
                } else {
                    format!(
                        "{}向{}传授{}，师徒在堂中反复拆解招式。",
                        teacher.disciple.name,
                        student.disciple.name,
                        art_display(&art)
                    )
                },
            ));
        }
        ActionKind::Spar => {
            let art_a = first.disciple.martial_art.clone();
            let art_b = second.disciple.martial_art.clone();
            let level_a = skill_level(&first.disciple, &art_a);
            let level_b = skill_level(&second.disciple, &art_b);
            let (funded_a, cost_a) = training_funding(first, &job.kind);
            let (funded_b, cost_b) = training_funding(second, &job.kind);
            let mut gain_a = skill_experience(
                &first.disciple,
                &art_a,
                (first.disciple.aptitudes.strength + first.disciple.aptitudes.agility) / 2,
                90 + (level_b - level_a).clamp(0, 50),
            );
            let mut gain_b = skill_experience(
                &second.disciple,
                &art_b,
                (second.disciple.aptitudes.strength + second.disciple.aptitudes.agility) / 2,
                90 + (level_a - level_b).clamp(0, 50),
            );
            if !funded_a {
                gain_a = (gain_a / 2).max(1);
            }
            if !funded_b {
                gain_b = (gain_b / 2).max(1);
            }
            gain_a = scale_experience(gain_a, first.practice_effectiveness);
            gain_b = scale_experience(gain_b, second.practice_effectiveness);
            gain_a = scale_department_experience(gain_a, &first.disciple, &ActionKind::Spar);
            gain_b = scale_department_experience(gain_b, &second.disciple, &ActionKind::Spar);
            let mut a = base_delta(first);
            a.personal_silver -= cost_a;
            a.qi -= rng.gen_range(5..=10);
            a.energy -= 6;
            a.attainment += scale_department_experience(
                18 + i64::from((level_b - level_a).clamp(-10, 30) / 2),
                &first.disciple,
                &ActionKind::Spar,
            );
            a.skill_experience.insert(art_a.clone(), gain_a);
            let mut b = base_delta(second);
            b.personal_silver -= cost_b;
            b.qi -= rng.gen_range(5..=10);
            b.energy -= 6;
            b.attainment += scale_department_experience(
                18 + i64::from((level_a - level_b).clamp(-10, 30) / 2),
                &second.disciple,
                &ActionKind::Spar,
            );
            b.skill_experience.insert(art_b.clone(), gain_b);
            settle_local_plan(&first.disciple, &job.kind, &mut a);
            settle_local_plan(&second.disciple, &job.kind, &mut b);
            result.disciples.extend([a, b]);
            let player_action = first.player || second.player;
            result.logs.push((
                player_action,
                if player_action {
                    format!(
                        "{}与{}在演武场切磋，{}经验 +{}，{}经验 +{}。",
                        first.disciple.name,
                        second.disciple.name,
                        art_display(&art_a),
                        gain_a,
                        art_display(&art_b),
                        gain_b
                    )
                } else {
                    format!(
                        "{}与{}在演武场切磋，各展所长，引来同门围观。",
                        first.disciple.name, second.disciple.name
                    )
                },
            ));
        }
        _ => unreachable!("双人任务仅用于传授或切磋"),
    }
    result
}

fn execute_solo(actor: &Actor, kind: &ActionKind, rng: &mut StdRng) -> JobResult {
    execute_solo_with_encounters(actor, kind, rng, true)
}

fn execute_solo_with_encounters(
    actor: &Actor,
    kind: &ActionKind,
    rng: &mut StdRng,
    travel_encounters: bool,
) -> JobResult {
    let d = &actor.disciple;
    let mut result = JobResult::default();
    let mut delta = base_delta(actor);
    let (training_funded, training_cost) = training_funding(actor, kind);
    delta.personal_silver -= training_cost;
    let log;

    if d.away_months > 0 {
        let department_kind = if matches!(kind, ActionKind::SectMission) {
            ActionKind::SectMission
        } else {
            ActionKind::Wander
        };
        let returning = d.away_months <= 1;
        let mut plan = d.action.clone().unwrap_or(ActionPlan {
            kind: kind.clone(),
            remaining_months: d.away_months,
            ..ActionPlan::default()
        });
        let mut journey = plan.journey.clone().unwrap_or_else(|| {
            create_journey(
                actor,
                kind,
                d.away_months.max(plan.remaining_months).max(1),
                travel_encounters,
                rng,
            )
        });
        journey.elapsed_months = (journey.elapsed_months + 1).min(journey.total_months.max(1));
        delta.away_months = Some((d.away_months - 1).max(0));
        if matches!(kind, ActionKind::SectMission) {
            delta.attainment += country::scale_positive_i64(
                scale_department_experience(5, d, &department_kind),
                actor.safety_percent,
            );
        } else {
            delta.attainment += country::scale_positive_i64(
                scale_department_experience(
                    8 + d.aptitudes.fortune as i64 / 4,
                    d,
                    &department_kind,
                ),
                actor.safety_percent,
            );
        }
        let art = d.martial_art.clone();
        let gain = country::scale_positive_i64(
            scale_department_experience(
                skill_experience(
                    d,
                    &art,
                    (d.aptitudes.strength + d.aptitudes.agility) / 2,
                    if matches!(kind, ActionKind::SectMission) {
                        55
                    } else {
                        85
                    },
                ),
                d,
                &department_kind,
            ),
            actor.safety_percent,
        );
        delta.skill_experience.insert(art.clone(), gain);
        log = if returning {
            let settlement =
                settle_journey(actor, kind, &mut journey, rng, &mut delta, &mut result);
            let encounter = if travel_encounters && !journey.encounter_resolved {
                let encounter = journey
                    .encounter_id
                    .as_deref()
                    .and_then(travel_encounter_from_id)
                    .unwrap_or(TravelEncounterKind::QuietRoad);
                journey.encounter_resolved = true;
                apply_travel_encounter(actor, kind, encounter, rng, &mut delta, &mut result)
            } else {
                String::new()
            };
            delta.action = Some(None);
            format!(
                "{}自{}风尘归山；{}经验 +{}。{}{}",
                d.name,
                journey_destination_name(&journey.destination_id),
                art_display(&art),
                gain,
                settlement,
                encounter
            )
        } else {
            plan.remaining_months = d.away_months - 1;
            plan.journey = Some(journey.clone());
            delta.action = Some(Some(plan));
            format!(
                "{}仍在{}办理{}，途中不忘磨炼{}，经验 +{}。",
                d.name,
                journey_destination_name(&journey.destination_id),
                journey_template_name(&journey.template_id),
                art_display(&art),
                gain
            )
        };
        result.disciples.push(delta);
        result.logs.push((
            actor.player,
            if actor.player {
                log
            } else {
                npc_solo_chronicle(&d.name, kind)
            },
        ));
        return result;
    }

    match kind {
        ActionKind::Read => {
            let art = preferred_book(actor);
            let intelligence = disciple::effective_intelligence(d);
            let gain = scale_department_experience(
                scale_experience(
                    adjusted_training_experience(
                        d,
                        &art,
                        skill_experience(d, &art, intelligence, 85),
                    ),
                    actor.scripture_effectiveness,
                ),
                d,
                kind,
            );
            let mut foundation = accompanying_basic_training(d, &art, intelligence, 35, rng);
            if let Some((_, basic_gain)) = &mut foundation {
                *basic_gain = scale_experience(*basic_gain, actor.scripture_effectiveness);
                *basic_gain = scale_department_experience(*basic_gain, d, kind);
            }
            delta.spirit -= 10;
            delta.energy -= 3;
            delta.skill_experience.insert(art.clone(), gain);
            if let Some((basic, basic_gain)) = &foundation {
                *delta.skill_experience.entry(basic.clone()).or_default() += *basic_gain;
            }
            log = format!(
                "{}研读{}有所领悟，{}经验 +{}{}。",
                d.name,
                art_display(&art),
                art_display(&art),
                gain,
                foundation
                    .map(|(basic, basic_gain)| format!(
                        "，并夯实{}，经验 +{}",
                        art_display(&basic),
                        basic_gain
                    ))
                    .unwrap_or_default()
            );
        }
        ActionKind::Practice => {
            let art = practice_art(d, rng);
            let aptitude = (d.aptitudes.strength + d.aptitudes.agility) / 2;
            let mut gain =
                adjusted_training_experience(d, &art, skill_experience(d, &art, aptitude, 115));
            if !training_funded {
                gain = (gain / 2).max(1);
            }
            gain = scale_experience(gain, actor.practice_effectiveness);
            gain = scale_department_experience(gain, d, kind);
            let mut foundation = accompanying_basic_training(d, &art, aptitude, 45, rng);
            if !training_funded {
                if let Some((_, basic_gain)) = &mut foundation {
                    *basic_gain = (*basic_gain / 2).max(1);
                }
            }
            if let Some((_, basic_gain)) = &mut foundation {
                *basic_gain = scale_experience(*basic_gain, actor.practice_effectiveness);
                *basic_gain = scale_department_experience(*basic_gain, d, kind);
            }
            delta.qi -= 4;
            delta.neili -= 4;
            delta.energy -= 10;
            delta.skill_experience.insert(art.clone(), gain);
            if let Some((basic, basic_gain)) = &foundation {
                *delta.skill_experience.entry(basic.clone()).or_default() += *basic_gain;
            }
            log = format!(
                "{}在演武场反复练习{}，{}经验 +{}{}。",
                d.name,
                art_display(&art),
                art_display(&art),
                gain,
                foundation
                    .map(|(basic, basic_gain)| format!(
                        "，并夯实{}，经验 +{}",
                        art_display(&basic),
                        basic_gain
                    ))
                    .unwrap_or_default()
            );
        }
        ActionKind::TemperBody => {
            let mut gain = 2 + d.aptitudes.constitution / 12;
            if !training_funded {
                gain = (gain / 2).max(1);
            }
            gain = scale_output(gain, actor.practice_effectiveness);
            gain = scale_department_output(gain, d, kind);
            delta.qi -= 11;
            delta.energy -= 5;
            delta.qi_max += gain;
            log = format!("{}打熬筋骨，气血上限添了{}点。", d.name, gain);
        }
        ActionKind::CultivateNeili => {
            let art = inner_skill(d);
            let level = disciple::effective_force_level(d).max(1);
            let cap = disciple::neili_training_cap(d);
            let at_cap = d.attributes.neili.maximum >= cap;
            let spirit_ready = d.attributes.spirit.current.saturating_mul(10)
                >= d.attributes.spirit.maximum.saturating_mul(7);
            let cost = if at_cap || !spirit_ready {
                0
            } else {
                (10 + disciple::effective_aptitudes(d).constitution / 4)
                    .min(d.attributes.qi.current.saturating_sub(1))
            };
            let mut gain = if !at_cap && cost >= 10 {
                (1 + level / 80 + d.aptitudes.constitution / 25)
                    .min((cap - d.attributes.neili.maximum).max(0))
            } else {
                0
            };
            if !training_funded {
                gain = if gain > 0 { (gain / 2).max(1) } else { 0 };
            }
            gain = scale_output(gain, actor.practice_effectiveness);
            gain = scale_department_output(gain, d, kind)
                .min((cap - d.attributes.neili.maximum).max(0));
            delta.qi -= cost;
            delta.neili_max += gain;
            delta.neili += gain;
            log = if at_cap {
                format!(
                    "{}盘膝打坐，但现有内力已达到{}所能承载的修炼上限。",
                    d.name,
                    art_display(&art)
                )
            } else if !spirit_ready {
                format!("{}心神不宁，精神不足七成，只得暂缓打坐。", d.name)
            } else if cost < 10 {
                format!("{}气血不济，打坐片刻便只得收功。", d.name)
            } else if gain > 0 {
                format!(
                    "{}打坐修炼，以{}点气血炼化真气，内力精进，上限 +{}。",
                    d.name, cost, gain
                )
            } else {
                format!("{}盘膝打坐，却觉内功修为已遇瓶颈。", d.name)
            };
        }
        ActionKind::Meditate => {
            let knowledge = disciple::knowledge_level(d).max(1);
            let intelligence = disciple::effective_intelligence(d);
            let cap = disciple::energy_training_cap(d);
            let at_cap = d.attributes.energy.maximum >= cap;
            let qi_ready = d.attributes.qi.current.saturating_mul(10)
                >= d.attributes.qi.maximum.saturating_mul(7);
            let cost = if at_cap || !qi_ready {
                0
            } else {
                (10 + intelligence / 4).min(d.attributes.spirit.current.saturating_sub(1))
            };
            let gain = if !at_cap && cost >= 10 {
                (1 + knowledge / 100 + intelligence / 30)
                    .min((cap - d.attributes.energy.maximum).max(0))
            } else {
                0
            };
            let gain =
                scale_department_output(scale_output(gain, actor.scripture_effectiveness), d, kind)
                    .min((cap - d.attributes.energy.maximum).max(0));
            delta.spirit -= cost;
            delta.energy_max += gain;
            delta.energy += gain;
            log = if at_cap {
                format!("{}澄心冥想，但现有精力已达到知识修为上限。", d.name)
            } else if !qi_ready {
                format!("{}气血未复七成，难以久坐存神，只得暂缓冥想。", d.name)
            } else if cost < 10 {
                format!("{}精神不济，冥想片刻便难以为继。", d.name)
            } else if gain > 0 {
                format!(
                    "{}澄心冥想，以{}点精神凝炼心神，精力上限添了{}点。",
                    d.name, cost, gain
                )
            } else {
                format!("{}澄心冥想，却觉精力修为已遇瓶颈。", d.name)
            };
        }
        ActionKind::SectMission => {
            let duration = rng.gen_range(1..=3);
            let journey = create_journey(actor, kind, duration, travel_encounters, rng);
            delta.away_months = Some(duration);
            let mut plan = d.action.clone().unwrap_or_default();
            plan.kind = ActionKind::SectMission;
            plan.remaining_months = duration;
            plan.journey = Some(journey.clone());
            delta.action = Some(Some(plan));
            delta.attainment += country::scale_positive_i64(
                scale_department_experience(8, d, kind),
                actor.safety_percent,
            );
            let art = d.martial_art.clone();
            let skill_gain = country::scale_positive_i64(
                scale_department_experience(
                    skill_experience(d, &art, d.aptitudes.strength, 35),
                    d,
                    kind,
                ),
                actor.safety_percent,
            );
            delta.skill_experience.insert(art.clone(), skill_gain);
            log = format!(
                "{}领下{}，启程前往{}，卷宗难度{}，约需{}个月方回；{}经验 +{}。赏银、声望与物资待归山验收后一次结算。",
                d.name,
                journey_template_name(&journey.template_id),
                journey_destination_name(&journey.destination_id),
                journey.difficulty,
                duration,
                art_display(&art),
                skill_gain
            );
        }
        ActionKind::Wander => {
            let duration = rng.gen_range(1..=4);
            let journey = create_journey(actor, kind, duration, travel_encounters, rng);
            delta.away_months = Some(duration);
            let mut plan = d.action.clone().unwrap_or_default();
            plan.kind = ActionKind::Wander;
            plan.remaining_months = duration;
            plan.journey = Some(journey.clone());
            delta.action = Some(Some(plan));
            let gain = country::scale_positive_i64(
                scale_department_experience(8 + d.aptitudes.fortune as i64 / 4, d, kind),
                actor.safety_percent,
            );
            delta.attainment += gain;
            delta.qi -= rng.gen_range(0..=8);
            let art = d.martial_art.clone();
            let skill_gain = country::scale_positive_i64(
                scale_department_experience(
                    skill_experience(d, &art, d.aptitudes.agility, 45),
                    d,
                    kind,
                ),
                actor.safety_percent,
            );
            delta.skill_experience.insert(art.clone(), skill_gain);
            log = format!(
                "{}负笈前往{}自由历练，卷宗难度{}，预备{}个月后归山；{}经验 +{}。本程奇遇与所得归山时揭晓。",
                d.name,
                journey_destination_name(&journey.destination_id),
                journey.difficulty,
                duration,
                art_display(&art),
                skill_gain
            );
        }
        ActionKind::Recover => {
            let multiplier = 100 + actor.recovery_bonus;
            delta.qi += (16 + d.aptitudes.constitution / 3) * multiplier / 100;
            delta.spirit += (16 + d.aptitudes.intelligence / 3) * multiplier / 100;
            delta.neili += 6 * multiplier / 100;
            delta.energy += 8 * multiplier / 100;
            log = format!("{}在门中调息静养，气色渐复。", d.name);
        }
        ActionKind::Maintain => {
            let target = d
                .action
                .as_ref()
                .and_then(|plan| plan.target_id.clone())
                .unwrap_or_else(|| "logistics".into());
            let work = scale_department_output(
                scale_output(6 + d.aptitudes.strength / 6, actor.logistics_effectiveness),
                d,
                kind,
            );
            *result
                .sects
                .entry(actor.sect_id.clone())
                .or_default()
                .building_maintenance
                .entry(target)
                .or_default() += work;
            delta.energy -= 8;
            delta.merit += 3;
            log = format!("{}巡检梁柱瓦石，投入了{}点维护工作。", d.name, work);
        }
        ActionKind::Construct => {
            let target = d
                .action
                .as_ref()
                .and_then(|plan| plan.target_id.clone())
                .unwrap_or_else(|| "logistics".into());
            let work = scale_department_output(
                scale_output(8 + d.aptitudes.strength / 5, actor.logistics_effectiveness),
                d,
                kind,
            );
            *result
                .sects
                .entry(actor.sect_id.clone())
                .or_default()
                .building_work
                .entry(target)
                .or_default() += work;
            delta.qi -= 5;
            delta.energy -= 10;
            delta.merit += 5;
            log = format!("{}参与营造，完成了{}点建造工作。", d.name, work);
        }
        ActionKind::Produce => {
            let quantity = scale_department_output(
                scale_output(
                    6 + d.aptitudes.constitution / 5,
                    actor.warehouse_effectiveness,
                ),
                d,
                kind,
            );
            *result
                .sects
                .entry(actor.sect_id.clone())
                .or_default()
                .inventory
                .entry("粮秣".into())
                .or_default() += quantity;
            delta.energy -= 7;
            delta.merit += 2;
            if dispatched_by(d, "warehouse") {
                delta.personal_silver += 3;
                log = format!(
                    "{}整理仓中账物，入库粮秣{}份，并得私银3两。",
                    d.name, quantity
                );
            } else {
                log = format!("{}操持门中生产，入库粮秣{}份。", d.name, quantity);
            }
        }
        ActionKind::Business => {
            let silver = country::scale_positive(
                scale_department_output(
                    scale_output(
                        10 + d.aptitudes.intelligence / 2 + rng.gen_range(0..=12),
                        actor.warehouse_effectiveness,
                    ),
                    d,
                    kind,
                ),
                actor.market_percent,
            );
            result
                .sects
                .entry(actor.sect_id.clone())
                .or_default()
                .silver += silver;
            delta.merit += 3;
            if dispatched_by(d, "warehouse") {
                delta.personal_silver += 5;
                log = format!("{}为仓库采买，带回库银{}两，并得私银5两。", d.name, silver);
            } else {
                log = format!("{}下山经营世俗产业，带回库银{}两。", d.name, silver);
            }
        }
        ActionKind::Gather => {
            let herbs = scale_department_output(
                scale_output(2 + d.aptitudes.fortune / 8, actor.herb_hall_effectiveness),
                d,
                kind,
            );
            let sect_delta = result.sects.entry(actor.sect_id.clone()).or_default();
            *sect_delta.inventory.entry("草药".into()).or_default() += herbs;
            let iron_chance =
                (0.35 * f64::from(actor.herb_hall_effectiveness.max(0)) / 100.0).min(1.0);
            if rng.gen_bool(iron_chance) {
                *sect_delta.inventory.entry("精铁".into()).or_default() += 1;
            }
            delta.energy -= 6;
            delta.merit += 3;
            log = format!("{}入山采集，带回草药{}份。", d.name, herbs);
        }
        ActionKind::Teach | ActionKind::Spar => unreachable!("独行任务已回退为研读"),
    }
    settle_local_plan(d, kind, &mut delta);
    result.disciples.push(delta);
    result.logs.push((
        actor.player,
        if actor.player {
            log
        } else {
            npc_solo_chronicle(&d.name, kind)
        },
    ));
    result
}

fn base_delta(actor: &Actor) -> DiscipleDelta {
    DiscipleDelta {
        id: actor.disciple.id.clone(),
        sect_id: actor.sect_id.clone(),
        ..DiscipleDelta::default()
    }
}

fn dispatched_by(disciple: &Disciple, building_id: &str) -> bool {
    disciple
        .action
        .as_ref()
        .and_then(|plan| plan.assigned_by.as_deref())
        .is_some_and(|source| source == format!("building:{building_id}"))
}

fn settle_local_plan(disciple: &Disciple, kind: &ActionKind, delta: &mut DiscipleDelta) {
    if let Some(plan) = disciple
        .action
        .as_ref()
        .filter(|_| !matches!(kind, ActionKind::SectMission | ActionKind::Wander))
    {
        let mut next = plan.clone();
        next.remaining_months -= 1;
        delta.action = Some((next.remaining_months > 0).then_some(next));
    }
}

fn is_martial_training(kind: &ActionKind) -> bool {
    matches!(
        kind,
        ActionKind::Practice
            | ActionKind::Spar
            | ActionKind::TemperBody
            | ActionKind::CultivateNeili
    )
}

/// 返回本月是否足额备齐练武耗材，以及实际扣除的私银。
fn training_funding(actor: &Actor, kind: &ActionKind) -> (bool, i32) {
    if !is_martial_training(kind) {
        return (true, 0);
    }
    let required =
        if matches!(kind, ActionKind::Practice) && dispatched_by(&actor.disciple, "practice") {
            2
        } else {
            1
        };
    if actor.disciple.personal_silver >= required {
        (true, required)
    } else {
        (false, 0)
    }
}

fn skill_level(d: &Disciple, art: &str) -> i32 {
    d.martial_progress
        .proficiencies
        .get(art)
        .map(|progress| progress.level)
        .unwrap_or(0)
}

/// 把 MUD 中连续多次练习压缩为一个月：基础技能越深，每月可获得的技能经验越多。
fn skill_experience(d: &Disciple, art: &str, aptitude: i32, intensity: i32) -> i64 {
    let level = skill_level(d, art).max(1);
    let sessions = 14 + aptitude.max(0) / 2;
    let difficulty = crate::models::martial_art::martial_art_by_id(art)
        .map(|candidate| candidate.difficulty)
        .unwrap_or(10)
        .max(5);
    let per_session = level / 5 + 1;
    (i64::from(per_session) * i64::from(sessions) * i64::from(intensity.max(1))
        / i64::from(80 + difficulty * 3))
    .max(1)
}

fn scale_experience(value: i64, effectiveness: i32) -> i64 {
    value
        .saturating_mul(i64::from(effectiveness.max(0)))
        .saturating_div(100)
}

fn scale_output(value: i32, effectiveness: i32) -> i32 {
    value
        .saturating_mul(effectiveness.max(0))
        .saturating_div(100)
}

fn department_bonus_percent(disciple: &Disciple, kind: &ActionKind) -> i32 {
    match (disciple.department.as_ref(), kind) {
        (
            Some(Department::Transmission),
            ActionKind::Teach
            | ActionKind::Practice
            | ActionKind::Spar
            | ActionKind::TemperBody
            | ActionKind::CultivateNeili,
        ) => 10,
        (Some(Department::Library), ActionKind::Read | ActionKind::Meditate) => 10,
        (Some(Department::Apothecary), ActionKind::Gather) => 15,
        (Some(Department::Treasury), ActionKind::Produce | ActionKind::Business) => 10,
        (Some(Department::Stewardship), ActionKind::Maintain | ActionKind::Construct) => 15,
        (Some(Department::ExternalAffairs), ActionKind::SectMission | ActionKind::Wander) => 10,
        _ => 0,
    }
}

/// 小整数成长采用向上取整，确保任职部门在内力、精力等低基数收益上
/// 仍能产生实际效果；未任职或职责不匹配时严格保持原始 100% 产出。
fn scale_department_output(value: i32, disciple: &Disciple, kind: &ActionKind) -> i32 {
    let bonus = department_bonus_percent(disciple, kind);
    if value <= 0 || bonus <= 0 {
        return value;
    }
    value.saturating_add(
        value
            .saturating_mul(bonus)
            .saturating_add(99)
            .saturating_div(100),
    )
}

fn scale_department_experience(value: i64, disciple: &Disciple, kind: &ActionKind) -> i64 {
    let bonus = i64::from(department_bonus_percent(disciple, kind));
    if value <= 0 || bonus <= 0 {
        return value;
    }
    value.saturating_add(
        value
            .saturating_mul(bonus)
            .saturating_add(99)
            .saturating_div(100),
    )
}

fn practice_art(d: &Disciple, rng: &mut impl Rng) -> String {
    let selected = d
        .action
        .as_ref()
        .and_then(|plan| plan.martial_art_id.clone())
        .filter(|art| d.martial_progress.proficiencies.contains_key(art))
        .unwrap_or_else(|| d.martial_art.clone());
    let selected = if selected == "basic_parry" {
        disciple::prepared_skill_id(d, "basic_parry")
            .unwrap_or("basic_parry")
            .to_string()
    } else {
        selected
    };
    if d.action.is_none() && rng.gen_bool(0.25) {
        if let Some(basic) = corresponding_basic(d, &selected) {
            return basic;
        }
    }
    selected
}

fn corresponding_basic(d: &Disciple, art_id: &str) -> Option<String> {
    let art = crate::models::martial_art::martial_art_by_id(art_id)?;
    (art.tier != crate::models::martial_art::MartialTier::Basic
        && !art.basic_skill.is_empty()
        && d.martial_progress
            .proficiencies
            .contains_key(&art.basic_skill))
    .then_some(art.basic_skill)
}

/// 基础武学修炼多得两成半；高级武学顶到基础等级后进入半效瓶颈。
fn adjusted_training_experience(d: &Disciple, art_id: &str, gain: i64) -> i64 {
    let Some(art) = crate::models::martial_art::martial_art_by_id(art_id) else {
        return gain;
    };
    if art.tier == crate::models::martial_art::MartialTier::Basic {
        return (gain * 125 / 100).max(1);
    }
    let at_basic_cap = d
        .martial_progress
        .proficiencies
        .get(&art.basic_skill)
        .is_some_and(|basic| skill_level(d, art_id) >= basic.level);
    if at_basic_cap {
        (gain / 2).max(1)
    } else {
        gain
    }
}

/// 研读或练习高级武学时，有三成机会同时夯实对应基础武学。
fn accompanying_basic_training(
    d: &Disciple,
    art_id: &str,
    aptitude: i32,
    intensity: i32,
    rng: &mut impl Rng,
) -> Option<(String, i64)> {
    let basic = corresponding_basic(d, art_id)?;
    if !rng.gen_bool(0.3) {
        return None;
    }
    let gain =
        adjusted_training_experience(d, &basic, skill_experience(d, &basic, aptitude, intensity));
    Some((basic, gain))
}

fn inner_skill(d: &Disciple) -> String {
    disciple::prepared_skill_id(d, "basic_force")
        .unwrap_or("basic_force")
        .to_string()
}

fn can_teach_art(teacher: &Disciple, student: &Disciple, art_id: &str) -> bool {
    skill_level(teacher, art_id) > skill_level(student, art_id) && can_study_book(student, art_id)
}

fn teaching_art(teacher: &Disciple, student: &Disciple) -> Option<String> {
    teacher
        .martial_progress
        .proficiencies
        .keys()
        .filter(|art| can_teach_art(teacher, student, art))
        .max_by_key(|art| skill_level(teacher, art) - skill_level(student, art))
        .cloned()
}

fn preferred_book(actor: &Actor) -> String {
    let d = &actor.disciple;
    if let Some(book) = d
        .action
        .as_ref()
        .and_then(|plan| plan.martial_art_id.clone())
    {
        return book;
    }
    d.martial_progress
        .private_books
        .iter()
        .chain(actor.public_books.iter())
        .filter(|book| can_study_book(d, book))
        .min_by_key(|book| {
            d.martial_progress
                .proficiencies
                .get(*book)
                .map(|progress| progress.level)
                .unwrap_or(0)
        })
        .cloned()
        .unwrap_or_else(|| d.martial_art.clone())
}

fn can_study_book(d: &Disciple, art_id: &str) -> bool {
    let Some(art) = crate::models::martial_art::martial_art_by_id(art_id) else {
        return false;
    };
    let rank_allowed = match art.tier {
        crate::models::martial_art::MartialTier::Basic
        | crate::models::martial_art::MartialTier::Chore => true,
        crate::models::martial_art::MartialTier::Outer => {
            matches!(d.rank, DiscipleRank::Outer | DiscipleRank::Inner)
        }
        crate::models::martial_art::MartialTier::Inner => d.rank == DiscipleRank::Inner,
    };
    if !rank_allowed || !art.is_combat || art.tier == crate::models::martial_art::MartialTier::Basic
    {
        return rank_allowed;
    }
    let knows_basic = art.basic_skill.is_empty() || skill_level(d, &art.basic_skill) > 0;
    let knows_knowledge = art
        .sect_id
        .as_deref()
        .map(crate::models::martial_art::knowledge_skill_id)
        .is_none_or(|knowledge| skill_level(d, &knowledge) > 0);
    knows_basic && knows_knowledge
}

fn create_journey(
    actor: &Actor,
    action: &ActionKind,
    duration: i32,
    with_encounter: bool,
    rng: &mut impl Rng,
) -> JourneyProgress {
    const MISSION_TEMPLATES: &[&str] = &[
        "escort_supplies",
        "seek_physician",
        "mediate_dispute",
        "clear_bandits",
    ];
    const DESTINATIONS: &[&str] = &[
        "xiangyang",
        "linan",
        "luoyang",
        "dali",
        "liangzhou",
        "taihu",
    ];
    let template_id = if matches!(action, ActionKind::SectMission) {
        MISSION_TEMPLATES[rng.gen_range(0..MISSION_TEMPLATES.len())]
    } else {
        "free_wander"
    };
    let destination_id = DESTINATIONS[rng.gen_range(0..DESTINATIONS.len())];
    let capability = journey_capability(&actor.disciple, template_id);
    let difficulty = (capability + rng.gen_range(-12..=18)).clamp(20, 300);
    let encounter_id = with_encounter.then(|| {
        travel_encounter_id(choose_travel_encounter(actor, action, true, rng)).to_string()
    });
    JourneyProgress {
        id: format!("journey:{}:{:016x}", actor.disciple.id, rng.gen::<u64>()),
        template_id: template_id.into(),
        destination_id: destination_id.into(),
        difficulty,
        total_months: duration.max(1),
        elapsed_months: 0,
        encounter_id,
        encounter_resolved: false,
        outcome: None,
        settled: false,
    }
}

fn settle_journey(
    actor: &Actor,
    action: &ActionKind,
    journey: &mut JourneyProgress,
    rng: &mut impl Rng,
    disciple_delta: &mut DiscipleDelta,
    result: &mut JobResult,
) -> String {
    if journey.settled {
        return "这份旅程卷宗早已验收，不再重复发赏。".into();
    }
    let capability = journey_capability(&actor.disciple, &journey.template_id);
    let road_support = (actor.safety_percent - 100) / 3;
    let margin = capability + road_support + rng.gen_range(-12..=12) - journey.difficulty;
    let outcome = journey
        .outcome
        .clone()
        .unwrap_or_else(|| journey_outcome(margin));
    journey.outcome = Some(outcome.clone());
    journey.settled = true;

    if matches!(action, ActionKind::SectMission) {
        settle_mission(actor, journey, &outcome, margin, disciple_delta, result)
    } else {
        settle_wander(journey, &outcome, disciple_delta)
    }
}

fn journey_outcome(margin: i32) -> JourneyOutcome {
    if margin >= 0 {
        JourneyOutcome::Success
    } else if margin >= -20 {
        JourneyOutcome::Partial
    } else {
        JourneyOutcome::Failed
    }
}

fn settle_mission(
    actor: &Actor,
    journey: &JourneyProgress,
    outcome: &JourneyOutcome,
    margin: i32,
    disciple_delta: &mut DiscipleDelta,
    result: &mut JobResult,
) -> String {
    let quality = (90 + margin / 2 + actor.disciple.aptitudes.fortune / 5).clamp(60, 120);
    let base_reward = scale_department_output(
        28 + journey.total_months * 14 + journey.difficulty / 3,
        &actor.disciple,
        &ActionKind::SectMission,
    );
    let reward = match outcome {
        JourneyOutcome::Success => base_reward * quality / 100,
        JourneyOutcome::Partial => base_reward * quality / 200,
        JourneyOutcome::Failed => 0,
    };
    let direction_percent = match actor.moral_direction {
        MoralDirection::Righteous => 90,
        MoralDirection::Neutral => 100,
        MoralDirection::Villainous => 120,
    };
    let reward = country::scale_positive(
        reward.saturating_mul(direction_percent) / 100,
        actor.external_percent,
    );
    let sect_delta = result.sects.entry(actor.sect_id.clone()).or_default();
    sect_delta.silver += reward;

    let (prestige_delta, morality_delta) = match outcome {
        JourneyOutcome::Success => {
            disciple_delta.merit += 12;
            disciple_delta.reputation += 2;
            disciple_delta.attainment += 12;
            match actor.moral_direction {
                MoralDirection::Righteous => (4, 1),
                MoralDirection::Neutral => (3, 0),
                MoralDirection::Villainous => (-1, -1),
            }
        }
        JourneyOutcome::Partial => {
            disciple_delta.merit += 6;
            disciple_delta.reputation += 1;
            disciple_delta.attainment += 6;
            match actor.moral_direction {
                MoralDirection::Righteous => (2, 1),
                MoralDirection::Neutral => (1, 0),
                MoralDirection::Villainous => (-1, -1),
            }
        }
        JourneyOutcome::Failed => {
            disciple_delta.merit -= 2;
            disciple_delta.loyalty -= 2;
            disciple_delta.attainment += 2;
            (
                if actor.moral_direction == MoralDirection::Villainous {
                    -2
                } else {
                    -1
                },
                0,
            )
        }
    };
    sect_delta.prestige += prestige_delta;
    sect_delta.morality += morality_delta;

    let item_text = mission_item_reward(
        &journey.template_id,
        outcome,
        journey.total_months,
        sect_delta,
    );
    match outcome {
        JourneyOutcome::Success => format!(
            "{}圆满验收：门派银两 +{}、声望 {}{}，个人功绩 +12、声名 +2{}。",
            journey_template_name(&journey.template_id),
            reward,
            signed_change(prestige_delta),
            morality_change_text(morality_delta),
            item_text
        ),
        JourneyOutcome::Partial => format!(
            "{}勉强办成：门派银两 +{}、声望 {}{}，个人功绩 +6、声名 +1{}。",
            journey_template_name(&journey.template_id),
            reward,
            signed_change(prestige_delta),
            morality_change_text(morality_delta),
            item_text
        ),
        JourneyOutcome::Failed => format!(
            "{}未通过验收：门派声望 {}，个人功绩 -2、门忠 -2。",
            journey_template_name(&journey.template_id),
            signed_change(prestige_delta)
        ),
    }
}

fn signed_change(delta: i32) -> String {
    if delta >= 0 {
        format!("+{delta}")
    } else {
        delta.to_string()
    }
}

fn morality_change_text(delta: i32) -> String {
    if delta == 0 {
        String::new()
    } else {
        format!("、道德 {}", signed_change(delta))
    }
}

fn settle_wander(
    journey: &JourneyProgress,
    outcome: &JourneyOutcome,
    disciple_delta: &mut DiscipleDelta,
) -> String {
    match outcome {
        JourneyOutcome::Success => {
            disciple_delta.attainment += 18;
            disciple_delta.reputation += 3;
            disciple_delta.merit += 2;
            "本程游历见闻丰厚：造诣 +18、个人声名 +3、功绩 +2。".into()
        }
        JourneyOutcome::Partial => {
            disciple_delta.attainment += 8;
            disciple_delta.reputation += 1;
            format!(
                "本程游历虽有波折，仍走完{}一带：造诣 +8、个人声名 +1。",
                journey_destination_name(&journey.destination_id)
            )
        }
        JourneyOutcome::Failed => {
            disciple_delta.qi -= 8;
            disciple_delta.spirit -= 4;
            disciple_delta.loyalty -= 1;
            disciple_delta.attainment += 3;
            "本程游历受阻而返：气血 -8、精神 -4、门忠 -1，仍从挫折中得到造诣 +3。".into()
        }
    }
}

fn mission_item_reward(
    template_id: &str,
    outcome: &JourneyOutcome,
    duration: i32,
    sect_delta: &mut SectDelta,
) -> String {
    let factor = match outcome {
        JourneyOutcome::Success => 2,
        JourneyOutcome::Partial => 1,
        JourneyOutcome::Failed => 0,
    };
    if factor == 0 {
        return String::new();
    }
    let quantity = |base: i32| (base + duration.max(1)) * factor;
    match template_id {
        "escort_supplies" => {
            let grain = quantity(2);
            *sect_delta.inventory.entry("粮秣".into()).or_default() += grain;
            format!("、粮秣 +{}", grain)
        }
        "seek_physician" => {
            let herbs = quantity(1);
            *sect_delta.inventory.entry("草药".into()).or_default() += herbs;
            format!("、草药 +{}", herbs)
        }
        "clear_bandits" => {
            let iron = factor;
            *sect_delta.inventory.entry("精铁".into()).or_default() += iron;
            format!("、精铁 +{}", iron)
        }
        _ => String::new(),
    }
}

fn journey_capability(d: &Disciple, template_id: &str) -> i32 {
    let aptitude = disciple::effective_aptitudes(d);
    let combat = disciple::get_combat_score(d).clamp(0, 600);
    match template_id {
        "escort_supplies" => aptitude.strength + aptitude.constitution + combat / 4,
        "seek_physician" => {
            aptitude.intelligence + aptitude.fortune + d.attributes.reputation.clamp(0, 1000) / 10
        }
        "mediate_dispute" => {
            aptitude.intelligence
                + d.attributes.reputation.clamp(0, 1000) / 5
                + d.attributes.morality.clamp(0, 100) / 10
        }
        "clear_bandits" => aptitude.strength + aptitude.agility + combat / 3,
        _ => aptitude.agility + aptitude.fortune + combat / 5,
    }
}

fn journey_template_name(id: &str) -> &'static str {
    match id {
        "escort_supplies" => "护送粮饷",
        "seek_physician" => "寻访名医",
        "mediate_dispute" => "调停地界",
        "clear_bandits" => "清剿路匪",
        _ => "江湖游历",
    }
}

fn journey_destination_name(id: &str) -> &'static str {
    match id {
        "xiangyang" => "襄阳",
        "linan" => "临安",
        "luoyang" => "洛阳",
        "dali" => "大理",
        "liangzhou" => "凉州",
        "taihu" => "太湖",
        _ => "江湖",
    }
}

fn travel_encounter_id(encounter: TravelEncounterKind) -> &'static str {
    match encounter {
        TravelEncounterKind::QuietRoad => "quiet_road",
        TravelEncounterKind::FriendlySpar => "friendly_spar",
        TravelEncounterKind::BanditAmbush => "bandit_ambush",
        TravelEncounterKind::FoundSupplies => "found_supplies",
        TravelEncounterKind::HermitGuidance => "hermit_guidance",
        TravelEncounterKind::LostManual => "lost_manual",
    }
}

fn travel_encounter_from_id(id: &str) -> Option<TravelEncounterKind> {
    match id {
        "quiet_road" => Some(TravelEncounterKind::QuietRoad),
        "friendly_spar" => Some(TravelEncounterKind::FriendlySpar),
        "bandit_ambush" => Some(TravelEncounterKind::BanditAmbush),
        "found_supplies" => Some(TravelEncounterKind::FoundSupplies),
        "hermit_guidance" => Some(TravelEncounterKind::HermitGuidance),
        "lost_manual" => Some(TravelEncounterKind::LostManual),
        _ => None,
    }
}

fn choose_travel_encounter(
    actor: &Actor,
    action: &ActionKind,
    allow_manual: bool,
    rng: &mut impl Rng,
) -> TravelEncounterKind {
    let fortune = actor.disciple.aptitudes.fortune.clamp(0, 100) as u32;
    let safe_bonus = ((actor.safety_percent - 85).max(0) / 5) as u32;
    let danger_bonus = ((100 - actor.safety_percent).max(0) / 3) as u32;
    let mission = matches!(action, ActionKind::SectMission);
    let manual_weight = if allow_manual && !available_adventure_manuals(actor).is_empty() {
        2 + fortune / 8
    } else {
        0
    };
    let weights = [
        (
            TravelEncounterKind::QuietRoad,
            if mission { 24 } else { 16 },
        ),
        (TravelEncounterKind::FriendlySpar, 16 + safe_bonus),
        (
            TravelEncounterKind::BanditAmbush,
            if mission {
                18 + danger_bonus
            } else {
                22 + danger_bonus
            },
        ),
        (
            TravelEncounterKind::FoundSupplies,
            if mission { 28 } else { 16 },
        ),
        (TravelEncounterKind::HermitGuidance, 4 + fortune / 7),
        (TravelEncounterKind::LostManual, manual_weight),
    ];
    let total: u32 = weights.iter().map(|(_, weight)| *weight).sum();
    let mut draw = rng.gen_range(0..total.max(1));
    for (encounter, weight) in weights {
        if draw < weight {
            return encounter;
        }
        draw -= weight;
    }
    TravelEncounterKind::QuietRoad
}

fn apply_travel_encounter(
    actor: &Actor,
    action: &ActionKind,
    encounter: TravelEncounterKind,
    rng: &mut impl Rng,
    disciple_delta: &mut DiscipleDelta,
    result: &mut JobResult,
) -> String {
    let d = &actor.disciple;
    let art = travel_training_art(d);
    match encounter {
        TravelEncounterKind::QuietRoad => {
            if matches!(action, ActionKind::SectMission) {
                "沿途驿路平稳，差事按部就班。".into()
            } else {
                "一路观山问俗，虽无惊险，也添了几分见闻。".into()
            }
        }
        TravelEncounterKind::FriendlySpar => {
            const OUTSIDERS: &[&str] =
                &["河朔刀客", "关外镖师", "云游女侠", "江南剑客", "西域行者"];
            let outsider = OUTSIDERS[rng.gen_range(0..OUTSIDERS.len())];
            let desired_qi_cost = rng.gen_range(2..=6);
            let qi_cost = (d.attributes.qi.current + disciple_delta.qi - 1)
                .max(0)
                .min(desired_qi_cost);
            let aptitude = (d.aptitudes.strength + d.aptitudes.agility) / 2;
            let gain = scale_department_experience(
                skill_experience(
                    d,
                    &art,
                    aptitude,
                    if matches!(action, ActionKind::SectMission) {
                        45
                    } else {
                        70
                    },
                ),
                d,
                action,
            );
            let prevailed =
                disciple::get_combat_score(d) + rng.gen_range(0..=80) >= 65 + rng.gen_range(0..=80);
            disciple_delta.qi -= qi_cost;
            disciple_delta.attainment += 4 + i64::from(d.aptitudes.fortune.max(0) / 10);
            *disciple_delta
                .skill_experience
                .entry(art.clone())
                .or_default() += gain;
            if prevailed {
                disciple_delta.reputation += 1;
            }
            format!(
                "路遇{}邀约点到为止，{}，{}经验 +{}、气血 -{}。",
                outsider,
                if prevailed {
                    "数十合后略占上风，个人声名 +1"
                } else {
                    "虽落下风，却也看清自身破绽"
                },
                art_display(&art),
                gain,
                qi_cost
            )
        }
        TravelEncounterKind::BanditAmbush => {
            let danger = (115 - actor.safety_percent).clamp(0, 60);
            let own_score = disciple::get_combat_score(d) + rng.gen_range(0..=70);
            let threat_score = 55 + danger * 2 + rng.gen_range(0..=65);
            let prevailed = own_score >= threat_score;
            let gain = scale_department_experience(
                skill_experience(
                    d,
                    &art,
                    d.aptitudes.strength,
                    if prevailed { 80 } else { 30 },
                ),
                d,
                action,
            );
            *disciple_delta
                .skill_experience
                .entry(art.clone())
                .or_default() += gain;
            if prevailed {
                let qi_cost = rng.gen_range(4..=10);
                let spoils = rng.gen_range(8..=24);
                disciple_delta.qi -= qi_cost;
                disciple_delta.attainment += 8;
                disciple_delta.reputation += 2;
                let sect_delta = result.sects.entry(actor.sect_id.clone()).or_default();
                sect_delta.prestige += 1;
                sect_delta.morality += 1;
                if matches!(action, ActionKind::SectMission) {
                    sect_delta.silver += spoils;
                } else {
                    disciple_delta.personal_silver += spoils;
                }
                format!(
                    "撞破一伙剪径悍匪，力战驱散群寇；{}经验 +{}、气血 -{}，缴获{}两，个人声名 +2、本派声望 +1。",
                    art_display(&art),
                    gain,
                    qi_cost,
                    spoils
                )
            } else {
                let qi_loss = rng.gen_range(14..=28);
                let spirit_loss = rng.gen_range(3..=9);
                let silver_loss = rng.gen_range(4..=12).min(d.personal_silver.max(0));
                disciple_delta.qi -= qi_loss;
                disciple_delta.spirit -= spirit_loss;
                disciple_delta.personal_silver -= silver_loss;
                disciple_delta.attainment += 2;
                format!(
                    "遭悍匪围攻，苦战脱身；{}经验 +{}、气血 -{}、精神 -{}{}。",
                    art_display(&art),
                    gain,
                    qi_loss,
                    spirit_loss,
                    if silver_loss > 0 {
                        format!("、遗失私银{}两", silver_loss)
                    } else {
                        String::new()
                    }
                )
            }
        }
        TravelEncounterKind::FoundSupplies => {
            let grain = rng.gen_range(3..=8);
            let herbs = rng.gen_range(1..=4);
            let iron = i32::from(rng.gen_bool(if matches!(action, ActionKind::SectMission) {
                0.45
            } else {
                0.25
            }));
            let sect_delta = result.sects.entry(actor.sect_id.clone()).or_default();
            *sect_delta.inventory.entry("粮秣".into()).or_default() += grain;
            *sect_delta.inventory.entry("草药".into()).or_default() += herbs;
            if iron > 0 {
                *sect_delta.inventory.entry("精铁".into()).or_default() += iron;
            }
            if matches!(action, ActionKind::SectMission) {
                disciple_delta.merit += 2;
            }
            format!(
                "在荒寺旧驿清点出可用物资，带回粮秣{}份、草药{}份{}{}。",
                grain,
                herbs,
                if iron > 0 { "、精铁1份" } else { "" },
                if matches!(action, ActionKind::SectMission) {
                    "，功绩 +2"
                } else {
                    ""
                }
            )
        }
        TravelEncounterKind::HermitGuidance => {
            let gain = rng.gen_range(2..=5);
            let attainment = 10 + i64::from(d.aptitudes.fortune.max(0) / 4);
            disciple_delta.attainment += attainment;
            if rng.gen_bool(0.5) {
                disciple_delta.qi_max += gain;
                format!(
                    "偶遇山中异人指点吐纳关窍，造诣 +{}、气血上限 +{}。",
                    attainment, gain
                )
            } else {
                disciple_delta.spirit_max += gain;
                format!(
                    "偶遇山中异人点破心障，造诣 +{}、精神上限 +{}。",
                    attainment, gain
                )
            }
        }
        TravelEncounterKind::LostManual => {
            let manuals = available_adventure_manuals(actor);
            if manuals.is_empty() {
                return apply_travel_encounter(
                    actor,
                    action,
                    TravelEncounterKind::HermitGuidance,
                    rng,
                    disciple_delta,
                    result,
                );
            }
            let manual = manuals[rng.gen_range(0..manuals.len())].clone();
            disciple_delta.private_books.push(manual.clone());
            disciple_delta.attainment += 12;
            disciple_delta.reputation += 1;
            format!(
                "归途中在残碑夹层寻得{}遗卷，收入私人行囊；造诣 +12、个人声名 +1。",
                art_display(&manual)
            )
        }
    }
}

fn travel_training_art(d: &Disciple) -> String {
    if d.martial_progress
        .proficiencies
        .contains_key(&d.martial_art)
    {
        return d.martial_art.clone();
    }
    d.martial_progress
        .proficiencies
        .iter()
        .filter(|(id, _)| {
            crate::models::martial_art::martial_art_by_id(id).is_some_and(|art| art.is_combat)
        })
        .max_by_key(|(_, progress)| progress.level)
        .map(|(id, _)| id.clone())
        .unwrap_or_else(|| d.martial_art.clone())
}

fn available_adventure_manuals(actor: &Actor) -> Vec<String> {
    crate::models::martial_art::all_martial_arts()
        .into_iter()
        .filter(|art| {
            art.is_combat
                && art.tier != crate::models::martial_art::MartialTier::Basic
                && !actor
                    .disciple
                    .martial_progress
                    .proficiencies
                    .contains_key(&art.id)
                && !actor
                    .disciple
                    .martial_progress
                    .private_books
                    .contains(&art.id)
                && !actor.public_books.contains(&art.id)
                && (art.basic_skill.is_empty()
                    || actor
                        .disciple
                        .martial_progress
                        .proficiencies
                        .contains_key(&art.basic_skill))
                && art
                    .sect_id
                    .as_deref()
                    .map(crate::models::martial_art::knowledge_skill_id)
                    .is_none_or(|knowledge| {
                        actor
                            .disciple
                            .martial_progress
                            .proficiencies
                            .contains_key(&knowledge)
                    })
        })
        .map(|art| art.id)
        .collect()
}

fn apply_results(state: &mut GameState, results: Vec<JobResult>) -> Vec<GameEvent> {
    let research_by_sect =
        std::iter::once(("player".to_string(), state.sect.martial_research.clone()))
            .chain(
                state
                    .npc_sects
                    .iter()
                    .map(|sect| (sect.id.clone(), sect.martial_research.clone())),
            )
            .collect::<BTreeMap<_, _>>();
    let mut deltas = Vec::new();
    let mut sect_deltas: BTreeMap<String, SectDelta> = BTreeMap::new();
    let mut logs = Vec::new();
    let mut npc_logs = Vec::new();
    let mut npc_actions = 0;
    for result in results {
        deltas.extend(result.disciples);
        for (id, delta) in result.sects {
            let total = sect_deltas.entry(id).or_default();
            total.silver += delta.silver;
            total.prestige += delta.prestige;
            total.morality += delta.morality;
            total.morale += delta.morale;
            for (item, quantity) in delta.inventory {
                *total.inventory.entry(item).or_default() += quantity;
            }
            for (building, work) in delta.building_work {
                *total.building_work.entry(building).or_default() += work;
            }
            for (building, work) in delta.building_maintenance {
                *total.building_maintenance.entry(building).or_default() += work;
            }
        }
        for (player, text) in result.logs {
            if player {
                logs.push(text);
            } else {
                npc_actions += 1;
                npc_logs.push(text);
            }
        }
    }
    for delta in deltas {
        let research = research_by_sect.get(&delta.sect_id);
        if let Some(d) = state
            .disciples
            .iter_mut()
            .chain(state.npc_disciples.iter_mut())
            .find(|d| d.id == delta.id)
        {
            apply_disciple_delta(d, delta, research);
        }
    }
    for (id, delta) in sect_deltas {
        let target = if id == "player" {
            Some(&mut state.sect)
        } else {
            state.npc_sects.iter_mut().find(|sect| sect.id == id)
        };
        if let Some(target) = target {
            target.attributes.silver = (target.attributes.silver + delta.silver).max(0);
            target.attributes.prestige =
                (target.attributes.prestige + delta.prestige).clamp(0, 1000);
            target.attributes.morality =
                (target.attributes.morality + delta.morality).clamp(0, 100);
            target.attributes.morale = (target.attributes.morale + delta.morale).clamp(0, 100);
            for (item, quantity) in delta.inventory {
                *target.inventory.entry(item).or_default() += quantity;
            }
            for (building_id, work) in delta.building_work {
                if let Some(building) = target
                    .buildings
                    .iter_mut()
                    .find(|building| building.id == building_id)
                {
                    building.work_invested =
                        (building.work_invested + work).min(building.work_required);
                }
            }
            for (building_id, work) in delta.building_maintenance {
                if let Some(building) = target
                    .buildings
                    .iter_mut()
                    .find(|building| building.id == building_id)
                {
                    building.condition = (building.condition + work).min(100);
                }
            }
        }
    }
    sect::sync_legacy_fields(state);
    let mut events: Vec<GameEvent> = logs
        .into_iter()
        .map(|text| GameEvent {
            text,
            mood: "good".into(),
            year: state.year,
            month: state.month,
            category: "sect".into(),
        })
        .collect();
    events.extend(npc_logs.into_iter().take(3).map(|text| GameEvent {
        text,
        mood: "neutral".into(),
        year: state.year,
        month: state.month,
        category: "world".into(),
    }));
    if npc_actions > 0 {
        events.push(GameEvent {
            text: "江湖诸派亦各有动静，门人或勤修武艺，或下山历练。".into(),
            mood: "neutral".into(),
            year: state.year,
            month: state.month,
            category: "world".into(),
        });
    }
    events
}

fn apply_disciple_delta(
    d: &mut Disciple,
    delta: DiscipleDelta,
    martial_research: Option<&BTreeMap<String, i64>>,
) {
    d.attribute_bonuses.qi += delta.qi_max;
    d.attribute_bonuses.spirit += delta.spirit_max;
    d.attributes.neili.maximum = d
        .attributes
        .neili
        .maximum
        .saturating_add(delta.neili_max)
        .max(1);
    d.attributes.energy.maximum = d
        .attributes
        .energy
        .maximum
        .saturating_add(delta.energy_max)
        .max(1);
    disciple::recalculate_attribute_maxima(d);
    d.attributes.qi.current =
        (d.attributes.qi.current + delta.qi).clamp(0, d.attributes.qi.maximum.max(0));
    d.attributes.spirit.current =
        (d.attributes.spirit.current + delta.spirit).clamp(0, d.attributes.spirit.maximum.max(0));
    d.attributes.neili.current =
        (d.attributes.neili.current + delta.neili).clamp(0, d.attributes.neili.maximum.max(0));
    d.attributes.energy.current =
        (d.attributes.energy.current + delta.energy).clamp(0, d.attributes.energy.maximum.max(0));
    d.attributes.attainment += delta.attainment;
    d.attributes.reputation = (d.attributes.reputation + delta.reputation).clamp(0, 1000);
    d.attributes.sect_loyalty = (d.attributes.sect_loyalty + delta.loyalty).clamp(0, 100);
    d.merit = (d.merit + delta.merit).max(0);
    d.personal_silver = (d.personal_silver + delta.personal_silver).max(0);
    for book in delta.private_books {
        if !d.martial_progress.private_books.contains(&book) {
            d.martial_progress.private_books.push(book);
        }
    }
    for (art, gain) in delta.skill_experience {
        let trained_art = if art == "basic_parry" {
            disciple::prepared_skill_id(d, "basic_parry")
                .unwrap_or("basic_parry")
                .to_string()
        } else {
            art
        };
        let research_cap = crate::models::martial_art::martial_art_by_id(&trained_art)
            .filter(|art| art.is_combat)
            .map(|art| {
                martial_research
                    .and_then(|research| research.get(&art.id))
                    .copied()
                    .unwrap_or(0)
                    .clamp(50, i64::from(i32::MAX)) as i32
            });
        disciple::gain_skill_experience_with_cap(d, &trained_art, gain, research_cap);
    }
    if let Some(months) = delta.away_months {
        d.away_months = months;
    }
    if let Some(action) = delta.action {
        d.action = action;
    }
    disciple::refresh_condition(d);
    disciple::sync_legacy_attributes(d);
}

fn art_display(id: &str) -> String {
    crate::models::martial_art::all_martial_arts()
        .into_iter()
        .find(|art| art.id == id)
        .map(|art| format!("《{}》", art.name))
        .unwrap_or_else(|| format!("《{}》", id))
}

fn npc_solo_chronicle(name: &str, kind: &ActionKind) -> String {
    let deed = match kind {
        ActionKind::Read => "闭门翻阅典籍，偶有所得",
        ActionKind::Practice => "在演武场揣摩招式，直至日暮",
        ActionKind::TemperBody => "在山中打熬筋骨，风雨无阻",
        ActionKind::CultivateNeili => "静坐运功，潜心锤炼内息",
        ActionKind::Meditate => "焚香澄心，参悟门中学问",
        ActionKind::SectMission => "奉命下山办事，仍未归门",
        ActionKind::Wander => "负笈远游，沿途访师问道",
        ActionKind::Recover => "留在门中调息静养，气色渐复",
        ActionKind::Maintain => "巡检门中梁柱瓦石，料理修缮",
        ActionKind::Construct => "随众营造堂舍，终日未歇",
        ActionKind::Produce => "操持门中生产，将所得收入库中",
        ActionKind::Business => "下山经营世俗产业，带回一批收益",
        ActionKind::Gather => "入山寻药采集，满载而归",
        ActionKind::Teach | ActionKind::Spar => "与同门切磋讲习，彼此印证",
    };
    format!("{}{}。", name, deed)
}

fn action_name(kind: &ActionKind) -> &'static str {
    match kind {
        ActionKind::Read => "read",
        ActionKind::Practice => "practice",
        ActionKind::Teach => "teach",
        ActionKind::Spar => "spar",
        ActionKind::TemperBody => "temper",
        ActionKind::CultivateNeili => "neili",
        ActionKind::Meditate => "meditate",
        ActionKind::SectMission => "mission",
        ActionKind::Wander => "wander",
        ActionKind::Recover => "recover",
        ActionKind::Maintain => "maintain",
        ActionKind::Construct => "construct",
        ActionKind::Produce => "produce",
        ActionKind::Business => "business",
        ActionKind::Gather => "gather",
    }
}

fn stable_hash(value: &str) -> u64 {
    value.bytes().fold(0xcbf29ce484222325_u64, |hash, byte| {
        (hash ^ byte as u64).wrapping_mul(0x100000001b3)
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::logic::world;

    fn world_state() -> GameState {
        let mut state = GameState {
            world_seed: 12345,
            ..GameState::default()
        };
        let (sects, disciples) = world::generate_npc_world(state.world_seed);
        state.npc_sects = sects;
        state.npc_disciples = disciples;
        state.disciples = state.npc_disciples.drain(0..2).collect();
        for d in &mut state.disciples {
            d.sect_id = Some("player".into());
        }
        state
    }

    fn country_test_actor(kind: ActionKind, percent: i32, away_months: i32) -> Actor {
        let mut disciple = Disciple {
            id: "country-output-test".into(),
            martial_art: "basic_unarmed".into(),
            department: Some(if kind == ActionKind::Business {
                Department::Treasury
            } else {
                Department::ExternalAffairs
            }),
            away_months,
            personal_silver: 20,
            action: Some(ActionPlan {
                kind: kind.clone(),
                remaining_months: away_months.max(1),
                ..ActionPlan::default()
            }),
            ..Disciple::default()
        };
        disciple.martial_progress.proficiencies.insert(
            "basic_unarmed".into(),
            crate::models::attributes::SkillProgress::new(25, 0),
        );
        Actor {
            disciple,
            sect_id: "player".into(),
            player: true,
            public_books: vec![],
            recovery_bonus: 0,
            practice_effectiveness: 100,
            scripture_effectiveness: 100,
            warehouse_effectiveness: 120,
            herb_hall_effectiveness: 100,
            logistics_effectiveness: 100,
            prosperity: match percent {
                85 => 0,
                115 => 100,
                _ => 70,
            },
            order: match percent {
                85 => 0,
                115 => 100,
                _ => 65,
            },
            market_percent: percent,
            safety_percent: percent,
            external_percent: percent,
            moral_direction: MoralDirection::Neutral,
        }
    }

    fn run_country_action(kind: ActionKind, percent: i32, away_months: i32) -> JobResult {
        let actor = country_test_actor(kind.clone(), percent, away_months);
        execute_solo_with_encounters(&actor, &kind, &mut StdRng::seed_from_u64(811), false)
    }

    #[test]
    fn parallel_turn_is_deterministic_from_snapshot() {
        let mut a = world_state();
        let mut b = a.clone();
        let logs_a = run_auto_actions(&mut a);
        let logs_b = run_auto_actions(&mut b);
        assert_eq!(
            serde_json::to_value(&a).unwrap(),
            serde_json::to_value(&b).unwrap()
        );
        assert_eq!(
            serde_json::to_value(&logs_a).unwrap(),
            serde_json::to_value(&logs_b).unwrap()
        );
        assert!(logs_a
            .iter()
            .filter(|event| event.category == "world")
            .all(|event| {
                !event
                    .text
                    .chars()
                    .any(|character| character.is_ascii_digit())
                    && !event.text.contains("点")
                    && !event.text.contains("两")
                    && !event.text.contains('+')
            }));
    }

    #[test]
    fn autonomous_economic_weights_follow_prosperity_and_order() {
        assert_eq!(
            country_action_weight_adjustment(&ActionKind::Business, 0, 65),
            -8
        );
        assert_eq!(
            country_action_weight_adjustment(&ActionKind::Business, 100, 65),
            6
        );
        assert_eq!(
            country_action_weight_adjustment(&ActionKind::Produce, 0, 65),
            8
        );
        assert_eq!(
            country_action_weight_adjustment(&ActionKind::Produce, 100, 65),
            -6
        );
        assert_eq!(
            country_action_weight_adjustment(&ActionKind::SectMission, 70, 0),
            8
        );
        assert_eq!(
            country_action_weight_adjustment(&ActionKind::SectMission, 70, 100),
            -5
        );
        assert_eq!(
            country_action_weight_adjustment(&ActionKind::Wander, 70, 0),
            -5
        );
        assert_eq!(
            country_action_weight_adjustment(&ActionKind::Wander, 70, 100),
            7
        );
    }

    #[test]
    fn moral_direction_and_chivalrous_policy_change_autonomous_action_weights() {
        assert_eq!(
            moral_action_weight_adjustment(&ActionKind::SectMission, &MoralDirection::Righteous),
            12
        );
        assert_eq!(
            moral_action_weight_adjustment(&ActionKind::Business, &MoralDirection::Villainous),
            10
        );
        assert_eq!(
            moral_action_weight_adjustment(&ActionKind::Business, &MoralDirection::Righteous),
            -5
        );
        assert_eq!(
            moral_action_weight_adjustment(&ActionKind::Wander, &MoralDirection::Neutral),
            0
        );

        let mut chivalrous = crate::models::sect::SectState {
            policy: crate::models::sect::SectPolicy::Chivalrous,
            ..crate::models::sect::SectState::default()
        };
        let chivalrous_bonus =
            policy_action_weight_adjustment(&chivalrous, &ActionKind::SectMission);
        chivalrous.policy = crate::models::sect::SectPolicy::Mercantile;
        let mercantile_bonus =
            policy_action_weight_adjustment(&chivalrous, &ActionKind::SectMission);
        assert!(chivalrous_bonus > mercantile_bonus);
        assert_eq!(
            policy_action_weight_adjustment(&chivalrous, &ActionKind::Practice),
            0
        );
    }

    #[test]
    fn country_multipliers_scale_business_mission_and_wander_exactly_once() {
        let low_business = run_country_action(ActionKind::Business, 85, 0);
        let mid_business = run_country_action(ActionKind::Business, 100, 0);
        let high_business = run_country_action(ActionKind::Business, 115, 0);
        let mid_business_silver = mid_business.sects["player"].silver;
        assert_eq!(
            low_business.sects["player"].silver,
            country::scale_positive(mid_business_silver, 85)
        );
        assert_eq!(
            high_business.sects["player"].silver,
            country::scale_positive(mid_business_silver, 115)
        );
        assert!(
            low_business.sects["player"].silver < mid_business_silver
                && mid_business_silver < high_business.sects["player"].silver
        );

        for kind in [ActionKind::SectMission, ActionKind::Wander] {
            let low = run_country_action(kind.clone(), 85, 0);
            let mid = run_country_action(kind.clone(), 100, 0);
            let high = run_country_action(kind.clone(), 115, 0);
            let mid_attainment = mid.disciples[0].attainment;
            let mid_skill = mid.disciples[0].skill_experience["basic_unarmed"];
            assert_eq!(
                low.disciples[0].attainment,
                country::scale_positive_i64(mid_attainment, 85)
            );
            assert_eq!(
                high.disciples[0].attainment,
                country::scale_positive_i64(mid_attainment, 115)
            );
            assert_eq!(
                low.disciples[0].skill_experience["basic_unarmed"],
                country::scale_positive_i64(mid_skill, 85)
            );
            assert_eq!(
                high.disciples[0].skill_experience["basic_unarmed"],
                country::scale_positive_i64(mid_skill, 115)
            );
            assert!(
                low.disciples[0].attainment < mid_attainment
                    && mid_attainment < high.disciples[0].attainment
            );
            if kind == ActionKind::SectMission {
                assert!(
                    [low, mid, high].iter().all(|result| result
                        .sects
                        .get("player")
                        .is_none_or(|delta| delta.silver == 0)),
                    "外派赏银必须留待归山验收"
                );
            }
        }
    }

    #[test]
    fn country_multipliers_apply_to_every_away_month_without_compounding() {
        for kind in [ActionKind::SectMission, ActionKind::Wander] {
            let low = run_country_action(kind.clone(), 85, 2);
            let mid = run_country_action(kind.clone(), 100, 2);
            let high = run_country_action(kind.clone(), 115, 2);
            let mid_attainment = mid.disciples[0].attainment;
            let mid_skill = mid.disciples[0].skill_experience["basic_unarmed"];
            assert_eq!(
                low.disciples[0].attainment,
                country::scale_positive_i64(mid_attainment, 85)
            );
            assert_eq!(
                high.disciples[0].attainment,
                country::scale_positive_i64(mid_attainment, 115)
            );
            assert_eq!(
                low.disciples[0].skill_experience["basic_unarmed"],
                country::scale_positive_i64(mid_skill, 85)
            );
            assert_eq!(
                high.disciples[0].skill_experience["basic_unarmed"],
                country::scale_positive_i64(mid_skill, 115)
            );
            if kind == ActionKind::SectMission {
                assert!(
                    [low, mid, high].iter().all(|result| result
                        .sects
                        .get("player")
                        .is_none_or(|delta| delta.silver == 0)),
                    "外派途中不得重复发放赏银"
                );
            }
        }
    }

    #[test]
    fn journey_outcome_boundaries_are_exact() {
        assert_eq!(journey_outcome(0), JourneyOutcome::Success);
        assert_eq!(journey_outcome(-1), JourneyOutcome::Partial);
        assert_eq!(journey_outcome(-20), JourneyOutcome::Partial);
        assert_eq!(journey_outcome(-21), JourneyOutcome::Failed);
    }

    #[test]
    fn mission_departure_creates_one_fixed_journey_without_paying_early() {
        let actor = country_test_actor(ActionKind::SectMission, 100, 0);
        let result = execute_solo(
            &actor,
            &ActionKind::SectMission,
            &mut StdRng::seed_from_u64(912),
        );
        let plan = result.disciples[0]
            .action
            .as_ref()
            .and_then(Option::as_ref)
            .expect("外派应保留行动卷宗");
        let journey = plan.journey.as_ref().expect("外派应建立旅程卷宗");

        assert!(!journey.id.is_empty());
        assert!(!journey.template_id.is_empty());
        assert!(!journey.destination_id.is_empty());
        assert!(journey.difficulty >= 20);
        assert!(journey.encounter_id.is_some());
        assert!(!journey.encounter_resolved);
        assert!(!journey.settled);
        assert!(result
            .sects
            .get("player")
            .is_none_or(|delta| delta.silver == 0 && delta.prestige == 0));
        assert!(result.logs[0].1.contains("归山验收后一次结算"));
    }

    #[test]
    fn journey_identity_and_encounter_persist_until_return_then_settle_once() {
        let base_actor = country_test_actor(ActionKind::SectMission, 100, 0);
        let departure = execute_solo(
            &base_actor,
            &ActionKind::SectMission,
            &mut StdRng::seed_from_u64(913),
        );
        let mut plan = departure.disciples[0]
            .action
            .as_ref()
            .and_then(Option::as_ref)
            .unwrap()
            .clone();
        let journey_id = plan.journey.as_ref().unwrap().id.clone();
        plan.remaining_months = 2;
        {
            let journey = plan.journey.as_mut().unwrap();
            journey.total_months = 2;
            journey.encounter_id = Some("found_supplies".into());
        }

        let mut travelling = base_actor.clone();
        travelling.disciple.away_months = 2;
        travelling.disciple.action = Some(plan);
        let middle = execute_solo(
            &travelling,
            &ActionKind::SectMission,
            &mut StdRng::seed_from_u64(914),
        );
        let middle_plan = middle.disciples[0]
            .action
            .as_ref()
            .and_then(Option::as_ref)
            .unwrap();
        assert_eq!(middle_plan.journey.as_ref().unwrap().id, journey_id);
        assert_eq!(middle_plan.journey.as_ref().unwrap().elapsed_months, 1);
        assert!(!middle_plan.journey.as_ref().unwrap().encounter_resolved);
        assert!(middle
            .sects
            .get("player")
            .is_none_or(|delta| delta.inventory.is_empty() && delta.silver == 0));

        let mut returning = travelling;
        returning.disciple.away_months = 1;
        let mut return_plan = middle_plan.clone();
        return_plan.remaining_months = 1;
        return_plan.journey.as_mut().unwrap().outcome = Some(JourneyOutcome::Success);
        returning.disciple.action = Some(return_plan);
        let returned = execute_solo(
            &returning,
            &ActionKind::SectMission,
            &mut StdRng::seed_from_u64(915),
        );
        assert!(matches!(returned.disciples[0].action, Some(None)));
        assert!(returned.sects["player"].silver > 0);
        assert!(returned.sects["player"].inventory["粮秣"] > 0);
        assert!(returned.sects["player"].inventory["草药"] > 0);
        assert!(returned.logs[0].1.contains("圆满验收"));
    }

    #[test]
    fn settled_journey_cannot_pay_twice() {
        let actor = country_test_actor(ActionKind::SectMission, 100, 1);
        let mut journey = JourneyProgress {
            template_id: "escort_supplies".into(),
            destination_id: "xiangyang".into(),
            difficulty: 60,
            total_months: 2,
            outcome: Some(JourneyOutcome::Success),
            ..JourneyProgress::default()
        };
        let mut delta = DiscipleDelta::default();
        let mut result = JobResult::default();
        let mut rng = StdRng::seed_from_u64(916);

        settle_journey(
            &actor,
            &ActionKind::SectMission,
            &mut journey,
            &mut rng,
            &mut delta,
            &mut result,
        );
        let first_silver = result.sects["player"].silver;
        let first_merit = delta.merit;
        let text = settle_journey(
            &actor,
            &ActionKind::SectMission,
            &mut journey,
            &mut rng,
            &mut delta,
            &mut result,
        );

        assert!(journey.settled);
        assert_eq!(result.sects["player"].silver, first_silver);
        assert_eq!(delta.merit, first_merit);
        assert!(text.contains("不再重复发赏"));
    }

    #[test]
    fn final_mission_reward_applies_external_multiplier_once() {
        let settle = |percent| {
            let actor = country_test_actor(ActionKind::SectMission, percent, 1);
            let journey = JourneyProgress {
                template_id: "mediate_dispute".into(),
                destination_id: "linan".into(),
                difficulty: 60,
                total_months: 2,
                ..JourneyProgress::default()
            };
            let mut delta = DiscipleDelta::default();
            let mut result = JobResult::default();
            settle_mission(
                &actor,
                &journey,
                &JourneyOutcome::Success,
                0,
                &mut delta,
                &mut result,
            );
            result.sects["player"].silver
        };
        let low = settle(85);
        let middle = settle(100);
        let high = settle(115);

        assert_eq!(low, country::scale_positive(middle, 85));
        assert_eq!(high, country::scale_positive(middle, 115));
    }

    #[test]
    fn mission_rewards_follow_the_selected_moral_direction() {
        let settle = |direction| {
            let mut actor = country_test_actor(ActionKind::SectMission, 100, 1);
            actor.moral_direction = direction;
            let journey = JourneyProgress {
                template_id: "mediate_dispute".into(),
                destination_id: "linan".into(),
                difficulty: 60,
                total_months: 2,
                ..JourneyProgress::default()
            };
            let mut delta = DiscipleDelta::default();
            let mut result = JobResult::default();
            settle_mission(
                &actor,
                &journey,
                &JourneyOutcome::Success,
                0,
                &mut delta,
                &mut result,
            );
            let sect = &result.sects["player"];
            (sect.silver, sect.prestige, sect.morality)
        };
        let righteous = settle(MoralDirection::Righteous);
        let neutral = settle(MoralDirection::Neutral);
        let villainous = settle(MoralDirection::Villainous);

        assert!(righteous.0 < neutral.0 && neutral.0 < villainous.0);
        assert!(righteous.1 > neutral.1 && neutral.1 > villainous.1);
        assert_eq!(righteous.2, 1);
        assert_eq!(neutral.2, 0);
        assert_eq!(villainous.2, -1);
    }

    #[test]
    fn rare_journey_manual_belongs_to_its_finder() {
        let mut actor = country_test_actor(ActionKind::Wander, 100, 1);
        actor.public_books = vec!["player_knowledge".into(), "hunyuan".into()];
        for art in [
            "player_knowledge",
            "basic_unarmed",
            "basic_dodge",
            "basic_force",
            "basic_sword",
        ] {
            actor
                .disciple
                .martial_progress
                .proficiencies
                .entry(art.into())
                .or_insert_with(|| crate::models::attributes::SkillProgress::new(30, 0));
        }
        let mut delta = base_delta(&actor);
        let mut result = JobResult::default();
        let text = apply_travel_encounter(
            &actor,
            &ActionKind::Wander,
            TravelEncounterKind::LostManual,
            &mut StdRng::seed_from_u64(917),
            &mut delta,
            &mut result,
        );

        assert_eq!(delta.private_books.len(), 1);
        assert!(!actor.public_books.contains(&delta.private_books[0]));
        assert!(text.contains("私人行囊"));
        let manual = delta.private_books[0].clone();
        apply_disciple_delta(&mut actor.disciple, delta, None);
        assert!(actor
            .disciple
            .martial_progress
            .private_books
            .contains(&manual));
    }

    #[test]
    fn friendly_travel_spar_is_always_nonlethal() {
        let mut actor = country_test_actor(ActionKind::Wander, 100, 1);
        actor.disciple.attributes.qi.current = 2;
        let mut delta = base_delta(&actor);
        let mut result = JobResult::default();

        apply_travel_encounter(
            &actor,
            &ActionKind::Wander,
            TravelEncounterKind::FriendlySpar,
            &mut StdRng::seed_from_u64(918),
            &mut delta,
            &mut result,
        );

        assert!(actor.disciple.attributes.qi.current + delta.qi >= 1);
    }

    #[test]
    fn assigned_actions_produce_distinct_individual_growth() {
        let mut state = world_state();
        state.npc_disciples.clear();
        let mut meditator = state.disciples[1].clone();
        meditator.id = "test_meditator".into();
        meditator.action = Some(ActionPlan {
            kind: ActionKind::Meditate,
            ..ActionPlan::default()
        });
        state.disciples.push(meditator);
        state.disciples[0].action = Some(ActionPlan {
            kind: ActionKind::Read,
            martial_art_id: Some(state.disciples[0].martial_art.clone()),
            ..ActionPlan::default()
        });
        state.disciples[1].action = Some(ActionPlan {
            kind: ActionKind::CultivateNeili,
            ..ActionPlan::default()
        });
        let read_art = state.disciples[0].martial_art.clone();
        let read_before = state.disciples[0].martial_progress.proficiencies[&read_art].clone();
        let read_neili_before = state.disciples[0].attributes.neili.maximum;
        let cultivate_skill_before = state.disciples[1].martial_progress.proficiencies.clone();
        let cultivate_neili_before = state.disciples[1].attributes.neili.maximum;
        let meditate_spirit_before = state.disciples[2].attributes.spirit.current;
        let meditate_energy_before = state.disciples[2].attributes.energy.maximum;

        let logs = run_auto_actions(&mut state);

        assert_ne!(
            state.disciples[0].martial_progress.proficiencies[&read_art],
            read_before
        );
        assert_eq!(
            state.disciples[0].attributes.neili.maximum,
            read_neili_before
        );
        assert_eq!(
            state.disciples[1].martial_progress.proficiencies,
            cultivate_skill_before
        );
        assert!(state.disciples[1].attributes.neili.maximum > cultivate_neili_before);
        assert!(state.disciples[2].attributes.spirit.current < meditate_spirit_before);
        assert!(state.disciples[2].attributes.energy.maximum > meditate_energy_before);
        assert!(logs.iter().any(|event| event.text.contains("经验 +")));
    }

    #[test]
    fn damaged_scripture_hall_reduces_reading_output_from_the_same_snapshot() {
        let mut full = world_state();
        full.npc_disciples.clear();
        full.disciples.truncate(1);
        full.disciples[0].martial_progress.proficiencies.insert(
            "basic_unarmed".into(),
            crate::models::attributes::SkillProgress::new(20, 0),
        );
        full.disciples[0].action = Some(ActionPlan {
            kind: ActionKind::Read,
            martial_art_id: Some("basic_unarmed".into()),
            ..ActionPlan::default()
        });
        let mut damaged = full.clone();
        damaged
            .sect
            .buildings
            .iter_mut()
            .find(|building| building.id == "scripture")
            .unwrap()
            .condition = 50;
        let full_actor = collect_actors(&full).into_iter().next().unwrap();
        let damaged_actor = collect_actors(&damaged).into_iter().next().unwrap();
        let mut full_rng = StdRng::seed_from_u64(4321);
        let mut damaged_rng = StdRng::seed_from_u64(4321);

        let full_result = execute_solo(&full_actor, &ActionKind::Read, &mut full_rng);
        let damaged_result = execute_solo(&damaged_actor, &ActionKind::Read, &mut damaged_rng);
        let full_gain = full_result.disciples[0].skill_experience["basic_unarmed"];
        let damaged_gain = damaged_result.disciples[0].skill_experience["basic_unarmed"];

        assert!(full_gain > 0);
        assert_eq!(damaged_gain, full_gain / 2);
    }

    #[test]
    fn directed_teaching_keeps_the_named_teacher_student_and_art() {
        let mut state = world_state();
        state.npc_disciples.clear();
        state.disciples.truncate(2);
        // 故意让学生排在教师之前，证明显式互动不会再被名册顺序抢先占用。
        state.disciples[0].name = "受教弟子".into();
        state.disciples[0].rank = DiscipleRank::Outer;
        state.disciples[0].martial_progress.proficiencies.insert(
            "basic_unarmed".into(),
            crate::models::attributes::SkillProgress::new(20, 0),
        );
        state.disciples[0].action = None;
        state.disciples[1].name = "传功师长".into();
        state.disciples[1].rank = DiscipleRank::Inner;
        state.disciples[1].martial_progress.proficiencies.insert(
            "basic_unarmed".into(),
            crate::models::attributes::SkillProgress::new(80, 0),
        );
        let student_id = state.disciples[0].id.clone();
        let teacher_id = state.disciples[1].id.clone();
        state.disciples[0].master_id = Some(teacher_id);
        state.disciples[1].action = Some(ActionPlan {
            kind: ActionKind::Teach,
            target_id: Some(student_id),
            martial_art_id: Some("basic_unarmed".into()),
            assigned_by: Some("掌门".into()),
            remaining_months: 1,
            ..ActionPlan::default()
        });
        let before = state.disciples[0].martial_progress.proficiencies["basic_unarmed"].clone();

        let logs = run_auto_actions(&mut state);

        assert!(state.disciples[0].martial_progress.proficiencies["basic_unarmed"] != before);
        assert!(
            logs.iter()
                .any(|event| event.text.contains("传功师长向受教弟子传授《基本拳脚》")),
            "实际纪事：{:?}",
            logs.iter().map(|event| &event.text).collect::<Vec<_>>()
        );
    }

    #[test]
    fn selected_foreign_knowledge_manual_can_be_learned_from_zero() {
        let mut state = world_state();
        state.npc_disciples.clear();
        state.disciples.truncate(1);
        state.sect.public_books.push("wudang_knowledge".into());
        state.disciples[0].rank = DiscipleRank::Inner;
        state.disciples[0]
            .martial_progress
            .proficiencies
            .remove("wudang_knowledge");
        state.disciples[0].action = Some(ActionPlan {
            kind: ActionKind::Read,
            martial_art_id: Some("wudang_knowledge".into()),
            assigned_by: Some("掌门".into()),
            remaining_months: 1,
            ..ActionPlan::default()
        });

        run_auto_actions(&mut state);

        let learned = &state.disciples[0].martial_progress.proficiencies["wudang_knowledge"];
        assert!(learned.level > 0 || learned.experience > 0);
    }

    #[test]
    fn autonomous_teaching_prefers_apprentices_and_never_uses_distant_peers() {
        let mut teacher = Disciple {
            id: "teacher".into(),
            rank: DiscipleRank::Inner,
            ..Disciple::default()
        };
        teacher.martial_progress.proficiencies.insert(
            "basic_unarmed".into(),
            crate::models::attributes::SkillProgress::new(50, 0),
        );
        let apprentice = Disciple {
            id: "apprentice".into(),
            master_id: Some("teacher".into()),
            ..Disciple::default()
        };
        let mut friend = Disciple {
            id: "friend".into(),
            ..Disciple::default()
        };
        friend.relations.insert("teacher".into(), 35);
        let mut state = GameState {
            disciples: vec![teacher, apprentice, friend],
            ..GameState::default()
        };
        let actors = collect_actors(&state);
        let mut rng = StdRng::seed_from_u64(731);

        assert_eq!(
            choose_teaching_partner(0, "player", &actors, &mut rng),
            Some(1)
        );

        state.disciples[1].master_id = None;
        let actors = collect_actors(&state);
        assert_eq!(
            choose_teaching_partner(0, "player", &actors, &mut rng),
            Some(2)
        );

        state.disciples[2].martial_progress.proficiencies.insert(
            "basic_unarmed".into(),
            crate::models::attributes::SkillProgress::new(50, 0),
        );
        let actors = collect_actors(&state);
        assert_eq!(
            choose_teaching_partner(0, "player", &actors, &mut rng),
            None,
            "关系合格但学生造诣不低于教师时不可自主授业"
        );

        state.disciples[2].relations.clear();
        let actors = collect_actors(&state);
        assert_eq!(
            choose_teaching_partner(0, "player", &actors, &mut rng),
            None
        );
    }

    #[test]
    fn all_six_departments_raise_matching_outputs_and_none_stays_at_baseline() {
        let run = |department: Option<Department>, kind: ActionKind| {
            let mut disciple = Disciple {
                id: "department-test".into(),
                martial_art: "basic_unarmed".into(),
                department,
                personal_silver: 20,
                ..Disciple::default()
            };
            disciple.martial_progress.proficiencies.insert(
                "basic_unarmed".into(),
                crate::models::attributes::SkillProgress::new(25, 0),
            );
            disciple.action = Some(ActionPlan {
                kind: kind.clone(),
                target_id: matches!(kind, ActionKind::Maintain | ActionKind::Construct)
                    .then(|| "logistics".into()),
                martial_art_id: matches!(kind, ActionKind::Read | ActionKind::Practice)
                    .then(|| "basic_unarmed".into()),
                ..ActionPlan::default()
            });
            let actor = Actor {
                disciple,
                sect_id: "player".into(),
                player: true,
                public_books: vec!["basic_unarmed".into()],
                recovery_bonus: 0,
                practice_effectiveness: 100,
                scripture_effectiveness: 100,
                warehouse_effectiveness: 100,
                herb_hall_effectiveness: 100,
                logistics_effectiveness: 100,
                prosperity: 70,
                order: 65,
                market_percent: 100,
                safety_percent: 100,
                external_percent: 100,
                moral_direction: MoralDirection::Neutral,
            };
            execute_solo(&actor, &kind, &mut StdRng::seed_from_u64(732))
        };

        let base_practice = run(None, ActionKind::Practice);
        let transmission = run(Some(Department::Transmission), ActionKind::Practice);
        assert!(
            transmission.disciples[0].skill_experience["basic_unarmed"]
                > base_practice.disciples[0].skill_experience["basic_unarmed"]
        );

        let base_read = run(None, ActionKind::Read);
        let library = run(Some(Department::Library), ActionKind::Read);
        assert!(
            library.disciples[0].skill_experience["basic_unarmed"]
                > base_read.disciples[0].skill_experience["basic_unarmed"]
        );

        let base_gather = run(None, ActionKind::Gather);
        let apothecary = run(Some(Department::Apothecary), ActionKind::Gather);
        assert!(
            apothecary.sects["player"].inventory["草药"]
                > base_gather.sects["player"].inventory["草药"]
        );

        let base_business = run(None, ActionKind::Business);
        let treasury = run(Some(Department::Treasury), ActionKind::Business);
        assert!(treasury.sects["player"].silver > base_business.sects["player"].silver);
        let unrelated_department = run(Some(Department::Library), ActionKind::Business);
        assert_eq!(
            unrelated_department.sects["player"].silver,
            base_business.sects["player"].silver
        );

        let base_construct = run(None, ActionKind::Construct);
        let stewardship = run(Some(Department::Stewardship), ActionKind::Construct);
        assert!(
            stewardship.sects["player"].building_work["logistics"]
                > base_construct.sects["player"].building_work["logistics"]
        );

        let base_mission = run(None, ActionKind::SectMission);
        let external = run(Some(Department::ExternalAffairs), ActionKind::SectMission);
        assert!(external.disciples[0].attainment > base_mission.disciples[0].attainment);
        assert!(base_mission.sects.get("player").is_none());
        assert!(external.sects.get("player").is_none());
    }

    #[test]
    fn transmission_teacher_increases_the_students_teaching_gain() {
        let make_actor = |id: &str, level: i32, department: Option<Department>| {
            let mut disciple = Disciple {
                id: id.into(),
                department,
                ..Disciple::default()
            };
            disciple.martial_progress.proficiencies.insert(
                "basic_unarmed".into(),
                crate::models::attributes::SkillProgress::new(level, 0),
            );
            Actor {
                disciple,
                sect_id: "player".into(),
                player: true,
                public_books: vec![],
                recovery_bonus: 0,
                practice_effectiveness: 100,
                scripture_effectiveness: 100,
                warehouse_effectiveness: 100,
                herb_hall_effectiveness: 100,
                logistics_effectiveness: 100,
                prosperity: 70,
                order: 65,
                market_percent: 100,
                safety_percent: 100,
                external_percent: 100,
                moral_direction: MoralDirection::Neutral,
            }
        };
        let student = make_actor("student", 20, None);
        let plain_teacher = make_actor("teacher", 70, None);
        let transmission_teacher = make_actor("teacher", 70, Some(Department::Transmission));
        let plain_job = ActionJob {
            id: "plain".into(),
            kind: ActionKind::Teach,
            actors: vec![plain_teacher, student.clone()],
        };
        let department_job = ActionJob {
            id: "department".into(),
            kind: ActionKind::Teach,
            actors: vec![transmission_teacher, student],
        };

        let plain = execute_pair(&plain_job, &mut StdRng::seed_from_u64(733));
        let boosted = execute_pair(&department_job, &mut StdRng::seed_from_u64(733));

        assert!(
            boosted.disciples[1].skill_experience["basic_unarmed"]
                > plain.disciples[1].skill_experience["basic_unarmed"]
        );
    }

    #[test]
    fn department_bonus_never_crosses_neili_or_energy_training_caps() {
        let actor_for = |mut disciple: Disciple| {
            disciple.personal_silver = 10;
            Actor {
                disciple,
                sect_id: "player".into(),
                player: true,
                public_books: vec![],
                recovery_bonus: 0,
                practice_effectiveness: 100,
                scripture_effectiveness: 100,
                warehouse_effectiveness: 100,
                herb_hall_effectiveness: 100,
                logistics_effectiveness: 100,
                prosperity: 70,
                order: 65,
                market_percent: 100,
                safety_percent: 100,
                external_percent: 100,
                moral_direction: MoralDirection::Neutral,
            }
        };

        let mut cultivator = Disciple {
            id: "cultivator".into(),
            department: Some(Department::Transmission),
            ..Disciple::default()
        };
        cultivator.martial_progress.proficiencies.insert(
            "basic_force".into(),
            crate::models::attributes::SkillProgress::new(100, 0),
        );
        let neili_cap = disciple::neili_training_cap(&cultivator);
        cultivator.attributes.neili.maximum = neili_cap - 1;
        cultivator.attributes.neili.current = neili_cap - 1;
        let cultivation = execute_solo(
            &actor_for(cultivator),
            &ActionKind::CultivateNeili,
            &mut StdRng::seed_from_u64(734),
        );
        assert_eq!(cultivation.disciples[0].neili_max, 1);

        let mut meditator = Disciple {
            id: "meditator".into(),
            department: Some(Department::Library),
            ..Disciple::default()
        };
        meditator.martial_progress.proficiencies.insert(
            "player_knowledge".into(),
            crate::models::attributes::SkillProgress::new(200, 0),
        );
        let energy_cap = disciple::energy_training_cap(&meditator);
        meditator.attributes.energy.maximum = energy_cap - 1;
        meditator.attributes.energy.current = energy_cap - 1;
        let meditation = execute_solo(
            &actor_for(meditator),
            &ActionKind::Meditate,
            &mut StdRng::seed_from_u64(735),
        );
        assert_eq!(meditation.disciples[0].energy_max, 1);
    }

    #[test]
    fn capped_cultivation_does_not_consume_qi_or_reduce_actual_neili() {
        let mut state = world_state();
        state.npc_disciples.clear();
        state.disciples.truncate(1);
        let cap = disciple::neili_training_cap(&state.disciples[0]);
        state.disciples[0].attributes.neili.maximum = cap + 20;
        state.disciples[0].attributes.neili.current = cap + 10;
        state.disciples[0].action = Some(ActionPlan {
            kind: ActionKind::CultivateNeili,
            ..ActionPlan::default()
        });
        let qi = state.disciples[0].attributes.qi.current;
        let neili = state.disciples[0].attributes.neili.clone();

        let logs = run_auto_actions(&mut state);

        assert_eq!(state.disciples[0].attributes.qi.current, qi);
        assert_eq!(state.disciples[0].attributes.neili.current, neili.current);
        assert_eq!(state.disciples[0].attributes.neili.maximum, neili.maximum);
        assert!(logs.iter().any(|event| event.text.contains("修炼上限")));
    }
}
