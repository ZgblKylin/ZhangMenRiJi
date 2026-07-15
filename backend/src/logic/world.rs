use crate::logic::disciple::{generate_disciple, sync_legacy_attributes};
use crate::models::attributes::{Department, DiscipleRank, MartialProgress};
use crate::models::sect::{default_buildings, Building, SectAttributes, SectPolicy, SectState};
use crate::models::Disciple;
use rand::{rngs::StdRng, SeedableRng};
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
        let prestige = 58 + (sect_index as i32 * 7 % 35);
        let mut buildings = default_buildings();
        buildings.push(Building {
            id: format!("{}_landmark", template.id),
            name: template.landmark.into(),
            level: 2 + (sect_index as i32 % 3),
            ..Building::default()
        });
        let sect = SectState {
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
            buildings,
            inventory: BTreeMap::from([
                ("粮秣".into(), 120 + sect_index as i32 * 3),
                ("草药".into(), 30 + sect_index as i32),
                ("精铁".into(), 18 + sect_index as i32 % 12),
            ]),
            public_books: vec![
                format!("{}_foundation", template.id),
                template.signature.into(),
            ],
            martial_research: BTreeMap::from([(template.signature.into(), 180 + prestige as i64)]),
            relations: BTreeMap::new(),
            active_orders: vec![],
        };

        for (member_index, name) in template.members.iter().enumerate() {
            let mut disciple = generate_disciple(&mut rng, 18 - member_index as i32 * 3);
            disciple.id = format!("npc_{}_{}", template.id, member_index + 1);
            disciple.sect_id = Some(template.id.into());
            disciple.name = (*name).into();
            disciple.martial_art = template.signature.into();
            disciple.rank = if member_index == 0 {
                DiscipleRank::Elder
            } else {
                DiscipleRank::Inner
            };
            disciple.department = Some(match member_index {
                0 => Department::Transmission,
                1 => Department::ExternalAffairs,
                _ => Department::Stewardship,
            });
            disciple.attributes.morality = template.morality;
            disciple.attributes.reputation = prestige / 2 + 20 - member_index as i32 * 4;
            disciple.attributes.attainment = 800 - member_index as i64 * 180 + prestige as i64 * 4;
            disciple.attributes.neili.maximum += 35 - member_index as i32 * 8;
            disciple.attributes.neili.current = disciple.attributes.neili.maximum;
            disciple.martial_progress = MartialProgress {
                proficiencies: BTreeMap::from([
                    (
                        format!("{}_foundation", template.id),
                        180 - member_index as i64 * 30,
                    ),
                    (template.signature.into(), 320 - member_index as i64 * 55),
                ]),
                specialties: vec!["门派绝学".into()],
                private_books: vec![],
            };
            sync_legacy_attributes(&mut disciple);
            disciples.push(disciple);
        }
        sects.push(sect);
    }

    initialize_relations(&mut sects);
    for (index, name) in WANDERERS.iter().enumerate() {
        let mut disciple = generate_disciple(&mut rng, 14);
        disciple.id = format!("wanderer_{}", index + 1);
        disciple.sect_id = None;
        disciple.name = (*name).into();
        disciple.rank = DiscipleRank::Elder;
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
}

#[cfg(test)]
mod tests {
    use super::*;

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
}
