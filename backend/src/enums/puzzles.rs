use serde::{Deserialize, Serialize};

/// How long a Puzzle Rush lasts.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RushKind {
    ThreeMinutes,
    FiveMinutes,
    /// No clock: only three mistakes end it.
    Survival,
}

impl RushKind {
    pub const ALL: [Self; 3] = [Self::ThreeMinutes, Self::FiveMinutes, Self::Survival];

    /// As stored in `puzzle.rush_runs.kind`, the same as on the wire.
    pub fn key(self) -> &'static str {
        match self {
            Self::ThreeMinutes => "three_minutes",
            Self::FiveMinutes => "five_minutes",
            Self::Survival => "survival",
        }
    }

    pub fn from_key(key: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|kind| kind.key() == key)
    }
}

/// Where a battle stands.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BattleStatus {
    Ongoing,
    Finished,
}

/// How a battle ended, for one of its players.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BattleResult {
    Won,
    Lost,
    Drawn,
}

#[cfg(test)]
mod tests {
    use super::RushKind;

    #[test]
    fn every_kind_reads_back_from_its_key() {
        for kind in RushKind::ALL {
            assert_eq!(RushKind::from_key(kind.key()), Some(kind));
        }
    }
}
