use crate::logic::disciple::{
    add_permanent_neili, assign_sect_curriculum, generate_disciple, sync_legacy_attributes,
};
use crate::models::attributes::{Department, DiscipleRank};
use crate::models::martial_art::{all_martial_arts, knowledge_skill_id};
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
            public_books: std::iter::once(knowledge_skill_id(template.id))
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
                .collect(),
            martial_research: BTreeMap::from([(template.signature.into(), 180 + prestige as i64)]),
            relations: BTreeMap::new(),
            active_orders: vec![],
            productions: vec![],
            auto_brew_index: 0,
            auto_brew_progress: 0,
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
    for canonical in crate::models::sect::default_countries() {
        if let Some(existing) = state
            .countries
            .iter_mut()
            .find(|country| country.id == canonical.id)
        {
            existing.name = canonical.name;
        } else {
            state.countries.push(canonical);
        }
    }
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
    fn seeded_world_is_stable() {
        let (sects_a, _) = generate_npc_world(9);
        let (sects_b, _) = generate_npc_world(9);
        assert_eq!(sects_a[7].attributes.silver, sects_b[7].attributes.silver);
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
