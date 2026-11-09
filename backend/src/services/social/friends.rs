use std::collections::{BTreeMap, HashMap};

use uuid::Uuid;

use crate::{
    enums::social::{FriendshipStatus, Relation},
    errors::{AppError, ChessCode},
    models::{
        play::messages::ServerMessage,
        rating::UserRatingRow,
        social::friend::{FoundUserModel, FriendModel, GetFriendsModel, UserRef},
    },
    repositories::{
        account::user,
        database::Db,
        rating,
        social::{block, friendship},
    },
    services::{play::PlayHub, rating::whole},
};

/// The fewest letters a search looks for, and the most players it brings back.
const SHORTEST_SEARCH: usize = 2;
const MOST_FOUND: i64 = 20;

/// Friends and friend requests between players; every change tells both players at once.
#[derive(Clone)]
pub struct FriendService {
    db: Db,
    play: PlayHub,
}

impl FriendService {
    pub fn new(db: Db, play: PlayHub) -> Self {
        Self { db, play }
    }

    /// Players whose name contains `query`, and where the searcher stands with each.
    pub async fn search(
        &self,
        user_id: Uuid,
        query: &str,
    ) -> Result<Vec<FoundUserModel>, AppError> {
        let query = query.trim().to_lowercase();
        if query.chars().count() < SHORTEST_SEARCH {
            return Ok(Vec::new());
        }
        let rows = friendship::search(&self.db, user_id, &like_escaped(&query), MOST_FOUND).await?;
        let response = rows
            .into_iter()
            .map(|row| FoundUserModel {
                avatar: row.avatar,
                relation: if row.blocked {
                    Relation::Blocked
                } else {
                    relation(user_id, (row.status.as_deref(), row.requester_id))
                },
                id: row.user_id,
                username: row.username,
            })
            .collect();
        Ok(response)
    }

    /// Friends, online first, and the requests each way.
    pub async fn list(&self, user_id: Uuid) -> Result<GetFriendsModel, AppError> {
        let rows = friendship::of_user(&self.db, user_id).await?;
        // Ratings only for friends, and the player: requests show none.
        let mut players: Vec<Uuid> = rows
            .iter()
            .filter(|row| {
                relation(user_id, (row.status.as_deref(), row.requester_id)) == Relation::Friends
            })
            .map(|row| row.user_id)
            .collect();
        players.push(user_id);
        let mut ratings = ratings_by_player(rating::of_users(&self.db, &players).await?);
        let mut response = GetFriendsModel {
            friends: Vec::new(),
            my_ratings: ratings.remove(&user_id).unwrap_or_default(),
            incoming: Vec::new(),
            outgoing: Vec::new(),
            blocked: block::of_blocker(&self.db, user_id).await?,
        };
        for row in rows {
            let relation = relation(user_id, (row.status.as_deref(), row.requester_id));
            let person = UserRef {
                id: row.user_id,
                username: row.username,
                avatar: row.avatar,
            };
            match relation {
                Relation::Friends => response.friends.push(FriendModel {
                    online: self.play.is_online(person.id),
                    away_s: row.away_s,
                    ratings: ratings.remove(&person.id).unwrap_or_default(),
                    id: person.id,
                    username: person.username,
                    avatar: person.avatar,
                }),
                Relation::RequestReceived => response.incoming.push(person),
                Relation::RequestSent => response.outgoing.push(person),
                Relation::Blocked | Relation::None => {}
            }
        }
        // Stable, so names stay in order within each group.
        response.friends.sort_by_key(|friend| !friend.online);
        Ok(response)
    }

    /// Asks `other` to be friends; if they had already asked, this accepts.
    pub async fn request(&self, user_id: Uuid, other: Uuid) -> Result<Relation, AppError> {
        if other == user_id {
            return Err(ChessCode::CannotBefriendYourself.into());
        }
        if user::get(&self.db, other).await?.is_none() {
            return Err(ChessCode::UserNotFound.into());
        }
        let pair = pair(user_id, other);
        let mut tx = self.db.begin().await?;
        friendship::lock_pair(&mut *tx, pair).await?;
        let blockers = block::blockers_between(&mut *tx, user_id, other).await?;
        if blockers.contains(&user_id) {
            // The blocker must unblock first.
            return Ok(Relation::Blocked);
        }
        if !blockers.is_empty() {
            // The blocked one is never told: their request seems to go nowhere.
            return Ok(Relation::RequestSent);
        }
        let current = friendship::between(&mut *tx, pair).await?;
        let status = current
            .as_ref()
            .and_then(|row| FriendshipStatus::from_key(&row.status));
        let asked_by_them = current
            .as_ref()
            .is_some_and(|row| row.requester_id == other);
        let relation = match status {
            Some(FriendshipStatus::Accepted) => Relation::Friends,
            Some(FriendshipStatus::Pending) if asked_by_them => {
                friendship::accept(&mut *tx, pair, other).await?;
                Relation::Friends
            }
            Some(FriendshipStatus::Pending) => Relation::RequestSent,
            Some(FriendshipStatus::Declined) | None => {
                friendship::request(&mut *tx, pair, user_id).await?;
                Relation::RequestSent
            }
        };
        tx.commit().await?;
        self.tell_both(user_id, other);
        Ok(relation)
    }

    /// Accepts the request `other` sent.
    pub async fn accept(&self, user_id: Uuid, other: Uuid) -> Result<(), AppError> {
        let pair = pair(user_id, other);
        let mut tx = self.db.begin().await?;
        friendship::lock_pair(&mut *tx, pair).await?;
        if !friendship::accept(&mut *tx, pair, other).await? {
            return Err(ChessCode::NoFriendRequest.into());
        }
        tx.commit().await?;
        self.tell_both(user_id, other);
        Ok(())
    }

    /// Unfriends `other`, declines their request, or withdraws one sent to them.
    pub async fn remove(&self, user_id: Uuid, other: Uuid) -> Result<(), AppError> {
        if friendship::remove(&self.db, pair(user_id, other)).await? {
            // A rematch is between two who just played, friends or not: it stays.
            self.play.drop_friend_challenges_between(user_id, other);
            self.play.drop_battle_invitations_between(user_id, other);
            self.tell_both(user_id, other);
        }
        Ok(())
    }

    /// Blocks `other`: nothing stays between the two, and `other` cannot ask or message again.
    pub async fn block(&self, user_id: Uuid, other: Uuid) -> Result<(), AppError> {
        if other == user_id {
            return Err(ChessCode::CannotBefriendYourself.into());
        }
        if user::get(&self.db, other).await?.is_none() {
            return Err(ChessCode::UserNotFound.into());
        }
        let mut tx = self.db.begin().await?;
        friendship::lock_pair(&mut *tx, pair(user_id, other)).await?;
        // The block first: it takes the users' key locks before any friendship row's lock, the
        // order closing an account takes them in too.
        block::insert(&mut *tx, user_id, other).await?;
        friendship::remove(&mut *tx, pair(user_id, other)).await?;
        tx.commit().await?;
        self.play.drop_challenges_between(user_id, other);
        self.play.drop_battle_invitations_between(user_id, other);
        self.tell_both(user_id, other);
        Ok(())
    }

    /// Lifts the player's block on `other`: the two are strangers again.
    pub async fn unblock(&self, user_id: Uuid, other: Uuid) -> Result<(), AppError> {
        if block::delete(&self.db, user_id, other).await? {
            self.tell_both(user_id, other);
        }
        Ok(())
    }

    fn tell_both(&self, first: Uuid, second: Uuid) {
        for user in [first, second] {
            self.play.send(user, ServerMessage::FriendsChanged);
        }
    }
}

/// A pair as `social.friendships` keys it: the smaller id first.
fn pair(first: Uuid, second: Uuid) -> (Uuid, Uuid) {
    (first.min(second), first.max(second))
}

/// Where `user_id` stands with the row's player.
fn relation(user_id: Uuid, (status, requester_id): (Option<&str>, Option<Uuid>)) -> Relation {
    match status.and_then(FriendshipStatus::from_key) {
        Some(FriendshipStatus::Accepted) => Relation::Friends,
        Some(FriendshipStatus::Pending) if requester_id == Some(user_id) => Relation::RequestSent,
        Some(FriendshipStatus::Pending) => Relation::RequestReceived,
        Some(FriendshipStatus::Declined) | None => Relation::None,
    }
}

/// `query` with LIKE's wildcards and escape taken literally.
fn like_escaped(query: &str) -> String {
    query
        .replace('\\', "\\\\")
        .replace('%', "\\%")
        .replace('_', "\\_")
}

/// The ratings, whole, gathered by player and then by category.
fn ratings_by_player(rows: Vec<UserRatingRow>) -> HashMap<Uuid, BTreeMap<String, i32>> {
    let mut by_player: HashMap<Uuid, BTreeMap<String, i32>> = HashMap::new();
    for row in rows {
        by_player
            .entry(row.user_id)
            .or_default()
            .insert(row.category_id, whole(row.rating));
    }
    by_player
}

#[cfg(test)]
mod tests {
    use uuid::Uuid;

    use super::{like_escaped, pair, ratings_by_player, relation};
    use crate::{enums::social::Relation, models::rating::UserRatingRow};

    #[test]
    fn each_side_of_a_request_sees_its_own_relation() {
        let (me, them) = (Uuid::from_u128(1), Uuid::from_u128(2));
        assert_eq!(
            relation(me, (Some("pending"), Some(me))),
            Relation::RequestSent
        );
        assert_eq!(
            relation(me, (Some("pending"), Some(them))),
            Relation::RequestReceived
        );
        assert_eq!(
            relation(me, (Some("accepted"), Some(them))),
            Relation::Friends
        );
        assert_eq!(relation(me, (None, None)), Relation::None);
    }

    #[test]
    fn pairs_put_the_smaller_id_first() {
        let (low, high) = (Uuid::from_u128(1), Uuid::from_u128(2));
        assert_eq!(pair(high, low), (low, high));
        assert_eq!(pair(low, high), (low, high));
    }

    #[test]
    fn wildcards_are_searched_for_literally() {
        assert_eq!(like_escaped("a_b%c\\d"), "a\\_b\\%c\\\\d");
    }

    #[test]
    fn ratings_are_gathered_by_player_and_rounded() {
        let (ann, bob) = (Uuid::from_u128(1), Uuid::from_u128(2));
        let row = |user_id, category: &str, rating| UserRatingRow {
            user_id,
            category_id: category.to_owned(),
            rating,
        };
        let by_player = ratings_by_player(vec![
            row(ann, "blitz", 1512.6),
            row(ann, "puzzle", 1800.2),
            row(bob, "bullet", 1499.5),
        ]);
        assert_eq!(by_player[&ann]["blitz"], 1513);
        assert_eq!(by_player[&ann]["puzzle"], 1800);
        assert_eq!(by_player[&bob]["bullet"], 1500);
        assert_eq!(by_player.len(), 2);
    }
}
