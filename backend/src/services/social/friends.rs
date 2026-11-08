use uuid::Uuid;

use crate::{
    enums::social::{FriendshipStatus, Relation},
    errors::{AppError, ChessCode},
    models::{
        play::messages::ServerMessage,
        social::friend::{FoundUserModel, FriendModel, FriendshipRow, GetFriendsModel, UserRef},
    },
    repositories::{account::user, database::Db, social::friendship},
    services::play::PlayHub,
};

/// The fewest letters a search looks for, and the most players it brings back.
const SHORTEST_SEARCH: usize = 2;
const MOST_FOUND: i64 = 20;

/// Friends and friend requests between players, guests included. Every change
/// tells both players over the live socket, so their lists follow at once.
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
                relation: relation(user_id, &row),
                id: row.user_id,
                username: row.username,
            })
            .collect();
        Ok(response)
    }

    /// Friends, online first, and the requests each way.
    pub async fn list(&self, user_id: Uuid) -> Result<GetFriendsModel, AppError> {
        let rows = friendship::of_user(&self.db, user_id).await?;
        let mut response = GetFriendsModel {
            friends: Vec::new(),
            incoming: Vec::new(),
            outgoing: Vec::new(),
        };
        for row in rows {
            let relation = relation(user_id, &row);
            let person = UserRef {
                id: row.user_id,
                username: row.username,
            };
            match relation {
                Relation::Friends => response.friends.push(FriendModel {
                    online: self.play.is_online(person.id),
                    id: person.id,
                    username: person.username,
                }),
                Relation::RequestReceived => response.incoming.push(person),
                Relation::RequestSent => response.outgoing.push(person),
                Relation::None => {}
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
        let current = friendship::between(&self.db, pair).await?;
        let status = current
            .as_ref()
            .and_then(|row| FriendshipStatus::from_key(&row.status));
        let asked_by_them = current
            .as_ref()
            .is_some_and(|row| row.requester_id == other);
        let relation = match status {
            Some(FriendshipStatus::Accepted) => Relation::Friends,
            Some(FriendshipStatus::Pending) if asked_by_them => {
                friendship::accept(&self.db, pair, other).await?;
                Relation::Friends
            }
            Some(FriendshipStatus::Pending) => Relation::RequestSent,
            // A block is never said: it looks like a request that goes unanswered.
            Some(FriendshipStatus::Blocked) => Relation::RequestSent,
            Some(FriendshipStatus::Declined) | None => {
                friendship::request(&self.db, pair, user_id).await?;
                Relation::RequestSent
            }
        };
        self.tell_both(user_id, other);
        Ok(relation)
    }

    /// Accepts the request `other` sent.
    pub async fn accept(&self, user_id: Uuid, other: Uuid) -> Result<(), AppError> {
        if !friendship::accept(&self.db, pair(user_id, other), other).await? {
            return Err(ChessCode::NoFriendRequest.into());
        }
        self.tell_both(user_id, other);
        Ok(())
    }

    /// Unfriends `other`, declines their request, or withdraws one sent to them.
    pub async fn remove(&self, user_id: Uuid, other: Uuid) -> Result<(), AppError> {
        if friendship::remove(&self.db, pair(user_id, other)).await? {
            self.play.drop_challenges_between(user_id, other);
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
fn relation(user_id: Uuid, row: &FriendshipRow) -> Relation {
    let status = row.status.as_deref().and_then(FriendshipStatus::from_key);
    match status {
        Some(FriendshipStatus::Accepted) => Relation::Friends,
        Some(FriendshipStatus::Pending) if row.requester_id == Some(user_id) => {
            Relation::RequestSent
        }
        Some(FriendshipStatus::Pending) => Relation::RequestReceived,
        Some(FriendshipStatus::Declined | FriendshipStatus::Blocked) | None => Relation::None,
    }
}

/// `query` with LIKE's wildcards and escape taken literally.
fn like_escaped(query: &str) -> String {
    query
        .replace('\\', "\\\\")
        .replace('%', "\\%")
        .replace('_', "\\_")
}

#[cfg(test)]
mod tests {
    use uuid::Uuid;

    use super::{like_escaped, pair, relation};
    use crate::{enums::social::Relation, models::social::friend::FriendshipRow};

    fn row(status: Option<&str>, requester: Option<Uuid>) -> FriendshipRow {
        FriendshipRow {
            user_id: Uuid::from_u128(2),
            username: "bob".to_owned(),
            status: status.map(str::to_owned),
            requester_id: requester,
        }
    }

    #[test]
    fn each_side_of_a_request_sees_its_own_relation() {
        let (me, them) = (Uuid::from_u128(1), Uuid::from_u128(2));
        assert_eq!(
            relation(me, &row(Some("pending"), Some(me))),
            Relation::RequestSent
        );
        assert_eq!(
            relation(me, &row(Some("pending"), Some(them))),
            Relation::RequestReceived
        );
        assert_eq!(
            relation(me, &row(Some("accepted"), Some(them))),
            Relation::Friends
        );
        assert_eq!(
            relation(me, &row(Some("blocked"), Some(them))),
            Relation::None
        );
        assert_eq!(relation(me, &row(None, None)), Relation::None);
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
}
