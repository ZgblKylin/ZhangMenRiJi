use crate::logic::{disciple as disc, interaction, sect};
use crate::models::tournament::{
    TournamentBout, TournamentLineupMember, TournamentMatch, TournamentRecord, TournamentResult,
    TournamentRound, TournamentSide, TOURNAMENT_FORMAT_VERSION,
};
use crate::models::{Disciple, GameState};
use rand::Rng;
use std::collections::BTreeMap;

#[derive(Debug, Clone)]
struct Entrant {
    slot: usize,
    id: String,
    name: String,
    seed: i32,
    seed_score: i32,
    lineup: Vec<TournamentLineupMember>,
}

#[derive(Debug, Clone, Copy, Default)]
struct StandingStats {
    elimination_round: i32,
    match_wins: i32,
    bout_wins: i32,
    total_score: i32,
}

#[derive(Debug, Clone, Copy, Default)]
struct Growth {
    experience: i64,
    reputation: i32,
}

struct BracketOutcome {
    player_rank: i32,
    champion_name: String,
    player_lineup: Vec<TournamentLineupMember>,
    rounds: Vec<TournamentRound>,
    npc_placings: Vec<(String, usize)>,
    growth: BTreeMap<(usize, String, String), Growth>,
}

pub fn run_tournament(rng: &mut impl Rng, state: &mut GameState) -> TournamentResult {
    let power = disc::get_sect_combat_power(&state.disciples);
    let (rank, total_sects, champion, player_lineup, rounds, npc_placings, growth) =
        if state.npc_sects.is_empty() {
            // 没有世界名册的旧档仍沿用抽象排名，避免凭空生成无法追溯的参赛门派。
            let total = 8 + state.year / 2;
            let prestige = state.sect.attributes.prestige.min(100);
            let rank_base = (total as f64
                * (1.0 - (power as f64 / 200.0 + prestige as f64 / 200.0)))
                .max(1.0) as i32;
            let rank = disc::clamp(rank_base + disc::rand_range(rng, -2, 2), 1, total);
            (
                rank,
                total,
                if rank == 1 {
                    state.sect.name.clone()
                } else {
                    "江湖群雄".to_string()
                },
                snapshot_lineup(state, 0, &state.sect.id),
                Vec::new(),
                Vec::new(),
                BTreeMap::new(),
            )
        } else {
            let outcome = simulate_bracket(rng, state);
            (
                outcome.player_rank,
                state.npc_sects.len() as i32 + 1,
                outcome.champion_name,
                outcome.player_lineup,
                outcome.rounds,
                outcome.npc_placings,
                outcome.growth,
            )
        };

    // 所有场次先使用开赛快照推演完成；到此才把实战成长统一落回真实人物。
    apply_tournament_growth(state, growth);

    for (sect_id, placing) in npc_placings {
        if placing > 3 {
            continue;
        }
        if let Some(npc_sect) = state.npc_sects.iter_mut().find(|sect| sect.id == sect_id) {
            let prestige_gain = match placing {
                1 => 12,
                2 => 8,
                _ => 5,
            };
            npc_sect.attributes.prestige = (npc_sect.attributes.prestige + prestige_gain).min(1000);
            npc_sect.attributes.morale = (npc_sect.attributes.morale + 3).min(100);
        }
    }

    let (reward_silver, reward_prestige, mut desc_text) = if rank == 1 {
        (
            300,
            15,
            "本派弟子技压群雄，夺得魁首！一时间名动江湖，四方来贺！".to_string(),
        )
    } else if rank <= 3 {
        (
            200,
            10,
            "本派位列三甲，战绩斐然。门下弟子扬眉吐气，掌门面上有光。".to_string(),
        )
    } else if rank <= total_sects / 2 {
        (
            100,
            5,
            "本派位列中游，虽未夺魁，亦属不易。弟子们还需勤加修炼。".to_string(),
        )
    } else {
        state.sect.attributes.morale = disc::clamp(state.sect.attributes.morale - 5, 0, 100);
        (
            30,
            0,
            "本派成绩不佳，位列末流。掌门面上无光，但来日方长，卧薪尝胆便是。".to_string(),
        )
    };
    if rank != 1 {
        desc_text.push_str(&format!(" 本届魁首为{}。", champion));
    }

    state.sect.attributes.silver += reward_silver;
    state.sect.attributes.prestige =
        disc::clamp(state.sect.attributes.prestige + reward_prestige, 0, 1000);
    state.sect.attributes.morale = disc::clamp(
        state.sect.attributes.morale + disc::rand_range(rng, 3, 8),
        0,
        100,
    );

    let record = TournamentRecord {
        year: state.year,
        rank,
        total_sects,
        power,
        format_version: TOURNAMENT_FORMAT_VERSION,
        champion,
        player_lineup,
        rounds,
    };
    state.tournament_history.push(record.clone());
    crate::logic::sect::sync_legacy_fields(state);

    TournamentResult {
        year: record.year,
        rank,
        total_sects,
        power,
        desc_text,
        reward_silver,
        reward_prestige,
        format_version: record.format_version,
        champion: record.champion,
        player_lineup: record.player_lineup,
        rounds: record.rounds,
    }
}

fn simulate_bracket(rng: &mut impl Rng, state: &GameState) -> BracketOutcome {
    let entrants = seeded_entrants(state);
    let player_id = state.sect.id.clone();
    let player_lineup = entrants
        .iter()
        .find(|entrant| entrant.id == player_id)
        .map(|entrant| entrant.lineup.clone())
        .unwrap_or_default();
    let bracket_size = entrants.len().max(1).next_power_of_two();
    let total_rounds = bracket_size.trailing_zeros() as i32;
    let mut by_seed = entrants
        .iter()
        .cloned()
        .map(|entrant| (entrant.seed, entrant))
        .collect::<BTreeMap<_, _>>();
    let mut current = seeded_positions(bracket_size)
        .into_iter()
        .map(|seed| by_seed.remove(&(seed as i32)))
        .collect::<Vec<_>>();
    let mut standings = entrants
        .iter()
        .map(|entrant| (entrant.id.clone(), StandingStats::default()))
        .collect::<BTreeMap<_, _>>();
    let mut growth = BTreeMap::new();
    let mut rounds = Vec::with_capacity(total_rounds as usize);

    for round_number in 1..=total_rounds {
        let field_size = bracket_size >> (round_number - 1);
        let mut matches = Vec::with_capacity(current.len() / 2);
        let mut next = Vec::with_capacity(current.len() / 2);
        for (match_index, pair) in current.chunks_mut(2).enumerate() {
            let left = pair[0].take();
            let right = pair.get_mut(1).and_then(Option::take);
            let (record, winner) = resolve_match(
                rng,
                state.year,
                round_number,
                match_index + 1,
                left,
                right,
                &mut standings,
                &mut growth,
            );
            matches.push(record);
            next.push(winner);
        }
        rounds.push(TournamentRound {
            number: round_number,
            name: round_name(field_size),
            matches,
        });
        current = next;
    }

    let champion = current
        .into_iter()
        .flatten()
        .next()
        .or_else(|| entrants.first().cloned())
        .expect("the player sect always occupies one tournament seed");
    let champion_id = champion.id.clone();
    let mut ranking = entrants.clone();
    ranking.sort_by(|left, right| {
        let left_stats = standings.get(&left.id).copied().unwrap_or_default();
        let right_stats = standings.get(&right.id).copied().unwrap_or_default();
        let left_stage = if left.id == champion_id {
            i32::MAX
        } else {
            left_stats.elimination_round
        };
        let right_stage = if right.id == champion_id {
            i32::MAX
        } else {
            right_stats.elimination_round
        };
        right_stage
            .cmp(&left_stage)
            .then_with(|| right_stats.match_wins.cmp(&left_stats.match_wins))
            .then_with(|| right_stats.bout_wins.cmp(&left_stats.bout_wins))
            .then_with(|| right_stats.total_score.cmp(&left_stats.total_score))
            .then_with(|| left.seed.cmp(&right.seed))
            .then_with(|| left.id.cmp(&right.id))
    });
    let player_rank = ranking
        .iter()
        .position(|entrant| entrant.id == player_id)
        .map(|position| position as i32 + 1)
        .unwrap_or(ranking.len() as i32);
    let npc_placings = ranking
        .iter()
        .enumerate()
        .filter(|(_, entrant)| entrant.id != player_id)
        .map(|(position, entrant)| (entrant.id.clone(), position + 1))
        .collect();

    BracketOutcome {
        player_rank,
        champion_name: champion.name,
        player_lineup,
        rounds,
        npc_placings,
        growth,
    }
}

#[allow(clippy::too_many_arguments)]
fn resolve_match(
    rng: &mut impl Rng,
    year: i32,
    round_number: i32,
    match_number: usize,
    left: Option<Entrant>,
    right: Option<Entrant>,
    stats: &mut BTreeMap<String, StandingStats>,
    growth: &mut BTreeMap<(usize, String, String), Growth>,
) -> (TournamentMatch, Option<Entrant>) {
    let match_id = format!("y{year}-r{round_number}-m{match_number}");
    match (left, right) {
        (Some(entrant), None) => {
            let winner_id = entrant.id.clone();
            (
                TournamentMatch {
                    id: match_id,
                    left: Some(match_side(&entrant, 0, 0)),
                    right: None,
                    winner_sect_id: winner_id,
                    bye: true,
                    bouts: Vec::new(),
                },
                Some(entrant),
            )
        }
        (None, Some(entrant)) => {
            let winner_id = entrant.id.clone();
            (
                TournamentMatch {
                    id: match_id,
                    left: None,
                    right: Some(match_side(&entrant, 0, 0)),
                    winner_sect_id: winner_id,
                    bye: true,
                    bouts: Vec::new(),
                },
                Some(entrant),
            )
        }
        (None, None) => (
            TournamentMatch {
                id: match_id,
                bye: true,
                ..TournamentMatch::default()
            },
            None,
        ),
        (Some(left), Some(right)) => {
            let mut left_wins = 0_i32;
            let mut right_wins = 0_i32;
            let mut left_total = 0_i32;
            let mut right_total = 0_i32;
            let mut bouts = Vec::with_capacity(3);
            for position in 0..3 {
                let left_member = left.lineup.get(position).cloned();
                let right_member = right.lineup.get(position).cloned();
                let left_score = left_member
                    .as_ref()
                    .map(|member| bout_score(rng, member))
                    .unwrap_or(0);
                let right_score = right_member
                    .as_ref()
                    .map(|member| bout_score(rng, member))
                    .unwrap_or(0);
                left_total = left_total.saturating_add(left_score);
                right_total = right_total.saturating_add(right_score);

                let bout_winner = match (&left_member, &right_member) {
                    (Some(_), None) => Some(left.id.clone()),
                    (None, Some(_)) => Some(right.id.clone()),
                    (Some(_), Some(_)) if left_score > right_score => Some(left.id.clone()),
                    (Some(_), Some(_)) if right_score > left_score => Some(right.id.clone()),
                    _ => None,
                };
                match bout_winner.as_deref() {
                    Some(id) if id == left.id => left_wins += 1,
                    Some(id) if id == right.id => right_wins += 1,
                    _ => {}
                }

                // 缺阵不算交手；真正上场的双方才获得实战经验。
                if let (Some(left_member), Some(right_member)) =
                    (left_member.as_ref(), right_member.as_ref())
                {
                    match bout_winner.as_deref() {
                        Some(id) if id == left.id => {
                            accrue_growth(growth, &left, left_member, 12, true);
                            accrue_growth(growth, &right, right_member, 7, false);
                        }
                        Some(id) if id == right.id => {
                            accrue_growth(growth, &left, left_member, 7, false);
                            accrue_growth(growth, &right, right_member, 12, true);
                        }
                        _ => {
                            accrue_growth(growth, &left, left_member, 7, false);
                            accrue_growth(growth, &right, right_member, 7, false);
                        }
                    }
                }
                bouts.push(TournamentBout {
                    position: position as i32 + 1,
                    left: left_member,
                    right: right_member,
                    left_score,
                    right_score,
                    winner_sect_id: bout_winner,
                });
            }

            let left_advances = left_wins > right_wins
                || (left_wins == right_wins && left_total > right_total)
                || (left_wins == right_wins && left_total == right_total && left.seed < right.seed);
            let (winner, loser) = if left_advances {
                (&left, &right)
            } else {
                (&right, &left)
            };
            {
                let standing = stats.entry(left.id.clone()).or_default();
                standing.bout_wins = standing.bout_wins.saturating_add(left_wins);
                standing.total_score = standing.total_score.saturating_add(left_total);
            }
            {
                let standing = stats.entry(right.id.clone()).or_default();
                standing.bout_wins = standing.bout_wins.saturating_add(right_wins);
                standing.total_score = standing.total_score.saturating_add(right_total);
            }
            stats.entry(winner.id.clone()).or_default().match_wins += 1;
            stats.entry(loser.id.clone()).or_default().elimination_round = round_number;
            let winner_id = winner.id.clone();
            let record = TournamentMatch {
                id: match_id,
                left: Some(match_side(&left, left_wins, left_total)),
                right: Some(match_side(&right, right_wins, right_total)),
                winner_sect_id: winner_id,
                bye: false,
                bouts,
            };
            (record, Some(if left_advances { left } else { right }))
        }
    }
}

fn bout_score(rng: &mut impl Rng, member: &TournamentLineupMember) -> i32 {
    member
        .combat_score
        .saturating_add(disc::rand_range(rng, -12, 12))
        .max(1)
}

fn accrue_growth(
    growth: &mut BTreeMap<(usize, String, String), Growth>,
    entrant: &Entrant,
    member: &TournamentLineupMember,
    experience: i64,
    won: bool,
) {
    let result = growth
        .entry((
            entrant.slot,
            member.disciple_id.clone(),
            member.martial_art_id.clone(),
        ))
        .or_default();
    result.experience = result.experience.saturating_add(experience);
    if won {
        result.reputation = result.reputation.saturating_add(1);
    }
}

fn apply_tournament_growth(
    state: &mut GameState,
    growth: BTreeMap<(usize, String, String), Growth>,
) {
    for ((slot, disciple_id, art_id), growth) in growth {
        let research_cap = if slot == 0 {
            sect::martial_research_level_cap(&state.sect, &art_id)
        } else {
            state
                .npc_sects
                .get(slot - 1)
                .and_then(|sect| sect::martial_research_level_cap(sect, &art_id))
        };
        let disciple = if slot == 0 {
            state
                .disciples
                .iter_mut()
                .find(|disciple| disciple.id == disciple_id)
        } else {
            state
                .npc_disciples
                .iter_mut()
                .find(|disciple| disciple.id == disciple_id)
        };
        let Some(disciple) = disciple else {
            continue;
        };
        if !art_id.is_empty() && growth.experience > 0 {
            interaction::gain_spar_experience(disciple, &art_id, growth.experience, research_cap);
        }
        disciple.attributes.reputation = disciple
            .attributes
            .reputation
            .saturating_add(growth.reputation)
            .clamp(0, 1000);
        disc::sync_legacy_attributes(disciple);
    }
}

fn seeded_entrants(state: &GameState) -> Vec<Entrant> {
    let mut entrants = std::iter::once((0, &state.sect))
        .chain(
            state
                .npc_sects
                .iter()
                .enumerate()
                .map(|(index, sect)| (index + 1, sect)),
        )
        .map(|(slot, sect)| {
            let lineup = snapshot_lineup(state, slot, &sect.id);
            let lineup_score = lineup.iter().fold(0_i32, |total, member| {
                total.saturating_add(member.combat_score)
            });
            let seed_score = lineup_score
                .saturating_mul(2)
                .saturating_add(sect.attributes.prestige.clamp(0, 300) / 2)
                .saturating_add(deterministic_seed_noise(
                    state.world_seed,
                    state.year,
                    &sect.id,
                ));
            Entrant {
                slot,
                id: sect.id.clone(),
                name: sect.name.clone(),
                seed: 0,
                seed_score,
                lineup,
            }
        })
        .collect::<Vec<_>>();
    entrants.sort_by(|left, right| {
        right
            .seed_score
            .cmp(&left.seed_score)
            .then_with(|| left.id.cmp(&right.id))
    });
    for (index, entrant) in entrants.iter_mut().enumerate() {
        entrant.seed = index as i32 + 1;
    }
    entrants
}

fn snapshot_lineup(state: &GameState, slot: usize, sect_id: &str) -> Vec<TournamentLineupMember> {
    let members: &[Disciple] = if slot == 0 {
        &state.disciples
    } else {
        &state.npc_disciples
    };
    let created_martial_arts = if slot == 0 {
        state.sect.created_martial_arts.as_slice()
    } else {
        &[]
    };
    let mut lineup = members
        .iter()
        .filter(|member| member.sect_id.as_deref() == Some(sect_id) && disc::can_act(member))
        .map(|member| TournamentLineupMember {
            disciple_id: member.id.clone(),
            name: member.name.clone(),
            martial_art_id: interaction::spar_art_with_created(member, created_martial_arts)
                .unwrap_or_default(),
            combat_score: disc::get_combat_score_with_created(member, created_martial_arts),
        })
        .collect::<Vec<_>>();
    lineup.sort_by(|left, right| {
        right
            .combat_score
            .cmp(&left.combat_score)
            .then_with(|| left.disciple_id.cmp(&right.disciple_id))
    });
    lineup.truncate(3);
    lineup
}

fn match_side(entrant: &Entrant, bout_wins: i32, total_score: i32) -> TournamentSide {
    TournamentSide {
        sect_id: entrant.id.clone(),
        sect_name: entrant.name.clone(),
        seed: entrant.seed,
        seed_score: entrant.seed_score,
        bout_wins,
        total_score,
    }
}

/// 递归展开标准淘汰赛种子位置：32签位中 1 对 32、2 对 31，以此保证前七种子轮空。
fn seeded_positions(size: usize) -> Vec<usize> {
    if size <= 1 {
        return vec![1];
    }
    let mut positions = vec![1, 2];
    while positions.len() < size {
        let complement = positions.len() * 2 + 1;
        positions = positions
            .into_iter()
            .flat_map(|seed| [seed, complement - seed])
            .collect();
    }
    positions
}

fn deterministic_seed_noise(world_seed: u64, year: i32, sect_id: &str) -> i32 {
    let mut hash = 0xcbf2_9ce4_8422_2325_u64 ^ world_seed ^ (year as u64).rotate_left(17);
    for byte in sect_id.bytes() {
        hash ^= u64::from(byte);
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    hash ^= hash >> 30;
    hash = hash.wrapping_mul(0xbf58_476d_1ce4_e5b9);
    hash ^= hash >> 27;
    hash = hash.wrapping_mul(0x94d0_49bb_1331_11eb);
    hash ^= hash >> 31;
    (hash % 25) as i32 - 12
}

fn round_name(field_size: usize) -> String {
    match field_size {
        2 => "决赛".into(),
        4 => "半决赛".into(),
        8 => "八强".into(),
        16 => "十六强".into(),
        32 => "三十二强".into(),
        _ => format!("{field_size}强"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::attributes::{DiscipleCondition, DiscipleRank};
    use crate::models::sect::SectState;
    use rand::{rngs::StdRng, SeedableRng};
    use std::collections::BTreeSet;

    fn simple_member(id: &str, sect_id: &str, neili: i32) -> Disciple {
        let mut member = Disciple {
            id: id.into(),
            name: id.into(),
            sect_id: Some(sect_id.into()),
            ..Disciple::default()
        };
        member.attributes.neili.maximum = neili;
        member.attributes.neili.current = neili;
        member
    }

    fn trained_member(id: &str, sect_id: &str, neili: i32) -> Disciple {
        let mut member = Disciple {
            id: id.into(),
            name: id.into(),
            sect_id: Some(sect_id.into()),
            rank: DiscipleRank::Inner,
            ..Disciple::default()
        };
        disc::assign_sect_curriculum(&mut member, sect_id, 80);
        member.sect_id = Some(sect_id.into());
        member.attributes.neili.maximum = neili;
        member.attributes.neili.current = neili;
        disc::recalculate_attribute_maxima(&mut member);
        disc::sync_legacy_attributes(&mut member);
        member
    }

    fn twenty_five_sect_state() -> GameState {
        let mut state = GameState {
            world_seed: 20_260_719,
            ..GameState::default()
        };
        state.disciples = (0..3)
            .map(|index| simple_member(&format!("player-{index}"), "player", 200 + index * 10))
            .collect();
        for index in 0..24 {
            let id = format!("npc-{index:02}");
            state.npc_sects.push(SectState {
                id: id.clone(),
                name: format!("第{index}派"),
                ..SectState::default()
            });
            for member_index in 0..3 {
                state.npc_disciples.push(simple_member(
                    &format!("{id}-{member_index}"),
                    &id,
                    80 + index * 7 + member_index * 3,
                ));
            }
        }
        state
    }

    #[test]
    fn tournament_uses_the_world_roster_and_persists_rewards_in_v3_fields() {
        let mut state = GameState::default();
        state.npc_sects = vec![
            SectState {
                id: "first".into(),
                name: "甲派".into(),
                ..SectState::default()
            },
            SectState {
                id: "second".into(),
                name: "乙派".into(),
                ..SectState::default()
            },
        ];
        state.sect.attributes.silver = 200;
        state.sect.attributes.prestige = 45;
        crate::logic::sect::sync_legacy_fields(&mut state);
        let mut rng = StdRng::seed_from_u64(20_260_718);

        let result = run_tournament(&mut rng, &mut state);

        assert_eq!(result.total_sects, 3);
        assert_eq!(state.sect.attributes.silver, 200 + result.reward_silver);
        assert_eq!(state.silver, state.sect.attributes.silver);
        assert_eq!(state.prestige, state.sect.attributes.prestige);
        assert_eq!(state.morale, state.sect.attributes.morale);
        assert_eq!(result.year, state.year);
        assert_eq!(result.format_version, TOURNAMENT_FORMAT_VERSION);
        assert_eq!(
            result.rounds,
            state.tournament_history.last().unwrap().rounds
        );
    }

    #[test]
    fn tournament_rank_is_compared_against_real_npc_combat_power() {
        let mut state = GameState::default();
        state.disciples.clear();
        state.sect.attributes.prestige = 0;
        state.npc_sects = vec![SectState {
            id: "strong".into(),
            name: "凌霄盟".into(),
            ..SectState::default()
        }];
        let mut champion = Disciple {
            id: "strong-champion".into(),
            sect_id: Some("strong".into()),
            ..Disciple::default()
        };
        champion.attributes.neili.maximum = 2_000;
        state.npc_disciples.push(champion);
        let prestige_before = state.npc_sects[0].attributes.prestige;
        let mut rng = StdRng::seed_from_u64(19);

        let result = run_tournament(&mut rng, &mut state);

        assert_eq!(result.total_sects, 2);
        assert_eq!(result.rank, 2);
        assert!(result.desc_text.contains("本届魁首为凌霄盟"));
        assert_eq!(state.npc_sects[0].attributes.prestige, prestige_before + 12);
    }

    #[test]
    fn twenty_five_sects_fill_a_32_draw_and_advance_only_winners() {
        let state = twenty_five_sect_state();
        let mut rng = StdRng::seed_from_u64(717);

        let outcome = simulate_bracket(&mut rng, &state);

        assert_eq!(outcome.rounds.len(), 5);
        assert_eq!(
            outcome
                .rounds
                .iter()
                .flat_map(|round| &round.matches)
                .filter(|game| game.bye)
                .count(),
            7
        );
        assert_eq!(
            outcome
                .rounds
                .iter()
                .flat_map(|round| &round.matches)
                .filter(|game| !game.bye)
                .count(),
            24
        );
        assert!(outcome.rounds[0]
            .matches
            .iter()
            .filter(|game| game.bye)
            .all(|game| game
                .left
                .as_ref()
                .or(game.right.as_ref())
                .is_some_and(|side| side.seed <= 7)));
        for round_index in 1..outcome.rounds.len() {
            let previous_winners = outcome.rounds[round_index - 1]
                .matches
                .iter()
                .map(|game| game.winner_sect_id.as_str())
                .collect::<BTreeSet<_>>();
            assert!(outcome.rounds[round_index]
                .matches
                .iter()
                .flat_map(|game| [game.left.as_ref(), game.right.as_ref()])
                .flatten()
                .all(|side| previous_winners.contains(side.sect_id.as_str())));
        }
        let final_match = &outcome.rounds.last().unwrap().matches[0];
        let champion_side = [final_match.left.as_ref(), final_match.right.as_ref()]
            .into_iter()
            .flatten()
            .find(|side| side.sect_id == final_match.winner_sect_id)
            .unwrap();
        assert_eq!(champion_side.sect_name, outcome.champion_name);

        let mut ranks = outcome
            .npc_placings
            .iter()
            .map(|(_, rank)| *rank)
            .collect::<Vec<_>>();
        ranks.push(outcome.player_rank as usize);
        ranks.sort_unstable();
        assert_eq!(ranks, (1..=25).collect::<Vec<_>>());
    }

    #[test]
    fn lineup_uses_only_the_three_strongest_healthy_present_members() {
        let mut state = GameState::default();
        state.npc_sects.push(SectState {
            id: "opponent".into(),
            name: "客队".into(),
            ..SectState::default()
        });
        state
            .npc_disciples
            .push(simple_member("opponent-one", "opponent", 30));
        state.disciples = vec![
            simple_member("eligible-1", "player", 500),
            simple_member("eligible-2", "player", 400),
            simple_member("eligible-3", "player", 300),
            simple_member("eligible-4", "player", 200),
            simple_member("wrong-sect", "opponent", 5_000),
            {
                let mut member = simple_member("away", "player", 4_000);
                member.away_months = 1;
                member
            },
            {
                let mut member = simple_member("injured", "player", 3_000);
                member.condition = DiscipleCondition::SeriouslyInjured;
                member
            },
            {
                let mut member = simple_member("dead", "player", 6_000);
                member.alive = false;
                member.condition = DiscipleCondition::Dead;
                member
            },
        ];
        let mut rng = StdRng::seed_from_u64(18);

        run_tournament(&mut rng, &mut state);

        let lineup = &state.tournament_history.last().unwrap().player_lineup;
        assert_eq!(
            lineup
                .iter()
                .map(|member| member.disciple_id.as_str())
                .collect::<Vec<_>>(),
            vec!["eligible-1", "eligible-2", "eligible-3"]
        );
    }

    #[test]
    fn an_empty_lineup_loses_each_missing_bout_to_a_present_opponent() {
        let mut state = GameState::default();
        state.disciples.clear();
        state.npc_sects.push(SectState {
            id: "present".into(),
            name: "齐整派".into(),
            ..SectState::default()
        });
        state
            .npc_disciples
            .push(simple_member("present-one", "present", 100));
        let mut rng = StdRng::seed_from_u64(91);

        let result = run_tournament(&mut rng, &mut state);

        assert_eq!(result.rank, 2);
        let game = &state.tournament_history[0].rounds[0].matches[0];
        assert!(!game.bye);
        assert_eq!(game.bouts.len(), 3);
        assert_eq!(game.winner_sect_id, "present");
        assert!(game.bouts.iter().all(|bout| {
            match (
                bout.left.as_ref(),
                bout.right.as_ref(),
                bout.winner_sect_id.as_deref(),
            ) {
                (Some(_), None, Some("present")) | (None, Some(_), Some("present")) => true,
                (None, None, None) => true,
                _ => false,
            }
        }));
    }

    #[test]
    fn a_whole_tournament_uses_one_snapshot_then_merges_real_skill_growth() {
        let mut state = GameState {
            world_seed: 722,
            ..GameState::default()
        };
        state.disciples = (0..3)
            .map(|index| trained_member(&format!("hero-{index}"), "player", 2_000))
            .collect();
        for (sect_id, name) in [
            ("wudang", "武当派"),
            ("huashan", "华山派"),
            ("shaolin", "少林派"),
        ] {
            state.npc_sects.push(SectState {
                id: sect_id.into(),
                name: name.into(),
                ..SectState::default()
            });
            for index in 0..3 {
                state.npc_disciples.push(trained_member(
                    &format!("{sect_id}-{index}"),
                    sect_id,
                    40,
                ));
            }
        }
        let hero_id = state.disciples[0].id.clone();
        let art_id = interaction::spar_art(&state.disciples[0]).unwrap();
        let progress_before = state.disciples[0].martial_progress.proficiencies[&art_id].clone();
        let reputation_before = state.disciples[0].attributes.reputation;
        let inner_power_before = state.disciples[0].inner_power;
        let mut rng = StdRng::seed_from_u64(722);

        let result = run_tournament(&mut rng, &mut state);

        assert_eq!(result.rank, 1);
        let record = state.tournament_history.last().unwrap();
        let snapshot_scores = record
            .rounds
            .iter()
            .flat_map(|round| &round.matches)
            .flat_map(|game| &game.bouts)
            .flat_map(|bout| [bout.left.as_ref(), bout.right.as_ref()])
            .flatten()
            .filter(|member| member.disciple_id == hero_id)
            .map(|member| member.combat_score)
            .collect::<Vec<_>>();
        assert_eq!(snapshot_scores.len(), 2);
        assert!(snapshot_scores
            .iter()
            .all(|score| *score == snapshot_scores[0]));
        let hero = &state.disciples[0];
        let progress_after = &hero.martial_progress.proficiencies[&art_id];
        assert_eq!(progress_after.level, progress_before.level);
        assert_eq!(progress_after.experience - progress_before.experience, 24);
        assert_eq!(hero.attributes.reputation, reputation_before + 2);
        assert_eq!(hero.inner_power, inner_power_before);
    }

    #[test]
    fn fixed_rng_replays_the_same_draw_from_the_same_snapshot() {
        let mut left = twenty_five_sect_state();
        let mut right = left.clone();
        let mut left_rng = StdRng::seed_from_u64(77);
        let mut right_rng = StdRng::seed_from_u64(77);

        run_tournament(&mut left_rng, &mut left);
        run_tournament(&mut right_rng, &mut right);

        let left_record = serde_json::to_value(left.tournament_history.last().unwrap()).unwrap();
        let right_record = serde_json::to_value(right.tournament_history.last().unwrap()).unwrap();
        assert_eq!(left_record, right_record);
    }
}
