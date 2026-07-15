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
    proficiencies: BTreeMap<String, i64>,
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
    state
        .disciples
        .iter()
        .map(|d| Actor {
            disciple: d.clone(),
            sect_id: "player".into(),
            player: true,
            public_books: state.sect.public_books.clone(),
        })
        .chain(state.npc_disciples.iter().map(|d| {
            Actor {
                disciple: d.clone(),
                sect_id: d.sect_id.clone().unwrap_or_else(|| "wanderer".into()),
                player: false,
                public_books: d
                    .sect_id
                    .as_ref()
                    .and_then(|id| state.npc_sects.iter().find(|sect| &sect.id == id))
                    .map(|sect| sect.public_books.clone())
                    .unwrap_or_default(),
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
                choose_partner(index, &actor.sect_id, actors, &mut rng)
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
        (ActionKind::Read, 12 + sect::policy_bonus(policy, "study")),
        (ActionKind::Teach, 8),
        (ActionKind::Spar, 12 + sect::policy_bonus(policy, "martial")),
        (
            ActionKind::TemperBody,
            13 + sect::policy_bonus(policy, "martial"),
        ),
        (
            ActionKind::CultivateNeili,
            15 + sect::policy_bonus(policy, "martial"),
        ),
        (
            ActionKind::Meditate,
            9 + sect::policy_bonus(policy, "study"),
        ),
        (
            ActionKind::SectMission,
            9 + sect::policy_bonus(policy, "income"),
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
            *index != actor && other.sect_id == sect_id && other.disciple.alive
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
                if first.disciple.attributes.attainment >= second.disciple.attributes.attainment {
                    (first, second)
                } else {
                    (second, first)
                };
            let art = teacher.disciple.martial_art.clone();
            let gain = 5 + teacher.disciple.aptitudes.intelligence as i64 / 5;
            let mut teacher_delta = base_delta(teacher);
            teacher_delta.spirit -= 6;
            teacher_delta.attainment += 2;
            let mut student_delta = base_delta(student);
            student_delta.spirit -= 5;
            student_delta.attainment += gain;
            student_delta.proficiencies.insert(art.clone(), gain);
            result.disciples.extend([teacher_delta, student_delta]);
            result.logs.push((
                teacher.player || student.player,
                format!(
                    "{}向{}传授{}，彼此印证，后者造诣增了{}点。",
                    teacher.disciple.name,
                    student.disciple.name,
                    art_display(&art),
                    gain
                ),
            ));
        }
        ActionKind::Spar => {
            let gain_a = 3 + first.disciple.aptitudes.agility as i64 / 8;
            let gain_b = 3 + second.disciple.aptitudes.agility as i64 / 8;
            let mut a = base_delta(first);
            a.qi -= rng.gen_range(5..=10);
            a.energy -= 6;
            a.attainment += gain_a;
            a.proficiencies
                .insert(first.disciple.martial_art.clone(), gain_a);
            let mut b = base_delta(second);
            b.qi -= rng.gen_range(5..=10);
            b.energy -= 6;
            b.attainment += gain_b;
            b.proficiencies
                .insert(second.disciple.martial_art.clone(), gain_b);
            result.disciples.extend([a, b]);
            result.logs.push((
                first.player || second.player,
                format!(
                    "{}与{}在演武场拆了数十招，各自颇有所得。",
                    first.disciple.name, second.disciple.name
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
            log = format!("{}办完差事，风尘仆仆回到山门。", d.name);
        } else {
            let mut plan = d.action.clone().unwrap_or(ActionPlan {
                kind: kind.clone(),
                ..ActionPlan::default()
            });
            plan.remaining_months = d.away_months - 1;
            delta.action = Some(Some(plan));
            log = format!("{}仍在外奔走，传回一封平安书。", d.name);
        }
        if matches!(kind, ActionKind::SectMission) {
            let silver = 8 + d.aptitudes.strength + rng.gen_range(0..=16);
            let sect_delta = result.sects.entry(actor.sect_id.clone()).or_default();
            sect_delta.silver += silver;
            sect_delta.prestige += 1;
            delta.attainment += 3;
        } else {
            delta.attainment += 4 + d.aptitudes.fortune as i64 / 6;
        }
        result.disciples.push(delta);
        result.logs.push((actor.player, log));
        return result;
    }

    match kind {
        ActionKind::Read => {
            let art = preferred_book(actor);
            let gain = 4 + d.aptitudes.intelligence as i64 / 4;
            delta.spirit -= 9;
            delta.energy -= 3;
            delta.attainment += gain;
            delta.proficiencies.insert(art.clone(), gain);
            log = format!(
                "{}闭门研读{}，武理造诣增了{}点。",
                d.name,
                art_display(&art),
                gain
            );
        }
        ActionKind::TemperBody => {
            let gain = 2 + d.aptitudes.constitution / 12;
            delta.qi -= 11;
            delta.energy -= 5;
            delta.qi_max += gain;
            delta.attainment += 3;
            log = format!("{}打熬筋骨，气血上限添了{}点。", d.name, gain);
        }
        ActionKind::CultivateNeili => {
            let gain = 2 + d.aptitudes.constitution / 15;
            delta.qi -= 8;
            delta.spirit -= 4;
            delta.neili_max += gain;
            delta.neili += gain;
            delta.attainment += 5;
            log = format!("{}吐纳行功，内力上限添了{}点。", d.name, gain);
        }
        ActionKind::Meditate => {
            let gain = 2 + d.aptitudes.intelligence / 15;
            delta.spirit -= 10;
            delta.energy_max += gain;
            delta.energy += gain;
            delta.attainment += 5;
            log = format!("{}静坐冥思，精力上限添了{}点。", d.name, gain);
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
            log = format!("{}奉命下山办事，约需{}个月方回。", d.name, duration);
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
            log = format!("{}负笈游历江湖，预备{}个月后归山。", d.name, duration);
        }
        ActionKind::Recover => {
            delta.qi += 16 + d.aptitudes.constitution / 3;
            delta.spirit += 16 + d.aptitudes.intelligence / 3;
            delta.neili += 6;
            delta.energy += 8;
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

fn preferred_book(actor: &Actor) -> String {
    let d = &actor.disciple;
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
                .copied()
                .unwrap_or(0)
        })
        .cloned()
        .unwrap_or_else(|| d.martial_art.clone())
}

fn apply_results(state: &mut GameState, results: Vec<JobResult>) -> Vec<GameEvent> {
    let mut deltas = Vec::new();
    let mut sect_deltas: BTreeMap<String, SectDelta> = BTreeMap::new();
    let mut logs = Vec::new();
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
    if npc_actions > 0 {
        logs.push(format!(
            "江湖诸派亦各有动静，共推演了{}桩门人行止。",
            npc_actions
        ));
    }
    logs.into_iter()
        .map(|text| GameEvent {
            text,
            mood: "neutral".into(),
            year: state.year,
            month: state.month,
        })
        .collect()
}

fn apply_disciple_delta(d: &mut Disciple, delta: DiscipleDelta) {
    d.attributes.qi.maximum += delta.qi_max;
    d.attributes.spirit.maximum += delta.spirit_max;
    d.attributes.neili.maximum += delta.neili_max;
    d.attributes.energy.maximum += delta.energy_max;
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
    for (art, gain) in delta.proficiencies {
        *d.martial_progress.proficiencies.entry(art).or_default() += gain;
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
    fn all_world_disciples_receive_one_merged_result() {
        let mut state = world_state();
        let before: i64 = state
            .disciples
            .iter()
            .chain(&state.npc_disciples)
            .map(|d| d.attributes.attainment)
            .sum();
        run_auto_actions(&mut state);
        let after: i64 = state
            .disciples
            .iter()
            .chain(&state.npc_disciples)
            .map(|d| d.attributes.attainment)
            .sum();
        assert!(after > before);
    }
}
