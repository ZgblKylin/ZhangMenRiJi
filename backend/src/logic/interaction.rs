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
    members: usize,
    defectors: usize,
}

#[derive(Clone, Copy)]
enum InteractionKind {
    Spar,
    Trade,
    BorderConflict,
    JointBandits,
    Defection,
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
    let mut events = Vec::with_capacity(target);

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
                candidates.extend(pair_candidates(a, b).into_iter().map(|(kind, weight)| {
                    Candidate {
                        a: a.clone(),
                        b: b.clone(),
                        kind,
                        weight,
                    }
                }));
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
                members: members.len(),
                defectors,
            }
        })
        .collect()
}

fn pair_candidates(a: &Faction, b: &Faction) -> Vec<(InteractionKind, u32)> {
    if is_court(a) && is_court(b) {
        return vec![(InteractionKind::CourtClash, 180)];
    }

    let mut kinds = Vec::new();
    if a.id == "court" || b.id == "court" {
        kinds.push((InteractionKind::YuanCourt, 60));
    }
    if a.id == "song_court" || b.id == "song_court" {
        kinds.push((InteractionKind::SongCourt, 60));
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

    if a.members > 0 && b.members > 0 {
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
    if a.defectors > 0 || b.defectors > 0 {
        let attraction = defection_attraction(a, b).max(defection_attraction(b, a));
        kinds.push((InteractionKind::Defection, (6 + attraction / 4) as u32));
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
        InteractionKind::Defection => defection(rng, state, a, b),
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
    let left = choose_member(rng, state, a.slot, false)?;
    let right = choose_member(rng, state, b.slot, false)?;
    let left_score = combat_score(&left) + rng.gen_range(0..=80);
    let right_score = combat_score(&right) + rng.gen_range(0..=80);
    let (winner, loser) = if left_score >= right_score {
        (a, b)
    } else {
        (b, a)
    };
    change_prestige(state, winner.slot, 2);
    change_prestige(state, loser.slot, -1);
    change_relation(state, a, b, 1);
    Some((
        format!(
            "{}弟子{}与{}弟子{}切磋，{}胜出。",
            a.name, left.name, b.name, right.name, winner.name
        ),
        "neutral",
    ))
}

fn trade(state: &mut GameState, a: &Faction, b: &Faction) -> (String, &'static str) {
    let income = 15 + (a.prestige + b.prestige).max(0) / 25;
    change_silver(state, a.slot, income);
    change_silver(state, b.slot, income);
    change_relation(state, a, b, 3);
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
    (
        format!("{}与{}联手清剿邻境悍匪，两派侠名俱增。", a.name, b.name),
        "good",
    )
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
    let member = choose_member(rng, state, source.slot, true)?;
    let mut disciple = take_member(state, source.slot, &member.id)?;
    disciple.sect_id = Some(target.id.clone());
    disciple.loyalty = 50;
    disciple.attributes.sect_loyalty = 50;
    disciple.department = None;
    disciple.master_id = None;
    disciple.action = None;
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

fn yuan_song_war(
    rng: &mut impl Rng,
    state: &mut GameState,
    a: &Faction,
    b: &Faction,
) -> (String, &'static str) {
    let (yuan, song) = if a.country == "yuan" { (a, b) } else { (b, a) };
    let yuan_score = yuan.prestige + rng.gen_range(0..=70);
    let song_score = song.prestige + rng.gen_range(0..=70);
    let (winner, loser, text) = if yuan_score >= song_score {
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
    (text, "bad")
}

fn dali_exchange(state: &mut GameState, a: &Faction, b: &Faction) -> (String, &'static str) {
    let (dali, guest) = if a.country == "dali" { (a, b) } else { (b, a) };
    change_morality(state, dali.slot, 1);
    change_morality(state, guest.slot, 1);
    change_relation(state, a, b, 4);
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
    (
        format!("大夏驼队与{}开市通商，奇货沿丝路往来不绝。", guest.name),
        "good",
    )
}

fn xia_assassination(state: &mut GameState, a: &Faction, b: &Faction) -> (String, &'static str) {
    let target = if a.country == "xia" { b } else { a };
    change_prestige(state, target.slot, -2);
    change_relation(state, a, b, -7);
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
    let (winner, loser) =
        if yuan.prestige + rng.gen_range(0..=80) >= song.prestige + rng.gen_range(0..=80) {
            (yuan, song)
        } else {
            (song, yuan)
        };
    change_prestige(state, winner.slot, 3);
    change_prestige(state, loser.slot, -3);
    change_relation(state, a, b, -10);
    (
        format!(
            "怯薛军与枢密院在边关正面交锋，{}得胜，{}败退。",
            winner.name, loser.name
        ),
        "bad",
    )
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

fn ordered_pair(a: &str, b: &str) -> (String, String) {
    if a <= b {
        (a.into(), b.into())
    } else {
        (b.into(), a.into())
    }
}

fn sect_mut(state: &mut GameState, slot: usize) -> &mut SectState {
    if slot == 0 {
        &mut state.sect
    } else {
        &mut state.npc_sects[slot - 1]
    }
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

    #[test]
    fn monthly_interactions_always_emit_three_to_five_world_events() {
        let mut state = world();
        let mut rng = StdRng::seed_from_u64(7);
        for _ in 0..24 {
            let events = run_monthly_interactions(&mut rng, &mut state);
            assert!((3..=5).contains(&events.len()));
            assert!(events.iter().all(|event| event.category == "world"));
        }
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
    fn defection_moves_a_real_disciple_between_rosters() {
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
        let before_wudang = wudang.members;
        let before_huashan = huashan.members;
        let mut rng = StdRng::seed_from_u64(19);
        let (text, _) = defection(&mut rng, &mut state, wudang, huashan).unwrap();
        assert!(text.contains("叛逃"));
        let after = faction_snapshot(&state);
        let after_wudang = after.iter().find(|faction| faction.id == "wudang").unwrap();
        let after_huashan = after
            .iter()
            .find(|faction| faction.id == "huashan")
            .unwrap();
        assert!(
            (after_wudang.members + 1 == before_wudang
                && after_huashan.members == before_huashan + 1)
                || (after_huashan.members + 1 == before_huashan
                    && after_wudang.members == before_wudang + 1)
        );
    }
}
