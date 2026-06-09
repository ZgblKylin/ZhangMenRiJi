use rand::Rng;
use crate::models::{Disciple, MartialArt};
use crate::models::martial_art::all_martial_arts;

const SURNAMES: &[&str] = &["风","云","萧","柳","慕容","上官","南宫","令狐","沈","陆","叶","林","楚","苏","燕","秦","白","莫","顾","谢"];
const GIVEN_MALE: &[&str] = &["清扬","无忌","破军","铁心","孤鸿","凌霄","子陵","天行","断水","逐云","惊鸿","寒江","远山","行空","九渊","纯钧","赤霄","湛卢","承影","泰阿"];
const GIVEN_FEMALE: &[&str] = &["若兰","灵素","紫烟","幽月","凝霜","凤歌","雪晴","梦蝶","碧落","紫菱","冰雁","霜华","念慈","倚天","芷若","飞燕","语嫣","龙儿","莫愁","秋水"];

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
    let given = if rng.gen_bool(0.5) { pick(rng, GIVEN_MALE) } else { pick(rng, GIVEN_FEMALE) };
    format!("{}{}", surname, given)
}

pub fn generate_disciple(rng: &mut impl Rng, talent_bonus: i32) -> Disciple {
    let talent = clamp(rand_range(rng, 20, 70) + talent_bonus, 10, 95);
    let inner_power = rand_range(rng, 10, 30);
    let arts = all_martial_arts();
    let eligible: Vec<&MartialArt> = arts.iter().filter(|a| a.req_talent <= talent + 10).collect();
    let art = if eligible.is_empty() { &arts[4] } else { pick(rng, &eligible) };

    Disciple {
        id: format!("d{}_{}", std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_millis(), rand_range(rng, 1000, 9999)),
        name: generate_name(rng),
        talent,
        inner_power,
        martial_art: art.id.clone(),
        loyalty: rand_range(rng, 40, 70),
        months_in_sect: 0,
        alive: true,
    }
}

pub fn generate_starting_disciples(rng: &mut impl Rng) -> Vec<Disciple> {
    let mut d1 = generate_disciple(rng, 15);
    d1.loyalty = rand_range(rng, 60, 80);
    d1.inner_power = rand_range(rng, 25, 40);
    d1.name = generate_name(rng);

    let mut d2 = generate_disciple(rng, 10);
    d2.loyalty = rand_range(rng, 55, 75);
    d2.inner_power = rand_range(rng, 20, 35);
    d2.name = generate_name(rng);

    vec![d1, d2]
}

pub fn get_combat_score(d: &Disciple) -> i32 {
    let arts = all_martial_arts();
    let art = arts.iter().find(|a| a.id == d.martial_art).unwrap_or(&arts[4]);
    (d.talent as f64 * 0.3 + d.inner_power as f64 * 0.3 + ((art.atk + art.def + art.spd) * 3) as f64 + d.loyalty as f64 * 0.1) as i32
}

pub fn get_sect_combat_power(disciples: &[Disciple]) -> i32 {
    let alive: Vec<&Disciple> = disciples.iter().filter(|d| d.alive).collect();
    if alive.is_empty() { return 10; }
    let total: i32 = alive.iter().map(|d| get_combat_score(d)).sum();
    (total / alive.len() as i32) + (alive.len() as i32 * 5)
}

/// 月度弟子成长
pub fn monthly_growth(rng: &mut impl Rng, disciples: &mut [Disciple], morale: i32) {
    for d in disciples.iter_mut() {
        if !d.alive { continue; }
        d.months_in_sect += 1;
        let gain = rand_range(rng, 0, 3) + (morale as f64 / 100.0 * 2.0) as i32;
        d.inner_power = clamp(d.inner_power + gain, 10, 100);
        let loyalty_delta = (morale - 50) / 20 + rand_range(rng, -2, 2);
        d.loyalty = clamp(d.loyalty + loyalty_delta, 0, 100);
    }
}

/// 叛逃检查，返回叛逃者名单
pub fn check_desertion(rng: &mut impl Rng, disciples: &mut Vec<Disciple>) -> Vec<String> {
    let mut deserters = vec![];
    disciples.retain(|d| {
        if !d.alive { return false; }
        if d.loyalty < 15 && rng.gen_bool(0.25) {
            deserters.push(d.name.clone());
            false
        } else {
            true
        }
    });
    deserters
}
