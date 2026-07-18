use serde::{Deserialize, Serialize};

pub const TOURNAMENT_FORMAT_VERSION: i32 = 1;

/// 论剑开赛时冻结的一名出阵弟子；整届赛事都使用同一份战力与武学快照。
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(default)]
pub struct TournamentLineupMember {
    pub disciple_id: String,
    pub name: String,
    pub martial_art_id: String,
    pub combat_score: i32,
}

/// 单场对阵的一方。阵胜与总分是该场数据，不会污染下一轮。
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(default)]
pub struct TournamentSide {
    pub sect_id: String,
    pub sect_name: String,
    pub seed: i32,
    pub seed_score: i32,
    pub bout_wins: i32,
    pub total_score: i32,
}

/// 一场三阵中的一阵。缺阵一方为 `None`，双方都缺阵时本阵无胜者。
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(default)]
pub struct TournamentBout {
    pub position: i32,
    pub left: Option<TournamentLineupMember>,
    pub right: Option<TournamentLineupMember>,
    pub left_score: i32,
    pub right_score: i32,
    pub winner_sect_id: Option<String>,
}

/// 三阵制单场；首轮轮空仍保留签位，但 `bouts` 为空且 `bye` 为真。
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(default)]
pub struct TournamentMatch {
    pub id: String,
    pub left: Option<TournamentSide>,
    pub right: Option<TournamentSide>,
    pub winner_sect_id: String,
    pub bye: bool,
    pub bouts: Vec<TournamentBout>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(default)]
pub struct TournamentRound {
    pub number: i32,
    pub name: String,
    pub matches: Vec<TournamentMatch>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct TournamentRecord {
    pub year: i32,
    pub rank: i32,
    pub total_sects: i32,
    pub power: i32,
    pub format_version: i32,
    pub champion: String,
    pub player_lineup: Vec<TournamentLineupMember>,
    pub rounds: Vec<TournamentRound>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct TournamentResult {
    pub year: i32,
    pub rank: i32,
    pub total_sects: i32,
    pub power: i32,
    pub desc_text: String,
    pub reward_silver: i32,
    pub reward_prestige: i32,
    pub format_version: i32,
    pub champion: String,
    pub player_lineup: Vec<TournamentLineupMember>,
    pub rounds: Vec<TournamentRound>,
}

#[cfg(test)]
mod tests {
    use super::TournamentRecord;

    #[test]
    fn legacy_tournament_record_defaults_the_additive_bracket_fields() {
        let record: TournamentRecord = serde_json::from_value(serde_json::json!({
            "year": 1,
            "rank": 4,
            "total_sects": 25,
            "power": 88
        }))
        .unwrap();

        assert_eq!(record.year, 1);
        assert_eq!(record.rank, 4);
        assert_eq!(record.format_version, 0);
        assert!(record.champion.is_empty());
        assert!(record.player_lineup.is_empty());
        assert!(record.rounds.is_empty());
    }
}
