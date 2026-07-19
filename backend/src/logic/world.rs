use crate::logic::country;
use crate::logic::disciple::{
    add_permanent_neili, assign_sect_curriculum, generate_disciple, sync_legacy_attributes,
};
use crate::models::attributes::{Department, DiscipleRank};
use crate::models::martial_art::{
    all_martial_arts, knowledge_skill_id, martial_art_by_id, MartialTier,
};
use crate::models::named_npc::{all_named_npcs, NpcPosition};
use crate::models::sect::{sect_buildings, MoralDirection, SectAttributes, SectPolicy, SectState};
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
        name: "金帐汗国",
        country: "yuan",
        policy: SectPolicy::Mercantile,
        signature: "xuantian",
        landmark: "黄金大帐",
        morality: 47,
        members: &[
            "忽必烈",
            "哲别",
            "木华黎",
            "康熙",
            "年羹尧",
            "韦小宝",
            "多隆",
            "海大富",
        ],
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
    SectTemplate {
        id: "song_court",
        name: "枢密院",
        country: "song",
        policy: SectPolicy::Scholarly,
        signature: "wumu",
        landmark: "垂拱殿",
        morality: 68,
        members: &[
            "宋江",
            "卢俊义",
            "花荣",
            "岳飞",
            "郭靖",
            "黄蓉",
            "萧峰",
            "令狐冲",
            "杨过",
            "张无忌",
        ],
    },
];

pub fn generate_npc_world(seed: u64) -> (Vec<SectState>, Vec<Disciple>) {
    let mut rng = StdRng::seed_from_u64(seed);
    let named_npcs = all_named_npcs();
    let mut sects = Vec::with_capacity(SECTS.len());
    let mut disciples = Vec::with_capacity(named_npcs.len() + SECTS.len() * 2);

    for (sect_index, template) in SECTS.iter().enumerate() {
        let prestige = 58 + (sect_index as i32 * 7 % 35);
        let mut buildings = sect_buildings(template.id);
        for building in &mut buildings {
            building.level = 2 + (sect_index as i32 % 3);
        }
        let public_books = std::iter::once(knowledge_skill_id(template.id))
            .chain(
                all_martial_arts()
                    .into_iter()
                    .filter(|art| {
                        art.sect_id.as_deref() == Some(template.id)
                            && art.is_combat
                            && art.category != crate::models::martial_art::SkillCategory::Parry
                    })
                    .map(|art| art.id),
            )
            .collect::<Vec<_>>();
        let mut martial_research = public_books
            .iter()
            .filter_map(|book| {
                let art = martial_art_by_id(book)?;
                art.is_combat.then(|| {
                    let tier_cap = match art.tier {
                        MartialTier::Basic => 70,
                        MartialTier::Chore => 80,
                        MartialTier::Outer => 120,
                        MartialTier::Inner => 180,
                    };
                    (art.id, i64::from(tier_cap + prestige / 2))
                })
            })
            .collect::<BTreeMap<_, _>>();
        martial_research
            .entry(template.signature.into())
            .and_modify(|cap| *cap = (*cap).max(180 + i64::from(prestige)))
            .or_insert(180 + i64::from(prestige));
        let mut sect = SectState {
            id: template.id.into(),
            name: template.name.into(),
            description: sect_description(template.id).into(),
            landmark: template.landmark.into(),
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
            public_books,
            martial_research,
            relations: BTreeMap::new(),
            active_orders: vec![],
            productions: vec![],
            auto_brew_queue: crate::models::medicine::PILL_RECIPES
                .iter()
                .map(|recipe| recipe.id.to_string())
                .collect(),
            auto_brew_index: 0,
            auto_brew_progress: 0,
            created_martial_arts: Vec::new(),
            heritage_arts: Vec::new(),
        };

        let sect_named_npcs: Vec<_> = named_npcs
            .iter()
            .filter(|npc| npc.sect_id.as_deref() == Some(template.id))
            .collect();
        let mut sect_disciple_count = 0;
        if sect_named_npcs.is_empty() {
            // 固定名册未覆盖的新门派仍沿用旧版随机名册，避免生成空门派。
            for (member_index, name) in template.members.iter().enumerate() {
                let mut disciple = generate_disciple(&mut rng, 18 - member_index as i32 * 3);
                disciple.id = format!("npc_{}_{}", template.id, member_index + 1);
                disciple.sect_id = Some(template.id.into());
                disciple.name = (*name).into();
                disciple.martial_art = template.signature.into();
                disciple.rank = match member_index {
                    0 | 1 => DiscipleRank::Inner,
                    _ => DiscipleRank::Outer,
                };
                disciple.department = Some(match member_index {
                    0 => Department::Transmission,
                    1 => Department::ExternalAffairs,
                    _ => Department::Stewardship,
                });
                disciple.attributes.morality = template.morality;
                disciple.attributes.reputation = prestige / 2 + 20 - member_index as i32 * 4;
                disciple.attributes.attainment =
                    800 - member_index as i64 * 180 + prestige as i64 * 4;
                add_permanent_neili(&mut disciple, 35 - member_index as i32 * 8);
                assign_sect_curriculum(&mut disciple, template.id, 320 - member_index as i32 * 70);
                disciple.martial_art = template.signature.into();
                sync_legacy_attributes(&mut disciple);
                disciples.push(disciple);
                sect_disciple_count += 1;
            }
            sect.buildings[0].elder_id = Some(format!("npc_{}_1", template.id));
            sect.buildings[1].elder_id = Some(format!("npc_{}_2", template.id));
        } else {
            for npc in sect_named_npcs {
                let mut disciple = npc.build_disciple();
                match npc.position {
                    NpcPosition::SectLeader => {
                        disciple.attributes.sect_loyalty = 90;
                        sect.buildings[0].elder_id = Some(disciple.id.clone());
                    }
                    NpcPosition::DeputyLeader => {
                        if sect.buildings[1].elder_id.is_none() {
                            sect.buildings[1].elder_id = Some(disciple.id.clone());
                        }
                    }
                    _ => {}
                }
                sync_legacy_attributes(&mut disciple);
                disciples.push(disciple);
                sect_disciple_count += 1;
            }
        }

        let filler_count = (template.members.len().max(5) + 2).saturating_sub(sect_disciple_count);
        for filler_index in 0..filler_count {
            let mut disciple = generate_disciple(&mut rng, 0);
            disciple.id = format!("npc_{}_filler_{}", template.id, filler_index + 1);
            disciple.sect_id = Some(template.id.into());
            disciple.rank = DiscipleRank::Chore;
            disciple.department = Some(Department::Stewardship);
            disciple.attributes.morality = template.morality;
            assign_sect_curriculum(&mut disciple, template.id, 80);
            sync_legacy_attributes(&mut disciple);
            disciples.push(disciple);
        }
        sects.push(sect);
    }

    initialize_relations(&mut sects);
    normalize_npc_master_lineages(&sects, &mut disciples);
    crate::logic::sect::compute_lineage_generations(&mut disciples);
    for npc in named_npcs.iter().filter(|npc| npc.sect_id.is_none()) {
        let mut disciple = npc.build_disciple();
        sync_legacy_attributes(&mut disciple);
        disciples.push(disciple);
    }
    (sects, disciples)
}

fn sect_description(sect_id: &str) -> &'static str {
    match sect_id {
        "court" => "蒙古铁骑与满清朝堂合流之势",
        "song_court" => "总领大宋军政机要，延揽忠义豪杰",
        _ => "江湖中传承已久的一方门派",
    }
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
    } else {
        migrate_courts(state);
    }
    hydrate_countries(state);
    for disciple in &mut state.npc_disciples {
        crate::logic::disciple::hydrate_v2_disciple(disciple);
    }
    for sect in &mut state.npc_sects {
        crate::logic::sect::normalize_buildings(sect);
        crate::logic::sect::normalize_elder_assignments(sect, &state.npc_disciples);
    }
    ensure_relations(&mut state.npc_sects);
    normalize_npc_master_lineages(&state.npc_sects, &mut state.npc_disciples);
}

/// 不重置旧世界的经营进度，只更新两座朝廷模板并补入旧档缺失的枢密院。
fn migrate_courts(state: &mut GameState) {
    let (canonical_sects, canonical_disciples) = generate_npc_world(state.world_seed);
    for court_id in ["court", "song_court"] {
        let Some(canonical) = canonical_sects.iter().find(|sect| sect.id == court_id) else {
            continue;
        };
        let existing_index = state.npc_sects.iter().position(|sect| sect.id == court_id);
        let refresh_roster = if let Some(index) = existing_index {
            let existing = &mut state.npc_sects[index];
            let legacy_identity =
                existing.name != canonical.name || existing.description.is_empty();
            existing.name = canonical.name.clone();
            existing.description = canonical.description.clone();
            existing.landmark = canonical.landmark.clone();
            existing.country_id = canonical.country_id.clone();
            existing.policy = canonical.policy.clone();
            for book in &canonical.public_books {
                if !existing.public_books.contains(book) {
                    existing.public_books.push(book.clone());
                }
            }
            legacy_identity
        } else {
            state.npc_sects.push(canonical.clone());
            true
        };

        // 新模板已经落盘后不再重建名册，以免把后续叛逃的弟子强行召回原派。
        if !refresh_roster {
            continue;
        }

        for canonical_member in canonical_disciples
            .iter()
            .filter(|disciple| disciple.sect_id.as_deref() == Some(court_id))
        {
            if state
                .disciples
                .iter()
                .any(|disciple| disciple.id == canonical_member.id)
            {
                continue;
            }
            if let Some(existing) = state
                .npc_disciples
                .iter_mut()
                .find(|disciple| disciple.id == canonical_member.id)
            {
                existing.name = canonical_member.name.clone();
                existing.sect_id = Some(court_id.into());
                existing.origin_sect_id = Some(court_id.into());
            } else {
                state.npc_disciples.push(canonical_member.clone());
            }
        }
    }
}

fn hydrate_countries(state: &mut GameState) {
    country::normalize_countries(&mut state.countries);
}

fn ensure_relations(sects: &mut [SectState]) {
    let snapshot: Vec<(String, i32)> = sects
        .iter()
        .map(|sect| (sect.id.clone(), sect.attributes.morality))
        .collect();
    for sect in sects {
        for (other_id, morality) in &snapshot {
            if other_id == &sect.id || sect.relations.contains_key(other_id) {
                continue;
            }
            let relation = 30 - (sect.attributes.morality - morality).abs();
            sect.relations
                .insert(other_id.clone(), relation.clamp(-80, 60));
        }
    }
}

const MAX_ACTIVE_STUDENTS_PER_MASTER: usize = 5;
const MIN_MASTER_RELATION: i32 = 50;

/// 为 NPC 门派补齐稳定的师承谱系。
///
/// 旧档中仍然合法且未超过五名在籍弟子的师承会原样保留；失效引用清理后，
/// 外门优先从在世内门中择师。掌门、在任长老排在最前，其余人再按功绩、
/// 战力和稳定 ID 排序，因此同一份世界数据反复水合不会改变结果。
pub(crate) fn normalize_npc_master_lineages(sects: &[SectState], disciples: &mut [Disciple]) {
    for sect in sects {
        for disciple in disciples.iter_mut().filter(|disciple| {
            disciple.sect_id.as_deref() == Some(sect.id.as_str())
                && (!disciple.alive || disciple.rank == DiscipleRank::Chore)
        }) {
            disciple.master_id = None;
        }
        let mut member_indices: Vec<usize> = disciples
            .iter()
            .enumerate()
            .filter(|(_, disciple)| {
                disciple.alive && disciple.sect_id.as_deref() == Some(sect.id.as_str())
            })
            .map(|(index, _)| index)
            .collect();
        member_indices.sort_by(|left, right| disciples[*left].id.cmp(&disciples[*right].id));

        let elder_ids: std::collections::BTreeSet<&str> = sect
            .buildings
            .iter()
            .filter_map(|building| building.elder_id.as_deref())
            .collect();
        let legacy_leader_id = format!("npc_{}_1", sect.id);
        let role_priority = |disciple: &Disciple| {
            if disciple.npc_position.as_deref() == Some(NpcPosition::SectLeader.display())
                || disciple.id == legacy_leader_id
            {
                0
            } else if elder_ids.contains(disciple.id.as_str())
                || matches!(disciple.npc_position.as_deref(), Some("副掌门" | "长老"))
            {
                1
            } else {
                2
            }
        };

        let mut master_indices: Vec<usize> = member_indices
            .iter()
            .copied()
            .filter(|index| disciples[*index].rank == DiscipleRank::Inner)
            .collect();
        master_indices.sort_by(|left, right| {
            role_priority(&disciples[*left])
                .cmp(&role_priority(&disciples[*right]))
                .then_with(|| disciples[*right].merit.cmp(&disciples[*left].merit))
                .then_with(|| {
                    crate::logic::disciple::get_combat_score(&disciples[*right])
                        .cmp(&crate::logic::disciple::get_combat_score(&disciples[*left]))
                })
                .then_with(|| disciples[*left].id.cmp(&disciples[*right].id))
        });
        let master_by_id: BTreeMap<String, usize> = master_indices
            .iter()
            .map(|index| (disciples[*index].id.clone(), *index))
            .collect();
        let master_order: BTreeMap<String, usize> = master_indices
            .iter()
            .enumerate()
            .map(|(order, index)| (disciples[*index].id.clone(), order))
            .collect();
        let mut master_load = BTreeMap::<String, usize>::new();
        let mut accepted_assignments = BTreeMap::<String, String>::new();
        let mut relationship_pairs = Vec::<(usize, usize)>::new();

        // 先保留合法旧师承；以弟子稳定 ID 排序，让异常旧档超额时的取舍可复现。
        for apprentice_index in member_indices.iter().copied() {
            let apprentice_id = disciples[apprentice_index].id.clone();
            let Some(master_id) = disciples[apprentice_index].master_id.clone() else {
                continue;
            };
            let Some(master_index) = master_by_id.get(&master_id).copied() else {
                disciples[apprentice_index].master_id = None;
                continue;
            };
            if master_index == apprentice_index {
                disciples[apprentice_index].master_id = None;
                continue;
            }
            if would_close_master_cycle(&accepted_assignments, &apprentice_id, &master_id) {
                disciples[apprentice_index].master_id = None;
                continue;
            }
            let load = master_load.entry(master_id).or_default();
            if *load >= MAX_ACTIVE_STUDENTS_PER_MASTER {
                disciples[apprentice_index].master_id = None;
                continue;
            }
            *load += 1;
            accepted_assignments.insert(apprentice_id, disciples[master_index].id.clone());
            relationship_pairs.push((apprentice_index, master_index));
        }

        // 外门最需要稳定授业，先于其他尚无师承的内门占用师资名额。
        let mut apprentice_indices: Vec<usize> = member_indices
            .iter()
            .copied()
            .filter(|index| {
                disciples[*index].rank != DiscipleRank::Chore
                    && disciples[*index].master_id.is_none()
            })
            .collect();
        apprentice_indices.sort_by(|left, right| {
            let rank_priority = |rank: &DiscipleRank| match rank {
                DiscipleRank::Outer => 0,
                DiscipleRank::Inner => 1,
                DiscipleRank::Chore => 2,
            };
            rank_priority(&disciples[*left].rank)
                .cmp(&rank_priority(&disciples[*right].rank))
                .then_with(|| disciples[*left].id.cmp(&disciples[*right].id))
        });

        for apprentice_index in apprentice_indices {
            let apprentice_id = disciples[apprentice_index].id.clone();
            let apprentice_order = master_order.get(&apprentice_id).copied();
            let chosen_master = master_indices.iter().copied().find(|master_index| {
                if *master_index == apprentice_index {
                    return false;
                }
                let master_id = &disciples[*master_index].id;
                if master_load.get(master_id).copied().unwrap_or(0)
                    >= MAX_ACTIVE_STUDENTS_PER_MASTER
                {
                    return false;
                }
                if would_close_master_cycle(&accepted_assignments, &apprentice_id, master_id) {
                    return false;
                }
                // 内门只拜排序更前的前辈为师，避免新补谱系形成环。
                apprentice_order.is_none_or(|order| {
                    master_order
                        .get(master_id)
                        .is_some_and(|master_order| *master_order < order)
                })
            });
            let Some(master_index) = chosen_master else {
                continue;
            };
            let master_id = disciples[master_index].id.clone();
            disciples[apprentice_index].master_id = Some(master_id.clone());
            *master_load.entry(master_id.clone()).or_default() += 1;
            accepted_assignments.insert(apprentice_id, master_id);
            relationship_pairs.push((apprentice_index, master_index));
        }

        for (apprentice_index, master_index) in relationship_pairs {
            ensure_mutual_disciple_relation(
                disciples,
                apprentice_index,
                master_index,
                MIN_MASTER_RELATION,
            );
        }
    }
}

fn would_close_master_cycle(
    accepted_assignments: &BTreeMap<String, String>,
    apprentice_id: &str,
    master_id: &str,
) -> bool {
    let mut current = master_id.to_string();
    let mut visited = std::collections::BTreeSet::new();
    loop {
        if current == apprentice_id {
            return true;
        }
        if !visited.insert(current.clone()) {
            return true;
        }
        let Some(next) = accepted_assignments.get(&current) else {
            return false;
        };
        current = next.clone();
    }
}

fn ensure_mutual_disciple_relation(
    disciples: &mut [Disciple],
    left_index: usize,
    right_index: usize,
    minimum: i32,
) {
    if left_index == right_index {
        return;
    }
    let (left, right) = if left_index < right_index {
        let (before, after) = disciples.split_at_mut(right_index);
        (&mut before[left_index], &mut after[0])
    } else {
        let (before, after) = disciples.split_at_mut(left_index);
        (&mut after[0], &mut before[right_index])
    };
    left.relations
        .entry(right.id.clone())
        .and_modify(|relation| *relation = (*relation).max(minimum))
        .or_insert(minimum);
    right
        .relations
        .entry(left.id.clone())
        .and_modify(|relation| *relation = (*relation).max(minimum))
        .or_insert(minimum);
}

fn chronicle_figure_candidates<'a>(
    sect: &SectState,
    disciples: &'a [Disciple],
) -> Vec<(&'a Disciple, &'static str)> {
    let legacy_leader_id = format!("npc_{}_1", sect.id);
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
                && (disciple.id == legacy_leader_id || elder_ids.contains(disciple.id.as_str()))
        })
        .map(|disciple| {
            let role = if disciple.id == legacy_leader_id
                || disciple.npc_position.as_deref() == Some(NpcPosition::SectLeader.display())
            {
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

fn named_npc_sect_name(state: &GameState, npc: &Disciple) -> String {
    let Some(sect_id) = npc.sect_id.as_deref() else {
        return "江湖".into();
    };
    state
        .npc_sects
        .iter()
        .find(|sect| sect.id == sect_id)
        .map(|sect| sect.name.clone())
        .or_else(|| {
            SECTS
                .iter()
                .find(|template| template.id == sect_id)
                .map(|template| template.name.into())
        })
        .unwrap_or_else(|| sect_id.into())
}

fn named_npc_personal_chronicle(rng: &mut impl Rng, state: &GameState, npc: &Disciple) -> String {
    let name = npc.name.as_str();
    let sect_name = named_npc_sect_name(state, npc);
    let is_leader = npc.npc_position.as_deref() == Some(NpcPosition::SectLeader.display());
    if is_leader {
        match npc.sect_id.as_deref() {
            Some("wudang") => {
                return "武当张三丰于紫霄宫中闭关参悟太极，山间云气翻涌三日不散。".into()
            }
            Some("shaolin") => {
                return format!(
                    "{}方丈{}升座说法，少林众僧齐诵经文，钟声远传数十里。",
                    sect_name, name
                )
            }
            Some("huashan") => {
                return format!(
                    "华山掌门{}在思过崖前考校弟子剑法，山风猎猎中剑气纵横。",
                    name
                )
            }
            Some("mingjiao") => {
                return "明教教主张无忌在光明顶上为教众演示乾坤大挪移，满堂喝彩。".into()
            }
            Some("gaibang") => {
                return format!("丐帮帮主{}在忠义堂召集群丐议事，破碗中酒香四溢。", name)
            }
            Some("gumu") => {
                return format!("古墓派{}在寒玉室中修行玉女心经，玉床寒气令洞壁凝霜。", name)
            }
            Some("emei") => {
                return format!("峨嵋掌门{}率众弟子于清音阁前练剑，剑风落处松涛如啸。", name)
            }
            Some("riyue") => {
                return format!(
                    "日月神教{}教主在黑木崖上观览教务，一纸令下便定数百人生死。",
                    name
                )
            }
            Some("xingxiu") => {
                return format!(
                    "星宿派{}在星宿海畔炼制毒药，浓烟滚滚中弟子们远远叩拜。",
                    name
                )
            }
            Some("taohua") => {
                return format!(
                    "桃花岛主{}在试剑亭中抚琴，琴声穿过桃林引得海鸥盘旋不去。",
                    name
                )
            }
            Some("murong") => {
                return format!(
                    "姑苏慕容{}在还施水阁中翻阅前朝典籍，自光复大燕之志日夜不忘。",
                    name
                )
            }
            Some("dalun") => {
                return format!(
                    "大轮寺{}在大经堂中传授龙象般若功，众弟子盘膝而坐凝心受教。",
                    name
                )
            }
            Some("court") => {
                return format!(
                    "金帐大汗{}于黄金大帐中宴请诸王，酒过三巡便议起南下军务。",
                    name
                )
            }
            Some("song_court") => {
                return format!("枢密院{}在垂拱殿中批阅边关战报，烛火通明直至鸡鸣。", name)
            }
            Some("lingjiu") => {
                return format!("灵鹫宫主{}缥缈峰上训诫各部，威仪令九天九部噤若寒蝉。", name)
            }
            Some("baituo") => {
                return format!(
                    "白驼山主{}在山巅驱蛇演练阵法，毒蛇蜿蜒间自成一派奇诡气象。",
                    name
                )
            }
            Some("jueqing") => {
                return format!(
                    "绝情谷主{}在厉鬼峰前孤坐半日，面色阴晴不定不知想起何事。",
                    name
                )
            }
            _ => {}
        }
    }

    match rng.gen_range(0..3) {
        0 => format!(
            "{}长老{}在演武场点拨弟子，剑光霍霍引得满山鹤唳。",
            sect_name, name
        ),
        1 => format!(
            "{}长老{}闭关参禅，据闻已近彻悟之境，连日不语。",
            sect_name, name
        ),
        _ => format!(
            "{}真人{}率弟子登坛诵经，重阳宫中香烟缭绕。",
            sect_name, name
        ),
    }
}

fn named_npc_linked_chronicle(
    rng: &mut impl Rng,
    state: &GameState,
    source: &Disciple,
    target: &Disciple,
) -> String {
    let source_sect = named_npc_sect_name(state, source);
    let target_sect = named_npc_sect_name(state, target);
    match rng.gen_range(0..4) {
        0 => format!(
            "{}掌门{}遣使前往{}，与{}长老{}互通音讯。",
            source_sect, source.name, target_sect, target_sect, target.name
        ),
        1 => format!(
            "{}弟子{}途经{}，与{}掌门{}于山门外偶遇，二人把酒言欢。",
            source_sect, source.name, target_sect, target_sect, target.name
        ),
        2 => format!(
            "{}掌门{}收到{}的{}一封书信，阅后沉吟良久。",
            source_sect, source.name, target_sect, target.name
        ),
        _ => format!(
            "有传言称{}的{}与{}的{}近来往来甚密，江湖中人议论纷纷。",
            source_sect, source.name, target_sect, target.name
        ),
    }
}

/// 为存活的小说人物生成个人江湖纪事，并偶尔追加一则跨门派联动。
pub fn generate_named_npc_chronicles(rng: &mut impl Rng, state: &GameState) -> Vec<GameEvent> {
    let mut candidates: Vec<&Disciple> = state
        .npc_disciples
        .iter()
        .filter(|npc| npc.is_named_npc && npc.alive)
        .collect();
    if candidates.is_empty() {
        return vec![];
    }

    candidates.shuffle(rng);
    let selected_count = rng.gen_range(1..=3.min(candidates.len()));
    let selected = candidates[..selected_count].to_vec();
    let mut events = Vec::with_capacity(selected_count * 2);
    for npc in selected {
        events.push(GameEvent {
            text: named_npc_personal_chronicle(rng, state, npc),
            mood: "neutral".into(),
            year: state.year,
            month: state.month,
            category: "world".into(),
        });

        if !rng.gen_bool(0.35) {
            continue;
        }
        let linked_candidates: Vec<&Disciple> = candidates
            .iter()
            .copied()
            .filter(|other| other.id != npc.id && other.sect_id != npc.sect_id)
            .collect();
        let Some(target) = linked_candidates.choose(rng).copied() else {
            continue;
        };
        events.push(GameEvent {
            text: named_npc_linked_chronicle(rng, state, npc, target),
            mood: "neutral".into(),
            year: state.year,
            month: state.month,
            category: "world".into(),
        });
    }
    events
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
        let country_id = state.npc_sects[sect_index].country_id.clone();
        let (prosperity, order) = country::country_values(state, &country_id);
        let recruit_permille = npc_recruit_probability_permille(prosperity, order, urgent);
        let mut notes = Vec::new();
        if rng.gen_bool(recruit_permille as f64 / 1000.0)
            && state.npc_sects[sect_index].attributes.silver >= 35
        {
            state.npc_sects[sect_index].attributes.silver -= 35;
            let recruit_id = next_npc_recruit_id(state, &sect_id);
            let prestige_bonus = state.npc_sects[sect_index].attributes.prestige / 15;
            let mut recruit =
                generate_disciple(rng, prestige_bonus.max(2));
            // 通用弟子生成器的临时 ID 带墙钟时间；NPC 月结须改为存档内可复现的序号。
            recruit.id = recruit_id;
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

    // NPC 门派深化：武学研究、丹药炼制与建筑维护
    deepen_npc_sect_management(rng, state);
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

/// 国势只改变 NPC 开山纳新的意愿；成本、名额和长老补贴仍沿用门派规则。
fn npc_recruit_probability_permille(prosperity: i32, order: i32, urgent: bool) -> u32 {
    let prosperity = prosperity.clamp(0, 100);
    let order = order.clamp(0, 100);
    // 繁荣度和治安共同决定人口吸引力，紧急时大幅放宽
    let adjustment = (prosperity - 70) * 2 + (order - 65);
    if urgent {
        (820 + adjustment).clamp(650, 950) as u32
    } else {
        (80 + adjustment).clamp(30, 200) as u32
    }
}

fn next_npc_recruit_id(state: &GameState, sect_id: &str) -> String {
    let prefix = format!("npc_recruit_{}_{}_{}", sect_id, state.year, state.month);
    let mut serial = state
        .npc_disciples
        .iter()
        .filter(|disciple| disciple.id.starts_with(&prefix))
        .count();
    loop {
        let candidate = format!("{prefix}_{serial}");
        if !state
            .npc_disciples
            .iter()
            .any(|disciple| disciple.id == candidate)
        {
            return candidate;
        }
        serial += 1;
    }
}

/// NPC 门派每月自动推进武学研究、丹药炼制与建筑维护。
fn deepen_npc_sect_management(rng: &mut impl Rng, state: &mut GameState) {
    // 先快照所有需要的不可变数据
    struct SectBuildSnapshot {
        has_scripture_elder: bool,
        has_herb_elder: bool,
        herbs: i32,
        scripture_eff: i32,
        first_combat_art: Option<String>,
        silver: i32,
        iron: i32,
        building_conditions: Vec<(usize, i32)>, // (index, condition)
    }
    let snapshots: Vec<SectBuildSnapshot> = state.npc_sects.iter().map(|sect| {
        let has_scripture_elder = sect.buildings.iter()
            .any(|b| b.id == "scripture" && b.elder_id.is_some() && b.condition > 0);
        let has_herb_elder = sect.buildings.iter()
            .any(|b| b.id == "herb_hall" && b.elder_id.is_some() && b.condition > 0);
        let herbs = *sect.inventory.get("草药").unwrap_or(&0);
        let scripture_eff = crate::logic::sect::building_effectiveness(sect, "scripture");
        let first_combat_art = sect.public_books.iter()
            .filter_map(|id| {
                let art = crate::models::martial_art::martial_art_by_id(id)?;
                art.is_combat.then_some(id.clone())
            })
            .next();
        let silver = sect.attributes.silver;
        let iron = *sect.inventory.get("精铁").unwrap_or(&0);
        let building_conditions = sect.buildings.iter()
            .enumerate()
            .map(|(i, b)| (i, b.condition))
            .collect();
        SectBuildSnapshot {
            has_scripture_elder, has_herb_elder, herbs, scripture_eff,
            first_combat_art, silver, iron, building_conditions,
        }
    }).collect();

    // 再根据快照执行修改
    for (sect_index, snap) in snapshots.iter().enumerate() {
        // 经文研究
        if snap.has_scripture_elder && snap.scripture_eff > 0 {
            if let Some(ref art_id) = snap.first_combat_art {
                let gain = 1_i64
                    + (snap.scripture_eff / 30).max(0) as i64
                    + i64::from(rng.gen_bool(0.3));
                *state.npc_sects[sect_index]
                    .martial_research
                    .entry(art_id.clone())
                    .or_insert(50) += gain;
            }
        }

        // 丹药炼制：有丹房长老时每月至少炼一炉；草药充裕时可炼多炉。
        let herb_stock = *state.npc_sects[sect_index].inventory.get("草药").unwrap_or(&0);
        let brew_count = if snap.has_herb_elder && herb_stock >= 8 {
            ((herb_stock / 16) + 1).clamp(1, ((herb_stock / 8).min(3)) as i32)
        } else {
            0
        };
        for _ in 0..brew_count {
            let current = *state.npc_sects[sect_index].inventory.get("草药").unwrap_or(&0);
            if current < 8 {
                break;
            }
            state.npc_sects[sect_index]
                .inventory
                .entry("草药".into())
                .and_modify(|h| *h = (*h - 8).max(0));
            let med = if rng.gen_bool(0.5) {
                crate::models::medicine::Medicine::Wound
            } else {
                crate::models::medicine::Medicine::Qi
            };
            *state.npc_sects[sect_index]
                .inventory
                .entry(med.name().into())
                .or_default() += 1;
        }

        // 建筑修缮与磨损：先预判每栋建筑的修缮操作
        let repairs: Vec<(usize, bool, bool, i32)> = snap.building_conditions.iter().map(
            |&(b_idx, condition)| {
                let can_repair = condition < 50 && snap.silver >= 15
                    && snap.iron >= (100 - condition + 24) / 25;
                let will_wear = condition > 0 && rng.gen_bool(0.3);
                let iron_cost = if can_repair { (100 - condition + 24) / 25 } else { 0 };
                (b_idx, can_repair, will_wear, iron_cost)
            }
        ).collect();

        for (b_idx, can_repair, will_wear, iron_cost) in repairs {
            if can_repair {
                *state.npc_sects[sect_index].inventory.entry("精铁".into()).or_default() -= iron_cost;
                state.npc_sects[sect_index].attributes.silver -= 15;
                state.npc_sects[sect_index].buildings[b_idx].condition = 100;
            }
            if will_wear {
                let cond = &mut state.npc_sects[sect_index].buildings[b_idx].condition;
                *cond = (*cond - 1).max(0);
            }
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::martial_art::{base_skill_ids, sect_combat_arts, MartialTier};

    #[test]
    fn world_contains_all_required_factions_and_people() {
        let (sects, disciples) = generate_npc_world(42);
        assert_eq!(sects.len(), 24);
        let court = sects.iter().find(|sect| sect.id == "court").unwrap();
        assert_eq!(court.name, "金帐汗国");
        assert_eq!(court.description, "蒙古铁骑与满清朝堂合流之势");
        let song_court = sects.iter().find(|sect| sect.id == "song_court").unwrap();
        assert_eq!(song_court.name, "枢密院");
        assert_eq!(song_court.landmark, "垂拱殿");
        let court_members: std::collections::BTreeSet<&str> = disciples
            .iter()
            .filter(|disciple| {
                disciple.sect_id.as_deref() == Some("court") && disciple.is_named_npc
            })
            .map(|disciple| disciple.name.as_str())
            .collect();
        assert_eq!(
            court_members,
            [
                "忽必烈",
                "哲别",
                "木华黎",
                "康熙",
                "年羹尧",
                "韦小宝",
                "多隆",
                "海大富",
            ]
            .into_iter()
            .collect()
        );
        let song_court_members: std::collections::BTreeSet<&str> = disciples
            .iter()
            .filter(|disciple| {
                disciple.sect_id.as_deref() == Some("song_court") && disciple.is_named_npc
            })
            .map(|disciple| disciple.name.as_str())
            .collect();
        assert_eq!(
            song_court_members,
            [
                "宋江",
                "卢俊义",
                "花荣",
                "岳飞",
                "郭靖",
                "黄蓉",
                "萧峰",
                "令狐冲",
                "杨过",
                "张无忌",
            ]
            .into_iter()
            .collect()
        );
        assert!(sects.iter().all(|sect| sect.buildings.len() >= 7));
        assert!(sects.iter().all(|sect| sect.public_books.len() >= 2));
        for sect in &sects {
            let members: Vec<_> = disciples
                .iter()
                .filter(|d| d.sect_id.as_deref() == Some(sect.id.as_str()))
                .collect();
            let leader = members
                .iter()
                .find(|d| d.npc_position.as_deref() == Some(NpcPosition::SectLeader.display()))
                .unwrap();
            assert_eq!(leader.loyalty, 90);
            assert_eq!(
                sect.buildings[0].elder_id.as_deref(),
                Some(leader.id.as_str())
            );
            if let Some(deputy) = members
                .iter()
                .find(|d| d.npc_position.as_deref() == Some(NpcPosition::DeputyLeader.display()))
            {
                assert_eq!(
                    sect.buildings[1].elder_id.as_deref(),
                    Some(deputy.id.as_str())
                );
            }
            assert!(members
                .iter()
                .filter(|d| !d.is_named_npc)
                .all(|d| d.rank == DiscipleRank::Chore));
        }
        assert_eq!(
            disciples
                .iter()
                .filter(|d| d.sect_id.is_none() && d.is_named_npc)
                .count(),
            7
        );
        assert!(disciples
            .iter()
            .filter(|d| d.sect_id.is_none())
            .all(|d| d.is_named_npc));
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
    fn legacy_world_gains_new_court_without_losing_progress() {
        let mut state = GameState::default();
        let (mut sects, mut disciples) = generate_npc_world(17);
        sects.retain(|sect| sect.id != "song_court");
        let court = sects.iter_mut().find(|sect| sect.id == "court").unwrap();
        court.name = "朝廷".into();
        court.attributes.silver = 4321;
        disciples
            .iter_mut()
            .find(|disciple| disciple.id == "npc_court_hubilie")
            .unwrap()
            .martial_progress
            .proficiencies
            .get_mut("xuantian")
            .unwrap()
            .level = 777;
        state.npc_sects = sects;
        state.npc_disciples = disciples
            .into_iter()
            .filter(|disciple| disciple.sect_id.as_deref() != Some("song_court"))
            .collect();
        state.countries[0].name = "大元".into();

        hydrate_world(&mut state);

        let court = state
            .npc_sects
            .iter()
            .find(|sect| sect.id == "court")
            .unwrap();
        assert_eq!(court.name, "金帐汗国");
        assert_eq!(court.attributes.silver, 4321);
        assert_eq!(
            state
                .npc_disciples
                .iter()
                .find(|disciple| disciple.id == "npc_court_hubilie")
                .unwrap()
                .martial_progress
                .proficiencies["xuantian"]
                .level,
            777
        );
        assert!(state.npc_sects.iter().any(|sect| sect.id == "song_court"));
        assert_eq!(
            state
                .npc_disciples
                .iter()
                .filter(|disciple| disciple.sect_id.as_deref() == Some("song_court"))
                .count(),
            12
        );
        assert_eq!(state.countries[0].name, "大元");
    }

    #[test]
    fn legacy_country_hydration_preserves_progress_fills_missing_and_clamps() {
        let mut state = GameState {
            countries: vec![
                crate::models::sect::Country {
                    id: "song".into(),
                    name: "旧宋".into(),
                    prosperity: 88,
                    order: 61,
                    population: 11200,
                },
                crate::models::sect::Country {
                    id: "yuan".into(),
                    name: "旧元".into(),
                    prosperity: 130,
                    order: -9,
                    population: 7800,
                },
            ],
            ..GameState::default()
        };

        hydrate_countries(&mut state);

        assert_eq!(state.countries.len(), 4);
        let song = country::find_country(&state.countries, "song").unwrap();
        assert_eq!(
            (song.name.as_str(), song.prosperity, song.order),
            ("大宋", 88, 61)
        );
        let yuan = country::find_country(&state.countries, "yuan").unwrap();
        assert_eq!(
            (yuan.name.as_str(), yuan.prosperity, yuan.order),
            ("大元", 100, 0)
        );
        assert!(country::find_country(&state.countries, "dali").is_some());
        assert!(country::find_country(&state.countries, "xia").is_some());
    }

    #[test]
    fn npc_recruit_probability_obeys_country_bounds_and_urgency() {
        assert_eq!(npc_recruit_probability_permille(70, 65, false), 80);
        assert_eq!(npc_recruit_probability_permille(0, 0, false), 30);
        assert_eq!(npc_recruit_probability_permille(100, 100, false), 175);
        assert_eq!(npc_recruit_probability_permille(70, 65, true), 820);
        assert_eq!(npc_recruit_probability_permille(0, 0, true), 650);
        assert_eq!(npc_recruit_probability_permille(100, 100, true), 915);
        assert_eq!(npc_recruit_probability_permille(-999, 999, false), 30);
        assert_eq!(npc_recruit_probability_permille(999, -999, true), 815);
    }

    #[test]
    fn npc_ai_is_deterministic_for_a_fixed_seed_and_country_snapshot() {
        let (sects, disciples) = generate_npc_world(97);
        let state = GameState {
            npc_sects: sects,
            npc_disciples: disciples,
            ..GameState::default()
        };
        let mut left = state.clone();
        let mut right = state;
        let mut left_rng = StdRng::seed_from_u64(20260719);
        let mut right_rng = StdRng::seed_from_u64(20260719);

        let left_events = run_npc_ai(&mut left_rng, &mut left);
        let right_events = run_npc_ai(&mut right_rng, &mut right);

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

    #[test]
    fn seeded_world_is_stable() {
        let (sects_a, disciples_a) = generate_npc_world(9);
        let (sects_b, disciples_b) = generate_npc_world(9);
        assert_eq!(sects_a[7].attributes.silver, sects_b[7].attributes.silver);
        let lineage_snapshot = |disciples: &[Disciple]| {
            disciples
                .iter()
                .filter(|disciple| disciple.sect_id.is_some())
                .map(|disciple| {
                    (
                        disciple.id.clone(),
                        disciple.master_id.clone(),
                        disciple.relations.clone(),
                    )
                })
                .collect::<Vec<_>>()
        };
        assert_eq!(
            lineage_snapshot(&disciples_a),
            lineage_snapshot(&disciples_b)
        );
    }

    #[test]
    fn all_npc_outer_disciples_have_valid_local_master_lineages() {
        let (sects, disciples) = generate_npc_world(73);
        assert_eq!(sects.len(), 24);
        let mut sects_with_teachers = 0;
        let mut outer_count = 0;

        for sect in &sects {
            let teachers: BTreeMap<&str, &Disciple> = disciples
                .iter()
                .filter(|disciple| {
                    disciple.alive
                        && disciple.rank == DiscipleRank::Inner
                        && disciple.sect_id.as_deref() == Some(sect.id.as_str())
                })
                .map(|disciple| (disciple.id.as_str(), disciple))
                .collect();
            if teachers.is_empty() {
                continue;
            }
            sects_with_teachers += 1;
            let mut loads = BTreeMap::<&str, usize>::new();
            for apprentice in disciples.iter().filter(|disciple| {
                disciple.alive && disciple.sect_id.as_deref() == Some(sect.id.as_str())
            }) {
                if apprentice.rank == DiscipleRank::Chore {
                    assert!(
                        apprentice.master_id.is_none(),
                        "{}的杂役{}不应有固定师承",
                        sect.name,
                        apprentice.name
                    );
                }
                if apprentice.rank == DiscipleRank::Outer {
                    outer_count += 1;
                    assert!(
                        apprentice.master_id.is_some(),
                        "{}的外门弟子{}没有师父",
                        sect.name,
                        apprentice.name
                    );
                }
                let Some(master_id) = apprentice.master_id.as_deref() else {
                    continue;
                };
                let master = teachers.get(master_id).unwrap_or_else(|| {
                    panic!(
                        "{}的弟子{}拜了无效或非内门师父{}",
                        sect.name, apprentice.name, master_id
                    )
                });
                assert_ne!(apprentice.id, master.id);
                assert_eq!(master.sect_id, apprentice.sect_id);
                assert!(
                    apprentice.relations.get(master_id).copied().unwrap_or(0)
                        >= MIN_MASTER_RELATION
                );
                assert!(
                    master.relations.get(&apprentice.id).copied().unwrap_or(0)
                        >= MIN_MASTER_RELATION
                );
                *loads.entry(master_id).or_default() += 1;
            }
            assert!(
                loads
                    .values()
                    .all(|load| *load <= MAX_ACTIVE_STUDENTS_PER_MASTER),
                "{}存在超过五名在籍弟子的师父",
                sect.name
            );
        }

        assert_eq!(sects_with_teachers, 24);
        assert!(outer_count > 0);
    }

    #[test]
    fn legacy_world_hydration_preserves_valid_masters_and_fills_missing_ones() {
        let seed = 81;
        let (sects, mut disciples) = generate_npc_world(seed);
        let missing_index = disciples
            .iter()
            .enumerate()
            .find(|(_, disciple)| {
                disciple.alive
                    && disciple.rank == DiscipleRank::Outer
                    && disciple.master_id.is_some()
            })
            .map(|(index, _)| index)
            .unwrap();
        let preserved_index = disciples
            .iter()
            .enumerate()
            .find(|(index, disciple)| {
                *index != missing_index
                    && disciple.alive
                    && disciple.rank != DiscipleRank::Chore
                    && disciple.master_id.is_some()
            })
            .map(|(index, _)| index)
            .unwrap();
        let preserved_id = disciples[preserved_index].id.clone();
        let missing_id = disciples[missing_index].id.clone();
        let preserved_master_id = disciples[preserved_index].master_id.clone().unwrap();
        let preserved_master_index = disciples
            .iter()
            .position(|disciple| disciple.id == preserved_master_id)
            .unwrap();
        disciples[preserved_index]
            .relations
            .insert(preserved_master_id.clone(), -20);
        disciples[preserved_master_index]
            .relations
            .insert(preserved_id.clone(), 7);
        disciples[missing_index].master_id = None;

        let mut state = GameState {
            world_seed: seed,
            npc_sects: sects,
            npc_disciples: disciples,
            ..GameState::default()
        };
        hydrate_world(&mut state);

        let preserved = state
            .npc_disciples
            .iter()
            .find(|disciple| disciple.id == preserved_id)
            .unwrap();
        assert_eq!(
            preserved.master_id.as_deref(),
            Some(preserved_master_id.as_str())
        );
        let preserved_master = state
            .npc_disciples
            .iter()
            .find(|disciple| disciple.id == preserved_master_id)
            .unwrap();
        assert!(
            preserved
                .relations
                .get(&preserved_master.id)
                .copied()
                .unwrap_or(0)
                >= MIN_MASTER_RELATION
        );
        assert!(
            preserved_master
                .relations
                .get(&preserved.id)
                .copied()
                .unwrap_or(0)
                >= MIN_MASTER_RELATION
        );

        let filled = state
            .npc_disciples
            .iter()
            .find(|disciple| disciple.id == missing_id)
            .unwrap();
        let filled_master_id = filled.master_id.as_deref().unwrap();
        let filled_master = state
            .npc_disciples
            .iter()
            .find(|disciple| disciple.id == filled_master_id)
            .unwrap();
        assert_eq!(filled_master.rank, DiscipleRank::Inner);
        assert_eq!(filled_master.sect_id, filled.sect_id);
        assert_ne!(filled_master.id, filled.id);
    }

    #[test]
    fn legacy_master_cycles_are_broken_in_stable_id_order() {
        let seed = 91;
        let (sects, mut disciples) = generate_npc_world(seed);
        let mut cycle_indices: Vec<usize> = disciples
            .iter()
            .enumerate()
            .filter(|(_, disciple)| {
                disciple.alive
                    && disciple.rank == DiscipleRank::Inner
                    && disciple.sect_id.as_deref() == Some("wudang")
            })
            .map(|(index, _)| index)
            .collect();
        cycle_indices.sort_by(|left, right| disciples[*left].id.cmp(&disciples[*right].id));
        cycle_indices.truncate(3);
        assert_eq!(cycle_indices.len(), 3);

        for disciple in disciples
            .iter_mut()
            .filter(|disciple| disciple.alive && disciple.sect_id.as_deref() == Some("wudang"))
        {
            disciple.master_id = None;
        }
        let cycle_ids: Vec<String> = cycle_indices
            .iter()
            .map(|index| disciples[*index].id.clone())
            .collect();
        disciples[cycle_indices[0]].master_id = Some(cycle_ids[1].clone());
        disciples[cycle_indices[1]].master_id = Some(cycle_ids[2].clone());
        disciples[cycle_indices[2]].master_id = Some(cycle_ids[0].clone());

        let state = GameState {
            world_seed: seed,
            npc_sects: sects,
            npc_disciples: disciples,
            ..GameState::default()
        };
        let mut first = state.clone();
        let mut second = state;
        hydrate_world(&mut first);
        hydrate_world(&mut second);

        let lineage_snapshot = |state: &GameState| {
            state
                .npc_disciples
                .iter()
                .filter(|disciple| disciple.sect_id.as_deref() == Some("wudang"))
                .map(|disciple| (disciple.id.clone(), disciple.master_id.clone()))
                .collect::<BTreeMap<_, _>>()
        };
        let first_lineage = lineage_snapshot(&first);
        assert_eq!(first_lineage, lineage_snapshot(&second));
        assert_eq!(
            first_lineage.get(&cycle_ids[0]).and_then(Option::as_ref),
            Some(&cycle_ids[1])
        );
        assert_eq!(
            first_lineage.get(&cycle_ids[1]).and_then(Option::as_ref),
            Some(&cycle_ids[2])
        );

        for start_id in first_lineage.keys() {
            let mut current = start_id;
            let mut visited = std::collections::BTreeSet::new();
            while let Some(Some(master_id)) = first_lineage.get(current) {
                assert!(
                    visited.insert(current.clone()),
                    "水合后仍存在以{start_id}为起点的师承环"
                );
                current = master_id;
            }
        }
    }

    #[test]
    fn understaffed_npc_sects_tend_to_recruit_and_follow_rank_rules() {
        let mut state = GameState::default();
        let (sects, disciples) = generate_npc_world(19);
        state.npc_sects = sects;
        state.npc_disciples = disciples;
        let sect_id = "qingcheng";
        let before = state
            .npc_disciples
            .iter()
            .filter(|disciple| disciple.sect_id.as_deref() == Some(sect_id))
            .count();
        assert!(state.npc_disciples.iter().any(|disciple| {
            disciple.sect_id.as_deref() == Some(sect_id) && disciple.rank == DiscipleRank::Chore
        }));
        let mut rng = StdRng::seed_from_u64(19);
        for _ in 0..8 {
            run_npc_ai(&mut rng, &mut state);
        }
        let members: Vec<_> = state
            .npc_disciples
            .iter()
            .filter(|disciple| disciple.sect_id.as_deref() == Some(sect_id))
            .collect();
        assert!(members.len() > before);
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
    fn named_npc_chronicles_ignore_dead_and_random_characters() {
        let mut named = Disciple {
            id: "named-wudang-leader".into(),
            name: "测试掌门".into(),
            sect_id: Some("wudang".into()),
            is_named_npc: true,
            npc_position: Some(NpcPosition::SectLeader.display().into()),
            ..Disciple::default()
        };
        let mut dead = named.clone();
        dead.id = "dead-named".into();
        dead.name = "亡者".into();
        dead.alive = false;
        let mut random = named.clone();
        random.id = "random-character".into();
        random.name = "无名氏".into();
        random.is_named_npc = false;

        let mut state = GameState {
            year: 3,
            month: 7,
            npc_disciples: vec![named.clone(), dead, random],
            ..GameState::default()
        };
        let mut rng = StdRng::seed_from_u64(41);
        let events = generate_named_npc_chronicles(&mut rng, &state);

        assert_eq!(events.len(), 1);
        assert_eq!(
            events[0].text,
            "武当张三丰于紫霄宫中闭关参悟太极，山间云气翻涌三日不散。"
        );
        assert_eq!(events[0].mood, "neutral");
        assert_eq!(events[0].category, "world");
        assert_eq!((events[0].year, events[0].month), (3, 7));

        named.alive = false;
        state.npc_disciples = vec![named];
        assert!(generate_named_npc_chronicles(&mut rng, &state).is_empty());
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
