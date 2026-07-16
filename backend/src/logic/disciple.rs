use crate::models::attributes::{
    AcquiredAttributes, Aptitudes, DiscipleCondition, DiscipleRank, MartialProgress, ResourcePool,
    SkillProgress,
};
use crate::models::martial_art::{
    base_skill_ids, canonical_skill_id, knowledge_skill_for_art, knowledge_skill_id,
    martial_art_by_id, sect_combat_arts, sect_ids, MartialTier, SkillCategory,
};
use crate::models::{Disciple, SkillEntry};
use rand::Rng;
use std::collections::BTreeMap;

const SURNAMES: &[&str] = &[
    "风", "云", "萧", "柳", "慕容", "上官", "南宫", "令狐", "沈", "陆", "叶", "林", "楚", "苏",
    "燕", "秦", "白", "莫", "顾", "谢",
];
const GIVEN_MALE: &[&str] = &[
    "清扬", "无忌", "破军", "铁心", "孤鸿", "凌霄", "子陵", "天行", "断水", "逐云", "惊鸿", "寒江",
    "远山", "行空", "九渊", "纯钧", "赤霄", "湛卢", "承影", "泰阿",
];
const GIVEN_FEMALE: &[&str] = &[
    "若兰", "灵素", "紫烟", "幽月", "凝霜", "凤歌", "雪晴", "梦蝶", "碧落", "紫菱", "冰雁", "霜华",
    "念慈", "倚天", "芷若", "飞燕", "语嫣", "龙儿", "莫愁", "秋水",
];

pub(crate) fn rand_range(rng: &mut impl Rng, min: i32, max: i32) -> i32 {
    rng.gen_range(min..=max)
}

fn pick<'a, T>(rng: &mut impl Rng, arr: &'a [T]) -> &'a T {
    &arr[rng.gen_range(0..arr.len())]
}

pub(crate) fn clamp(v: i32, lo: i32, hi: i32) -> i32 {
    v.max(lo).min(hi)
}

fn generate_name(rng: &mut impl Rng) -> String {
    let surname = pick(rng, SURNAMES);
    let given = if rng.gen_bool(0.5) {
        pick(rng, GIVEN_MALE)
    } else {
        pick(rng, GIVEN_FEMALE)
    };
    format!("{}{}", surname, given)
}

pub fn generate_disciple(rng: &mut impl Rng, talent_bonus: i32) -> Disciple {
    let aptitudes = Aptitudes {
        strength: clamp(rand_range(rng, 12, 30) + talent_bonus / 4, 8, 40),
        intelligence: clamp(rand_range(rng, 12, 30) + talent_bonus / 4, 8, 40),
        constitution: clamp(rand_range(rng, 12, 30) + talent_bonus / 4, 8, 40),
        agility: clamp(rand_range(rng, 12, 30) + talent_bonus / 4, 8, 40),
        fortune: clamp(rand_range(rng, 8, 32), 5, 40),
    };
    let talent = (aptitudes.strength
        + aptitudes.intelligence
        + aptitudes.constitution
        + aptitudes.agility
        + aptitudes.fortune)
        / 5;
    let age = rand_range(rng, 15, 28);
    let origins: Vec<&str> = sect_ids().collect();
    let origin = *pick(rng, &origins);

    let mut disciple = Disciple {
        id: format!(
            "d{}_{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_millis(),
            rand_range(rng, 1000, 9999)
        ),
        name: generate_name(rng),
        talent,
        origin_sect_id: Some(origin.into()),
        inner_power: 0,
        martial_art: String::new(),
        loyalty: rand_range(rng, 40, 70),
        months_in_sect: 0,
        alive: true,
        age,
        aptitudes,
        ..Disciple::default()
    };
    let knowledge_level = (disciple.aptitudes.intelligence * 2 + rand_range(rng, -12, 18)).max(20);
    let basic_level = (32 + talent + rand_range(rng, -8, 8)).min(knowledge_level);
    let mut proficiencies: BTreeMap<String, SkillProgress> = base_skill_ids(origin)
        .into_iter()
        .map(|id| {
            let level = if id == knowledge_skill_id(origin) {
                knowledge_level
            } else {
                (basic_level + rand_range(rng, -5, 5)).max(10)
            };
            (id, SkillProgress::new(level, 0))
        })
        .collect();
    // 江湖来客保留原门知识，同时从入门起修习现所属门派的义理。
    proficiencies
        .entry("player_knowledge".into())
        .or_insert_with(|| SkillProgress::new((knowledge_level * 2 / 3).max(20), 0));
    let low_count = rand_range(rng, 2, 5) as usize;
    let mid_count = if talent >= 23 {
        rand_range(rng, 0, 3) as usize
    } else {
        0
    };
    let mut low = sect_combat_arts(origin, MartialTier::Chore);
    let mut middle = sect_combat_arts(origin, MartialTier::Outer);
    shuffle(rng, &mut low);
    shuffle(rng, &mut middle);
    for art in low.into_iter().take(low_count) {
        proficiencies.insert(
            art.id,
            SkillProgress::new(rand_range(rng, 24, 48).min(knowledge_level), 0),
        );
    }
    for art in middle.into_iter().take(mid_count) {
        proficiencies.insert(
            art.id,
            SkillProgress::new(rand_range(rng, 20, 42).min(knowledge_level), 0),
        );
    }
    disciple.martial_art = proficiencies
        .iter()
        .filter(|(id, _)| {
            martial_art_by_id(id).is_some_and(|art| art.is_combat && art.tier != MartialTier::Basic)
        })
        .max_by_key(|(_, progress)| progress.level)
        .map(|(id, _)| id.clone())
        .unwrap_or_else(|| "basic_unarmed".into());
    disciple.martial_progress = MartialProgress {
        proficiencies,
        specialties: vec![origin.into()],
        private_books: vec![],
    };
    disciple.martial_schema_version = 1;
    disciple.attributes = initial_attributes(&disciple);
    sync_legacy_attributes(&mut disciple);
    disciple
}

fn shuffle<T>(rng: &mut impl Rng, values: &mut [T]) {
    for index in (1..values.len()).rev() {
        values.swap(index, rng.gen_range(0..=index));
    }
}

fn initial_attributes(d: &Disciple) -> AcquiredAttributes {
    let mut attributes = AcquiredAttributes {
        sect_loyalty: d.loyalty,
        morality: (40 + d.aptitudes.fortune / 2).clamp(0, 100),
        ..AcquiredAttributes::default()
    };
    let maxima = attribute_maxima(d);
    attributes.neili = ResourcePool {
        current: maxima.neili,
        maximum: maxima.neili,
    };
    attributes.energy = ResourcePool {
        current: maxima.energy,
        maximum: maxima.energy,
    };
    attributes.qi = ResourcePool {
        current: maxima.qi,
        maximum: maxima.qi,
    };
    attributes.spirit = ResourcePool {
        current: maxima.spirit,
        maximum: maxima.spirit,
    };
    attributes
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AttributeMaxima {
    pub qi: i32,
    pub spirit: i32,
    pub neili: i32,
    pub energy: i32,
}

fn skill_level(d: &Disciple, id: &str) -> i32 {
    d.martial_progress
        .proficiencies
        .get(id)
        .map(|progress| progress.level)
        .unwrap_or(0)
}

pub fn knowledge_level(d: &Disciple) -> i32 {
    let preferred = d.origin_sect_id.as_deref().unwrap_or("player");
    let preferred_id = knowledge_skill_id(preferred);
    skill_level(d, &preferred_id).max(
        d.martial_progress
            .proficiencies
            .iter()
            .filter(|(id, _)| {
                martial_art_by_id(id).is_some_and(|art| art.category == SkillCategory::Knowledge)
            })
            .map(|(_, progress)| progress.level)
            .max()
            .unwrap_or(0),
    )
}

pub fn force_level(d: &Disciple) -> i32 {
    d.martial_progress
        .proficiencies
        .iter()
        .filter(|(id, _)| {
            martial_art_by_id(id).is_some_and(|art| art.category == SkillCategory::Force)
        })
        .map(|(_, progress)| progress.level)
        .max()
        .unwrap_or(0)
}

/// 知识每 20 级提供 1 点有效悟性，用于研读、冥想与精神上限。
pub fn effective_intelligence(d: &Disciple) -> i32 {
    d.aptitudes.intelligence + knowledge_level(d) / 20
}

/// 四项派生上限：基础 + 天赋 + 对应技能 + 丹药/事件/长期修炼加成。
pub fn attribute_maxima(d: &Disciple) -> AttributeMaxima {
    let force = force_level(d);
    let knowledge = knowledge_level(d);
    let neili = 20 + d.aptitudes.constitution * 2 + force / 2 + d.attribute_bonuses.neili;
    let energy = 20 + effective_intelligence(d) * 2 + knowledge / 2 + d.attribute_bonuses.energy;
    AttributeMaxima {
        neili: neili.max(1),
        energy: energy.max(1),
        qi: qi_maximum(d.age, &d.aptitudes, neili) + d.attribute_bonuses.qi,
        spirit: spirit_maximum(d.age, effective_intelligence(d), energy)
            + d.attribute_bonuses.spirit,
    }
}

pub fn recalculate_attribute_maxima(d: &mut Disciple) {
    let new = attribute_maxima(d);
    d.attributes.qi.maximum = new.qi.max(1);
    d.attributes.spirit.maximum = new.spirit.max(1);
    d.attributes.neili.maximum = new.neili.max(1);
    d.attributes.energy.maximum = new.energy.max(1);
    d.attributes.qi.current = d.attributes.qi.current.clamp(0, new.qi.max(1));
    d.attributes.spirit.current = d.attributes.spirit.current.clamp(0, new.spirit.max(1));
    d.attributes.neili.current = d.attributes.neili.current.clamp(0, new.neili.max(1));
    d.attributes.energy.current = d.attributes.energy.current.clamp(0, new.energy.max(1));
}

/// 35 岁前按固定年龄刻度增长，35 岁后按固定刻度衰减。
pub fn age_qi_modifier(age: i32) -> i32 {
    if age <= 35 {
        (age - 14).max(0) * 3
    } else {
        63 - (age - 35) * 4
    }
}

pub fn age_spirit_modifier(age: i32) -> i32 {
    if age <= 35 {
        (age - 14).max(0) * 2
    } else {
        42 - (age - 35) * 3
    }
}

pub fn qi_maximum(age: i32, aptitude: &Aptitudes, max_neili: i32) -> i32 {
    20 + aptitude.constitution * 4 + max_neili / 2 + age_qi_modifier(age)
}

pub fn spirit_maximum(age: i32, intelligence: i32, max_energy: i32) -> i32 {
    20 + intelligence * 4 + max_energy / 2 + age_spirit_modifier(age)
}

pub fn sync_legacy_attributes(d: &mut Disciple) {
    d.talent = (d.aptitudes.strength
        + d.aptitudes.intelligence
        + d.aptitudes.constitution
        + d.aptitudes.agility
        + d.aptitudes.fortune)
        / 5;
    d.inner_power = d.attributes.neili.maximum;
    d.loyalty = d.attributes.sect_loyalty.clamp(0, 100);
    d.alive = d.condition != DiscipleCondition::Dead;
    sync_skills_from_progress(d);
}

/// 对外的顺序列表与内部熟练度映射保持一致，便于前端及存档直接读取个人武学。
pub fn sync_skills_from_progress(d: &mut Disciple) {
    d.skills = d
        .martial_progress
        .proficiencies
        .iter()
        .map(|(martial_art_id, progress)| SkillEntry {
            martial_art_id: martial_art_id.clone(),
            level: progress.level,
            experience: progress.experience,
        })
        .collect();
}

/// 将尚沿用 v2 字段的决策与事件效果汇入 v3 属性。
pub fn absorb_legacy_attributes(d: &mut Disciple) {
    if !d.alive {
        d.attributes.qi.maximum = 0;
        d.attributes.qi.current = 0;
        d.condition = DiscipleCondition::Dead;
        sync_legacy_attributes(d);
        return;
    }
    let neili_delta = d.inner_power - d.attributes.neili.maximum;
    if neili_delta != 0 {
        d.attribute_bonuses.neili += neili_delta;
        recalculate_attribute_maxima(d);
    }
    d.attributes.sect_loyalty = d.loyalty.clamp(0, 100);
    refresh_condition(d);
    sync_legacy_attributes(d);
}

pub fn hydrate_v2_disciple(d: &mut Disciple) {
    if d.martial_progress.proficiencies.is_empty() && !d.skills.is_empty() {
        d.martial_progress.proficiencies = d
            .skills
            .iter()
            .map(|skill| {
                (
                    skill.martial_art_id.clone(),
                    SkillProgress::new(skill.level, skill.experience),
                )
            })
            .collect();
    }
    canonicalize_proficiencies(d);
    if d.martial_progress.proficiencies.is_empty() && !d.martial_art.is_empty() {
        d.martial_progress.proficiencies.insert(
            canonical_skill_id(&d.martial_art),
            SkillProgress::new(25, 0),
        );
    }
    d.martial_art = canonical_skill_id(&d.martial_art);
    let origin = infer_origin(d);
    d.origin_sect_id = Some(origin.clone());
    complete_required_skills(d, &origin);

    if d.martial_schema_version < 1 {
        infer_permanent_bonuses(d);
        d.martial_schema_version = 1;
    }
    recalculate_attribute_maxima(d);
    d.attributes.sect_loyalty = d.loyalty.clamp(0, 100);
    refresh_condition(d);
    sync_legacy_attributes(d);
}

fn canonicalize_proficiencies(d: &mut Disciple) {
    let old = std::mem::take(&mut d.martial_progress.proficiencies);
    for (id, progress) in old {
        let canonical = canonical_skill_id(&id);
        let stored = d
            .martial_progress
            .proficiencies
            .entry(canonical)
            .or_default();
        if progress.level > stored.level
            || (progress.level == stored.level && progress.experience > stored.experience)
        {
            *stored = progress;
        }
    }
}

fn infer_origin(d: &Disciple) -> String {
    if let Some(origin) = d.origin_sect_id.as_deref().filter(|id| *id != "player") {
        return origin.into();
    }
    martial_art_by_id(&d.martial_art)
        .and_then(|art| art.sect_id)
        .filter(|sect| sect != "player")
        .or_else(|| {
            d.martial_progress.proficiencies.keys().find_map(|id| {
                martial_art_by_id(id)
                    .and_then(|art| art.sect_id)
                    .filter(|sect| sect != "player")
            })
        })
        .or_else(|| d.sect_id.clone())
        .unwrap_or_else(|| "player".into())
}

fn rank_tier(rank: &DiscipleRank) -> MartialTier {
    match rank {
        DiscipleRank::Chore => MartialTier::Chore,
        DiscipleRank::Outer => MartialTier::Outer,
        DiscipleRank::Inner | DiscipleRank::Elder => MartialTier::Inner,
    }
}

/// 为旧人物补齐六门基础技能和其身份对应的一整套门派战斗武学。
fn complete_required_skills(d: &mut Disciple, origin: &str) {
    let existing_highest = d
        .martial_progress
        .proficiencies
        .values()
        .map(|progress| progress.level)
        .max()
        .unwrap_or(30);
    let baseline = match d.rank {
        DiscipleRank::Chore => 45,
        DiscipleRank::Outer => 90,
        DiscipleRank::Inner => 160,
        DiscipleRank::Elder => 220,
    };
    let knowledge_id = knowledge_skill_id(origin);
    let knowledge = existing_highest.max(baseline);
    for id in base_skill_ids(origin) {
        let level = if id == knowledge_id {
            knowledge
        } else {
            (baseline * 3 / 4).max(25)
        };
        d.martial_progress
            .proficiencies
            .entry(id)
            .or_insert_with(|| SkillProgress::new(level, 0));
    }
    if let Some(current_sect) = d.sect_id.as_deref().filter(|sect| *sect != origin) {
        d.martial_progress
            .proficiencies
            .entry(knowledge_skill_id(current_sect))
            .or_insert_with(|| SkillProgress::new((baseline * 2 / 3).max(20), 0));
    }
    for art in sect_combat_arts(origin, rank_tier(&d.rank)) {
        d.martial_progress
            .proficiencies
            .entry(art.id)
            .or_insert_with(|| SkillProgress::new(baseline.min(knowledge), 0));
    }
    // 保留已有修为时抬高知识门槛，不倒扣玩家已经取得的等级。
    let faction_peak = d
        .martial_progress
        .proficiencies
        .iter()
        .filter(|(id, _)| {
            martial_art_by_id(id).is_some_and(|art| {
                art.is_combat
                    && art.tier != MartialTier::Basic
                    && art.sect_id.as_deref() == Some(origin)
            })
        })
        .map(|(_, progress)| progress.level)
        .max()
        .unwrap_or(knowledge);
    if let Some(progress) = d.martial_progress.proficiencies.get_mut(&knowledge_id) {
        progress.level = progress.level.max(faction_peak);
    }
}

/// NPC 世界生成使用：按身份装入六项基础与整套对应层级武学。
pub fn assign_sect_curriculum(d: &mut Disciple, origin: &str, combat_level: i32) {
    d.origin_sect_id = Some(origin.into());
    d.martial_progress.proficiencies.clear();
    let knowledge_id = knowledge_skill_id(origin);
    let knowledge = (combat_level + 20).max(30);
    for id in base_skill_ids(origin) {
        let level = if id == knowledge_id {
            knowledge
        } else {
            (combat_level * 3 / 4).max(20)
        };
        d.martial_progress
            .proficiencies
            .insert(id, SkillProgress::new(level, 0));
    }
    let arts = sect_combat_arts(origin, rank_tier(&d.rank));
    for (index, art) in arts.iter().enumerate() {
        d.martial_progress.proficiencies.insert(
            art.id.clone(),
            SkillProgress::new((combat_level - index as i32 * 3).min(knowledge), 0),
        );
    }
    d.martial_art = arts
        .iter()
        .max_by_key(|art| art.atk + art.def + art.spd)
        .map(|art| art.id.clone())
        .unwrap_or_else(|| "basic_unarmed".into());
    d.martial_progress.specialties = vec![origin.into()];
    d.martial_schema_version = 1;
    recalculate_attribute_maxima(d);
    d.attributes.qi.current = d.attributes.qi.maximum;
    d.attributes.spirit.current = d.attributes.spirit.maximum;
    d.attributes.neili.current = d.attributes.neili.maximum;
    d.attributes.energy.current = d.attributes.energy.maximum;
    sync_legacy_attributes(d);
}

fn infer_permanent_bonuses(d: &mut Disciple) {
    let old = AttributeMaxima {
        qi: d.attributes.qi.maximum.max(1),
        spirit: d.attributes.spirit.maximum.max(1),
        neili: d.attributes.neili.maximum.max(d.inner_power).max(1),
        energy: d.attributes.energy.maximum.max(1),
    };
    let calculated = attribute_maxima(d);
    d.attribute_bonuses.neili += (old.neili - calculated.neili).max(0);
    d.attribute_bonuses.energy += (old.energy - calculated.energy).max(0);
    let with_resources = attribute_maxima(d);
    d.attribute_bonuses.qi += (old.qi - with_resources.qi).max(0);
    d.attribute_bonuses.spirit += (old.spirit - with_resources.spirit).max(0);
}

/// 门派战斗武学不能越过对应知识等级；基础技能和知识本身不受此限制。
pub fn gain_skill_experience(d: &mut Disciple, art_id: &str, amount: i64) -> i32 {
    let art_id = canonical_skill_id(art_id);
    let cap = knowledge_skill_for_art(&art_id).map(|knowledge_id| skill_level(d, &knowledge_id));
    let progress = d.martial_progress.proficiencies.entry(art_id).or_default();
    let old_level = progress.level;
    progress.gain_experience(amount);
    if let Some(cap) = cap {
        progress.level = progress.level.min(cap);
        if progress.level >= cap {
            progress.experience = progress.experience.min(
                i64::from(progress.level.saturating_add(1))
                    .pow(2)
                    .saturating_sub(1),
            );
        }
    }
    let gained = progress.level - old_level;
    recalculate_attribute_maxima(d);
    gained
}

pub fn refresh_condition(d: &mut Disciple) {
    d.condition = if d.attributes.qi.maximum < 1 || d.attributes.spirit.maximum < 1 {
        DiscipleCondition::Dead
    } else if d.attributes.qi.current <= 0 {
        DiscipleCondition::SeriouslyInjured
    } else if d.attributes.spirit.current <= 0 {
        DiscipleCondition::Unconscious
    } else if d.attributes.energy.current <= 0 {
        DiscipleCondition::Exhausted
    } else {
        DiscipleCondition::Healthy
    };
    d.alive = d.condition != DiscipleCondition::Dead;
}

pub fn can_act(d: &Disciple) -> bool {
    d.alive && d.away_months <= 0 && d.condition == DiscipleCondition::Healthy
}

pub fn generate_starting_disciples(rng: &mut impl Rng) -> Vec<Disciple> {
    let mut d1 = generate_disciple(rng, 15);
    d1.attributes.sect_loyalty = rand_range(rng, 60, 80);
    d1.attribute_bonuses.neili += rand_range(rng, 5, 15);
    recalculate_attribute_maxima(&mut d1);
    d1.name = generate_name(rng);
    sync_legacy_attributes(&mut d1);

    let mut d2 = generate_disciple(rng, 10);
    d2.attributes.sect_loyalty = rand_range(rng, 55, 75);
    d2.attribute_bonuses.neili += rand_range(rng, 0, 10);
    recalculate_attribute_maxima(&mut d2);
    d2.name = generate_name(rng);
    sync_legacy_attributes(&mut d2);

    vec![d1, d2]
}

pub fn get_combat_score(d: &Disciple) -> i32 {
    let art = martial_art_by_id(&d.martial_art)
        .or_else(|| martial_art_by_id("basic_unarmed"))
        .expect("basic unarmed must exist");
    let category_total: i32 = SkillCategory::COMBAT
        .into_iter()
        .map(|category| {
            d.martial_progress
                .proficiencies
                .iter()
                .filter(|(id, _)| martial_art_by_id(id).is_some_and(|art| art.category == category))
                .map(|(_, progress)| progress.level)
                .max()
                .unwrap_or(0)
        })
        .sum();
    let energy_ratio =
        d.attributes.energy.current.max(0) as f64 / d.attributes.energy.maximum.max(1) as f64;
    (d.talent as f64 * 0.2
        + d.attributes.neili.maximum as f64 * 0.3
        + category_total.min(2500) as f64 * 0.025
        + ((art.atk + art.def + art.spd) * 3) as f64
        + energy_ratio.min(1.5) * 10.0
        + d.attributes.sect_loyalty as f64 * 0.05) as i32
}

pub fn get_sect_combat_power(disciples: &[Disciple]) -> i32 {
    let alive: Vec<&Disciple> = disciples.iter().filter(|d| d.alive).collect();
    if alive.is_empty() {
        return 10;
    }
    let total: i32 = alive.iter().map(|d| get_combat_score(d)).sum();
    (total / alive.len() as i32) + (alive.len() as i32 * 5)
}

/// 结算入门时长、年龄和门忠。修为与资源变化只由弟子本月实际行动产生。
pub fn settle_month(disciples: &mut [Disciple], morale: i32) {
    for d in disciples.iter_mut() {
        if !d.alive {
            continue;
        }
        d.months_in_sect += 1;
        if d.months_in_sect % 12 == 0 {
            apply_birthday(d);
        }

        let loyalty_delta = (morale - 50) / 25;
        d.attributes.sect_loyalty = clamp(d.attributes.sect_loyalty + loyalty_delta, 0, 100);
        refresh_condition(d);
        sync_legacy_attributes(d);
    }
}

/// 叛逃检查，返回叛逃者名单
pub fn check_desertion(rng: &mut impl Rng, disciples: &mut Vec<Disciple>) -> Vec<String> {
    let mut deserters = vec![];
    disciples.retain(|d| {
        if !d.alive {
            return false;
        }
        if d.attributes.sect_loyalty < 15 && rng.gen_bool(0.25) {
            deserters.push(d.name.clone());
            false
        } else {
            true
        }
    });
    deserters
}

/// 弟子在入门周年增长一岁。年龄仅带来固定点数变化，修行、药物等额外上限不会丢失。
pub fn apply_birthday(d: &mut Disciple) {
    d.age += 1;
    recalculate_attribute_maxima(d);
    refresh_condition(d);
    sync_legacy_attributes(d);
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::{rngs::StdRng, SeedableRng};

    #[test]
    fn age_curve_grows_then_declines_by_fixed_points() {
        assert_eq!(age_qi_modifier(35) - age_qi_modifier(34), 3);
        assert_eq!(age_qi_modifier(36) - age_qi_modifier(35), -4);
        assert_eq!(age_spirit_modifier(35) - age_spirit_modifier(34), 2);
        assert_eq!(age_spirit_modifier(36) - age_spirit_modifier(35), -3);
    }

    #[test]
    fn zero_resources_set_expected_conditions() {
        let mut d = Disciple::default();
        d.attributes.qi.current = 0;
        refresh_condition(&mut d);
        assert_eq!(d.condition, DiscipleCondition::SeriouslyInjured);

        d.attributes.qi.current = 10;
        d.attributes.spirit.current = 0;
        refresh_condition(&mut d);
        assert_eq!(d.condition, DiscipleCondition::Unconscious);

        d.attributes.spirit.maximum = 0;
        refresh_condition(&mut d);
        assert_eq!(d.condition, DiscipleCondition::Dead);
        assert!(!d.alive);
    }

    #[test]
    fn generated_disciple_has_consistent_v3_attributes() {
        let mut rng = StdRng::seed_from_u64(7);
        let d = generate_disciple(&mut rng, 10);
        assert_eq!(d.inner_power, d.attributes.neili.maximum);
        assert_eq!(d.loyalty, d.attributes.sect_loyalty);
        assert!(d
            .martial_progress
            .proficiencies
            .contains_key(&d.martial_art));
        assert!(d
            .skills
            .iter()
            .any(|skill| skill.martial_art_id == d.martial_art));
        assert!(d.attributes.qi.maximum > 0);
        let origin = d.origin_sect_id.as_deref().unwrap();
        assert!(base_skill_ids(origin)
            .iter()
            .all(|id| d.martial_progress.proficiencies.contains_key(id)));
        assert!(
            d.martial_progress
                .proficiencies
                .keys()
                .filter_map(|id| martial_art_by_id(id))
                .filter(|art| art.tier == MartialTier::Chore)
                .count()
                >= 2
        );

        let mut progress = SkillProgress::new(4, 24);
        assert_eq!(progress.gain_experience(1), 1);
        assert_eq!(progress, SkillProgress::new(5, 0));
        let legacy: SkillProgress = serde_json::from_str("50").unwrap();
        assert_eq!(legacy, SkillProgress::new(50, 0));
    }

    #[test]
    fn knowledge_caps_faction_combat_but_not_basics() {
        let mut d = Disciple::default();
        d.origin_sect_id = Some("wudang".into());
        d.martial_schema_version = 1;
        d.martial_progress.proficiencies = BTreeMap::from([
            ("wudang_knowledge".into(), SkillProgress::new(30, 0)),
            ("wudang_chore_unarmed".into(), SkillProgress::new(30, 0)),
            ("basic_unarmed".into(), SkillProgress::new(30, 0)),
        ]);
        gain_skill_experience(&mut d, "wudang_chore_unarmed", 100_000);
        gain_skill_experience(&mut d, "basic_unarmed", 100_000);
        assert_eq!(
            d.martial_progress.proficiencies["wudang_chore_unarmed"].level,
            30
        );
        assert!(d.martial_progress.proficiencies["basic_unarmed"].level > 30);
    }

    #[test]
    fn derived_maxima_include_skills_aptitudes_and_permanent_bonuses() {
        let mut d = Disciple::default();
        d.origin_sect_id = Some("wudang".into());
        d.martial_progress.proficiencies = BTreeMap::from([
            ("basic_force".into(), SkillProgress::new(60, 0)),
            ("wudang_knowledge".into(), SkillProgress::new(80, 0)),
        ]);
        let before = attribute_maxima(&d);
        d.attribute_bonuses.neili = 7;
        d.attribute_bonuses.energy = 9;
        d.attribute_bonuses.qi = 11;
        d.attribute_bonuses.spirit = 13;
        let after = attribute_maxima(&d);
        assert_eq!(after.neili - before.neili, 7);
        assert_eq!(after.energy - before.energy, 9);
        assert_eq!(after.qi - before.qi, 11 + 7 / 2);
        assert_eq!(after.spirit - before.spirit, 13 + 9 / 2);
    }

    #[test]
    fn old_chinese_skill_ids_are_migrated_and_completed() {
        let mut d = Disciple {
            origin_sect_id: None,
            sect_id: Some("wudang".into()),
            martial_art: "taiji".into(),
            martial_progress: MartialProgress {
                proficiencies: BTreeMap::from([("基本内功".into(), SkillProgress::new(55, 0))]),
                ..MartialProgress::default()
            },
            ..Disciple::default()
        };
        hydrate_v2_disciple(&mut d);
        assert_eq!(d.origin_sect_id.as_deref(), Some("wudang"));
        assert!(!d.martial_progress.proficiencies.contains_key("基本内功"));
        assert!(base_skill_ids("wudang")
            .iter()
            .all(|id| d.martial_progress.proficiencies.contains_key(id)));
        assert_eq!(sect_combat_arts("wudang", MartialTier::Outer).len(), 5);
        assert!(sect_combat_arts("wudang", MartialTier::Outer)
            .iter()
            .all(|art| d.martial_progress.proficiencies.contains_key(&art.id)));
    }
}
