use rand::Rng;
use crate::models::GameState;
use crate::models::tournament::{TournamentRecord, TournamentResult};
use crate::logic::disciple as disc;

pub fn run_tournament(rng: &mut impl Rng, state: &mut GameState) -> TournamentResult {
    let power = disc::get_sect_combat_power(&state.disciples);
    let total_sects = 8 + state.year / 2;
    let rank_base = (total_sects as f64 * (1.0 - (power as f64 / 200.0 + state.prestige as f64 / 200.0))).max(1.0) as i32;
    let rank = disc::clamp(rank_base + disc::rand_range(rng, -2, 2), 1, total_sects);

    let (reward_silver, reward_prestige, desc_text) = if rank == 1 {
        (300, 15, "本派弟子技压群雄，夺得魁首！一时间名动江湖，四方来贺！".to_string())
    } else if rank <= 3 {
        (200, 10, "本派位列三甲，战绩斐然。门下弟子扬眉吐气，掌门面上有光。".to_string())
    } else if rank <= total_sects / 2 {
        (100, 5, "本派位列中游，虽未夺魁，亦属不易。弟子们还需勤加修炼。".to_string())
    } else {
        state.morale = disc::clamp(state.morale - 5, 0, 100);
        (30, 0, "本派成绩不佳，位列末流。掌门面上无光，但来日方长，卧薪尝胆便是。".to_string())
    };

    state.silver += reward_silver;
    state.prestige = disc::clamp(state.prestige + reward_prestige, 0, 100);
    state.morale = disc::clamp(state.morale + disc::rand_range(rng, 3, 8), 0, 100);

    for d in state.disciples.iter_mut().filter(|d| d.alive) {
        d.inner_power = disc::clamp(d.inner_power + disc::rand_range(rng, 2, 6), 10, 100);
    }

    state.tournament_history.push(TournamentRecord {
        year: state.year,
        rank,
        total_sects,
        power,
    });

    TournamentResult {
        rank,
        total_sects,
        power,
        desc_text,
        reward_silver,
        reward_prestige,
    }
}
