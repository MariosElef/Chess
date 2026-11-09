use std::time::Duration;

use chess_domain::TimeControl;
use uuid::Uuid;

use super::{PlayHub, lock, seek_refusal};
use crate::{
    enums::{play::ChallengeStatus, social::FriendshipStatus},
    errors::AppError,
    models::{
        play::{
            challenge::{ChallengeModel, NewChallenge},
            messages::ServerMessage,
            player::Player,
        },
        social::friend::UserRef,
    },
    repositories::{account::user, play::challenge, social::friendship},
};

/// How long a challenge waits for an answer.
const CHALLENGE_WAIT: Duration = Duration::from_secs(60);

/// A challenge waiting for its answer, with what starting the game needs.
pub(super) struct PendingChallenge {
    model: ChallengeModel,
    control: TimeControl,
    from: Player,
    to: Player,
}

impl PendingChallenge {
    fn involves(&self, player: Uuid) -> bool {
        self.from.id == player || self.to.id == player
    }
}

impl PlayHub {
    /// Invites an online, free friend to a game for a minute, withdrawing any earlier challenge.
    pub(super) async fn challenge(
        &self,
        player: &Player,
        to: Uuid,
        (initial_s, increment_s): (u32, u32),
        rated: bool,
    ) {
        if let Err(reason) = seek_refusal(player, (initial_s, increment_s), rated) {
            return self.refuse(player.id, reason);
        }
        let opponent = match self.challengeable(player, to).await {
            Ok(opponent) => opponent,
            Err(reason) => return self.refuse(player.id, &reason),
        };
        self.withdraw_challenge_from(player.id);
        let new = NewChallenge {
            id: Uuid::now_v7(),
            challenger_id: player.id,
            challenged_id: opponent.id,
            initial_s: i32::try_from(initial_s).unwrap_or(i32::MAX),
            increment_s: i32::try_from(increment_s).unwrap_or(i32::MAX),
            rated,
            expires_in_s: i32::try_from(CHALLENGE_WAIT.as_secs()).unwrap_or(i32::MAX),
        };
        if let Err(error) = challenge::insert(&self.inner.db, &new).await {
            tracing::error!(?error, "storing a challenge failed");
            return self.refuse(player.id, "The challenge could not be sent: try again");
        }
        let pending = PendingChallenge {
            model: ChallengeModel {
                id: new.id,
                from: user_ref(player),
                to: user_ref(&opponent),
                initial_s,
                increment_s,
                rated,
            },
            control: TimeControl {
                initial: Duration::from_secs(initial_s.into()),
                increment: Duration::from_secs(increment_s.into()),
            },
            from: player.clone(),
            to: opponent,
        };
        let (from, to) = (pending.from.id, pending.to.id);
        lock(&self.inner.challenges).push(pending);
        self.send_challenges(from);
        self.send_challenges(to);
        self.expire_later(new.id);
    }

    /// Whether the two are friends now: a block or unfriending since a challenge calls it off.
    pub(super) async fn are_friends(&self, first: Uuid, second: Uuid) -> Result<bool, AppError> {
        let pair = (first.min(second), first.max(second));
        let friendship = friendship::between(&self.inner.db, pair).await?;
        Ok(friendship.is_some_and(|row| {
            FriendshipStatus::from_key(&row.status) == Some(FriendshipStatus::Accepted)
        }))
    }

    /// The friend to challenge, or why they cannot be.
    pub(super) async fn challengeable(&self, player: &Player, to: Uuid) -> Result<Player, String> {
        if to == player.id {
            return Err("You cannot challenge yourself".to_owned());
        }
        let friends = self
            .are_friends(player.id, to)
            .await
            .map_err(|_| "Challenges are unavailable right now".to_owned())?;
        if !friends {
            return Err("You can only challenge friends".to_owned());
        }
        let found = user::get(&self.inner.db, to)
            .await
            .ok()
            .flatten()
            .ok_or_else(|| "No such player".to_owned())?;
        let opponent = Player {
            id: found.id,
            username: found.username,
            guest: found.guest,
        };
        if !self.is_online(to) {
            return Err(format!("{} is offline", opponent.username));
        }
        if !self.games_of(player.id).is_empty() {
            return Err("Finish your game first".to_owned());
        }
        if !self.games_of(to).is_empty() {
            return Err(format!("{} is playing a game", opponent.username));
        }
        Ok(opponent)
    }

    /// Starts the challenged game, calling off every other challenge either player has waiting.
    pub(super) async fn accept_challenge(&self, player: &Player, challenge_id: Uuid) {
        let found = lock(&self.inner.challenges)
            .iter()
            .find(|pending| pending.model.id == challenge_id && pending.to.id == player.id)
            .map(|pending| (pending.from.clone(), pending.control, pending.model.rated));
        let Some((from, control, rated)) = found else {
            return self.refuse(player.id, "That challenge is no longer open");
        };
        if !self.are_friends(from.id, player.id).await.unwrap_or(false) {
            self.drop_challenges_between(from.id, player.id);
            return self.refuse(player.id, "That challenge is no longer open");
        }
        if !self.games_of(player.id).is_empty() {
            return self.refuse(player.id, "Finish your game first");
        }
        if !self.games_of(from.id).is_empty() {
            return self.refuse(player.id, &format!("{} is playing a game", from.username));
        }
        let Some(pending) = self.take_challenge(|pending| pending.model.id == challenge_id) else {
            return self.refuse(player.id, "That challenge is no longer open");
        };
        let game_id = self.start_game((&from, player), control, rated).await;
        let status = if game_id.is_some() {
            ChallengeStatus::Accepted
        } else {
            ChallengeStatus::Cancelled
        };
        self.record(pending.model.id, status, game_id);
        self.send_challenges(from.id);
        self.send_challenges(player.id);
    }

    pub(super) fn decline_challenge(&self, player: &Player, challenge_id: Uuid) {
        let declined = self.take_challenge(|pending| {
            pending.model.id == challenge_id && pending.to.id == player.id
        });
        self.settle(declined, ChallengeStatus::Declined);
    }

    pub(super) fn cancel_challenge(&self, player: &Player, challenge_id: Uuid) {
        let cancelled = self.take_challenge(|pending| {
            pending.model.id == challenge_id && pending.from.id == player.id
        });
        self.settle(cancelled, ChallengeStatus::Cancelled);
    }

    /// Every challenge to or from the player is called off: they left, or started a game.
    pub(super) fn drop_challenges_of(&self, player: Uuid) {
        while let Some(pending) = self.take_challenge(|pending| pending.involves(player)) {
            self.settle(Some(pending), ChallengeStatus::Cancelled);
        }
    }

    /// Calls off any challenge between the two: they are no longer friends.
    pub fn drop_challenges_between(&self, first: Uuid, second: Uuid) {
        let between =
            |pending: &PendingChallenge| pending.involves(first) && pending.involves(second);
        while let Some(pending) = self.take_challenge(between) {
            self.settle(Some(pending), ChallengeStatus::Cancelled);
        }
    }

    /// Sends the player the challenges waiting each way.
    pub(super) fn send_challenges(&self, player: Uuid) {
        let (sent, received) = {
            let challenges = lock(&self.inner.challenges);
            let models_where = |keep: &dyn Fn(&PendingChallenge) -> bool| {
                challenges
                    .iter()
                    .filter(|pending| keep(pending))
                    .map(|pending| pending.model.clone())
                    .collect::<Vec<_>>()
            };
            (
                models_where(&|pending| pending.from.id == player),
                models_where(&|pending| pending.to.id == player),
            )
        };
        self.send(player, ServerMessage::Challenges { sent, received });
    }

    fn withdraw_challenge_from(&self, player: Uuid) {
        let earlier = self.take_challenge(|pending| pending.from.id == player);
        self.settle(earlier, ChallengeStatus::Cancelled);
    }

    /// After the wait, a challenge still unanswered expires.
    fn expire_later(&self, challenge_id: Uuid) {
        let hub = self.clone();
        tokio::spawn(async move {
            tokio::time::sleep(CHALLENGE_WAIT).await;
            let expired = hub.take_challenge(|pending| pending.model.id == challenge_id);
            hub.settle(expired, ChallengeStatus::Expired);
        });
    }

    fn take_challenge(
        &self,
        matches: impl Fn(&PendingChallenge) -> bool,
    ) -> Option<PendingChallenge> {
        let mut challenges = lock(&self.inner.challenges);
        let index = challenges.iter().position(matches)?;
        Some(challenges.remove(index))
    }

    /// Stores how a challenge taken off the list ended, and tells both players.
    fn settle(&self, taken: Option<PendingChallenge>, status: ChallengeStatus) {
        let Some(pending) = taken else {
            return;
        };
        self.record(pending.model.id, status, None);
        self.send_challenges(pending.from.id);
        self.send_challenges(pending.to.id);
    }

    /// Stores how a challenge ended, without holding anyone up.
    fn record(&self, challenge_id: Uuid, status: ChallengeStatus, game_id: Option<Uuid>) {
        let db = self.inner.db.clone();
        tokio::spawn(async move {
            if let Err(error) = challenge::resolve(&db, challenge_id, status, game_id).await {
                tracing::error!(?error, "storing how a challenge ended failed");
            }
        });
    }
}

pub(super) fn user_ref(player: &Player) -> UserRef {
    UserRef {
        id: player.id,
        username: player.username.clone(),
    }
}
