use crate::logic::{disciple as disc, sect};
use crate::models::attributes::DiscipleRank;
use crate::models::martial_art::{
    all_martial_arts, knowledge_skill_id, martial_art_by_id, MartialArt, MartialTier,
};
use crate::models::{Disciple, GameEvent, GameState};
use rand::Rng;

fn clamp(v: i32, lo: i32, hi: i32) -> i32 {
    v.max(lo).min(hi)
}

/// 执行决策，返回产生的事件
pub fn execute_decision(
    rng: &mut impl Rng,
    state: &mut GameState,
    decision_id: &str,
) -> Vec<GameEvent> {
    let mut events = vec![];
    if state.game_over || state.pending_event.is_some() {
        return events;
    }

    match decision_id {
        "recruit" => {
            if state.sect.attributes.silver < 50 {
                return events;
            }
            state.sect.attributes.silver -= 50;
            let count = disc::rand_range(rng, 1, 2);
            let mut recruits = Vec::with_capacity(count as usize);
            for _ in 0..count {
                let bonus = if state.sect.attributes.prestige > 50 {
                    10
                } else {
                    0
                };
                let mut recruit = disc::generate_disciple(rng, bonus);
                recruit.sect_id = Some(state.sect.id.clone());
                recruits.push(recruit);
            }
            crate::logic::sect::assign_recruit_ranks(&state.sect, &state.disciples, &mut recruits);
            let names = recruits
                .iter()
                .map(|recruit| format!("{}（{}）", recruit.name, rank_name(&recruit.rank)))
                .collect::<Vec<_>>();
            state.disciples.extend(recruits);
            state.total_disciples_recruited += count;
            sect::sync_legacy_fields(state);
            events.push(GameEvent {
                text: format!("招贤榜贴出，{}前来拜山投师。", names.join("、")),
                mood: "good".into(),
                year: state.year,
                month: state.month,
                category: "sect".into(),
            });
        }
        "train" => {
            if state.injury >= 30 {
                return events;
            }
            let effectiveness = sect::building_effectiveness(&state.sect, "practice");
            if effectiveness <= 0 {
                return events;
            }
            let training = state
                .disciples
                .iter()
                .enumerate()
                .filter(|(_, disciple)| available_player_disciple(state, disciple))
                .filter_map(|(index, disciple)| {
                    let art_id = trainable_known_art(state, disciple)?;
                    let aptitude = (disciple.aptitudes.strength + disciple.aptitudes.agility) / 2;
                    let gain = training_experience(
                        disciple,
                        &art_id,
                        aptitude,
                        95 + disc::rand_range(rng, 0, 20),
                        effectiveness,
                    );
                    Some((
                        index,
                        disciple.name.clone(),
                        art_id.clone(),
                        gain,
                        sect::martial_research_level_cap(&state.sect, &art_id),
                        disc::rand_range(rng, 1, 3),
                    ))
                })
                .collect::<Vec<_>>();
            if training.is_empty() {
                return events;
            }
            for (index, _, art_id, gain, research_cap, loyalty_gain) in &training {
                let disciple = &mut state.disciples[*index];
                disc::gain_skill_experience_with_cap(disciple, art_id, *gain, *research_cap);
                disciple.attributes.sect_loyalty =
                    clamp(disciple.attributes.sect_loyalty + loyalty_gain, 0, 100);
                disc::sync_legacy_attributes(disciple);
            }
            state.injury = clamp(state.injury + disc::rand_range(rng, 3, 8), 0, 100);
            state.sect.attributes.morale = clamp(state.sect.attributes.morale + 2, 0, 100);
            sect::sync_legacy_fields(state);
            events.push(GameEvent {
                text: format!(
                    "掌门在演武场督课，{}依各自所学磨炼武艺；修习仍受根基、造诣与门派参研所限。",
                    training
                        .iter()
                        .map(|(_, name, _, _, _, _)| name.as_str())
                        .collect::<Vec<_>>()
                        .join("、")
                ),
                mood: "good".into(),
                year: state.year,
                month: state.month,
                category: "sect".into(),
            });
        }
        "mission" => {
            let effectiveness = sect::building_effectiveness(&state.sect, "affairs");
            if effectiveness <= 0 {
                return events;
            }
            let Some(steward_name) = state
                .disciples
                .iter()
                .find(|disciple| available_player_disciple(state, disciple))
                .map(|disciple| disciple.name.clone())
            else {
                return events;
            };
            // 这是门派级委托，由执事堂安排轮值门众共同办理；不占用某名弟子的
            // ActionPlan，也不会与真正的外派/游历行动重复结算个人收益。
            let silver_gain = scaled_output(disc::rand_range(rng, 18, 36), effectiveness);
            let prestige_gain = scaled_output(disc::rand_range(rng, 1, 3), effectiveness);
            state.sect.attributes.silver = state.sect.attributes.silver.saturating_add(silver_gain);
            state.sect.attributes.prestige =
                clamp(state.sect.attributes.prestige + prestige_gain, 0, 1000);
            state.sect.attributes.morale = clamp(state.sect.attributes.morale + 1, 0, 100);
            sect::sync_legacy_fields(state);
            events.push(GameEvent {
                text: format!(
                    "执事堂以门派名义承接乡里委托，由{}当值统筹、门众共同料理，结得银{}两、声望{}点。",
                    steward_name, silver_gain, prestige_gain
                ),
                mood: "good".into(),
                year: state.year,
                month: state.month,
                category: "sect".into(),
            });
        }
        "rest" => {
            let heal = disc::rand_range(rng, 15, 30);
            state.injury = clamp(state.injury - heal, 0, 100);
            state.sect.attributes.morale = clamp(state.sect.attributes.morale - 1, 0, 100);
            sect::sync_legacy_fields(state);
            events.push(GameEvent {
                text: "掌门闭门静养月余，伤势大有好转。".into(),
                mood: "good".into(),
                year: state.year,
                month: state.month,
                category: "sect".into(),
            });
        }
        "study" => {
            let effectiveness = sect::building_effectiveness(&state.sect, "scripture");
            if effectiveness <= 0 || state.sect.attributes.silver < 30 {
                return events;
            }
            let unlearned = all_martial_arts()
                .into_iter()
                .filter(|art| {
                    art.is_combat
                        && art.sect_id.as_deref() == Some("player")
                        && !state.martial_arts_learned.contains(&art.id)
                        && !state.sect.public_books.contains(&art.id)
                })
                .collect::<Vec<_>>();
            let existing = state
                .sect
                .public_books
                .iter()
                .filter_map(|id| martial_art_by_id(id))
                .filter(|art| art.is_combat)
                .collect::<Vec<_>>();
            if unlearned.is_empty() && existing.is_empty() {
                return events;
            }
            state.sect.attributes.silver -= 30;
            state.injury = clamp(state.injury + disc::rand_range(rng, 5, 12), 0, 100);
            let created_new_art =
                !unlearned.is_empty() && (existing.is_empty() || rng.gen_bool(0.5));
            if created_new_art {
                let art = &unlearned[disc::rand_range(rng, 0, unlearned.len() as i32 - 1) as usize];
                if !state.martial_arts_learned.contains(&art.id) {
                    state.martial_arts_learned.push(art.id.clone());
                }
                if !state.sect.public_books.contains(&art.id) {
                    state.sect.public_books.push(art.id.clone());
                }
                state.sect.martial_research.insert(
                    art.id.clone(),
                    i64::from(50 + scaled_output(6, effectiveness)),
                );
                events.push(GameEvent {
                    text: format!(
                        "掌门闭阁推演，终于将《{}》定稿成册，收入公藏并立下门派参研上限。",
                        art.name
                    ),
                    mood: "good".into(),
                    year: state.year,
                    month: state.month,
                    category: "sect".into(),
                });
            } else {
                let art = &existing[disc::rand_range(rng, 0, existing.len() as i32 - 1) as usize];
                let gain = scaled_output(4, effectiveness);
                *state
                    .sect
                    .martial_research
                    .entry(art.id.clone())
                    .or_insert(50) += i64::from(gain);
                events.push(GameEvent {
                    text: format!(
                        "掌门转而校订《{}》，令其门派可授上限提升{}级。",
                        art.name, gain
                    ),
                    mood: "neutral".into(),
                    year: state.year,
                    month: state.month,
                    category: "sect".into(),
                });
            }
            if created_new_art {
                state.sect.attributes.prestige = clamp(state.sect.attributes.prestige + 3, 0, 1000);
            }
            sect::sync_legacy_fields(state);
        }
        "research" => {
            let Ok(text) = crate::logic::management::research_new_martial(state) else {
                return events;
            };
            crate::logic::sect::sync_legacy_fields(state);
            events.push(GameEvent {
                text,
                mood: "good".into(),
                year: state.year,
                month: state.month,
                category: "sect".into(),
            });
        }
        "teach" => {
            let effectiveness = sect::building_effectiveness(&state.sect, "practice");
            if effectiveness <= 0 || state.sect.attributes.silver < 20 {
                return events;
            }
            let Some(session) = select_teaching_session(state) else {
                return events;
            };
            state.sect.attributes.silver -= 20;
            let teacher_name = state.disciples[session.teacher_index].name.clone();
            let teacher_level =
                skill_level(&state.disciples[session.teacher_index], &session.art_id);
            let teacher_intelligence =
                disc::effective_intelligence(&state.disciples[session.teacher_index]);
            let art_name = martial_art_by_id(&session.art_id)
                .map(|art| art.name)
                .unwrap_or_else(|| session.art_id.clone());
            let mut names = Vec::with_capacity(session.student_indices.len());
            for index in session.student_indices {
                let research_cap = sect::martial_research_level_cap(&state.sect, &session.art_id);
                let student_level = skill_level(&state.disciples[index], &session.art_id);
                let student_intelligence = disc::effective_intelligence(&state.disciples[index]);
                let gain = training_experience(
                    &state.disciples[index],
                    &session.art_id,
                    (teacher_intelligence + student_intelligence) / 2,
                    105 + (teacher_level - student_level).clamp(0, 60),
                    effectiveness,
                );
                let disciple = &mut state.disciples[index];
                disc::gain_skill_experience_with_cap(disciple, &session.art_id, gain, research_cap);
                disciple.attributes.sect_loyalty = clamp(
                    disciple.attributes.sect_loyalty + disc::rand_range(rng, 2, 5),
                    0,
                    100,
                );
                names.push(disciple.name.clone());
                disc::sync_legacy_attributes(disciple);
            }
            sect::sync_legacy_fields(state);
            events.push(GameEvent {
                text: format!(
                    "{}在演武场开坛，向{}讲授自己确已掌握的《{}》；所得进境均受根基、知识、造诣与门派参研上限约束。",
                    teacher_name,
                    names.join("、"),
                    art_name
                ),
                mood: "good".into(),
                year: state.year,
                month: state.month,
                category: "sect".into(),
            });
        }
        _ => return events,
    }

    state.decisions_used += 1;

    // 与月结、经营接口一致，卷宗统一保留最近 80 条。
    for ev in &events {
        state.event_log.push(ev.clone());
    }
    if state.event_log.len() > 80 {
        let excess = state.event_log.len() - 80;
        state.event_log.drain(0..excess);
    }

    events
}

fn available_player_disciple(state: &GameState, disciple: &Disciple) -> bool {
    disciple.sect_id.as_deref() == Some(state.sect.id.as_str())
        && disc::can_act(disciple)
        && disciple.action.is_none()
}

fn skill_level(disciple: &Disciple, art_id: &str) -> i32 {
    disciple
        .martial_progress
        .proficiencies
        .get(art_id)
        .map(|progress| progress.level)
        .unwrap_or(0)
}

fn learning_level_cap(state: &GameState, disciple: &Disciple, art: &MartialArt) -> Option<i32> {
    disc::skill_level_cap(disciple, &art.id)
        .into_iter()
        .chain(sect::martial_research_level_cap(&state.sect, &art.id))
        .min()
}

fn trainable_known_art(state: &GameState, disciple: &Disciple) -> Option<String> {
    disciple
        .martial_progress
        .proficiencies
        .iter()
        .filter_map(|(id, progress)| {
            let art = martial_art_by_id(id)?;
            if !art.is_combat
                || learning_level_cap(state, disciple, &art)
                    .is_some_and(|cap| progress.level >= cap)
            {
                return None;
            }
            Some((id == &disciple.martial_art, progress.level, id.clone()))
        })
        .max_by_key(|(prepared, level, _)| (*prepared, *level))
        .map(|(_, _, id)| id)
}

fn training_experience(
    disciple: &Disciple,
    art_id: &str,
    aptitude: i32,
    intensity: i32,
    effectiveness: i32,
) -> i64 {
    let level = skill_level(disciple, art_id).max(1);
    let sessions = 14 + aptitude.max(0) / 2;
    let difficulty = martial_art_by_id(art_id)
        .map(|art| art.difficulty)
        .unwrap_or(10)
        .max(5);
    let per_session = level / 5 + 1;
    (i64::from(per_session)
        .saturating_mul(i64::from(sessions))
        .saturating_mul(i64::from(intensity.max(1)))
        / i64::from(80 + difficulty * 3))
    .saturating_mul(i64::from(effectiveness.max(0)))
    .saturating_div(100)
    .max(1)
}

fn rank_can_receive_tier(rank: &DiscipleRank, tier: MartialTier) -> bool {
    match tier {
        MartialTier::Basic | MartialTier::Chore => true,
        MartialTier::Outer => matches!(rank, DiscipleRank::Outer | DiscipleRank::Inner),
        MartialTier::Inner => matches!(rank, DiscipleRank::Inner),
    }
}

fn has_learning_foundations(disciple: &Disciple, art: &MartialArt) -> bool {
    if !art.is_combat || art.tier == MartialTier::Basic {
        return true;
    }
    if art.basic_skill.is_empty()
        || skill_level(disciple, &art.basic_skill) <= 0
        || art
            .sect_id
            .as_deref()
            .is_some_and(|sect_id| skill_level(disciple, &knowledge_skill_id(sect_id)) <= 0)
    {
        return false;
    }
    true
}

fn can_receive_teaching(
    state: &GameState,
    teacher: &Disciple,
    student: &Disciple,
    art: &MartialArt,
    teacher_level: i32,
) -> bool {
    let student_level = skill_level(student, &art.id);
    teacher_level > student_level
        && sect::teaching_relationship_eligible(teacher, student)
        && rank_can_receive_tier(&student.rank, art.tier)
        && has_learning_foundations(student, art)
        && learning_level_cap(state, student, art).is_none_or(|cap| student_level < cap)
}

struct TeachingSession {
    teacher_index: usize,
    student_indices: Vec<usize>,
    art_id: String,
}

fn select_teaching_session(state: &GameState) -> Option<TeachingSession> {
    let available = state
        .disciples
        .iter()
        .enumerate()
        .filter(|(_, disciple)| available_player_disciple(state, disciple))
        .map(|(index, _)| index)
        .collect::<Vec<_>>();
    let mut best_score: Option<(usize, i32, i32)> = None;
    let mut best_session = None;

    for &teacher_index in &available {
        let teacher = &state.disciples[teacher_index];
        for (art_id, progress) in &teacher.martial_progress.proficiencies {
            if progress.level <= 0 {
                continue;
            }
            let Some(art) = martial_art_by_id(art_id) else {
                continue;
            };
            let mut students = available
                .iter()
                .copied()
                .filter(|index| *index != teacher_index)
                .filter(|index| {
                    can_receive_teaching(
                        state,
                        teacher,
                        &state.disciples[*index],
                        &art,
                        progress.level,
                    )
                })
                .collect::<Vec<_>>();
            students.sort_by_key(|index| skill_level(&state.disciples[*index], art_id));
            students.truncate(3);
            if students.is_empty() {
                continue;
            }
            let total_gap = students
                .iter()
                .map(|index| progress.level - skill_level(&state.disciples[*index], art_id))
                .sum();
            let score = (students.len(), total_gap, progress.level);
            if best_score.is_none_or(|current| score > current) {
                best_score = Some(score);
                best_session = Some(TeachingSession {
                    teacher_index,
                    student_indices: students,
                    art_id: art.id,
                });
            }
        }
    }
    best_session
}

fn scaled_output(value: i32, effectiveness: i32) -> i32 {
    value
        .saturating_mul(effectiveness.max(0))
        .saturating_div(100)
        .max(1)
}

fn rank_name(rank: &crate::models::attributes::DiscipleRank) -> &'static str {
    match rank {
        crate::models::attributes::DiscipleRank::Chore => "杂役",
        crate::models::attributes::DiscipleRank::Outer => "外门",
        crate::models::attributes::DiscipleRank::Inner => "内门",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::attributes::SkillProgress;
    use rand::{rngs::StdRng, SeedableRng};
    use std::collections::BTreeMap;

    fn test_disciple(
        id: &str,
        name: &str,
        hunyuan_level: i32,
        basic_force_level: i32,
        knowledge_level: i32,
    ) -> Disciple {
        let mut disciple = Disciple {
            id: id.into(),
            name: name.into(),
            sect_id: Some("player".into()),
            origin_sect_id: Some("player".into()),
            rank: DiscipleRank::Inner,
            martial_art: "hunyuan".into(),
            martial_schema_version: 3,
            ..Disciple::default()
        };
        disciple.martial_progress.proficiencies = BTreeMap::from([
            (
                "player_knowledge".into(),
                SkillProgress::new(knowledge_level, 0),
            ),
            (
                "basic_force".into(),
                SkillProgress::new(basic_force_level, 0),
            ),
            ("hunyuan".into(), SkillProgress::new(hunyuan_level, 0)),
        ]);
        disciple.attributes.attainment = disc::attainment_required_for_level(200);
        disc::normalize_prepared_skills(&mut disciple);
        disc::recalculate_attribute_maxima(&mut disciple);
        disc::sync_legacy_attributes(&mut disciple);
        disciple
    }

    #[test]
    fn research_decision_creates_a_public_scripture_manual() {
        let mut state = GameState::default();
        let silver_before = state.silver;
        let books_before = state.sect.public_books.clone();
        let mut rng = StdRng::seed_from_u64(7);

        let events = execute_decision(&mut rng, &mut state, "research");

        assert_eq!(events.len(), 1);
        assert_eq!(state.decisions_used, 1);
        assert_eq!(state.silver, silver_before - 120); // silver_before = GameState::default().silver
        let new_book = state
            .sect
            .public_books
            .iter()
            .find(|book| !books_before.contains(book))
            .expect("研发武学应写入藏经阁公册");
        assert!(state.martial_arts_learned.contains(new_book));
        assert_eq!(state.sect.martial_research.get(new_book), Some(&60));
    }

    #[test]
    fn recruit_and_rest_write_the_v3_sect_and_keep_legacy_mirrors_in_step() {
        let mut recruit_state = GameState::default();
        let silver_before = recruit_state.sect.attributes.silver;
        let mut recruit_rng = StdRng::seed_from_u64(701);

        let recruit_events = execute_decision(&mut recruit_rng, &mut recruit_state, "recruit");

        assert_eq!(recruit_events.len(), 1);
        assert_eq!(recruit_state.sect.attributes.silver, silver_before - 50);
        assert_eq!(recruit_state.silver, recruit_state.sect.attributes.silver);
        assert!(!recruit_state.disciples.is_empty());
        assert!(recruit_state
            .disciples
            .iter()
            .all(|disciple| disciple.sect_id.as_deref() == Some(recruit_state.sect.id.as_str())));

        let mut rest_state = GameState {
            injury: 60,
            ..GameState::default()
        };
        let morale_before = rest_state.sect.attributes.morale;
        let mut rest_rng = StdRng::seed_from_u64(702);

        let rest_events = execute_decision(&mut rest_rng, &mut rest_state, "rest");

        assert_eq!(rest_events.len(), 1);
        assert!(rest_state.injury < 60);
        assert_eq!(rest_state.sect.attributes.morale, morale_before - 1);
        assert_eq!(rest_state.morale, rest_state.sect.attributes.morale);
    }

    #[test]
    fn removed_hidden_legacy_decision_ids_are_true_no_ops() {
        for decision_id in ["repair", "diplomacy", "unknown"] {
            let mut state = GameState::default();
            let before = serde_json::to_value(&state).unwrap();
            let mut rng = StdRng::seed_from_u64(703);

            let events = execute_decision(&mut rng, &mut state, decision_id);

            assert!(events.is_empty());
            assert_eq!(serde_json::to_value(&state).unwrap(), before);
        }
    }

    #[test]
    fn train_grows_real_v3_skill_without_mutating_legacy_inner_power() {
        let active = test_disciple("active", "岳青", 20, 80, 80);
        let mut away = test_disciple("away", "陆远", 20, 80, 80);
        away.away_months = 2;
        let active_progress_before = active.martial_progress.proficiencies["hunyuan"].clone();
        let away_before = serde_json::to_value(&away).unwrap();
        let inner_power_before = active.inner_power;
        let neili_before = active.attributes.neili.maximum;
        let loyalty_before = active.attributes.sect_loyalty;
        let mut state = GameState {
            disciples: vec![active, away],
            ..GameState::default()
        };
        state.sect.martial_research.insert("hunyuan".into(), 100);
        let mut rng = StdRng::seed_from_u64(31);

        let events = execute_decision(&mut rng, &mut state, "train");

        assert_eq!(events.len(), 1);
        assert_eq!(state.decisions_used, 1);
        assert_ne!(
            state.disciples[0].martial_progress.proficiencies["hunyuan"],
            active_progress_before
        );
        assert_eq!(state.disciples[0].attributes.neili.maximum, neili_before);
        assert_eq!(state.disciples[0].inner_power, inner_power_before);
        assert!(state.disciples[0].attributes.sect_loyalty > loyalty_before);
        assert_eq!(
            state.disciples[0].loyalty,
            state.disciples[0].attributes.sect_loyalty
        );
        assert_eq!(
            serde_json::to_value(&state.disciples[1]).unwrap(),
            away_before
        );
    }

    #[test]
    fn study_always_updates_the_v3_library_or_research_cap() {
        let mut state = GameState::default();
        let books_before = state.sect.public_books.clone();
        let research_before = state.sect.martial_research.clone();
        let silver_before = state.sect.attributes.silver;
        let mut rng = StdRng::seed_from_u64(32);

        let events = execute_decision(&mut rng, &mut state, "study");

        assert_eq!(events.len(), 1);
        assert_eq!(state.sect.attributes.silver, silver_before - 30);
        assert_eq!(state.silver, silver_before - 30);
        if let Some(new_book) = state
            .sect
            .public_books
            .iter()
            .find(|book| !books_before.contains(book))
        {
            assert!(state.martial_arts_learned.contains(new_book));
            assert!(state.sect.martial_research.contains_key(new_book));
        } else {
            assert!(state
                .sect
                .martial_research
                .iter()
                .any(|(id, level)| { *level > research_before.get(id).copied().unwrap_or(50) }));
        }
    }

    #[test]
    fn teach_uses_a_real_teacher_skill_and_stops_at_the_lowest_v3_cap() {
        let teacher = test_disciple("teacher", "沈师姐", 60, 12, 15);
        let mut student = test_disciple("student", "顾小山", 11, 12, 15);
        student.master_id = Some("teacher".into());
        student
            .martial_progress
            .proficiencies
            .insert("hunyuan".into(), SkillProgress::new(11, 143));
        disc::sync_legacy_attributes(&mut student);
        let mut away = test_disciple("away", "莫行舟", 1, 12, 15);
        away.away_months = 1;
        let away_before = serde_json::to_value(&away).unwrap();
        let student_inner_power = student.inner_power;
        let mut state = GameState {
            disciples: vec![teacher, student, away],
            ..GameState::default()
        };
        state.sect.martial_research.insert("hunyuan".into(), 80);
        let mut rng = StdRng::seed_from_u64(33);

        let events = execute_decision(&mut rng, &mut state, "teach");

        assert_eq!(events.len(), 1);
        assert!(events[0].text.contains("沈师姐"));
        assert!(events[0].text.contains("混元功"));
        let learned = &state.disciples[1].martial_progress.proficiencies["hunyuan"];
        assert_eq!(learned.level, 12, "对应基础内功为最低上限");
        assert_eq!(state.disciples[1].inner_power, student_inner_power);
        assert_eq!(
            state.disciples[1].loyalty,
            state.disciples[1].attributes.sect_loyalty
        );
        assert_eq!(
            serde_json::to_value(&state.disciples[2]).unwrap(),
            away_before
        );
    }

    #[test]
    fn teach_requires_a_real_master_or_close_relationship() {
        let teacher = test_disciple("teacher", "沈师姐", 60, 80, 80);
        let student = test_disciple("student", "顾小山", 10, 80, 80);
        let mut state = GameState {
            disciples: vec![teacher, student],
            ..GameState::default()
        };
        let silver_before = state.sect.attributes.silver;
        let disciples_before = serde_json::to_value(&state.disciples).unwrap();
        let mut rng = StdRng::seed_from_u64(330);

        let events = execute_decision(&mut rng, &mut state, "teach");

        assert!(events.is_empty());
        assert_eq!(state.decisions_used, 0);
        assert_eq!(state.sect.attributes.silver, silver_before);
        assert_eq!(
            serde_json::to_value(&state.disciples).unwrap(),
            disciples_before
        );
    }

    #[test]
    fn mission_is_a_sect_resource_decision_and_never_rewards_a_disciple_twice() {
        let steward = test_disciple("steward", "周执事", 30, 60, 60);
        let mut traveler = test_disciple("traveler", "叶行", 30, 60, 60);
        traveler.away_months = 2;
        traveler.personal_silver = 17;
        let steward_before = serde_json::to_value(&steward).unwrap();
        let traveler_before = serde_json::to_value(&traveler).unwrap();
        let mut state = GameState {
            disciples: vec![steward, traveler],
            ..GameState::default()
        };
        let silver_before = state.sect.attributes.silver;
        let mut rng = StdRng::seed_from_u64(34);

        let events = execute_decision(&mut rng, &mut state, "mission");

        assert_eq!(events.len(), 1);
        assert!(events[0].text.contains("执事堂以门派名义"));
        assert!(state.sect.attributes.silver > silver_before);
        assert_eq!(state.silver, state.sect.attributes.silver);
        assert_eq!(
            serde_json::to_value(&state.disciples[0]).unwrap(),
            steward_before
        );
        assert_eq!(
            serde_json::to_value(&state.disciples[1]).unwrap(),
            traveler_before
        );
    }

    #[test]
    fn mission_cannot_create_resources_when_every_disciple_is_away() {
        let mut traveler = test_disciple("traveler", "叶行", 30, 60, 60);
        traveler.away_months = 2;
        let mut state = GameState {
            disciples: vec![traveler],
            ..GameState::default()
        };
        let silver_before = state.sect.attributes.silver;
        let prestige_before = state.sect.attributes.prestige;
        let mut rng = StdRng::seed_from_u64(340);

        let events = execute_decision(&mut rng, &mut state, "mission");

        assert!(events.is_empty());
        assert_eq!(state.decisions_used, 0);
        assert_eq!(state.sect.attributes.silver, silver_before);
        assert_eq!(state.sect.attributes.prestige, prestige_before);
    }

    #[test]
    fn study_keeps_raising_public_research_after_every_player_art_is_unlocked() {
        let player_arts = all_martial_arts()
            .into_iter()
            .filter(|art| art.is_combat && art.sect_id.as_deref() == Some("player"))
            .map(|art| art.id)
            .collect::<Vec<_>>();
        let mut state = GameState::default();
        for id in player_arts {
            if !state.martial_arts_learned.contains(&id) {
                state.martial_arts_learned.push(id.clone());
            }
            if !state.sect.public_books.contains(&id) {
                state.sect.public_books.push(id);
            }
        }
        let research_before = state.sect.martial_research.clone();
        let silver_before = state.sect.attributes.silver;
        let mut rng = StdRng::seed_from_u64(341);

        let events = execute_decision(&mut rng, &mut state, "study");

        assert_eq!(events.len(), 1);
        assert_eq!(state.decisions_used, 1);
        assert_eq!(state.sect.attributes.silver, silver_before - 30);
        assert!(state
            .sect
            .martial_research
            .iter()
            .any(|(id, level)| *level > research_before.get(id).copied().unwrap_or(50)));
    }

    #[test]
    fn damaged_buildings_block_all_four_v3_decisions_before_spending_or_growth() {
        for (decision_id, building_id) in [
            ("train", "practice"),
            ("teach", "practice"),
            ("study", "scripture"),
            ("mission", "affairs"),
        ] {
            let mut state = GameState {
                disciples: vec![
                    test_disciple("teacher", "师长", 60, 80, 80),
                    test_disciple("student", "门人", 10, 80, 80),
                ],
                ..GameState::default()
            };
            state
                .sect
                .buildings
                .iter_mut()
                .find(|building| building.id == building_id)
                .unwrap()
                .condition = 0;
            let before = serde_json::to_value(&state).unwrap();
            let mut rng = StdRng::seed_from_u64(35);

            let events = execute_decision(&mut rng, &mut state, decision_id);

            assert!(events.is_empty(), "{decision_id} 应被损毁建筑阻止");
            assert_eq!(state.decisions_used, 0);
            assert_eq!(serde_json::to_value(&state).unwrap(), before);
        }
    }

    #[test]
    fn legacy_decisions_cannot_mutate_pending_or_sealed_games() {
        let mut rng = StdRng::seed_from_u64(8);
        let mut sealed = GameState {
            game_over: true,
            game_won: true,
            ..GameState::default()
        };
        let sealed_silver = sealed.silver;

        let events = execute_decision(&mut rng, &mut sealed, "recruit");

        assert!(events.is_empty());
        assert_eq!(sealed.silver, sealed_silver);
        assert_eq!(sealed.decisions_used, 0);
        assert!(sealed.disciples.is_empty());

        let mut pending = GameState::default();
        pending.pending_event = Some(serde_json::json!({ "id": "pending" }));
        let pending_injury = pending.injury;
        let events = execute_decision(&mut rng, &mut pending, "rest");

        assert!(events.is_empty());
        assert_eq!(pending.injury, pending_injury);
        assert_eq!(pending.decisions_used, 0);
    }
}
