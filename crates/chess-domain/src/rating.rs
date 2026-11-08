//! Glicko-2, as described in Mark Glickman's "Example of the Glicko-2 system".
#![expect(
    clippy::float_arithmetic,
    reason = "Glicko-2 is defined in floating point"
)]

use std::f64::consts::PI;

/// Converts between the displayed scale (1500 ± 350) and Glicko-2's internal one.
const SCALE: f64 = 173.7178;
const BASE: f64 = 1500.0;
/// How much volatility may change per update; Glickman suggests 0.3–1.2, his example uses 0.5.
const TAU: f64 = 0.5;
const CONVERGENCE: f64 = 0.000_001;
/// Above this deviation a rating is provisional, as Lichess shows it with a "?".
const PROVISIONAL_DEVIATION: f64 = 110.0;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Rating {
    pub rating: f64,
    pub deviation: f64,
    pub volatility: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Score {
    Win,
    Draw,
    Loss,
}

/// One game against an opponent, as seen by the player being rated.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RatedResult {
    pub opponent: Rating,
    pub score: Score,
}

impl Rating {
    /// Every new player, in every category.
    pub const NEW: Self = Self {
        rating: 1500.0,
        deviation: 350.0,
        volatility: 0.06,
    };

    pub fn is_provisional(&self) -> bool {
        self.deviation > PROVISIONAL_DEVIATION
    }

    /// The rating after `results`; with none, only the deviation grows, as for a player who sat out.
    pub fn after(self, results: &[RatedResult]) -> Self {
        let player = Internal::from(self);
        if results.is_empty() {
            return player.idle().into();
        }
        let variance = estimated_variance(player, results);
        let improvement = improvement_sum(player, results);
        let volatility = new_volatility(player, variance, variance * improvement);
        player.updated(variance, improvement, volatility).into()
    }
}

impl Score {
    fn value(self) -> f64 {
        match self {
            Self::Win => 1.0,
            Self::Draw => 0.5,
            Self::Loss => 0.0,
        }
    }
}

/// A rating on Glicko-2's internal scale: μ and φ in the paper.
#[derive(Debug, Clone, Copy, PartialEq)]
struct Internal {
    mu: f64,
    phi: f64,
    sigma: f64,
}

impl From<Rating> for Internal {
    fn from(rating: Rating) -> Self {
        Self {
            mu: (rating.rating - BASE) / SCALE,
            phi: rating.deviation / SCALE,
            sigma: rating.volatility,
        }
    }
}

impl From<Internal> for Rating {
    fn from(internal: Internal) -> Self {
        Self {
            rating: internal.mu * SCALE + BASE,
            deviation: internal.phi * SCALE,
            volatility: internal.sigma,
        }
    }
}

impl Internal {
    fn idle(self) -> Self {
        Self {
            phi: pre_period_deviation(self.phi, self.sigma),
            ..self
        }
    }

    /// Steps 6–7: the new deviation and rating from the period's variance and results.
    fn updated(self, variance: f64, improvement: f64, sigma: f64) -> Self {
        let pre = pre_period_deviation(self.phi, sigma);
        let phi = 1.0 / (1.0 / (pre * pre) + 1.0 / variance).sqrt();
        Self {
            mu: self.mu + phi * phi * improvement,
            phi,
            sigma,
        }
    }
}

/// Step 3, v: how much the period's results can tell us.
fn estimated_variance(player: Internal, results: &[RatedResult]) -> f64 {
    let information: f64 = results
        .iter()
        .map(|result| {
            let opponent = Internal::from(result.opponent);
            let expected = expected_score(player, opponent);
            weight(opponent.phi).powi(2) * expected * (1.0 - expected)
        })
        .sum();
    1.0 / information
}

/// Step 4 without the factor v: the weighted sum of actual minus expected scores.
fn improvement_sum(player: Internal, results: &[RatedResult]) -> f64 {
    results
        .iter()
        .map(|result| {
            let opponent = Internal::from(result.opponent);
            weight(opponent.phi) * (result.score.value() - expected_score(player, opponent))
        })
        .sum()
}

/// g(φ): an uncertain opponent's result counts for less.
fn weight(phi: f64) -> f64 {
    1.0 / (1.0 + 3.0 * phi * phi / (PI * PI)).sqrt()
}

/// E(μ, μj, φj): the score the player was expected to make.
fn expected_score(player: Internal, opponent: Internal) -> f64 {
    1.0 / (1.0 + (-weight(opponent.phi) * (player.mu - opponent.mu)).exp())
}

/// Step 6, φ*: the deviation grows with volatility before the new results shrink it.
fn pre_period_deviation(phi: f64, sigma: f64) -> f64 {
    (phi * phi + sigma * sigma).sqrt()
}

/// Step 5: the new volatility, found with the Illinois algorithm.
fn new_volatility(player: Internal, variance: f64, delta: f64) -> f64 {
    let target = VolatilityTarget::new(player, variance, delta);
    let (mut low, mut high) = (target.a, target.upper_bound());
    let (mut f_low, mut f_high) = (target.at(low), target.at(high));
    while (high - low).abs() > CONVERGENCE {
        let next = low + (low - high) * f_low / (f_high - f_low);
        let f_next = target.at(next);
        if f_next * f_high <= 0.0 {
            low = high;
            f_low = f_high;
        } else {
            f_low /= 2.0;
        }
        high = next;
        f_high = f_next;
    }
    (low / 2.0).exp()
}

/// The function f(x) whose root is ln(σ'²).
struct VolatilityTarget {
    a: f64,
    phi_squared: f64,
    variance: f64,
    delta_squared: f64,
}

impl VolatilityTarget {
    fn new(player: Internal, variance: f64, delta: f64) -> Self {
        Self {
            a: (player.sigma * player.sigma).ln(),
            phi_squared: player.phi * player.phi,
            variance,
            delta_squared: delta * delta,
        }
    }

    fn at(&self, x: f64) -> f64 {
        let ex = x.exp();
        let spread = self.phi_squared + self.variance + ex;
        ex * (self.delta_squared - spread) / (2.0 * spread * spread) - (x - self.a) / (TAU * TAU)
    }

    /// The second starting point B of the bracket around the root.
    fn upper_bound(&self) -> f64 {
        let excess = self.delta_squared - self.phi_squared - self.variance;
        if excess > 0.0 {
            return excess.ln();
        }
        let mut k = 1.0;
        while self.at(self.a - k * TAU) < 0.0 {
            k += 1.0;
        }
        self.a - k * TAU
    }
}

#[cfg(test)]
mod tests {
    use super::{RatedResult, Rating, Score};

    fn rating(rating: f64, deviation: f64) -> Rating {
        Rating {
            rating,
            deviation,
            ..Rating::NEW
        }
    }

    fn close(actual: f64, expected: f64, tolerance: f64) -> bool {
        (actual - expected).abs() < tolerance
    }

    // Glickman's worked example: 1500 ± 200 beats 1400 ± 30, then loses to 1550 ± 100 and 1700 ± 300.
    #[test]
    fn matches_the_worked_example_in_glickmans_paper() {
        let player = rating(1500.0, 200.0);
        let after = player.after(&[
            RatedResult {
                opponent: rating(1400.0, 30.0),
                score: Score::Win,
            },
            RatedResult {
                opponent: rating(1550.0, 100.0),
                score: Score::Loss,
            },
            RatedResult {
                opponent: rating(1700.0, 300.0),
                score: Score::Loss,
            },
        ]);
        assert!(close(after.rating, 1464.06, 0.01), "{after:?}");
        assert!(close(after.deviation, 151.52, 0.01), "{after:?}");
        assert!(close(after.volatility, 0.05999, 0.000_01), "{after:?}");
    }

    #[test]
    fn beating_an_equal_player_raises_the_rating() {
        let after = Rating::NEW.after(&[RatedResult {
            opponent: Rating::NEW,
            score: Score::Win,
        }]);
        assert!(after.rating > Rating::NEW.rating);
        assert!(after.deviation < Rating::NEW.deviation);
    }

    #[test]
    fn a_draw_between_equals_changes_no_rating() {
        let after = Rating::NEW.after(&[RatedResult {
            opponent: Rating::NEW,
            score: Score::Draw,
        }]);
        assert!(close(after.rating, Rating::NEW.rating, 0.000_1));
    }

    #[test]
    fn sitting_out_only_grows_the_deviation() {
        let player = rating(1600.0, 80.0);
        let after = player.after(&[]);
        assert!(close(after.rating, 1600.0, 0.000_1));
        assert!(after.deviation > 80.0);
    }

    #[test]
    fn new_players_are_provisional() {
        assert!(Rating::NEW.is_provisional());
        assert!(!rating(1500.0, 60.0).is_provisional());
    }
}
