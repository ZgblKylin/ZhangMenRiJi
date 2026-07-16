use crate::models::attributes::{
    AcquiredAttributes, Aptitudes, DiscipleCondition, MartialProgress, ResourcePool, SkillProgress,
};
use crate::models::martial_art::all_martial_arts;
use crate::models::{Disciple, MartialArt};
use rand::Rng;

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
    let inner_power = rand_range(rng, 20, 45);
    let age = rand_range(rng, 15, 28);
    let arts = all_martial_arts();
    let eligible: Vec<&MartialArt> = arts
        .iter()
        .filter(|a| a.req_talent <= talent + 10)
        .collect();
    let art = if eligible.is_empty() {
        &arts[4]
    } else {
        pick(rng, &eligible)
    };

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
        inner_power,
        martial_art: art.id.clone(),
        loyalty: rand_range(rng, 40, 70),
        months_in_sect: 0,
        alive: true,
        age,
        aptitudes,
        ..Disciple::default()
    };
    disciple.attributes = initial_attributes(&disciple, inner_power);
    disciple.martial_progress = MartialProgress {
        proficiencies: [(art.id.clone(), SkillProgress::new(50, 0))]
            .into_iter()
            .collect(),
        specialties: vec![art.art_type.clone()],
        private_books: vec![],
    };
    sync_legacy_attributes(&mut disciple);
    disciple
}

fn initial_attributes(d: &Disciple, neili: i32) -> AcquiredAttributes {
    let energy = 30 + d.aptitudes.intelligence;
    let mut attributes = AcquiredAttributes {
        neili: ResourcePool {
            current: neili,
            maximum: neili,
        },
        energy: ResourcePool {
            current: energy,
            maximum: energy,
        },
        sect_loyalty: d.loyalty,
        morality: (40 + d.aptitudes.fortune / 2).clamp(0, 100),
        ..AcquiredAttributes::default()
    };
    attributes.qi.maximum = qi_maximum(d.age, &d.aptitudes, neili);
    attributes.qi.current = attributes.qi.maximum;
    attributes.spirit.maximum = spirit_maximum(d.age, &d.aptitudes, energy);
    attributes.spirit.current = attributes.spirit.maximum;
    attributes
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

pub fn spirit_maximum(age: i32, aptitude: &Aptitudes, max_energy: i32) -> i32 {
    20 + aptitude.intelligence * 4 + max_energy / 2 + age_spirit_modifier(age)
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
    d.attributes.neili.maximum = d.inner_power.max(0);
    d.attributes.neili.current =
        (d.attributes.neili.current + neili_delta).clamp(0, d.attributes.neili.maximum);
    d.attributes.sect_loyalty = d.loyalty.clamp(0, 100);
    refresh_condition(d);
    sync_legacy_attributes(d);
}

pub fn hydrate_v2_disciple(d: &mut Disciple) {
    if d.martial_progress.proficiencies.is_empty() {
        d.attributes.neili = ResourcePool {
            current: d.inner_power,
            maximum: d.inner_power,
        };
        d.attributes.sect_loyalty = d.loyalty;
        d.attributes.qi.maximum = qi_maximum(d.age, &d.aptitudes, d.inner_power);
        d.attributes.qi.current = d.attributes.qi.maximum;
        d.attributes.spirit.maximum =
            spirit_maximum(d.age, &d.aptitudes, d.attributes.energy.maximum);
        d.attributes.spirit.current = d.attributes.spirit.maximum;
        d.martial_progress
            .proficiencies
            .insert(d.martial_art.clone(), SkillProgress::new(25, 0));
    }
    refresh_condition(d);
    sync_legacy_attributes(d);
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
    let d1_neili = rand_range(rng, 25, 40);
    d1.attributes.neili = ResourcePool {
        current: d1_neili,
        maximum: d1_neili,
    };
    d1.name = generate_name(rng);
    sync_legacy_attributes(&mut d1);

    let mut d2 = generate_disciple(rng, 10);
    d2.attributes.sect_loyalty = rand_range(rng, 55, 75);
    let d2_neili = rand_range(rng, 20, 35);
    d2.attributes.neili = ResourcePool {
        current: d2_neili,
        maximum: d2_neili,
    };
    d2.name = generate_name(rng);
    sync_legacy_attributes(&mut d2);

    vec![d1, d2]
}

pub fn get_combat_score(d: &Disciple) -> i32 {
    let arts = all_martial_arts();
    let art = arts
        .iter()
        .find(|a| a.id == d.martial_art)
        .unwrap_or(&arts[4]);
    let proficiency = d
        .martial_progress
        .proficiencies
        .get(&d.martial_art)
        .map(|progress| progress.level)
        .unwrap_or(0)
        .min(500) as f64;
    (d.talent as f64 * 0.2
        + d.attributes.neili.maximum as f64 * 0.3
        + proficiency * 0.08
        + ((art.atk + art.def + art.spd) * 3) as f64
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

/// 月度弟子成长
pub fn monthly_growth(rng: &mut impl Rng, disciples: &mut [Disciple], morale: i32) {
    for d in disciples.iter_mut() {
        if !d.alive {
            continue;
        }
        d.months_in_sect += 1;
        if d.months_in_sect % 12 == 0 {
            apply_birthday(d);
        }

        let neili_recovery = 2 + d.aptitudes.constitution / 10;
        let energy_recovery = 3 + d.aptitudes.intelligence / 10;
        let qi_recovery = 6 + d.aptitudes.constitution / 4;
        let spirit_recovery = 6 + d.aptitudes.intelligence / 4;
        recover(&mut d.attributes.neili, neili_recovery);
        recover(&mut d.attributes.energy, energy_recovery);
        recover(&mut d.attributes.qi, qi_recovery);
        recover(&mut d.attributes.spirit, spirit_recovery);

        let loyalty_delta = (morale - 50) / 20 + rand_range(rng, -2, 2);
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

fn recover(pool: &mut ResourcePool, amount: i32) {
    pool.current = (pool.current + amount).clamp(0, pool.maximum.max(0));
}

/// 弟子在入门周年增长一岁。年龄仅带来固定点数变化，修行、药物等额外上限不会丢失。
pub fn apply_birthday(d: &mut Disciple) {
    let old_age = d.age;
    d.age += 1;
    d.attributes.qi.maximum += age_qi_modifier(d.age) - age_qi_modifier(old_age);
    d.attributes.spirit.maximum += age_spirit_modifier(d.age) - age_spirit_modifier(old_age);
    d.attributes.qi.current = d.attributes.qi.current.min(d.attributes.qi.maximum.max(0));
    d.attributes.spirit.current = d
        .attributes
        .spirit
        .current
        .min(d.attributes.spirit.maximum.max(0));
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
        assert!(d.attributes.qi.maximum > 0);
    }
}
