use crate::{Move, enums::engine::UciScore};

/// One `info` line of a search: how deep it went and the line it found.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SearchInfo {
    pub depth: u32,
    /// Which of the best lines this is, 1 for the best.
    pub multipv: usize,
    pub score: UciScore,
    pub nodes: u64,
    pub nps: u64,
    pub pv: Vec<Move>,
}

/// An `info` line worth showing: an exact score and a line to play.
pub(crate) fn parse_info<'a>(mut words: impl Iterator<Item = &'a str>) -> Option<SearchInfo> {
    let mut fields = InfoFields::default();
    while let Some(word) = words.next() {
        match word {
            "string" | "lowerbound" | "upperbound" => return None,
            "depth" => fields.depth = number(words.next()),
            "multipv" => fields.multipv = number(words.next()),
            "nodes" => fields.nodes = number(words.next()),
            "nps" => fields.nps = number(words.next()),
            "score" => fields.score = score(words.next(), words.next()),
            "pv" => fields.pv = words.by_ref().map_while(|mv| mv.parse().ok()).collect(),
            _ => {}
        }
    }
    fields.complete()
}

#[derive(Default)]
struct InfoFields {
    depth: Option<u32>,
    multipv: Option<usize>,
    score: Option<UciScore>,
    nodes: Option<u64>,
    nps: Option<u64>,
    pv: Vec<Move>,
}

impl InfoFields {
    fn complete(self) -> Option<SearchInfo> {
        if self.pv.is_empty() {
            return None;
        }
        Some(SearchInfo {
            depth: self.depth?,
            multipv: self.multipv.unwrap_or(1),
            score: self.score?,
            nodes: self.nodes.unwrap_or(0),
            nps: self.nps.unwrap_or(0),
            pv: self.pv,
        })
    }
}

fn number<T: std::str::FromStr>(word: Option<&str>) -> Option<T> {
    word?.parse().ok()
}

fn score(kind: Option<&str>, value: Option<&str>) -> Option<UciScore> {
    match kind? {
        "cp" => number(value).map(UciScore::Cp),
        "mate" => number(value).map(UciScore::Mate),
        _ => None,
    }
}
