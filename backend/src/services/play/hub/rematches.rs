use uuid::Uuid;

use super::{
    PlayHub,
    challenges::{Terms, Unaccepted},
    seek_refusal,
};
use crate::{
    models::play::{game::RematchTerms, player::Player},
    repositories::{account::user, play::game, social::block},
};

/// A rematch refused because the database failed, not because of anyone.
const UNAVAILABLE: &str = "A rematch is unavailable right now";

impl PlayHub {
    /// Asks the last opponent to play again, colours swapped; if they asked first, it starts.
    pub(super) async fn offer_rematch(&self, player: &Player, game_id: Uuid) {
        if let Err(reason) = self.post_rematch(player, game_id).await {
            self.refuse_for(player.id, Some(game_id), &reason);
        }
    }

    /// Posts the rematch, or accepts theirs, or says why not.
    async fn post_rematch(&self, player: &Player, game_id: Uuid) -> Result<(), String> {
        let gone = "That game can no longer be played again".to_owned();
        let terms = match game::rematch_terms(&self.inner.db, game_id, player.id).await {
            Ok(Some(terms)) => terms,
            Ok(None) => return Err(gone),
            Err(error) => {
                tracing::error!(?error, "reading a game to replay failed");
                return Err(UNAVAILABLE.to_owned());
            }
        };
        // Signed in again since, or a shared guest gone to someone new: not who played it.
        if !self.still_seated(player.id, game_id).await? {
            return Err("You signed in again since that game: start a new one".to_owned());
        }
        if self.rematch_from(player.id, game_id).is_some() {
            self.asked_elsewhere(player.id, game_id);
            return Ok(());
        }
        let opponent_id = other_player(&terms, player.id);
        if let Some(theirs) = self.rematch_from(opponent_id, game_id) {
            match self.try_accept(player, theirs).await {
                Ok(()) => return Ok(()),
                Err(Unaccepted::Refused(reason)) => return Err(reason),
                // Theirs went just now: the player's own is asked after all.
                Err(Unaccepted::Gone) => {}
            }
        }
        let opponent = self
            .rematch_opponent(player, (opponent_id, game_id))
            .await?;
        let (initial_s, increment_s) = terms_seconds(&terms);
        seek_refusal(player, (initial_s, increment_s), terms.rated).map_err(str::to_owned)?;
        let rematch = Terms {
            initial_s,
            increment_s,
            rated: terms.rated,
            rematch_of: Some(game_id),
            // Whoever had Black plays White this time.
            white: Some(terms.black_id),
        };
        let (new, pending) = self.draw_up((player, opponent), rematch);
        self.post_challenge(new, pending).await;
        Ok(())
    }

    /// Whether the player is still who played the game; a database failure says so.
    async fn still_seated(&self, player: Uuid, game_id: Uuid) -> Result<bool, String> {
        game::seated_since(&self.inner.db, game_id, player)
            .await
            .map_err(|_| UNAVAILABLE.to_owned())
    }

    /// Whether neither of the two has blocked the other; a database failure says so.
    async fn unblocked_or_unavailable(&self, first: Uuid, second: Uuid) -> Result<bool, String> {
        block::blockers_between(&self.inner.db, first, second)
            .await
            .map(|blockers| blockers.is_empty())
            .map_err(|_| UNAVAILABLE.to_owned())
    }

    /// The rematch of this game `from` asked for, if it is still waiting.
    fn rematch_from(&self, from: Uuid, game_id: Uuid) -> Option<Uuid> {
        self.find_challenge(|pending| pending.is_rematch_by(from, game_id))
    }

    /// The opponent to ask, or why they cannot be: gone, offline, busy, or blocked either way.
    async fn rematch_opponent(
        &self,
        player: &Player,
        (opponent, game_id): (Uuid, Uuid),
    ) -> Result<Player, String> {
        let opponent = user::get(&self.inner.db, opponent)
            .await
            .ok()
            .flatten()
            .map(Player::from)
            .ok_or_else(|| "Your opponent has left".to_owned())?;
        // A block is not named: the rematch is simply not to be had.
        let there = self.is_online(opponent.id) && self.still_seated(opponent.id, game_id).await?;
        if !there
            || !self
                .unblocked_or_unavailable(player.id, opponent.id)
                .await?
        {
            return Err(format!("{} is not taking a rematch", opponent.username));
        }
        match self.busy_refusal(player.id, &opponent) {
            Some(busy) => Err(busy),
            None => Ok(opponent),
        }
    }
}

/// The game's other player.
fn other_player(terms: &RematchTerms, player: Uuid) -> Uuid {
    if terms.white_id == player {
        terms.black_id
    } else {
        terms.white_id
    }
}

/// The game's clock, in the seconds a challenge counts in.
fn terms_seconds(terms: &RematchTerms) -> (u32, u32) {
    (
        u32::try_from(terms.initial_s).unwrap_or_default(),
        u32::try_from(terms.increment_s).unwrap_or_default(),
    )
}
