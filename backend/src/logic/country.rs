use crate::logic::sect;
use crate::models::sect::{default_countries, Country, SectState};
use crate::models::{GameEvent, GameState};
use std::collections::BTreeMap;

const NEUTRAL_PROSPERITY: i32 = 70;
const NEUTRAL_ORDER: i32 = 65;

/// 国势只在 0..=100 内参与结算，旧档或外部输入越界时先行钳制。
pub fn clamp_value(value: i32) -> i32 {
    value.clamp(0, 100)
}

/// 商路倍率：繁荣七十为常态，极端国势也只令收益浮动至 85%..=115%。
pub fn market_percent(prosperity: i32) -> i32 {
    (100 + (clamp_value(prosperity) - 70) / 2).clamp(85, 115)
}

/// 行路倍率：治安六十五为常态，极端国势也只令历练浮动至 85%..=115%。
pub fn safety_percent(order: i32) -> i32 {
    (100 + (clamp_value(order) - 65) / 2).clamp(85, 115)
}

/// 外务同时受商路与行路影响，但只取两者均值，避免重复连乘。
pub fn external_percent(prosperity: i32, order: i32) -> i32 {
    (market_percent(prosperity) + safety_percent(order)) / 2
}

/// 战争支援只取规范化后的国势基础值。
pub fn war_support_values(prosperity: i32, order: i32) -> i32 {
    clamp_value(prosperity) / 6 + clamp_value(order) / 3
}

pub fn find_country<'a>(countries: &'a [Country], country_id: &str) -> Option<&'a Country> {
    countries.iter().find(|country| country.id == country_id)
}

pub fn country_values_from_slice(countries: &[Country], country_id: &str) -> (i32, i32) {
    find_country(countries, country_id)
        .map(|country| (clamp_value(country.prosperity), clamp_value(country.order)))
        .unwrap_or((NEUTRAL_PROSPERITY, NEUTRAL_ORDER))
}

pub fn country_values(state: &GameState, country_id: &str) -> (i32, i32) {
    country_values_from_slice(&state.countries, country_id)
}

pub fn market_percent_for(state: &GameState, country_id: &str) -> i32 {
    let (prosperity, _) = country_values(state, country_id);
    market_percent(prosperity)
}

pub fn safety_percent_for(state: &GameState, country_id: &str) -> i32 {
    let (_, order) = country_values(state, country_id);
    safety_percent(order)
}

pub fn external_percent_for(state: &GameState, country_id: &str) -> i32 {
    let (prosperity, order) = country_values(state, country_id);
    external_percent(prosperity, order)
}

pub fn war_support(state: &GameState, country_id: &str) -> i32 {
    let (prosperity, order) = country_values(state, country_id);
    war_support_values(prosperity, order)
}

/// 正值收益按百分比缩放；负数不会借倍率反向变成支出。
pub fn scale_positive(value: i32, percent: i32) -> i32 {
    if value <= 0 {
        return 0;
    }
    i64::from(value)
        .saturating_mul(i64::from(percent.max(0)))
        .saturating_div(100)
        .min(i64::from(i32::MAX)) as i32
}

pub fn scale_positive_i64(value: i64, percent: i32) -> i64 {
    if value <= 0 {
        return 0;
    }
    value
        .saturating_mul(i64::from(percent.max(0)))
        .saturating_div(100)
}

/// 补齐固定四国，保留旧档已有进度，并统一国名及数值边界。
pub fn normalize_countries(countries: &mut Vec<Country>) {
    for canonical in default_countries() {
        if let Some(existing) = countries
            .iter_mut()
            .find(|country| country.id == canonical.id)
        {
            existing.name = canonical.name;
            existing.prosperity = clamp_value(existing.prosperity);
            existing.order = clamp_value(existing.order);
        } else {
            countries.push(canonical);
        }
    }
    for country in countries {
        country.prosperity = clamp_value(country.prosperity);
        country.order = clamp_value(country.order);
    }
}

/// 调整一国国势并返回钳制后的实际变动，供战争纪事准确落笔。
pub fn adjust_country(
    state: &mut GameState,
    country_id: &str,
    prosperity_delta: i32,
    order_delta: i32,
) -> (i32, i32) {
    let Some(country) = state
        .countries
        .iter_mut()
        .find(|country| country.id == country_id)
    else {
        return (0, 0);
    };
    let before_prosperity = clamp_value(country.prosperity);
    let before_order = clamp_value(country.order);
    country.prosperity = before_prosperity
        .saturating_add(prosperity_delta)
        .clamp(0, 100);
    country.order = before_order.saturating_add(order_delta).clamp(0, 100);
    (
        country.prosperity - before_prosperity,
        country.order - before_order,
    )
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
struct CountrySignal {
    prosperity: i32,
    order: i32,
}

fn country_signal(sects: &[&SectState]) -> CountrySignal {
    if sects.is_empty() {
        return CountrySignal::default();
    }
    let count = sects.len() as i64;
    let morale = sects
        .iter()
        .map(|sect| i64::from(sect.attributes.morale.clamp(0, 100)))
        .sum::<i64>();
    let morality = sects
        .iter()
        .map(|sect| i64::from(sect.attributes.morality.clamp(0, 100)))
        .sum::<i64>();
    let warehouse = sects
        .iter()
        .map(|sect| i64::from(sect::building_effectiveness(sect, "warehouse").max(0)))
        .sum::<i64>();

    let prosperity_signal =
        threshold_signal(morale, count, 65, 40) + threshold_signal(warehouse, count, 115, 75);
    let order_signal =
        threshold_signal(morality, count, 60, 40) + threshold_signal(morale, count, 70, 35);
    CountrySignal {
        prosperity: prosperity_signal.signum(),
        order: order_signal.signum(),
    }
}

fn threshold_signal(total: i64, count: i64, high: i64, low: i64) -> i32 {
    if total >= high.saturating_mul(count) {
        1
    } else if total <= low.saturating_mul(count) {
        -1
    } else {
        0
    }
}

/// 由本国诸派月况推动基础国势；无随机数，同一月初快照必得同一结果。
///
/// 所有实际变化合并为至多一条世界纪事。
pub fn evolve_monthly(state: &mut GameState) -> Option<GameEvent> {
    normalize_countries(&mut state.countries);
    let signals = state
        .countries
        .iter()
        .map(|country| {
            let sects = std::iter::once(&state.sect)
                .chain(state.npc_sects.iter())
                .filter(|sect| sect.country_id == country.id)
                .collect::<Vec<_>>();
            (country.id.clone(), country_signal(&sects))
        })
        .collect::<BTreeMap<_, _>>();

    let mut reports = Vec::new();
    for country in &mut state.countries {
        let signal = signals.get(&country.id).copied().unwrap_or_default();
        let before_prosperity = country.prosperity;
        let before_order = country.order;
        country.prosperity = country
            .prosperity
            .saturating_add(signal.prosperity)
            .clamp(0, 100);
        country.order = country.order.saturating_add(signal.order).clamp(0, 100);
        let prosperity_delta = country.prosperity - before_prosperity;
        let order_delta = country.order - before_order;
        if prosperity_delta != 0 {
            let phrase = if prosperity_delta > 0 {
                "商旅渐盛"
            } else {
                "百业稍疲"
            };
            reports.push(format!(
                "{}{}（繁荣{}）",
                country.name,
                phrase,
                signed_delta(prosperity_delta)
            ));
        }
        if order_delta != 0 {
            let phrase = if order_delta > 0 {
                "道途稍靖"
            } else {
                "盗风渐炽"
            };
            reports.push(format!(
                "{}{}（治安{}）",
                country.name,
                phrase,
                signed_delta(order_delta)
            ));
        }
    }
    (!reports.is_empty()).then(|| GameEvent {
        text: format!("【四境月报】{}。", reports.join("；")),
        mood: "neutral".into(),
        year: state.year,
        month: state.month,
        category: "world".into(),
    })
}

fn signed_delta(delta: i32) -> String {
    if delta > 0 {
        format!("＋{delta}")
    } else {
        format!("－{}", delta.saturating_abs())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn country_multipliers_obey_neutral_points_and_boundaries() {
        assert_eq!(market_percent(-50), 85);
        assert_eq!(market_percent(0), 85);
        assert_eq!(market_percent(70), 100);
        assert_eq!(market_percent(100), 115);
        assert_eq!(market_percent(500), 115);

        assert_eq!(safety_percent(-50), 85);
        assert_eq!(safety_percent(0), 85);
        assert_eq!(safety_percent(65), 100);
        assert_eq!(safety_percent(100), 115);
        assert_eq!(safety_percent(500), 115);
        assert_eq!(external_percent(70, 65), 100);
        assert_eq!(war_support_values(100, 100), 49);

        assert_eq!(scale_positive(101, 85), 85);
        assert_eq!(scale_positive_i64(101, 115), 116);
        assert_eq!(scale_positive(-5, 115), 0);
    }

    #[test]
    fn missing_country_uses_neutral_values_and_normalization_preserves_progress() {
        let state = GameState {
            countries: vec![],
            ..GameState::default()
        };
        assert_eq!(country_values(&state, "unknown"), (70, 65));
        assert_eq!(market_percent_for(&state, "unknown"), 100);
        assert_eq!(safety_percent_for(&state, "unknown"), 100);

        let mut countries = vec![Country {
            id: "song".into(),
            name: "旧名".into(),
            prosperity: 101,
            order: -1,
            population: 11200,
        }];
        normalize_countries(&mut countries);
        assert_eq!(countries.len(), 4);
        let song = find_country(&countries, "song").unwrap();
        assert_eq!(song.name, "大宋");
        assert_eq!((song.prosperity, song.order), (100, 0));
    }

    #[test]
    fn monthly_evolution_is_deterministic_bounded_and_at_most_one_step() {
        let mut base = GameState::default();
        base.countries = default_countries();
        base.sect.country_id = "song".into();
        base.sect.attributes.morale = 80;
        base.sect.attributes.morality = 80;
        base.sect
            .buildings
            .iter_mut()
            .find(|building| building.id == "warehouse")
            .unwrap()
            .level = 3;
        base.npc_sects.clear();
        let mut first = base.clone();
        let mut second = base;
        let before = first.countries.clone();

        let first_event = evolve_monthly(&mut first);
        let second_event = evolve_monthly(&mut second);

        assert_eq!(
            serde_json::to_value(&first.countries).unwrap(),
            serde_json::to_value(&second.countries).unwrap()
        );
        assert_eq!(
            serde_json::to_value(&first_event).unwrap(),
            serde_json::to_value(&second_event).unwrap()
        );
        for (old, new) in before.iter().zip(&first.countries) {
            assert!((new.prosperity - old.prosperity).abs() <= 1);
            assert!((new.order - old.order).abs() <= 1);
            assert!((0..=100).contains(&new.prosperity));
            assert!((0..=100).contains(&new.order));
        }
        let song = find_country(&first.countries, "song").unwrap();
        assert_eq!((song.prosperity, song.order), (86, 63));
        let event = first_event.expect("本国两项国势均应上升");
        assert_eq!(event.category, "world");
        assert!(event.text.starts_with("【四境月报】"));
        assert!(event.text.contains("大宋商旅渐盛（繁荣＋1）"));
        assert!(event.text.contains("大宋道途稍靖（治安＋1）"));
    }

    #[test]
    fn monthly_evolution_respects_zero_and_hundred_boundaries() {
        let mut high = GameState::default();
        high.npc_sects.clear();
        high.countries = vec![Country {
            id: "song".into(),
            name: "大宋".into(),
            prosperity: 100,
            order: 100,
            population: 11200,
        }];
        high.sect.attributes.morale = 100;
        high.sect.attributes.morality = 100;
        high.sect
            .buildings
            .iter_mut()
            .find(|building| building.id == "warehouse")
            .unwrap()
            .level = 3;
        evolve_monthly(&mut high);
        let song = find_country(&high.countries, "song").unwrap();
        assert_eq!((song.prosperity, song.order), (100, 100));

        let mut low = GameState::default();
        low.npc_sects.clear();
        low.countries = vec![Country {
            id: "song".into(),
            name: "大宋".into(),
            prosperity: 0,
            order: 0,
            population: 11200,
        }];
        low.sect.attributes.morale = 0;
        low.sect.attributes.morality = 0;
        low.sect
            .buildings
            .iter_mut()
            .find(|building| building.id == "warehouse")
            .unwrap()
            .condition = 0;
        evolve_monthly(&mut low);
        let song = find_country(&low.countries, "song").unwrap();
        assert_eq!((song.prosperity, song.order), (0, 0));
    }

    #[test]
    fn two_negative_signals_still_move_each_country_value_by_only_one() {
        let mut state = GameState::default();
        state.npc_sects.clear();
        state.sect.country_id = "song".into();
        state.sect.attributes.morale = 30;
        state.sect.attributes.morality = 30;
        state
            .sect
            .buildings
            .iter_mut()
            .find(|building| building.id == "warehouse")
            .unwrap()
            .condition = 50;
        let song = state
            .countries
            .iter_mut()
            .find(|country| country.id == "song")
            .unwrap();
        song.prosperity = 50;
        song.order = 50;

        let event = evolve_monthly(&mut state).expect("低迷月况应写入四境月报");

        let song = find_country(&state.countries, "song").unwrap();
        assert_eq!((song.prosperity, song.order), (49, 49));
        assert!(event.text.contains("大宋百业稍疲（繁荣－1）"));
        assert!(event.text.contains("大宋盗风渐炽（治安－1）"));
    }

    #[test]
    fn adjust_country_reports_only_actual_clamped_change() {
        let mut state = GameState::default();
        let song = state
            .countries
            .iter_mut()
            .find(|country| country.id == "song")
            .unwrap();
        song.prosperity = 100;
        song.order = 1;

        assert_eq!(adjust_country(&mut state, "song", 5, -2), (0, -1));
        assert_eq!(adjust_country(&mut state, "missing", 1, 1), (0, 0));
    }
}
