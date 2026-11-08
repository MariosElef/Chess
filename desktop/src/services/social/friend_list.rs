use std::collections::BTreeMap;

use crate::{
    enums::social::{FriendSort, LeaderBoard},
    models::social::Friend,
    services::account::category_of,
};

/// How many places each board of the leaderboard shows before the player's own.
pub const PODIUM: usize = 3;

/// One place on a board of the friends' leaderboard.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LeaderRow {
    pub place: usize,
    pub name: String,
    pub rating: i32,
    /// The signed-in player themselves.
    pub me: bool,
}

/// The server's key for the puzzle rating.
const PUZZLE_KEY: &str = "puzzle";

/// The rating a board ranks, from ratings keyed as the server keys them: a speed by its name.
fn rating_on(ratings: &BTreeMap<String, i32>, board: LeaderBoard) -> Option<i32> {
    match board {
        LeaderBoard::Speed(category) => ratings
            .iter()
            .find(|(key, _)| category_of(key) == Some(category))
            .map(|(_, rating)| *rating),
        LeaderBoard::Puzzles => ratings.get(PUZZLE_KEY).copied(),
    }
}

/// The friends in the order picked: online first and then most recently online, or by name.
pub fn sorted_friends(friends: &[Friend], sort: FriendSort) -> Vec<&Friend> {
    let mut sorted: Vec<&Friend> = friends.iter().collect();
    match sort {
        FriendSort::RecentlyOnline => sorted.sort_by_key(|friend| {
            (
                !friend.online,
                friend.away_s.unwrap_or(i64::MAX),
                friend.username.to_lowercase(),
            )
        }),
        FriendSort::Username => sorted.sort_by_key(|friend| friend.username.to_lowercase()),
    }
    sorted
}

/// The best three on `board` among friends and the player, then the player's place if lower.
pub fn leaderboard(
    friends: &[Friend],
    (my_name, my_ratings): (&str, &BTreeMap<String, i32>),
    board: LeaderBoard,
) -> Vec<LeaderRow> {
    let mut rated: Vec<(String, i32, bool)> = friends
        .iter()
        .filter_map(|friend| {
            let rating = rating_on(&friend.ratings, board)?;
            Some((friend.username.clone(), rating, false))
        })
        .collect();
    if let Some(mine) = rating_on(my_ratings, board) {
        rated.push((my_name.to_owned(), mine, true));
    }
    rated.sort_by(|left, right| right.1.cmp(&left.1).then_with(|| left.0.cmp(&right.0)));
    rated
        .into_iter()
        .enumerate()
        .filter(|(index, (_, _, me))| *index < PODIUM || *me)
        .map(|(index, (name, rating, me))| LeaderRow {
            place: index + 1,
            name,
            rating,
            me,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use chess_domain::Category;

    use super::{leaderboard, sorted_friends};
    use crate::{
        enums::social::{FriendSort, LeaderBoard},
        models::social::Friend,
    };

    const BLITZ: LeaderBoard = LeaderBoard::Speed(Category::Blitz);

    fn friend(name: &str, online: bool, away_s: Option<i64>, blitz: Option<i32>) -> Friend {
        Friend {
            id: name.to_owned(),
            username: name.to_owned(),
            online,
            away_s,
            ratings: blitz
                .map(|rating| BTreeMap::from([("blitz".to_owned(), rating)]))
                .unwrap_or_default(),
        }
    }

    #[test]
    fn friends_are_listed_online_first_then_by_how_recently() {
        let friends = [
            friend("cat", false, None, None),
            friend("ann", false, Some(7200), None),
            friend("bob", true, Some(0), None),
            friend("dan", false, Some(60), None),
        ];
        let names = |sort| -> Vec<String> {
            sorted_friends(&friends, sort)
                .iter()
                .map(|friend| friend.username.clone())
                .collect()
        };
        assert_eq!(
            names(FriendSort::RecentlyOnline),
            ["bob", "dan", "ann", "cat"]
        );
        assert_eq!(names(FriendSort::Username), ["ann", "bob", "cat", "dan"]);
    }

    #[test]
    fn the_leaderboard_ranks_the_player_among_their_friends() {
        let friends = [
            friend("ann", true, None, Some(1500)),
            friend("bob", true, None, Some(1700)),
            friend("cat", true, None, None),
            friend("dan", true, None, Some(1200)),
        ];
        let mine = BTreeMap::from([("blitz".to_owned(), 1600)]);
        let board = leaderboard(&friends, ("me", &mine), BLITZ);
        let shown: Vec<(usize, &str, i32, bool)> = board
            .iter()
            .map(|row| (row.place, row.name.as_str(), row.rating, row.me))
            .collect();
        assert_eq!(
            shown,
            [
                (1, "bob", 1700, false),
                (2, "me", 1600, true),
                (3, "ann", 1500, false)
            ]
        );
        assert!(
            leaderboard(
                &friends,
                ("me", &mine),
                LeaderBoard::Speed(Category::Bullet)
            )
            .is_empty()
        );
        let low = BTreeMap::from([("blitz".to_owned(), 1000)]);
        let places: Vec<(usize, bool)> = leaderboard(&friends, ("me", &low), BLITZ)
            .iter()
            .map(|row| (row.place, row.me))
            .collect();
        assert_eq!(
            places,
            [(1, false), (2, false), (3, false), (4, true)],
            "the player's own place under the best three"
        );
    }
}
