//! The rating a side played like in one game: chess.com's Game Rating. Chess.com does not
//! publish theirs; this one is measured. Every move is sorted by how much of the mover's winning
//! chances it gave away, how the mover stood before it, and the phase of the game, and the
//! estimate is the rating under which the side's moves were most likely, as Ken Regan's
//! performance ratings work. Then the bias that a game's length brings is taken off: short games
//! end in blunders, long ones go on because nobody blundered.
//!
//! The model was fitted to 683,632 sides of analysed rated Lichess games (bullet to classical,
//! January 2024, Lichess's ratings) and checked on 169,888 more it never saw: the estimate is
//! typically within 450 of the player's real rating, in a game of any length. One game says only
//! so much, so it swings from game to game, as chess.com's does.
#![expect(
    clippy::float_arithmetic,
    reason = "the estimate is the most likely rating under a fitted model"
)]

use chess_domain::{Side, engine::Eval};

use super::accuracy::win_percent;

/// Fewer moves than this in all, and the game says too little to rate.
pub const FEWEST_PLIES: usize = 10;
/// The ratings the estimate is looked for between, every ten points.
const LOWEST: u32 = 400;
const HIGHEST: u32 = 3000;
const GRID_STEP: usize = 10;
/// Estimates are given to the nearest fifty: anything finer would claim more than one game says.
const ROUNDING: f64 = 50.0;

/// What a move gave away, in the mover's winning chances out of 100.
const LOSS_EDGES: [f64; 7] = [0.0, 1.0, 2.0, 4.0, 8.0, 15.0, 30.0];
/// How the mover stood before it: lost, worse, level, better, won.
const STANDING_EDGES: [f64; 4] = [10.0, 30.0, 70.0, 90.0];
/// The phase, by the move's place in the game: opening, middlegame, the rest.
const PHASE_EDGES: [f64; 2] = [20.0, 60.0];
/// Every phase, standing and loss together.
pub const KINDS: usize =
    (PHASE_EDGES.len() + 1) * (STANDING_EDGES.len() + 1) * (LOSS_EDGES.len() + 1);

/// The game-length bias, a quadratic in the natural log of the number of moves.
const LENGTH_BIAS: [f64; 3] = [5039.018, -3402.807, 522.358];

/// For each kind of move, the log of how often a player rated `r` thousand plays it, before the
/// kinds are scaled to add up: a cubic in `r`, fitted to the games above.
#[expect(
    clippy::approx_constant,
    reason = "fitted numbers, one happening to be near e"
)]
const MOVE_KINDS: [[f64; 4]; KINDS] = [
    [-3.97638, -2.7178, 0.561081, -0.138998],
    [-4.33309, -1.918, -0.275981, 0.0943202],
    [-5.07245, -2.17314, -0.0665322, 0.00993437],
    [-5.02553, -2.10511, -0.31253, 0.082314],
    [-6.22874, -0.811534, -1.19662, 0.267295],
    [-8.47535, -8.72936, 2.7183, -0.0340562],
    [-8.47535, -8.72936, 2.7183, -0.0340562],
    [-8.47535, -8.72936, 2.7183, -0.0340562],
    [-5.1026, 0.0290154, -0.519287, 0.0497616],
    [-5.06051, -0.296683, -0.438322, 0.0415356],
    [-5.60033, -0.114668, -0.590488, 0.0771005],
    [-4.79643, -1.10082, -0.0452953, -0.0341163],
    [-4.28036, -1.69455, 0.235566, -0.0892347],
    [-4.24228, -2.29197, 0.596882, -0.177534],
    [-5.28273, -1.78076, 0.0571258, -0.0602531],
    [-8.47535, -8.72936, 2.7183, -0.0340562],
    [-2.91727, 0.480791, -0.22246, 0.0357402],
    [-3.57533, 0.700272, -0.223329, 0.0275219],
    [-3.73526, 0.715413, -0.218485, 0.0177962],
    [-3.66966, 0.769885, -0.251323, 0.00865383],
    [-3.28828, 0.40654, -0.184732, -0.0166842],
    [-3.39229, -0.0101088, -0.179443, -0.024584],
    [-3.57748, -0.35281, -0.250782, -0.0156783],
    [-3.8549, -1.05172, -0.291474, 0.0260901],
    [-4.77735, 0.483341, -0.909494, 0.156851],
    [-4.98407, 0.293862, -0.805374, 0.134144],
    [-5.36499, -0.595138, -0.271165, 0.04047],
    [-5.4081, -0.27894, -0.487673, 0.069888],
    [-4.90282, -0.882482, -0.00439403, -0.043126],
    [-4.9802, -0.594813, -0.129014, -0.0354157],
    [-4.99183, -0.25931, -0.361309, -0.00671304],
    [-4.30903, -1.38244, -0.30737, 0.0544406],
    [-4.11217, -1.76578, -0.0898604, 0.0139136],
    [-4.34623, -1.48179, -0.353701, 0.0935145],
    [-4.96522, -2.50307, 0.162207, -0.00339638],
    [-6.09998, -0.181359, -1.39682, 0.309776],
    [-5.68972, -1.15127, -0.685859, 0.158247],
    [-5.76862, -1.85621, 0.0359682, 0.00321254],
    [-4.76723, -4.31372, 1.856, -0.469358],
    [-4.79896, -2.68349, 0.370927, -0.0692735],
    [-1.43567, -2.33934, 0.604753, -0.105879],
    [-3.48017, 0.00538283, -0.564341, 0.0821006],
    [-4.09326, -0.527676, -0.309339, 0.0427945],
    [-3.75423, -1.30737, 0.108386, -0.0279933],
    [-4.17845, -1.32082, 0.228189, -0.0556484],
    [-8.47535, -8.72936, 2.7183, -0.0340562],
    [-8.47535, -8.72936, 2.7183, -0.0340562],
    [-8.47535, -8.72936, 2.7183, -0.0340562],
    [-4.8853, 1.43073, -0.655529, 0.0688933],
    [-4.77821, 1.5789, -0.836239, 0.104608],
    [-5.32641, 1.51906, -0.810543, 0.100809],
    [-4.84596, 0.938188, -0.520926, 0.0518933],
    [-4.23046, -0.0110145, -0.0670359, -0.0215686],
    [-4.17975, -0.607628, 0.22012, -0.0739663],
    [-5.18586, -0.956027, 0.374713, -0.108705],
    [-8.47535, -8.72936, 2.7183, -0.0340562],
    [-5.12488, 2.32244, -0.717032, 0.0882144],
    [-5.67098, 2.58428, -0.818906, 0.0995549],
    [-6.2651, 2.61161, -0.807091, 0.091113],
    [-6.1663, 2.73473, -0.854023, 0.0867262],
    [-5.73572, 2.53671, -0.826396, 0.0782664],
    [-5.38522, 2.06589, -0.721087, 0.0617657],
    [-4.73967, 1.09474, -0.429085, 0.0179408],
    [-4.33198, -0.0864723, -0.0866472, -0.0343358],
    [-4.63876, 1.69626, -0.839544, 0.122623],
    [-4.92693, 1.88834, -1.01262, 0.152757],
    [-5.67845, 1.64009, -0.863375, 0.123608],
    [-5.37702, 1.16517, -0.628716, 0.0804208],
    [-5.29022, 1.01542, -0.535443, 0.0597718],
    [-5.24307, 0.509429, -0.199295, -0.010101],
    [-5.31146, 0.839327, -0.455458, 0.0371301],
    [-4.0381, -0.41374, -0.165893, 0.00215261],
    [-2.01885, -1.01829, -0.116622, 0.045863],
    [-3.86522, 0.756422, -0.978683, 0.179387],
    [-4.49363, 0.0116405, -0.601553, 0.105729],
    [-4.25511, -0.3849, -0.482034, 0.102241],
    [-3.91809, -1.55129, 0.303598, -0.0577241],
    [-4.93166, -1.03793, 0.0145846, 0.00182222],
    [-5.06044, -1.98536, 0.667671, -0.130409],
    [-4.09695, -1.71983, 0.210797, -0.0282223],
    [-3.52789, 0.618739, -0.597303, 0.130132],
    [-7.19545, 3.07345, -1.44611, 0.220721],
    [-7.79204, 2.67915, -1.19123, 0.166856],
    [-7.3671, 1.94801, -0.907819, 0.137286],
    [-7.15629, 1.32886, -0.558763, 0.069619],
    [-8.47535, -8.72936, 2.7183, -0.0340562],
    [-8.47535, -8.72936, 2.7183, -0.0340562],
    [-8.47535, -8.72936, 2.7183, -0.0340562],
    [-8.06997, 3.23833, -1.04355, 0.123825],
    [-7.92353, 3.425, -1.19051, 0.143986],
    [-8.45723, 3.40518, -1.20232, 0.151821],
    [-8.11067, 2.84212, -0.912387, 0.101465],
    [-7.67477, 1.89784, -0.475398, 0.0346971],
    [-7.54113, 1.61449, -0.503317, 0.0572568],
    [-8.25594, 0.167414, 0.323959, -0.0932674],
    [-8.47535, -8.72936, 2.7183, -0.0340562],
    [-7.66309, 4.09498, -1.40894, 0.20882],
    [-8.65145, 3.63058, -1.06953, 0.141251],
    [-9.68954, 3.75362, -1.05421, 0.123852],
    [-9.60019, 3.47599, -0.836813, 0.0715831],
    [-9.46294, 3.37908, -0.850921, 0.0776012],
    [-9.29499, 3.2782, -0.85731, 0.0764361],
    [-8.48498, 2.5725, -0.679672, 0.0588482],
    [-7.52372, 2.0376, -0.797269, 0.114731],
    [-7.65785, 3.40973, -1.15938, 0.152046],
    [-8.0921, 3.6066, -1.33897, 0.186869],
    [-8.86737, 3.37051, -1.22003, 0.16639],
    [-9.06443, 3.59724, -1.40312, 0.205275],
    [-8.92502, 2.74194, -0.817335, 0.0855382],
    [-9.76677, 3.46471, -1.1652, 0.140524],
    [-9.19634, 3.2877, -1.24304, 0.17942],
    [-7.64291, 2.03414, -0.893218, 0.134867],
    [-4.00681, 1.5188, -1.05772, 0.217861],
    [-7.40184, 3.29089, -1.54909, 0.246617],
    [-8.15067, 3.12357, -1.55052, 0.252133],
    [-7.71223, 2.47224, -1.30619, 0.228133],
    [-7.57618, 2.16511, -1.19411, 0.213554],
    [-7.9566, 1.31468, -0.712157, 0.133513],
    [-9.48079, 2.01133, -0.993115, 0.173948],
    [-7.24602, 1.73597, -1.2503, 0.262266],
];

/// How many of each kind of move White, then Black, made, from the evaluation of every analysed
/// position, the one before the first move included; `first_mover` made the first move.
pub fn move_kinds(evals: &[Eval], first_mover: Side) -> [[u32; KINDS]; 2] {
    let wins: Vec<f64> = evals.iter().copied().map(win_percent).collect();
    let mut kinds = [[0; KINDS]; 2];
    for (index, pair) in wins.windows(2).enumerate() {
        let mover = if index % 2 == 0 {
            first_mover
        } else {
            first_mover.opponent()
        };
        let (before, after, side) = match mover {
            Side::White => (pair[0], pair[1], 0),
            Side::Black => (100.0 - pair[0], 100.0 - pair[1], 1),
        };
        kinds[side][move_kind(before, after, index)] += 1;
    }
    kinds
}

/// A move by its phase, the mover's standing before it, and what it gave away.
fn move_kind(before: f64, after: f64, index: usize) -> usize {
    let losses = LOSS_EDGES.len() + 1;
    let standings = STANDING_EDGES.len() + 1;
    let loss = bucket((before - after).max(0.0), &LOSS_EDGES);
    let standing = bucket(before, &STANDING_EDGES);
    let phase = bucket(count(index), &PHASE_EDGES);
    (phase * standings + standing) * losses + loss
}

/// How many edges `value` is past.
fn bucket(value: f64, edges: &[f64]) -> usize {
    edges.iter().filter(|edge| value > **edge).count()
}

/// The rating a side's moves were most likely at, in a game of `plies` moves in all, to the
/// nearest fifty.
pub fn estimated_rating(kinds: &[u32; KINDS], plies: usize) -> u32 {
    let most_likely = rounded(most_likely_rating(kinds));
    let length = count(plies).ln();
    let bias = LENGTH_BIAS[0] + LENGTH_BIAS[1] * length + LENGTH_BIAS[2] * length * length;
    let estimate = rounded(most_likely - bias).clamp(f64::from(LOWEST), f64::from(HIGHEST));
    // Between 400 and 3000 by the clamp.
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "400..=3000"
    )]
    let whole = estimate as u32;
    whole
}

/// The rating, every ten points, under which these moves were most likely; the lowest one at a
/// tie.
fn most_likely_rating(kinds: &[u32; KINDS]) -> f64 {
    let mut best = (f64::from(LOWEST), f64::NEG_INFINITY);
    for rating in (LOWEST..=HIGHEST).step_by(GRID_STEP) {
        let rating = f64::from(rating);
        let likelihood: f64 = kind_logs(rating)
            .iter()
            .zip(kinds)
            .filter(|(_, made)| **made > 0)
            .map(|(log, made)| log * f64::from(*made))
            .sum();
        if likelihood > best.1 {
            best = (rating, likelihood);
        }
    }
    best.0
}

/// The log of how often a player at `rating` makes each kind of move, the kinds adding up to one.
fn kind_logs(rating: f64) -> [f64; KINDS] {
    let r = rating / 1000.0;
    let raw = MOVE_KINDS.map(|[a, b, c, d]| a + b * r + c * r * r + d * r * r * r);
    let top = raw.iter().copied().fold(f64::NEG_INFINITY, f64::max);
    let total = top + raw.iter().map(|log| (log - top).exp()).sum::<f64>().ln();
    raw.map(|log| log - total)
}

/// To the nearest fifty, a half going to the even fifty.
fn rounded(rating: f64) -> f64 {
    (rating / ROUNDING).round_ties_even() * ROUNDING
}

fn count(n: usize) -> f64 {
    f64::from(u32::try_from(n).unwrap_or(u32::MAX))
}

#[cfg(test)]
mod tests {
    use chess_domain::{Side, engine::Eval};

    use super::{KINDS, estimated_rating, move_kinds};

    /// A side of a held-out game: the moves in the game, the kinds of move it made, and the
    /// estimate the fitting script gave it.
    type HeldOut = (usize, &'static [(usize, u32)], u32);

    /// Counts of a few kinds of move, the rest none.
    fn kinds(made: &[(usize, u32)]) -> [u32; KINDS] {
        let mut kinds = [0; KINDS];
        for (kind, count) in made {
            kinds[*kind] = *count;
        }
        kinds
    }

    #[test]
    fn held_out_lichess_games_get_the_estimate_the_fitted_model_gives() {
        // Sides of games the model was not fitted to, with the estimate the fitting script gave.
        let games: [HeldOut; 6] = [
            (
                242,
                &[
                    (16, 2),
                    (17, 3),
                    (18, 3),
                    (20, 1),
                    (26, 1),
                    (29, 1),
                    (48, 1),
                    (56, 2),
                    (57, 3),
                    (58, 2),
                    (59, 1),
                    (60, 2),
                    (61, 6),
                    (62, 1),
                    (68, 1),
                    (70, 1),
                    (88, 1),
                    (92, 1),
                    (94, 1),
                    (96, 28),
                    (97, 6),
                    (99, 1),
                    (102, 1),
                    (104, 15),
                    (105, 5),
                    (106, 4),
                    (107, 2),
                    (108, 2),
                    (110, 3),
                    (111, 1),
                    (112, 15),
                    (113, 2),
                    (118, 1),
                    (119, 1),
                ],
                900,
            ),
            (
                242,
                &[
                    (11, 1),
                    (16, 2),
                    (17, 2),
                    (18, 1),
                    (19, 1),
                    (20, 2),
                    (22, 1),
                    (56, 2),
                    (57, 5),
                    (58, 1),
                    (59, 1),
                    (60, 1),
                    (61, 4),
                    (67, 2),
                    (68, 1),
                    (69, 2),
                    (70, 1),
                    (80, 15),
                    (81, 1),
                    (83, 1),
                    (88, 11),
                    (89, 7),
                    (90, 4),
                    (91, 2),
                    (92, 3),
                    (93, 2),
                    (96, 31),
                    (97, 5),
                    (102, 4),
                    (103, 1),
                    (104, 2),
                    (108, 1),
                    (119, 1),
                ],
                900,
            ),
            (
                42,
                &[
                    (16, 1),
                    (17, 4),
                    (18, 2),
                    (19, 1),
                    (20, 1),
                    (21, 1),
                    (22, 1),
                    (40, 2),
                    (41, 2),
                    (42, 1),
                    (43, 2),
                    (54, 1),
                    (62, 2),
                ],
                900,
            ),
            (
                42,
                &[
                    (16, 1),
                    (17, 3),
                    (18, 1),
                    (19, 2),
                    (20, 2),
                    (21, 1),
                    (58, 1),
                    (68, 1),
                    (70, 1),
                    (72, 1),
                    (73, 5),
                    (74, 1),
                    (75, 1),
                ],
                1200,
            ),
            (
                58,
                &[
                    (16, 2),
                    (17, 2),
                    (18, 6),
                    (20, 1),
                    (56, 2),
                    (57, 4),
                    (58, 3),
                    (59, 2),
                    (60, 3),
                    (61, 2),
                    (64, 1),
                    (68, 1),
                ],
                2650,
            ),
            (
                58,
                &[
                    (16, 1),
                    (17, 3),
                    (18, 2),
                    (19, 4),
                    (48, 1),
                    (56, 7),
                    (57, 3),
                    (58, 1),
                    (59, 3),
                    (60, 2),
                    (61, 1),
                    (65, 1),
                ],
                2750,
            ),
        ];
        for (plies, made, expected) in games {
            assert_eq!(estimated_rating(&kinds(made), plies), expected);
        }
    }

    #[test]
    fn a_lichess_game_sorts_its_moves_as_the_fitting_script_did() {
        // lichess.org/iSjpt213, Lichess's evaluations in centipawns, its +15 start first.
        let pawns = [
            15, 20, 17, 29, 32, 21, 76, 56, 75, 13, 43, 13, 70, 50, 42, 33, 81, 48, 52, 10, 253,
            178, 273, 305, 318, 164, 446, 446, 478, 453, 526, 570, 572, 579, 579, 417, 396, 388,
            392, 402, 516, 431, 450, 342, 462, 437, 485, 398, 400, 409, 397, 400, 602, 590, 590,
            603, 608, 579, 591, 592, 594, 572, 564, 508, 535, 525, 567, 545, 551, 555, 570, 516,
            579, 497, 529, 506, 557, 553, 554, 551, 562, 557, 557, 514, 607, 571, 571, 541, 574,
            583, 586, 559, 570, 451, 574, 561, 557, 560, 560, 583, 571, 561, 556, 548, 557, 544,
            923, 3476,
        ];
        let evals: Vec<Eval> = pawns.into_iter().map(Eval::Centipawns).collect();
        let [white, black] = move_kinds(&evals, Side::White);
        assert_eq!(
            white,
            kinds(&[
                (16, 2),
                (17, 1),
                (18, 3),
                (19, 3),
                (20, 1),
                (28, 1),
                (64, 9),
                (65, 2),
                (66, 2),
                (67, 1),
                (68, 3),
                (69, 1),
                (73, 2),
                (104, 4),
                (105, 11),
                (106, 2),
                (107, 3),
                (108, 1),
                (112, 1),
                (114, 1)
            ])
        );
        assert_eq!(
            black,
            kinds(&[
                (16, 2),
                (17, 2),
                (18, 1),
                (19, 1),
                (20, 3),
                (22, 1),
                (41, 1),
                (48, 4),
                (49, 7),
                (50, 1),
                (51, 2),
                (52, 2),
                (53, 1),
                (60, 1),
                (62, 1),
                (88, 7),
                (89, 7),
                (90, 4),
                (91, 3),
                (92, 1),
                (93, 1)
            ])
        );
    }

    #[test]
    fn blunders_read_as_a_weaker_player() {
        // Forty middlegame moves in a level game: all close to best, then the same with six of
        // them giving away more than thirty in a hundred.
        let clean = kinds(&[(56, 30), (57, 10)]);
        let blundering = kinds(&[(56, 30), (57, 4), (63, 6)]);
        assert!(estimated_rating(&blundering, 80) < estimated_rating(&clean, 80));
    }
}
