use super::attributes::{
    AcquiredAttributes, Aptitudes, AttributeBonuses, DiscipleCondition, DiscipleRank,
    MartialProgress, ResourcePool, SkillProgress,
};
use super::disciple::{Disciple, SkillEntry};
use super::martial_art::{
    base_skill_ids, martial_art_by_id, sect_combat_arts, MartialTier, SkillCategory,
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fmt;

const MARTIAL_SCHEMA_VERSION: i32 = 3;

/// 小说 NPC 在门派或江湖中的固定身份。
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub enum NpcPosition {
    SectLeader,
    DeputyLeader,
    Elder,
    InnerDisciple,
    OuterDisciple,
    Chore,
    Wanderer,
}

impl NpcPosition {
    pub const fn display(self) -> &'static str {
        match self {
            Self::SectLeader => "掌门",
            Self::DeputyLeader => "副掌门",
            Self::Elder => "长老",
            Self::InnerDisciple => "内门弟子",
            Self::OuterDisciple => "外门弟子",
            Self::Chore => "杂役",
            Self::Wanderer => "散人",
        }
    }

    pub const fn to_disciple_rank(self) -> DiscipleRank {
        match self {
            Self::Chore => DiscipleRank::Chore,
            Self::OuterDisciple => DiscipleRank::Outer,
            Self::SectLeader
            | Self::DeputyLeader
            | Self::Elder
            | Self::InnerDisciple
            | Self::Wanderer => DiscipleRank::Inner,
        }
    }

    const fn maximum_martial_tier(self) -> MartialTier {
        match self {
            Self::Chore => MartialTier::Chore,
            Self::OuterDisciple => MartialTier::Outer,
            Self::SectLeader
            | Self::DeputyLeader
            | Self::Elder
            | Self::InnerDisciple
            | Self::Wanderer => MartialTier::Inner,
        }
    }
}

impl fmt::Display for NpcPosition {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.display())
    }
}

/// 一个不随世界种子变化的金庸小说人物模板。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NamedNpcTemplate {
    pub id: String,
    pub name: String,
    pub sect_id: Option<String>,
    pub position: NpcPosition,
    pub age: i32,
    pub aptitudes: Aptitudes,
    pub morality: i32,
    pub reputation: i32,
    pub attainment: i64,
    /// 人物自身修炼所得的内力上限，不含奇遇、药物等额外修正。
    pub permanent_neili: i32,
    pub attribute_bonuses: AttributeBonuses,
    pub skills: Vec<SkillEntry>,
    /// 基础技能 id 到当前准备武学 id 的映射。
    pub prepared: BTreeMap<String, String>,
}

impl NamedNpcTemplate {
    /// 将静态模板完整转换为游戏中的弟子数据。
    pub fn build_disciple(&self) -> Disciple {
        let proficiencies: BTreeMap<String, SkillProgress> = self
            .skills
            .iter()
            .map(|skill| {
                (
                    skill.martial_art_id.clone(),
                    SkillProgress::new(skill.level, skill.experience),
                )
            })
            .collect();
        let origin_sect_id = self.sect_id.clone().or_else(|| {
            self.skills.iter().find_map(|skill| {
                martial_art_by_id(&skill.martial_art_id).and_then(|art| art.sect_id)
            })
        });
        let effective = effective_aptitudes(&self.aptitudes, &self.skills);
        let knowledge_level = highest_knowledge_level(&self.skills);

        // 内力是模板修为与永久修正之和；精力遵循悟性、知识等级与永久修正公式。
        let neili = self
            .permanent_neili
            .saturating_add(self.attribute_bonuses.neili)
            .max(1);
        let energy = (20_i32)
            .saturating_add(effective.intelligence.saturating_mul(2))
            .saturating_add(knowledge_level / 2)
            .saturating_add(self.attribute_bonuses.energy)
            .max(1);
        // 气血、精神沿用弟子系统的年龄刻度和派生属性公式。
        let qi = (20_i32)
            .saturating_add(effective.constitution.saturating_mul(4))
            .saturating_add(neili / 2)
            .saturating_add(age_qi_modifier(self.age))
            .saturating_add(self.attribute_bonuses.qi)
            .max(1);
        let spirit = (20_i32)
            .saturating_add(effective.intelligence.saturating_mul(4))
            .saturating_add(energy / 2)
            .saturating_add(age_spirit_modifier(self.age))
            .saturating_add(self.attribute_bonuses.spirit)
            .max(1);
        let loyalty = if self.sect_id.is_some() { 85 } else { 50 };
        let talent = (self.aptitudes.strength
            + self.aptitudes.intelligence
            + self.aptitudes.constitution
            + self.aptitudes.agility
            + self.aptitudes.fortune)
            / 5;
        let martial_art =
            strongest_combat_skill(&self.skills).unwrap_or_else(|| "basic_unarmed".to_string());

        Disciple {
            id: self.id.clone(),
            sect_id: self.sect_id.clone(),
            origin_sect_id: origin_sect_id.clone(),
            is_named_npc: true,
            npc_position: Some(self.position.display().into()),
            name: self.name.clone(),
            talent,
            inner_power: neili,
            martial_art,
            prepared_skills: self.prepared.clone(),
            loyalty,
            months_in_sect: 0,
            alive: true,
            age: self.age,
            aptitudes: self.aptitudes.clone(),
            attributes: AcquiredAttributes {
                qi: full_pool(qi),
                spirit: full_pool(spirit),
                neili: full_pool(neili),
                energy: full_pool(energy),
                attainment: self.attainment,
                reputation: self.reputation,
                morality: self.morality,
                sect_loyalty: loyalty,
            },
            attribute_bonuses: self.attribute_bonuses.clone(),
            martial_schema_version: MARTIAL_SCHEMA_VERSION,
            condition: DiscipleCondition::Healthy,
            rank: self.position.to_disciple_rank(),
            merit: self.attainment / 20,
            department: None,
            master_id: None,
            relations: BTreeMap::new(),
            skills: self.skills.clone(),
            martial_progress: MartialProgress {
                proficiencies,
                specialties: origin_sect_id.into_iter().collect(),
                private_books: vec![],
            },
            action: None,
            away_months: 0,
            personal_silver: 0,
            personal_rations: 0,
            lineage_generation: 0,
            department_months: 0,
            department_merit: 0,
        }
    }
}

fn full_pool(maximum: i32) -> ResourcePool {
    ResourcePool {
        current: maximum,
        maximum,
    }
}

fn skill_level(skills: &[SkillEntry], id: &str) -> i32 {
    skills
        .iter()
        .find(|skill| skill.martial_art_id == id)
        .map(|skill| skill.level)
        .unwrap_or(0)
}

fn highest_knowledge_level(skills: &[SkillEntry]) -> i32 {
    skills
        .iter()
        .filter(|skill| {
            martial_art_by_id(&skill.martial_art_id)
                .is_some_and(|art| art.category == SkillCategory::Knowledge)
        })
        .map(|skill| skill.level)
        .max()
        .unwrap_or(0)
}

fn effective_aptitudes(aptitudes: &Aptitudes, skills: &[SkillEntry]) -> Aptitudes {
    aptitudes.with_skill_bonuses(
        skill_level(skills, "basic_unarmed"),
        highest_knowledge_level(skills),
        skill_level(skills, "basic_force"),
        skill_level(skills, "basic_dodge"),
    )
}

fn age_qi_modifier(age: i32) -> i32 {
    if age <= 35 {
        (age - 14).max(0) * 3
    } else {
        63 - (age - 35) * 4
    }
}

fn age_spirit_modifier(age: i32) -> i32 {
    if age <= 35 {
        (age - 14).max(0) * 2
    } else {
        42 - (age - 35) * 3
    }
}

fn strongest_combat_skill(skills: &[SkillEntry]) -> Option<String> {
    skills
        .iter()
        .filter_map(|skill| {
            let art = martial_art_by_id(&skill.martial_art_id)?;
            (art.is_combat && art.tier != MartialTier::Basic).then_some((
                skill.level,
                art.atk + art.def + art.spd,
                skill.martial_art_id.clone(),
            ))
        })
        .max_by_key(|(level, power, _)| (*level, *power))
        .map(|(_, _, id)| id)
}

#[derive(Clone, Copy)]
struct NpcSeed {
    id: &'static str,
    name: &'static str,
    position: NpcPosition,
    age: i32,
    aptitudes: [i32; 5],
    morality: i32,
    martial_level: i32,
}

const fn seed(
    id: &'static str,
    name: &'static str,
    position: NpcPosition,
    age: i32,
    aptitudes: [i32; 5],
    morality: i32,
    martial_level: i32,
) -> NpcSeed {
    NpcSeed {
        id,
        name,
        position,
        age,
        aptitudes,
        morality,
        martial_level,
    }
}

fn add_faction(npcs: &mut Vec<NamedNpcTemplate>, sect_id: &str, seeds: &[NpcSeed]) {
    npcs.extend(
        seeds
            .iter()
            .map(|seed| build_template(*seed, Some(sect_id), sect_id)),
    );
}

fn add_wanderer(npcs: &mut Vec<NamedNpcTemplate>, source_sect: &str, seed: NpcSeed) {
    npcs.push(build_template(seed, None, source_sect));
}

fn build_template(seed: NpcSeed, sect_id: Option<&str>, source_sect: &str) -> NamedNpcTemplate {
    let aptitudes = Aptitudes {
        strength: seed.aptitudes[0],
        intelligence: seed.aptitudes[1],
        constitution: seed.aptitudes[2],
        agility: seed.aptitudes[3],
        fortune: seed.aptitudes[4],
    };
    let legend = (seed.martial_level - 140).max(0);
    let attribute_bonuses = AttributeBonuses {
        qi: legend / 2,
        spirit: legend / 3,
        neili: legend / 3,
        energy: legend / 4,
    };
    let permanent_neili = seed
        .martial_level
        .saturating_mul(2)
        .saturating_add(aptitudes.constitution.saturating_mul(4));
    let position_reputation = match seed.position {
        NpcPosition::SectLeader => 35,
        NpcPosition::DeputyLeader => 28,
        NpcPosition::Elder => 22,
        NpcPosition::InnerDisciple => 14,
        NpcPosition::OuterDisciple => 7,
        NpcPosition::Chore => 0,
        NpcPosition::Wanderer => 18,
    };
    let skills = faction_skills(source_sect, seed.position, seed.martial_level);
    let prepared = prepared_skills(&skills);

    NamedNpcTemplate {
        id: seed.id.into(),
        name: seed.name.into(),
        sect_id: sect_id.map(str::to_owned),
        position: seed.position,
        age: seed.age,
        aptitudes,
        morality: seed.morality.clamp(0, 100),
        reputation: (seed.martial_level / 2 + position_reputation).clamp(0, 220),
        attainment: i64::from(seed.martial_level)
            .saturating_mul(i64::from(seed.martial_level))
            .saturating_mul(24),
        permanent_neili,
        attribute_bonuses,
        skills,
        prepared,
    }
}

fn faction_skills(sect_id: &str, position: NpcPosition, martial_level: i32) -> Vec<SkillEntry> {
    let knowledge_id = format!("{sect_id}_knowledge");
    let mut skills: Vec<SkillEntry> = base_skill_ids(sect_id)
        .into_iter()
        .enumerate()
        .map(|(index, id)| {
            let adjustment = match id.as_str() {
                "basic_force" | "basic_unarmed" => 18,
                "basic_parry" => 14,
                "basic_dodge" => 10,
                _ if id == knowledge_id => 24,
                _ => 12 - index as i32,
            };
            SkillEntry {
                martial_art_id: id,
                level: martial_level.saturating_add(adjustment).max(20),
                experience: 0,
            }
        })
        .collect();

    let maximum_tier = position.maximum_martial_tier();
    for (tier, reduction) in [
        (MartialTier::Chore, 44),
        (MartialTier::Outer, 22),
        (MartialTier::Inner, 0),
    ] {
        if tier > maximum_tier {
            continue;
        }
        skills.extend(sect_combat_arts(sect_id, tier).into_iter().enumerate().map(
            |(index, art)| {
                SkillEntry {
                    martial_art_id: art.id,
                    level: martial_level
                        .saturating_sub(reduction)
                        .saturating_sub(index as i32 * 2)
                        .max(12),
                    experience: 0,
                }
            },
        ));
    }
    skills
}

fn prepared_skills(skills: &[SkillEntry]) -> BTreeMap<String, String> {
    let mut prepared = BTreeMap::new();
    for skill in skills {
        let Some(art) = martial_art_by_id(&skill.martial_art_id) else {
            continue;
        };
        if art.category == SkillCategory::Knowledge {
            prepared.insert("knowledge".into(), art.id);
            continue;
        }
        if !art.is_combat || art.tier == MartialTier::Basic {
            continue;
        }
        prepared.insert(art.basic_skill.clone(), art.id.clone());
        if matches!(art.category, SkillCategory::Unarmed | SkillCategory::Weapon)
            || art.usable_for_parry
        {
            prepared.insert("basic_parry".into(), art.id);
        }
    }
    prepared
}

/// 返回 24 个势力与江湖散人的全量固定小说 NPC。
pub fn all_named_npcs() -> Vec<NamedNpcTemplate> {
    use NpcPosition as P;

    let mut npcs = Vec::with_capacity(109);

    add_faction(
        &mut npcs,
        "wudang",
        &[
            seed(
                "npc_wudang_1",
                "张三丰",
                P::SectLeader,
                124,
                [34, 45, 45, 38, 38],
                95,
                380,
            ),
            seed(
                "npc_wudang_2",
                "宋远桥",
                P::DeputyLeader,
                56,
                [32, 36, 37, 32, 29],
                88,
                245,
            ),
            seed(
                "npc_wudang_3",
                "俞莲舟",
                P::Elder,
                52,
                [36, 34, 38, 34, 27],
                86,
                260,
            ),
            seed(
                "npc_wudang_4",
                "俞岱岩",
                P::Elder,
                49,
                [32, 33, 36, 29, 25],
                86,
                215,
            ),
            seed(
                "npc_wudang_5",
                "张松溪",
                P::Elder,
                47,
                [29, 38, 33, 35, 28],
                87,
                225,
            ),
            seed(
                "npc_wudang_6",
                "张翠山",
                P::Elder,
                34,
                [34, 37, 35, 37, 31],
                90,
                235,
            ),
            seed(
                "npc_wudang_7",
                "殷梨亭",
                P::Elder,
                39,
                [30, 32, 32, 35, 24],
                82,
                195,
            ),
            seed(
                "npc_wudang_8",
                "莫声谷",
                P::Elder,
                36,
                [34, 29, 34, 34, 27],
                84,
                200,
            ),
            seed(
                "npc_wudang_9",
                "宋青书",
                P::OuterDisciple,
                22,
                [29, 30, 27, 31, 18],
                48,
                125,
            ),
            seed(
                "npc_wudang_10",
                "冲虚道长",
                P::Elder,
                67,
                [30, 38, 35, 34, 30],
                80,
                230,
            ),
        ],
    );
    add_faction(
        &mut npcs,
        "shaolin",
        &[
            seed(
                "npc_shaolin_1",
                "玄慈",
                P::SectLeader,
                67,
                [37, 37, 40, 31, 28],
                82,
                285,
            ),
            seed(
                "npc_shaolin_2",
                "空闻",
                P::DeputyLeader,
                64,
                [31, 36, 37, 29, 27],
                85,
                235,
            ),
            seed(
                "npc_shaolin_3",
                "玄悲",
                P::Elder,
                61,
                [34, 34, 38, 30, 26],
                88,
                225,
            ),
            seed(
                "npc_shaolin_4",
                "玄难",
                P::Elder,
                59,
                [35, 32, 37, 29, 25],
                84,
                220,
            ),
            seed(
                "npc_shaolin_5",
                "空智",
                P::Elder,
                58,
                [35, 33, 36, 31, 24],
                80,
                225,
            ),
            seed(
                "npc_shaolin_6",
                "渡厄",
                P::Elder,
                79,
                [34, 42, 42, 32, 29],
                90,
                305,
            ),
            seed(
                "npc_shaolin_7",
                "渡劫",
                P::Elder,
                77,
                [38, 38, 41, 30, 27],
                88,
                295,
            ),
            seed(
                "npc_shaolin_8",
                "渡难",
                P::Elder,
                75,
                [36, 39, 40, 31, 28],
                89,
                290,
            ),
        ],
    );
    add_faction(
        &mut npcs,
        "huashan",
        &[
            seed(
                "npc_huashan_1",
                "岳不群",
                P::SectLeader,
                45,
                [31, 38, 33, 34, 23],
                35,
                245,
            ),
            seed(
                "npc_huashan_2",
                "宁中则",
                P::Elder,
                42,
                [30, 34, 32, 35, 29],
                82,
                205,
            ),
            seed(
                "npc_huashan_3",
                "令狐冲",
                P::InnerDisciple,
                27,
                [35, 39, 32, 42, 37],
                72,
                305,
            ),
            seed(
                "npc_huashan_4",
                "风清扬",
                P::Elder,
                86,
                [32, 45, 37, 43, 35],
                78,
                355,
            ),
        ],
    );
    add_faction(
        &mut npcs,
        "mingjiao",
        &[
            seed(
                "npc_mingjiao_1",
                "张无忌",
                P::SectLeader,
                24,
                [38, 42, 44, 39, 40],
                88,
                350,
            ),
            seed(
                "npc_mingjiao_2",
                "杨逍",
                P::DeputyLeader,
                48,
                [34, 40, 34, 38, 27],
                62,
                270,
            ),
            seed(
                "npc_mingjiao_3",
                "范遥",
                P::DeputyLeader,
                47,
                [36, 38, 35, 37, 25],
                67,
                265,
            ),
            seed(
                "npc_mingjiao_4",
                "谢逊",
                P::Elder,
                53,
                [42, 34, 40, 31, 20],
                55,
                280,
            ),
            seed(
                "npc_mingjiao_5",
                "韦一笑",
                P::Elder,
                49,
                [29, 35, 31, 45, 24],
                57,
                255,
            ),
            seed(
                "npc_mingjiao_6",
                "殷天正",
                P::Elder,
                68,
                [40, 34, 39, 33, 26],
                68,
                270,
            ),
        ],
    );
    add_faction(
        &mut npcs,
        "quanzhen",
        &[
            seed(
                "npc_quanzhen_1",
                "王重阳",
                P::SectLeader,
                58,
                [37, 45, 43, 40, 36],
                92,
                365,
            ),
            seed(
                "npc_quanzhen_2",
                "马钰",
                P::DeputyLeader,
                56,
                [29, 39, 36, 31, 32],
                90,
                225,
            ),
            seed(
                "npc_quanzhen_3",
                "丘处机",
                P::Elder,
                53,
                [37, 33, 37, 35, 27],
                79,
                245,
            ),
            seed(
                "npc_quanzhen_4",
                "王处一",
                P::Elder,
                51,
                [34, 34, 35, 34, 25],
                82,
                220,
            ),
            seed(
                "npc_quanzhen_5",
                "郝大通",
                P::Elder,
                50,
                [32, 35, 34, 31, 26],
                80,
                205,
            ),
            seed(
                "npc_quanzhen_6",
                "谭处端",
                P::Elder,
                52,
                [33, 34, 36, 32, 25],
                84,
                210,
            ),
            seed(
                "npc_quanzhen_7",
                "刘处玄",
                P::Elder,
                49,
                [31, 36, 34, 33, 27],
                83,
                205,
            ),
            seed(
                "npc_quanzhen_8",
                "孙不二",
                P::Elder,
                48,
                [29, 36, 33, 34, 29],
                82,
                200,
            ),
            seed(
                "npc_quanzhen_9",
                "尹志平",
                P::InnerDisciple,
                28,
                [28, 31, 29, 31, 20],
                52,
                135,
            ),
        ],
    );
    add_faction(
        &mut npcs,
        "tianlong",
        &[
            seed(
                "npc_tianlong_1",
                "段正明",
                P::SectLeader,
                52,
                [34, 38, 37, 34, 35],
                88,
                255,
            ),
            seed(
                "npc_tianlong_2",
                "本因",
                P::DeputyLeader,
                61,
                [31, 38, 37, 30, 30],
                91,
                235,
            ),
            seed(
                "npc_tianlong_3",
                "枯荣大师",
                P::Elder,
                86,
                [29, 44, 43, 27, 33],
                94,
                315,
            ),
            seed(
                "npc_tianlong_4",
                "本相",
                P::Elder,
                59,
                [33, 36, 36, 31, 29],
                89,
                220,
            ),
            seed(
                "npc_tianlong_5",
                "本参",
                P::Elder,
                57,
                [32, 37, 35, 32, 28],
                90,
                215,
            ),
        ],
    );
    add_faction(
        &mut npcs,
        "taohua",
        &[
            seed(
                "npc_taohua_1",
                "黄药师",
                P::SectLeader,
                63,
                [36, 45, 38, 42, 30],
                58,
                350,
            ),
            seed(
                "npc_taohua_2",
                "陈玄风",
                P::Elder,
                38,
                [38, 30, 37, 36, 20],
                28,
                215,
            ),
            seed(
                "npc_taohua_3",
                "梅超风",
                P::Elder,
                36,
                [35, 33, 35, 39, 19],
                30,
                225,
            ),
        ],
    );
    add_faction(
        &mut npcs,
        "gumu",
        &[
            seed(
                "npc_gumu_1",
                "小龙女",
                P::SectLeader,
                27,
                [29, 37, 37, 45, 31],
                78,
                300,
            ),
            seed(
                "npc_gumu_2",
                "杨过",
                P::DeputyLeader,
                29,
                [41, 43, 38, 41, 33],
                76,
                335,
            ),
            seed(
                "npc_gumu_3",
                "孙婆婆",
                P::Elder,
                64,
                [27, 31, 34, 29, 25],
                86,
                145,
            ),
        ],
    );
    add_faction(
        &mut npcs,
        "gaibang",
        &[
            seed(
                "npc_gaibang_1",
                "洪七公",
                P::SectLeader,
                68,
                [43, 38, 42, 39, 34],
                90,
                345,
            ),
            seed(
                "npc_gaibang_2",
                "黄蓉",
                P::DeputyLeader,
                28,
                [28, 45, 31, 40, 38],
                84,
                275,
            ),
            seed(
                "npc_gaibang_3",
                "鲁有脚",
                P::Elder,
                49,
                [34, 29, 35, 30, 25],
                82,
                175,
            ),
        ],
    );
    add_faction(
        &mut npcs,
        "emei",
        &[
            seed(
                "npc_emei_1",
                "郭襄",
                P::SectLeader,
                42,
                [31, 42, 37, 39, 39],
                91,
                285,
            ),
            seed(
                "npc_emei_2",
                "灭绝师太",
                P::Elder,
                54,
                [35, 36, 37, 34, 22],
                64,
                250,
            ),
            seed(
                "npc_emei_3",
                "周芷若",
                P::InnerDisciple,
                22,
                [28, 39, 29, 37, 26],
                55,
                220,
            ),
        ],
    );
    add_faction(
        &mut npcs,
        "riyue",
        &[
            seed(
                "npc_riyue_1",
                "东方不败",
                P::SectLeader,
                39,
                [36, 42, 37, 47, 28],
                18,
                365,
            ),
            seed(
                "npc_riyue_2",
                "任我行",
                P::DeputyLeader,
                54,
                [40, 39, 41, 35, 25],
                24,
                315,
            ),
            seed(
                "npc_riyue_3",
                "向问天",
                P::Elder,
                46,
                [38, 35, 37, 36, 27],
                48,
                240,
            ),
            seed(
                "npc_riyue_4",
                "任盈盈",
                P::InnerDisciple,
                23,
                [27, 42, 29, 36, 34],
                62,
                180,
            ),
        ],
    );
    add_faction(
        &mut npcs,
        "xingxiu",
        &[
            seed(
                "npc_xingxiu_1",
                "丁春秋",
                P::SectLeader,
                72,
                [35, 41, 38, 33, 21],
                5,
                310,
            ),
            seed(
                "npc_xingxiu_2",
                "摘星子",
                P::InnerDisciple,
                31,
                [29, 30, 28, 33, 18],
                12,
                145,
            ),
        ],
    );
    add_faction(
        &mut npcs,
        "murong",
        &[
            seed(
                "npc_murong_1",
                "慕容复",
                P::SectLeader,
                30,
                [35, 41, 35, 39, 25],
                38,
                285,
            ),
            seed(
                "npc_murong_2",
                "慕容博",
                P::Elder,
                66,
                [39, 43, 40, 38, 24],
                25,
                335,
            ),
            seed(
                "npc_murong_3",
                "邓百川",
                P::Elder,
                43,
                [37, 31, 36, 32, 26],
                61,
                190,
            ),
            seed(
                "npc_murong_4",
                "公冶乾",
                P::Elder,
                40,
                [34, 34, 34, 35, 25],
                58,
                185,
            ),
        ],
    );
    add_faction(
        &mut npcs,
        "dalun",
        &[
            seed(
                "npc_dalun_1",
                "金轮法王",
                P::SectLeader,
                58,
                [44, 36, 44, 32, 25],
                32,
                335,
            ),
            seed(
                "npc_dalun_2",
                "霍都",
                P::InnerDisciple,
                31,
                [33, 34, 31, 35, 24],
                22,
                185,
            ),
            seed(
                "npc_dalun_3",
                "达尔巴",
                P::InnerDisciple,
                36,
                [41, 25, 40, 27, 28],
                58,
                195,
            ),
        ],
    );
    add_faction(
        &mut npcs,
        "lingjiu",
        &[
            seed(
                "npc_lingjiu_1",
                "天山童姥",
                P::SectLeader,
                96,
                [35, 44, 43, 44, 29],
                48,
                350,
            ),
            seed(
                "npc_lingjiu_2",
                "虚竹",
                P::DeputyLeader,
                25,
                [31, 35, 43, 32, 46],
                95,
                330,
            ),
        ],
    );
    add_faction(
        &mut npcs,
        "baituo",
        &[
            seed(
                "npc_baituo_1",
                "欧阳锋",
                P::SectLeader,
                70,
                [43, 40, 42, 38, 21],
                18,
                350,
            ),
            seed(
                "npc_baituo_2",
                "欧阳克",
                P::InnerDisciple,
                32,
                [34, 35, 32, 37, 26],
                20,
                200,
            ),
        ],
    );
    add_faction(
        &mut npcs,
        "xueshan",
        &[
            seed(
                "npc_xueshan_1",
                "白自在",
                P::SectLeader,
                63,
                [39, 31, 40, 32, 23],
                49,
                245,
            ),
            seed(
                "npc_xueshan_2",
                "封万里",
                P::Elder,
                44,
                [34, 29, 34, 31, 24],
                46,
                165,
            ),
        ],
    );
    add_faction(
        &mut npcs,
        "tiandihui",
        &[seed(
            "npc_tiandihui_1",
            "陈近南",
            P::SectLeader,
            43,
            [37, 40, 38, 37, 31],
            92,
            275,
        )],
    );
    add_faction(
        &mut npcs,
        "shenlong",
        &[
            seed(
                "npc_shenlong_1",
                "洪安通",
                P::SectLeader,
                61,
                [38, 38, 40, 34, 20],
                12,
                285,
            ),
            seed(
                "npc_shenlong_2",
                "苏荃",
                P::DeputyLeader,
                29,
                [27, 39, 30, 38, 29],
                32,
                180,
            ),
        ],
    );
    add_faction(
        &mut npcs,
        "jueqing",
        &[
            seed(
                "npc_jueqing_1",
                "公孙止",
                P::SectLeader,
                48,
                [37, 35, 36, 35, 19],
                20,
                240,
            ),
            seed(
                "npc_jueqing_2",
                "裘千尺",
                P::Elder,
                46,
                [32, 39, 35, 27, 17],
                16,
                225,
            ),
            seed(
                "npc_jueqing_3",
                "公孙绿萼",
                P::InnerDisciple,
                21,
                [25, 32, 27, 31, 25],
                78,
                115,
            ),
        ],
    );
    add_faction(
        &mut npcs,
        "wudu",
        &[
            seed(
                "npc_wudu_1",
                "蓝凤凰",
                P::SectLeader,
                28,
                [29, 36, 31, 38, 30],
                44,
                205,
            ),
            seed(
                "npc_wudu_2",
                "何铁手",
                P::Elder,
                26,
                [31, 35, 30, 37, 26],
                38,
                195,
            ),
        ],
    );
    add_faction(
        &mut npcs,
        "qingcheng",
        &[
            seed(
                "npc_qingcheng_1",
                "余沧海",
                P::SectLeader,
                51,
                [34, 34, 35, 34, 18],
                24,
                220,
            ),
            seed(
                "npc_qingcheng_2",
                "侯人英",
                P::InnerDisciple,
                29,
                [29, 27, 29, 30, 20],
                28,
                125,
            ),
            seed(
                "npc_qingcheng_3",
                "洪人雄",
                P::InnerDisciple,
                31,
                [31, 25, 31, 28, 19],
                25,
                120,
            ),
        ],
    );
    add_faction(
        &mut npcs,
        "court",
        &[
            seed(
                "npc_court_hubilie",
                "忽必烈",
                P::SectLeader,
                48,
                [30, 43, 36, 31, 38],
                55,
                230,
            ),
            seed(
                "npc_court_2",
                "哲别",
                P::Elder,
                51,
                [40, 34, 38, 39, 30],
                52,
                250,
            ),
            seed(
                "npc_court_3",
                "木华黎",
                P::Elder,
                54,
                [39, 37, 39, 34, 29],
                58,
                245,
            ),
            seed(
                "npc_court_4",
                "康熙",
                P::DeputyLeader,
                34,
                [28, 44, 33, 31, 36],
                63,
                190,
            ),
            seed(
                "npc_court_5",
                "年羹尧",
                P::Elder,
                43,
                [37, 38, 36, 34, 24],
                42,
                215,
            ),
            seed(
                "npc_court_6",
                "韦小宝",
                P::InnerDisciple,
                21,
                [18, 37, 23, 32, 48],
                45,
                105,
            ),
            seed(
                "npc_court_7",
                "多隆",
                P::InnerDisciple,
                32,
                [32, 28, 33, 29, 34],
                50,
                125,
            ),
            seed(
                "npc_court_8",
                "海大富",
                P::Elder,
                57,
                [34, 35, 37, 36, 22],
                28,
                205,
            ),
        ],
    );
    add_faction(
        &mut npcs,
        "song_court",
        &[
            seed(
                "npc_song_court_1",
                "宋江",
                P::SectLeader,
                42,
                [28, 40, 34, 29, 39],
                78,
                180,
            ),
            seed(
                "npc_song_court_2",
                "卢俊义",
                P::DeputyLeader,
                39,
                [43, 35, 41, 37, 31],
                82,
                270,
            ),
            seed(
                "npc_song_court_3",
                "花荣",
                P::Elder,
                34,
                [36, 34, 34, 42, 32],
                84,
                225,
            ),
            seed(
                "npc_song_court_4",
                "岳飞",
                P::Elder,
                39,
                [40, 43, 40, 36, 34],
                97,
                305,
            ),
            seed(
                "npc_song_court_5",
                "郭靖",
                P::Elder,
                35,
                [43, 31, 43, 35, 39],
                98,
                335,
            ),
            seed(
                "npc_song_court_6",
                "黄蓉",
                P::Elder,
                30,
                [28, 45, 31, 40, 38],
                86,
                270,
            ),
            seed(
                "npc_song_court_7",
                "萧峰",
                P::Elder,
                34,
                [46, 37, 44, 39, 28],
                94,
                350,
            ),
            seed(
                "npc_song_court_8",
                "令狐冲",
                P::InnerDisciple,
                28,
                [35, 39, 32, 42, 37],
                73,
                300,
            ),
            seed(
                "npc_song_court_9",
                "杨过",
                P::InnerDisciple,
                30,
                [41, 43, 38, 41, 33],
                77,
                330,
            ),
            seed(
                "npc_song_court_10",
                "张无忌",
                P::InnerDisciple,
                25,
                [38, 42, 44, 39, 40],
                89,
                345,
            ),
        ],
    );

    add_wanderer(
        &mut npcs,
        "xueshan",
        seed(
            "wanderer_1",
            "胡斐",
            P::Wanderer,
            25,
            [39, 34, 36, 41, 32],
            86,
            245,
        ),
    );
    add_wanderer(
        &mut npcs,
        "huashan",
        seed(
            "wanderer_2",
            "袁承志",
            P::Wanderer,
            28,
            [37, 39, 38, 38, 34],
            93,
            270,
        ),
    );
    add_wanderer(
        &mut npcs,
        "xueshan",
        seed(
            "wanderer_3",
            "狄云",
            P::Wanderer,
            27,
            [38, 28, 43, 32, 24],
            96,
            255,
        ),
    );
    add_wanderer(
        &mut npcs,
        "lingjiu",
        seed(
            "wanderer_4",
            "石破天",
            P::Wanderer,
            22,
            [44, 25, 46, 40, 45],
            98,
            340,
        ),
    );
    add_wanderer(
        &mut npcs,
        "xueshan",
        seed(
            "wanderer_5",
            "苗人凤",
            P::Wanderer,
            39,
            [41, 35, 40, 39, 29],
            92,
            285,
        ),
    );
    add_wanderer(
        &mut npcs,
        "wudu",
        seed(
            "wanderer_6",
            "程灵素",
            P::Wanderer,
            20,
            [19, 45, 25, 31, 34],
            95,
            130,
        ),
    );
    add_wanderer(
        &mut npcs,
        "murong",
        seed(
            "wanderer_7",
            "阿朱",
            P::Wanderer,
            22,
            [23, 39, 27, 38, 31],
            91,
            145,
        ),
    );

    npcs
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::{BTreeMap, BTreeSet};

    #[test]
    fn roster_has_expected_counts_and_unique_ids() {
        let npcs = all_named_npcs();
        assert_eq!(npcs.len(), 109);
        assert_eq!(
            npcs.iter()
                .map(|npc| &npc.id)
                .collect::<BTreeSet<_>>()
                .len(),
            npcs.len()
        );

        let counts = npcs.iter().fold(BTreeMap::new(), |mut counts, npc| {
            *counts
                .entry(npc.sect_id.as_deref().unwrap_or("wanderer"))
                .or_insert(0) += 1;
            counts
        });
        assert_eq!(
            counts,
            BTreeMap::from([
                ("baituo", 2),
                ("court", 8),
                ("dalun", 3),
                ("emei", 3),
                ("gaibang", 3),
                ("gumu", 3),
                ("huashan", 4),
                ("jueqing", 3),
                ("lingjiu", 2),
                ("mingjiao", 6),
                ("murong", 4),
                ("qingcheng", 3),
                ("quanzhen", 9),
                ("riyue", 4),
                ("shaolin", 8),
                ("shenlong", 2),
                ("song_court", 10),
                ("taohua", 3),
                ("tianlong", 5),
                ("tiandihui", 1),
                ("wanderer", 7),
                ("wudang", 10),
                ("wudu", 2),
                ("xingxiu", 2),
                ("xueshan", 2),
            ])
        );
    }

    #[test]
    fn built_npcs_are_full_and_use_registered_skills() {
        for template in all_named_npcs() {
            assert!(!template.skills.is_empty());
            assert!(template
                .skills
                .iter()
                .all(|skill| martial_art_by_id(&skill.martial_art_id).is_some()));

            let mut disciple = template.build_disciple();
            assert!(disciple.is_named_npc);
            assert_eq!(
                disciple.npc_position.as_deref(),
                Some(template.position.display())
            );
            assert_eq!(
                disciple.attributes.qi.current,
                disciple.attributes.qi.maximum
            );
            assert_eq!(
                disciple.attributes.spirit.current,
                disciple.attributes.spirit.maximum
            );
            assert_eq!(
                disciple.attributes.neili.current,
                disciple.attributes.neili.maximum
            );
            assert_eq!(
                disciple.attributes.energy.current,
                disciple.attributes.energy.maximum
            );

            let maxima = crate::logic::disciple::attribute_maxima(&disciple);
            assert_eq!(disciple.attributes.qi.maximum, maxima.qi);
            assert_eq!(disciple.attributes.spirit.maximum, maxima.spirit);
            assert_eq!(disciple.attributes.neili.maximum, maxima.neili);
            assert_eq!(disciple.attributes.energy.maximum, maxima.energy);

            let prepared = disciple.prepared_skills.clone();
            crate::logic::disciple::normalize_prepared_skills(&mut disciple);
            assert_eq!(disciple.prepared_skills, prepared);
        }
    }
}
