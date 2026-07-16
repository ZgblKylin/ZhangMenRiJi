use crate::logic::{disciple, sect};
use crate::models::attributes::{ActionKind, ActionPlan};
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
}

#[derive(Clone)]
struct PlannedAction {
    actor: usize,
    kind: ActionKind,
    target: Option<usize>,
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
    skill_experience: BTreeMap<String, i64>,
    away_months: Option<i32>,
    action: Option<Option<ActionPlan>>,
}

#[derive(Default)]
struct SectDelta {
    silver: i32,
    prestige: i32,
    morality: i32,
    morale: i32,
}

#[derive(Default)]
struct JobResult {
    disciples: Vec<DiscipleDelta>,
    sects: BTreeMap<String, SectDelta>,
    logs: Vec<(bool, String)>,
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
    state
        .disciples
        .iter()
        .map(|d| Actor {
            disciple: d.clone(),
            sect_id: "player".into(),
            player: true,
            public_books: state.sect.public_books.clone(),
            recovery_bonus: player_recovery,
        })
        .chain(state.npc_disciples.iter().map(|d| {
            let npc_sect = d
                .sect_id
                .as_ref()
                .and_then(|id| state.npc_sects.iter().find(|sect| &sect.id == id));
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
            let target = if matches!(kind, ActionKind::Teach | ActionKind::Spar) {
                actor
                    .disciple
                    .action
                    .as_ref()
                    .and_then(|plan| plan.target_id.as_ref())
                    .and_then(|id| actors.iter().position(|other| &other.disciple.id == id))
                    .filter(|target| {
                        actors[*target].sect_id == actor.sect_id
                            && disciple::can_act(&actors[*target].disciple)
                    })
                    .or_else(|| choose_partner(index, &actor.sect_id, actors, &mut rng))
            } else {
                None
            };
            PlannedAction {
                actor: index,
                kind,
                target,
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
    let mut choices = vec![
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
    ];
    if d.attributes.qi.current < d.attributes.qi.maximum / 3
        || d.attributes.spirit.current < d.attributes.spirit.maximum / 3
    {
        choices.push((ActionKind::Recover, 45));
    }
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

fn build_jobs(actors: &[Actor], plans: &[PlannedAction]) -> Vec<ActionJob> {
    let mut consumed = BTreeSet::new();
    let mut jobs = Vec::with_capacity(plans.len());
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
            let (teacher, student) =
                if teaching_score(&first.disciple) >= teaching_score(&second.disciple) {
                    (first, second)
                } else {
                    (second, first)
                };
            let art = teaching_art(&teacher.disciple, &student.disciple);
            let teacher_level = skill_level(&teacher.disciple, &art);
            let student_level = skill_level(&student.disciple, &art);
            let gap_bonus = (teacher_level - student_level).max(0) as i64;
            let gain = skill_experience(
                &student.disciple,
                &art,
                student.disciple.aptitudes.intelligence,
                105 + gap_bonus.min(80) as i32,
            );
            let teacher_gain = skill_experience(
                &teacher.disciple,
                &art,
                teacher.disciple.aptitudes.intelligence,
                25,
            );
            let mut teacher_delta = base_delta(teacher);
            teacher_delta.spirit -= 7;
            teacher_delta
                .skill_experience
                .insert(art.clone(), teacher_gain);
            let mut student_delta = base_delta(student);
            student_delta.spirit -= 9;
            student_delta.skill_experience.insert(art.clone(), gain);
            result.disciples.extend([teacher_delta, student_delta]);
            result.logs.push((
                teacher.player || student.player,
                format!(
                    "{}向{}传授{}，彼此印证；{}经验 +{}，{}经验 +{}。",
                    teacher.disciple.name,
                    student.disciple.name,
                    art_display(&art),
                    art_display(&art),
                    gain,
                    art_display(&art),
                    teacher_gain
                ),
            ));
        }
        ActionKind::Spar => {
            let art_a = first.disciple.martial_art.clone();
            let art_b = second.disciple.martial_art.clone();
            let level_a = skill_level(&first.disciple, &art_a);
            let level_b = skill_level(&second.disciple, &art_b);
            let gain_a = skill_experience(
                &first.disciple,
                &art_a,
                (first.disciple.aptitudes.strength + first.disciple.aptitudes.agility) / 2,
                90 + (level_b - level_a).clamp(0, 50),
            );
            let gain_b = skill_experience(
                &second.disciple,
                &art_b,
                (second.disciple.aptitudes.strength + second.disciple.aptitudes.agility) / 2,
                90 + (level_a - level_b).clamp(0, 50),
            );
            let mut a = base_delta(first);
            a.qi -= rng.gen_range(5..=10);
            a.energy -= 6;
            a.attainment += 4 + level_b.max(1) as i64 / 40;
            a.skill_experience.insert(art_a.clone(), gain_a);
            let mut b = base_delta(second);
            b.qi -= rng.gen_range(5..=10);
            b.energy -= 6;
            b.attainment += 4 + level_a.max(1) as i64 / 40;
            b.skill_experience.insert(art_b.clone(), gain_b);
            result.disciples.extend([a, b]);
            result.logs.push((
                first.player || second.player,
                format!(
                    "{}与{}在演武场切磋，{}经验 +{}，{}经验 +{}。",
                    first.disciple.name,
                    second.disciple.name,
                    art_display(&art_a),
                    gain_a,
                    art_display(&art_b),
                    gain_b
                ),
            ));
        }
        _ => unreachable!("双人任务仅用于传授或切磋"),
    }
    result
}

fn execute_solo(actor: &Actor, kind: &ActionKind, rng: &mut StdRng) -> JobResult {
    let d = &actor.disciple;
    let mut result = JobResult::default();
    let mut delta = base_delta(actor);
    let log;

    if d.away_months > 0 {
        delta.away_months = Some((d.away_months - 1).max(0));
        if d.away_months <= 1 {
            delta.action = Some(None);
            delta.reputation += 2;
            delta.merit += 5;
        } else {
            let mut plan = d.action.clone().unwrap_or(ActionPlan {
                kind: kind.clone(),
                ..ActionPlan::default()
            });
            plan.remaining_months = d.away_months - 1;
            delta.action = Some(Some(plan));
        }
        if matches!(kind, ActionKind::SectMission) {
            let silver = 8 + d.aptitudes.strength + rng.gen_range(0..=16);
            let sect_delta = result.sects.entry(actor.sect_id.clone()).or_default();
            sect_delta.silver += silver;
            sect_delta.prestige += 1;
            delta.attainment += 4;
        } else {
            delta.attainment += 5 + d.aptitudes.fortune as i64 / 6;
        }
        let art = d.martial_art.clone();
        let gain = skill_experience(
            d,
            &art,
            (d.aptitudes.strength + d.aptitudes.agility) / 2,
            if matches!(kind, ActionKind::SectMission) {
                55
            } else {
                85
            },
        );
        delta.skill_experience.insert(art.clone(), gain);
        log = if d.away_months <= 1 {
            format!(
                "{}办完差事，风尘仆仆回到山门；{}经验 +{}。",
                d.name,
                art_display(&art),
                gain
            )
        } else {
            format!(
                "{}仍在外奔走，途中不忘磨炼{}，经验 +{}。",
                d.name,
                art_display(&art),
                gain
            )
        };
        result.disciples.push(delta);
        result.logs.push((actor.player, log));
        return result;
    }

    match kind {
        ActionKind::Read => {
            let art = preferred_book(actor);
            let gain = skill_experience(d, &art, disciple::effective_intelligence(d), 85);
            delta.spirit -= 10;
            delta.energy -= 3;
            delta.skill_experience.insert(art.clone(), gain);
            log = format!(
                "{}研读{}有所领悟，{}经验 +{}。",
                d.name,
                art_display(&art),
                art_display(&art),
                gain
            );
        }
        ActionKind::Practice => {
            let art = practice_art(d);
            let aptitude = (d.aptitudes.strength + d.aptitudes.agility) / 2;
            let gain = skill_experience(d, &art, aptitude, 115);
            delta.qi -= 4;
            delta.neili -= 4;
            delta.energy -= 10;
            delta.skill_experience.insert(art.clone(), gain);
            log = format!(
                "{}在演武场反复练习{}，{}经验 +{}。",
                d.name,
                art_display(&art),
                art_display(&art),
                gain
            );
        }
        ActionKind::TemperBody => {
            let gain = 2 + d.aptitudes.constitution / 12;
            delta.qi -= 11;
            delta.energy -= 5;
            delta.qi_max += gain;
            log = format!("{}打熬筋骨，气血上限添了{}点。", d.name, gain);
        }
        ActionKind::CultivateNeili => {
            let art = inner_skill(d);
            let level = skill_level(d, &art).max(1);
            let cap = disciple::neili_training_cap(d);
            let at_cap = d.attributes.neili.maximum >= cap;
            let cost = if at_cap {
                0
            } else {
                (10 + disciple::effective_aptitudes(d).constitution / 4)
                    .min(d.attributes.qi.current.saturating_sub(1))
            };
            let gain = if !at_cap && cost >= 10 {
                (1 + level / 80 + d.aptitudes.constitution / 25)
                    .min((cap - d.attributes.neili.maximum).max(0))
            } else {
                0
            };
            delta.qi -= cost;
            delta.neili_max += gain;
            delta.neili += gain;
            log = if at_cap {
                format!(
                    "{}盘膝打坐，但现有内力已达到{}所能承载的修炼上限。",
                    d.name,
                    art_display(&art)
                )
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
            let cost = if at_cap {
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
            delta.spirit -= cost;
            delta.energy_max += gain;
            delta.energy += gain;
            log = if at_cap {
                format!("{}澄心冥想，但现有精力已达到知识修为上限。", d.name)
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
            delta.away_months = Some(duration);
            delta.action = Some(Some(ActionPlan {
                kind: ActionKind::SectMission,
                remaining_months: duration,
                ..ActionPlan::default()
            }));
            delta.merit += 4;
            let silver = 10 + d.aptitudes.strength + rng.gen_range(0..=12);
            let sect_delta = result.sects.entry(actor.sect_id.clone()).or_default();
            sect_delta.silver += silver;
            sect_delta.prestige += 1;
            delta.attainment += 2;
            let art = d.martial_art.clone();
            let skill_gain = skill_experience(d, &art, d.aptitudes.strength, 35);
            delta.skill_experience.insert(art.clone(), skill_gain);
            log = format!(
                "{}奉命下山办事，约需{}个月方回；{}经验 +{}。",
                d.name,
                duration,
                art_display(&art),
                skill_gain
            );
        }
        ActionKind::Wander => {
            let duration = rng.gen_range(1..=4);
            delta.away_months = Some(duration);
            delta.action = Some(Some(ActionPlan {
                kind: ActionKind::Wander,
                remaining_months: duration,
                ..ActionPlan::default()
            }));
            let gain = 5 + d.aptitudes.fortune as i64 / 5;
            delta.attainment += gain;
            delta.qi -= rng.gen_range(0..=8);
            let art = d.martial_art.clone();
            let skill_gain = skill_experience(d, &art, d.aptitudes.agility, 45);
            delta.skill_experience.insert(art.clone(), skill_gain);
            log = format!(
                "{}负笈游历江湖，预备{}个月后归山；{}经验 +{}。",
                d.name,
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
        ActionKind::Teach | ActionKind::Spar => unreachable!("独行任务已回退为研读"),
    }
    if d.action.is_some() && !matches!(kind, ActionKind::SectMission | ActionKind::Wander) {
        delta.action = Some(None);
    }
    result.disciples.push(delta);
    result.logs.push((actor.player, log));
    result
}

fn base_delta(actor: &Actor) -> DiscipleDelta {
    DiscipleDelta {
        id: actor.disciple.id.clone(),
        ..DiscipleDelta::default()
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

fn practice_art(d: &Disciple) -> String {
    d.action
        .as_ref()
        .and_then(|plan| plan.martial_art_id.clone())
        .filter(|art| d.martial_progress.proficiencies.contains_key(art))
        .unwrap_or_else(|| d.martial_art.clone())
}

fn inner_skill(d: &Disciple) -> String {
    disciple::equipped_skill_id(d, "basic_force")
        .unwrap_or("basic_force")
        .to_string()
}

fn teaching_score(d: &Disciple) -> i64 {
    let highest = d
        .martial_progress
        .proficiencies
        .values()
        .map(|progress| progress.level)
        .max()
        .unwrap_or(0);
    d.attributes.attainment + i64::from(highest) * 10
}

fn teaching_art(teacher: &Disciple, student: &Disciple) -> String {
    teacher
        .martial_progress
        .proficiencies
        .keys()
        .max_by_key(|art| skill_level(teacher, art) - skill_level(student, art))
        .cloned()
        .unwrap_or_else(|| teacher.martial_art.clone())
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
    if let Some(book) = d.martial_progress.private_books.first().cloned() {
        return book;
    }
    actor
        .public_books
        .iter()
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

fn apply_results(state: &mut GameState, results: Vec<JobResult>) -> Vec<GameEvent> {
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
        if let Some(d) = state
            .disciples
            .iter_mut()
            .chain(state.npc_disciples.iter_mut())
            .find(|d| d.id == delta.id)
        {
            apply_disciple_delta(d, delta);
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
        }
    }
    sect::sync_legacy_fields(state);
    npc_logs.sort_by_key(|text| !(text.contains("经验 +") || text.contains("内力精进")));
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
            text: format!("江湖诸派亦各有动静，共推演了{}桩门人行止。", npc_actions),
            mood: "neutral".into(),
            year: state.year,
            month: state.month,
            category: "world".into(),
        });
    }
    events
}

fn apply_disciple_delta(d: &mut Disciple, delta: DiscipleDelta) {
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
    for (art, gain) in delta.skill_experience {
        disciple::gain_skill_experience(d, &art, gain);
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
