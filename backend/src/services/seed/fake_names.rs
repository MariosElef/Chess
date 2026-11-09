//! Usernames that look like people picked them: a word, maybe two, maybe a number.
#![expect(
    clippy::float_arithmetic,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "a roll from 0 to 1 becomes a number"
)]

use super::fake_games::index_for;

const FIRST: [&str; 40] = [
    "Silent", "Dark", "Lucky", "Quiet", "Swift", "Iron", "Golden", "Crazy", "Tactical", "Brave",
    "Lazy", "Grand", "Rapid", "Frozen", "Royal", "Sneaky", "Wild", "Calm", "Bold", "Clever",
    "nikos", "giorgos", "eleni", "maria", "kostas", "andreas", "sofia", "dimitris", "anna",
    "petros", "alex", "sam", "leo", "mia", "max", "noah", "zoe", "theo", "ivan", "luca",
];

const SECOND: [&str; 32] = [
    "Rook",
    "Knight",
    "Bishop",
    "Pawn",
    "Queen",
    "King",
    "Gambit",
    "Castle",
    "Fork",
    "Pin",
    "Endgame",
    "Opening",
    "Tempo",
    "Zugzwang",
    "Blitz",
    "Mate",
    "Checker",
    "Sacrifice",
    "Fianchetto",
    "Storm",
    "Player",
    "Master",
    "Fan",
    "Lover",
    "Hunter",
    "Wizard",
    "Rider",
    "Slayer",
    "Shark",
    "Fox",
    "Owl",
    "Tiger",
];

/// A username from three rolls between 0 and 1: which words, and what comes after them.
pub fn fake_username((first, second, ending): (f64, f64, f64)) -> String {
    let base = format!("{}{}", pick(&FIRST, first), pick(&SECOND, second));
    let ending = ending_for(ending);
    let name = format!("{base}{ending}");
    // Usernames are 3 to 20 letters, digits and underscores.
    name.chars().take(20).collect()
}

/// Nothing, a year, a number, or an underscore and a number.
fn ending_for(roll: f64) -> String {
    let number = (roll * 10_000.0) as u32;
    match number % 4 {
        0 => String::new(),
        1 => format!("{}", 1985 + number % 25),
        2 => format!("{}", number % 100),
        _ => format!("_{}", number % 1000),
    }
}

fn pick(words: &[&'static str], roll: f64) -> &'static str {
    words[index_for(roll, words.len())]
}

#[cfg(test)]
mod tests {
    use super::fake_username;

    #[test]
    fn a_name_is_a_valid_username() {
        for rolls in [
            (0.0, 0.0, 0.0),
            (0.99, 0.99, 0.99),
            (0.5, 0.3, 0.77),
            (0.21, 0.8, 0.0003),
        ] {
            let name = fake_username(rolls);
            assert!((3..=20).contains(&name.chars().count()), "{name}");
            assert!(
                name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_'),
                "{name}"
            );
        }
    }
}
