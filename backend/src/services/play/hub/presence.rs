use super::PlayHub;
use crate::{
    models::play::{messages::ServerMessage, player::Player},
    repositories::social::friendship,
};

impl PlayHub {
    /// Tells the player's friends who are online to fetch their friends again: the player came
    /// or went.
    pub(super) fn tell_friends(&self, player: &Player) {
        let (hub, player_id) = (self.clone(), player.id);
        tokio::spawn(async move {
            match friendship::friend_ids(&hub.inner.db, player_id).await {
                Ok(friends) => {
                    for friend in friends {
                        hub.send(friend, ServerMessage::FriendsChanged);
                    }
                }
                Err(error) => tracing::error!(?error, "finding friends to tell failed"),
            }
        });
    }
}
