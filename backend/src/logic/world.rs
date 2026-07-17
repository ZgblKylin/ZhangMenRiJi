use crate::logic::disciple::{
    add_permanent_neili, assign_sect_curriculum, generate_disciple, sync_legacy_attributes,
};
use crate::models::attributes::{Department, DiscipleRank};
use crate::models::martial_art::{all_martial_arts, knowledge_skill_id};
use crate::models::sect::{
    default_buildings, MoralDirection, SectAttributes, SectPolicy, SectState,
};
use crate::models::{Disciple, GameEvent, GameState};
use rand::{rngs::StdRng, seq::SliceRandom, Rng, SeedableRng};
use std::collections::BTreeMap;

struct SectTemplate {
    id: &'static str,
    name: &'static str,
    country: &'static str,
    policy: SectPolicy,
    signature: &'static str,
    landmark: &'static str,
    morality: i32,
    members: &'static [&'static str],
}

const SECTS: &[SectTemplate] = &[
    SectTemplate {
        id: "wudang",
        name: "武当派",
        country: "song",
        policy: SectPolicy::Scholarly,
        signature: "taiji",
        landmark: "紫霄宫",
        morality: 82,
        members: &["张三丰", "宋远桥", "俞莲舟"],
    },
    SectTemplate {
        id: "huashan",
        name: "华山派",
        country: "song",
        policy: SectPolicy::Martial,
        signature: "dugu",
        landmark: "有所不为轩",
        morality: 58,
        members: &["岳不群", "宁中则", "令狐冲"],
    },
    SectTemplate {
        id: "mingjiao",
        name: "明教",
        country: "yuan",
        policy: SectPolicy::Chivalrous,
        signature: "qiankun",
        landmark: "光明顶",
        morality: 65,
        members: &["张无忌", "杨逍", "范遥"],
    },
    SectTemplate {
        id: "quanzhen",
        name: "全真派",
        country: "song",
        policy: SectPolicy::Scholarly,
        signature: "xiantian",
        landmark: "重阳宫",
        morality: 78,
        members: &["王重阳", "丘处机", "马钰"],
    },
    SectTemplate {
        id: "tianlong",
        name: "天龙寺",
        country: "dali",
        policy: SectPolicy::Reclusive,
        signature: "liumai",
        landmark: "牟尼堂",
        morality: 86,
        members: &["段正明", "本因", "本相"],
    },
    SectTemplate {
        id: "taohua",
        name: "桃花岛",
        country: "song",
        policy: SectPolicy::Reclusive,
        signature: "luoying",
        landmark: "试剑亭",
        morality: 54,
        members: &["黄药师", "陈玄风", "梅超风"],
    },
    SectTemplate {
        id: "shaolin",
        name: "少林派",
        country: "song",
        policy: SectPolicy::Scholarly,
        signature: "yijin",
        landmark: "达摩院",
        morality: 88,
        members: &["玄慈", "空闻", "渡厄"],
    },
    SectTemplate {
        id: "gumu",
        name: "古墓派",
        country: "song",
        policy: SectPolicy::Reclusive,
        signature: "yunu",
        landmark: "寒玉室",
        morality: 69,
        members: &["小龙女", "杨过", "孙婆婆"],
    },
    SectTemplate {
        id: "gaibang",
        name: "丐帮",
        country: "song",
        policy: SectPolicy::Chivalrous,
        signature: "dagou",
        landmark: "忠义堂",
        morality: 76,
        members: &["洪七公", "黄蓉", "鲁有脚"],
    },
    SectTemplate {
        id: "emei",
        name: "峨嵋派",
        country: "song",
        policy: SectPolicy::Chivalrous,
        signature: "jiuyin",
        landmark: "清音阁",
        morality: 74,
        members: &["郭襄", "灭绝师太", "周芷若"],
    },
    SectTemplate {
        id: "riyue",
        name: "日月神教",
        country: "yuan",
        policy: SectPolicy::Martial,
        signature: "kuihua",
        landmark: "成德殿",
        morality: 25,
        members: &["东方不败", "任我行", "向问天"],
    },
    SectTemplate {
        id: "xingxiu",
        name: "星宿派",
        country: "xia",
        policy: SectPolicy::Martial,
        signature: "huagong",
        landmark: "星宿海",
        morality: 12,
        members: &["丁春秋", "摘星子", "阿紫"],
    },
    SectTemplate {
        id: "murong",
        name: "姑苏慕容",
        country: "song",
        policy: SectPolicy::Scholarly,
        signature: "douzhuan",
        landmark: "还施水阁",
        morality: 42,
        members: &["慕容复", "慕容博", "邓百川"],
    },
    SectTemplate {
        id: "dalun",
        name: "大轮寺",
        country: "yuan",
        policy: SectPolicy::Martial,
        signature: "longxiang",
        landmark: "大经堂",
        morality: 39,
        members: &["金轮法王", "霍都", "达尔巴"],
    },
    SectTemplate {
        id: "court",
        name: "朝廷",
        country: "yuan",
        policy: SectPolicy::Mercantile,
        signature: "xuantian",
        landmark: "神武门",
        morality: 47,
        members: &["韦小宝", "多隆", "海大富"],
    },
    SectTemplate {
        id: "lingjiu",
        name: "灵鹫宫",
        country: "xia",
        policy: SectPolicy::Reclusive,
        signature: "bahuang",
        landmark: "缥缈峰",
        morality: 51,
        members: &["天山童姥", "虚竹", "余婆"],
    },
    SectTemplate {
        id: "baituo",
        name: "白驼山",
        country: "xia",
        policy: SectPolicy::Martial,
        signature: "hamagong",
        landmark: "蛇窟",
        morality: 19,
        members: &["欧阳锋", "欧阳克", "姬人"],
    },
    SectTemplate {
        id: "xueshan",
        name: "雪山派",
        country: "dali",
        policy: SectPolicy::Martial,
        signature: "xueshan",
        landmark: "凌霄城",
        morality: 48,
        members: &["白自在", "封万里", "花万紫"],
    },
    SectTemplate {
        id: "tiandihui",
        name: "天地会",
        country: "song",
        policy: SectPolicy::Chivalrous,
        signature: "ningxue",
        landmark: "青木堂",
        morality: 83,
        members: &["陈近南", "吴六奇", "玄贞道长"],
    },
    SectTemplate {
        id: "shenlong",
        name: "神龙教",
        country: "yuan",
        policy: SectPolicy::Martial,
        signature: "shenlong",
        landmark: "五龙门",
        morality: 18,
        members: &["洪安通", "苏荃", "陆高轩"],
    },
    SectTemplate {
        id: "jueqing",
        name: "绝情谷",
        country: "dali",
        policy: SectPolicy::Reclusive,
        signature: "yinyang",
        landmark: "厉鬼峰",
        morality: 31,
        members: &["公孙止", "裘千尺", "公孙绿萼"],
    },
    SectTemplate {
        id: "wudu",
        name: "五毒教",
        country: "dali",
        policy: SectPolicy::Reclusive,
        signature: "wudu",
        landmark: "炼蛊房",
        morality: 36,
        members: &["蓝凤凰", "何铁手", "齐云璈"],
    },
    SectTemplate {
        id: "qingcheng",
        name: "青城派",
        country: "song",
        policy: SectPolicy::Martial,
        signature: "songfeng",
        landmark: "松风观",
        morality: 44,
        members: &["余沧海", "侯人英", "洪人雄"],
    },
];

const WANDERERS: &[&str] = &[
    "胡斐",
    "袁承志",
    "狄云",
    "石破天",
    "苗人凤",
    "萧峰",
    "程灵素",
    "阿朱",
];

pub fn generate_npc_world(seed: u64) -> (Vec<SectState>, Vec<Disciple>) {
    let mut rng = StdRng::seed_from_u64(seed);
    let mut sects = Vec::with_capacity(SECTS.len());
    let mut disciples = Vec::with_capacity(SECTS.len() * 3 + WANDERERS.len());

    for (sect_index, template) in SECTS.iter().enumerate() {
        let _traditional_landmark = template.landmark;
        let prestige = 58 + (sect_index as i32 * 7 % 35);
        let mut buildings = default_buildings();
        for building in &mut buildings {
            building.level = 2 + (sect_index as i32 % 3);
        }
        let mut sect = SectState {
            id: template.id.into(),
            name: template.name.into(),
            country_id: template.country.into(),
            player_controlled: false,
            attributes: SectAttributes {
                prestige,
                silver: 700 + sect_index as i32 * 45,
                morality: template.morality,
                morale: 55 + sect_index as i32 % 30,
            },
            policy: template.policy.clone(),
            moral_direction: if template.morality >= 65 {
                MoralDirection::Righteous
            } else if template.morality <= 35 {
                MoralDirection::Villainous
            } else {
                MoralDirection::Neutral
            },
            rank_rules: Default::default(),
            buildings,
            inventory: BTreeMap::from([
                ("粮秣".into(), 120 + sect_index as i32 * 3),
                ("草药".into(), 30 + sect_index as i32),
                ("精铁".into(), 18 + sect_index as i32 % 12),
            ]),
            public_books: std::iter::once(knowledge_skill_id(template.id))
                .chain(
                    all_martial_arts()
                        .into_iter()
                        .filter(|art| {
                            art.sect_id.as_deref() == Some(template.id)
                                && art.is_combat
                                && art.category
                                    != crate::models::martial_art::SkillCategory::Parry
                        })
                        .map(|art| art.id),
                )
                .collect(),
            martial_research: BTreeMap::from([(template.signature.into(), 180 + prestige as i64)]),
            relations: BTreeMap::new(),
            active_orders: vec![],
            productions: vec![],
            auto_brew_index: 0,
            auto_brew_progress: 0,
        };

        for (member_index, name) in template.members.iter().enumerate() {
            let mut disciple = generate_disciple(&mut rng, 18 - member_index as i32 * 3);
            disciple.id = format!("npc_{}_{}", template.id, member_index + 1);
            disciple.sect_id = Some(template.id.into());
            disciple.name = (*name).into();
            disciple.martial_art = template.signature.into();
            disciple.rank = match member_index {
                0 => DiscipleRank::Inner,
                1 => DiscipleRank::Inner,
                _ => DiscipleRank::Outer,
            };
            disciple.department = Some(match member_index {
                0 => Department::Transmission,
                1 => Department::ExternalAffairs,
                _ => Department::Stewardship,
            });
            disciple.attributes.morality = template.morality;
            disciple.attributes.reputation = prestige / 2 + 20 - member_index as i32 * 4;
            disciple.attributes.attainment = 800 - member_index as i64 * 180 + prestige as i64 * 4;
            add_permanent_neili(&mut disciple, 35 - member_index as i32 * 8);
            assign_sect_curriculum(&mut disciple, template.id, 320 - member_index as i32 * 70);
            disciple.martial_art = template.signature.into();
            sync_legacy_attributes(&mut disciple);
            disciples.push(disciple);
        }
        sect.buildings[0].elder_id = Some(format!("npc_{}_1", template.id));
        sect.buildings[1].elder_id = Some(format!("npc_{}_2", template.id));
        sects.push(sect);
    }

    initialize_relations(&mut sects);
    for (index, name) in WANDERERS.iter().enumerate() {
        let mut disciple = generate_disciple(&mut rng, 14);
        disciple.id = format!("wanderer_{}", index + 1);
        disciple.sect_id = None;
        disciple.name = (*name).into();
        disciple.rank = DiscipleRank::Inner;
        disciple.attributes.attainment = 700 + index as i64 * 65;
        disciple.attributes.reputation = 55 + index as i32 * 3;
        sync_legacy_attributes(&mut disciple);
        disciples.push(disciple);
    }
    (sects, disciples)
}

fn initialize_relations(sects: &mut [SectState]) {
    let snapshot: Vec<(String, i32)> = sects
        .iter()
        .map(|sect| (sect.id.clone(), sect.attributes.morality))
        .collect();
    for sect in sects {
        for (other_id, morality) in &snapshot {
            if other_id == &sect.id {
                continue;
            }
            let relation = 30 - (sect.attributes.morality - morality).abs();
            sect.relations
                .insert(other_id.clone(), relation.clamp(-80, 60));
        }
    }
}

pub fn hydrate_world(state: &mut crate::models::GameState) {
    if state.npc_sects.len() < 18 || state.npc_disciples.is_empty() {
        let (sects, disciples) = generate_npc_world(state.world_seed);
        state.npc_sects = sects;
        state.npc_disciples = disciples;
    }
    for disciple in &mut state.npc_disciples {
        crate::logic::disciple::hydrate_v2_disciple(disciple);
    }
    for sect in &mut state.npc_sects {
        crate::logic::sect::normalize_buildings(sect);
        crate::logic::sect::normalize_elder_assignments(sect, &state.npc_disciples);
    }
}

fn chronicle_figure_candidates<'a>(
    sect: &SectState,
    disciples: &'a [Disciple],
) -> Vec<(&'a Disciple, &'static str)> {
    let leader_id = format!("npc_{}_1", sect.id);
    let elder_ids: std::collections::BTreeSet<&str> = sect
        .buildings
        .iter()
        .filter_map(|building| building.elder_id.as_deref())
        .collect();
    let senior: Vec<_> = disciples
        .iter()
        .filter(|disciple| {
            disciple.alive
                && disciple.sect_id.as_deref() == Some(sect.id.as_str())
                && (disciple.id == leader_id || elder_ids.contains(disciple.id.as_str()))
        })
        .map(|disciple| {
            let role = if disciple.id == leader_id {
                "掌门"
            } else {
                "长老"
            };
            (disciple, role)
        })
        .collect();
    if !senior.is_empty() {
        return senior;
    }

    // 兼容旧存档中的随机人物 ID：若掌门、长老 ID 均已失配，仍从该派内门取材。
    disciples
        .iter()
        .filter(|disciple| {
            disciple.alive
                && disciple.sect_id.as_deref() == Some(sect.id.as_str())
                && disciple.rank == DiscipleRank::Inner
        })
        .map(|disciple| (disciple, "内门弟子"))
        .collect()
}

/// NPC 门派按玩家相同的名额和长老规则逐月经营。
pub fn run_npc_ai(rng: &mut impl Rng, state: &mut GameState) -> Vec<GameEvent> {
    let sect_ids: Vec<String> = state.npc_sects.iter().map(|sect| sect.id.clone()).collect();
    let mut notable_sects = std::collections::BTreeSet::new();
    for sect_id in sect_ids {
        let Some(sect_index) = state.npc_sects.iter().position(|sect| sect.id == sect_id) else {
            continue;
        };
        let members: Vec<Disciple> = state
            .npc_disciples
            .iter()
            .filter(|disciple| {
                disciple.alive && disciple.sect_id.as_deref() == Some(sect_id.as_str())
            })
            .cloned()
            .collect();
        let (_, inner_limit) =
            crate::logic::sect::rank_limits(&state.npc_sects[sect_index], &members);
        let elder_count = state.npc_sects[sect_index]
            .buildings
            .iter()
            .filter(|building| building.elder_id.is_some())
            .count();
        let urgent = inner_limit < elder_count;
        let mut notes = Vec::new();
        if rng.gen_bool(if urgent { 0.82 } else { 0.08 })
            && state.npc_sects[sect_index].attributes.silver >= 35
        {
            state.npc_sects[sect_index].attributes.silver -= 35;
            let mut recruit =
                generate_disciple(rng, state.npc_sects[sect_index].attributes.prestige / 20);
            recruit.sect_id = Some(sect_id.clone());
            recruit.origin_sect_id = Some(sect_id.clone());
            recruit.rank = DiscipleRank::Chore;
            assign_sect_curriculum(&mut recruit, &sect_id, 80);
            state.npc_disciples.push(recruit);
            notes.push("广开山门纳得新人");
        }

        let snapshot: Vec<Disciple> = state
            .npc_disciples
            .iter()
            .filter(|disciple| {
                disciple.alive && disciple.sect_id.as_deref() == Some(sect_id.as_str())
            })
            .cloned()
            .collect();
        let (outer_limit, _) =
            crate::logic::sect::rank_limits(&state.npc_sects[sect_index], &snapshot);
        let outer_count = snapshot
            .iter()
            .filter(|disciple| disciple.rank == DiscipleRank::Outer)
            .count();
        if outer_count < outer_limit {
            if let Some(candidate) = state
                .npc_disciples
                .iter_mut()
                .filter(|disciple| {
                    disciple.alive
                        && disciple.sect_id.as_deref() == Some(sect_id.as_str())
                        && disciple.rank == DiscipleRank::Chore
                })
                .max_by_key(|disciple| disciple.merit)
            {
                candidate.rank = DiscipleRank::Outer;
                notes.push("擢升一名外门弟子");
            }
        }
        let snapshot: Vec<Disciple> = state
            .npc_disciples
            .iter()
            .filter(|disciple| {
                disciple.alive && disciple.sect_id.as_deref() == Some(sect_id.as_str())
            })
            .cloned()
            .collect();
        let (_, inner_limit) =
            crate::logic::sect::rank_limits(&state.npc_sects[sect_index], &snapshot);
        let inner_count = snapshot
            .iter()
            .filter(|disciple| disciple.rank == DiscipleRank::Inner)
            .count();
        if inner_count < inner_limit {
            if let Some(candidate) = state
                .npc_disciples
                .iter_mut()
                .filter(|disciple| {
                    disciple.alive
                        && disciple.sect_id.as_deref() == Some(sect_id.as_str())
                        && disciple.rank == DiscipleRank::Outer
                })
                .max_by_key(|disciple| disciple.merit)
            {
                candidate.rank = DiscipleRank::Inner;
                notes.push("擢升一名内门弟子");
            }
        }

        let assigned: std::collections::BTreeSet<String> = state.npc_sects[sect_index]
            .buildings
            .iter()
            .filter_map(|building| building.elder_id.clone())
            .collect();
        let mut candidates: Vec<String> = state
            .npc_disciples
            .iter()
            .filter(|disciple| {
                disciple.alive
                    && disciple.sect_id.as_deref() == Some(sect_id.as_str())
                    && disciple.rank == DiscipleRank::Inner
                    && !assigned.contains(&disciple.id)
            })
            .map(|disciple| disciple.id.clone())
            .collect();
        for building in &mut state.npc_sects[sect_index].buildings {
            if building.elder_id.is_none() {
                if let Some(id) = candidates.pop() {
                    building.elder_id = Some(id);
                    notes.push("补授一席长老");
                }
            }
            if building.elder_id.is_some() {
                building.elder_action_used = true;
            }
        }
        if elder_count > 0 {
            state.npc_sects[sect_index].attributes.silver += elder_count as i32 * 2;
        }
        if !notes.is_empty() {
            notable_sects.insert(sect_id);
        }
    }

    // 江湖纪事不再按门派数组顺序截取，改由各派掌门、在任长老中随机取材。
    // 这里只写人物行止，不把幕后经营数值直接摊在纪事中。
    let mut figures = Vec::new();
    for sect in &state.npc_sects {
        let candidates = chronicle_figure_candidates(sect, &state.npc_disciples);
        if let Some((figure, role)) = candidates.choose(rng).copied() {
            figures.push((
                sect.id.clone(),
                sect.name.clone(),
                figure.name.clone(),
                role,
            ));
        }
    }
    figures.shuffle(rng);
    let take = rng.gen_range(2..=4).min(figures.len());
    figures
        .into_iter()
        .take(take)
        .map(|(sect_id, sect_name, name, role)| {
            let text = if notable_sects.contains(&sect_id) {
                format!(
                    "{}{}{}召集门人整顿堂务，直到暮色漫过山门才收卷离席。",
                    sect_name, role, name
                )
            } else {
                match rng.gen_range(0..4) {
                    0 => format!(
                        "{}{}{}晨起巡视山门，沿途与弟子谈武论道，至午方归。",
                        sect_name, role, name
                    ),
                    1 => format!(
                        "{}{}{}在灯下校阅门中簿册，又召来几名弟子细问近况。",
                        sect_name, role, name
                    ),
                    2 => format!(
                        "{}{}{}于堂前考校门人，众弟子各展所学，山中颇为热闹。",
                        sect_name, role, name
                    ),
                    _ => format!(
                        "{}{}{}闭门会见远客，席间所谈无人知晓，只闻更鼓数声。",
                        sect_name, role, name
                    ),
                }
            };
            GameEvent {
                text,
                mood: "neutral".into(),
                year: state.year,
                month: state.month,
                category: "world".into(),
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::martial_art::{base_skill_ids, sect_combat_arts, MartialTier};

    #[test]
    fn world_contains_all_required_factions_and_people() {
        let (sects, disciples) = generate_npc_world(42);
        assert_eq!(sects.len(), 23);
        assert!(sects.iter().all(|sect| sect.buildings.len() >= 7));
        assert!(sects.iter().all(|sect| sect.public_books.len() >= 2));
        assert!(sects.iter().all(|sect| {
            disciples
                .iter()
                .filter(|d| d.sect_id.as_deref() == Some(sect.id.as_str()))
                .count()
                >= 3
        }));
        assert!(disciples.iter().filter(|d| d.sect_id.is_none()).count() >= 8);
        for disciple in disciples.iter().filter(|d| d.sect_id.is_some()) {
            let sect_id = disciple.sect_id.as_deref().unwrap();
            assert!(base_skill_ids(sect_id)
                .iter()
                .all(|id| disciple.martial_progress.proficiencies.contains_key(id)));
            let tier = match disciple.rank {
                DiscipleRank::Outer => MartialTier::Outer,
                DiscipleRank::Inner => MartialTier::Inner,
                DiscipleRank::Chore => MartialTier::Chore,
            };
            assert!(sect_combat_arts(sect_id, tier).iter().all(|art| disciple
                .martial_progress
                .proficiencies
                .contains_key(&art.id)));
        }
    }

    #[test]
    fn seeded_world_is_stable() {
        let (sects_a, disciples_a) = generate_npc_world(9);
        let (sects_b, disciples_b) = generate_npc_world(9);
        assert_eq!(sects_a[7].attributes.silver, sects_b[7].attributes.silver);
        assert_eq!(disciples_a[12].name, disciples_b[12].name);
        assert_eq!(
            disciples_a[12].aptitudes.strength,
            disciples_b[12].aptitudes.strength
        );
    }

    #[test]
    fn understaffed_npc_sects_tend_to_recruit_and_follow_rank_rules() {
        let mut state = GameState::default();
        let (sects, disciples) = generate_npc_world(19);
        state.npc_sects = sects;
        state.npc_disciples = disciples;
        let before = state
            .npc_disciples
            .iter()
            .filter(|disciple| disciple.sect_id.as_deref() == Some("wudang"))
            .count();
        let mut rng = StdRng::seed_from_u64(19);
        for _ in 0..8 {
            run_npc_ai(&mut rng, &mut state);
        }
        let members: Vec<_> = state
            .npc_disciples
            .iter()
            .filter(|disciple| disciple.sect_id.as_deref() == Some("wudang"))
            .collect();
        assert!(members.len() > before);
        assert!(members
            .iter()
            .any(|disciple| disciple.rank == DiscipleRank::Chore));
        assert!(state.npc_sects.iter().all(|sect| sect.buildings.len() == 7));
    }

    #[test]
    fn chronicles_randomly_draw_from_leaders_and_elders_across_sects() {
        let mut state = GameState::default();
        let (sects, disciples) = generate_npc_world(27);
        state.npc_sects = sects;
        state.npc_disciples = disciples;
        let mut seen_sects = std::collections::BTreeSet::new();
        for seed in 0..32 {
            let mut rng = StdRng::seed_from_u64(seed);
            let events = run_npc_ai(&mut rng, &mut state);
            assert!((2..=4).contains(&events.len()));
            assert!(events.iter().all(|event| event.category == "world"));
            assert!(events.iter().all(|event| {
                !event
                    .text
                    .chars()
                    .any(|character| character.is_ascii_digit())
                    && !event.text.contains("点")
                    && !event.text.contains("两")
                    && !event.text.contains('+')
            }));
            for sect in &state.npc_sects {
                if events.iter().any(|event| event.text.contains(&sect.name)) {
                    seen_sects.insert(sect.id.clone());
                }
            }
        }
        assert!(seen_sects.len() >= 10);
    }

    #[test]
    fn chronicle_candidates_fall_back_to_inner_disciples_for_legacy_ids() {
        let (mut sects, mut disciples) = generate_npc_world(31);
        let sect = sects
            .iter_mut()
            .find(|sect| sect.id == "qingcheng")
            .unwrap();
        for building in &mut sect.buildings {
            building.elder_id = None;
        }
        for (index, disciple) in disciples
            .iter_mut()
            .filter(|disciple| disciple.sect_id.as_deref() == Some("qingcheng"))
            .enumerate()
        {
            disciple.id = format!("legacy-person-{index}");
        }

        let candidates = chronicle_figure_candidates(sect, &disciples);
        assert!(!candidates.is_empty());
        assert!(candidates.iter().all(|(disciple, role)| {
            disciple.rank == DiscipleRank::Inner && *role == "内门弟子"
        }));
    }
}
