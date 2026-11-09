use std::time::Duration;

use chess_domain::Rating;
use tokio::sync::mpsc;
use uuid::Uuid;

use super::{PlayHub, challenges::user_ref, lock};
use crate::{
    models::{
        play::{messages::ServerMessage, player::Player},
        puzzles::battle::{BattleChallengeModel, NewBattle},
    },
    repositories::{puzzles::battle, rating},
    services::{
        play::live_battle::{BattleCommand, BattlePlayer, BattleStart, spawn_battle},
        puzzles::{puzzle::battle_ladder, rating::BATTLE_CATEGORY},
        rating::{from_row, whole},
    },
};

/// How long a battle invitation waits for an answer.
const INVITATION_WAIT: Duration = Duration::from_secs(60);

/// A player waiting for a battle opponent, rated or not.
pub(super) struct BattleSeek {
    player: Player,
    rated: bool,
}

/// A battle invitation waiting for its answer.
pub(super) struct PendingBattle {
    model: BattleChallengeModel,
    from: Player,
    to: Player,
}

impl PendingBattle {
    fn involves(&self, player: Uuid) -> bool {
        self.from.id == player || self.to.id == player
    }
}

/// A battle being played: its players, and where their answers go.
pub(super) struct BattleEntry {
    players: [Uuid; 2],
    commands: mpsc::Sender<BattleCommand>,
}

impl PlayHub {
    /// Pairs the player with the next one wanting the same battle, or queues them.
    pub(super) async fn seek_battle(&self, player: &Player, rated: bool) {
        let rated = rated && !player.guest;
        if self.is_battling(player.id) {
            return self.refuse(player.id, "Finish your battle first");
        }
        let opponent = {
            let mut seeks = lock(&self.inner.battle_seeks);
            seeks.retain(|seek| seek.player.id != player.id);
            match seeks.iter().position(|seek| seek.rated == rated) {
                Some(found) => Some(seeks.remove(found).player),
                None => {
                    seeks.push(BattleSeek {
                        player: player.clone(),
                        rated,
                    });
                    None
                }
            }
        };
        match opponent {
            Some(opponent) => self.start_battle([opponent, player.clone()], rated).await,
            None => self.send(player.id, ServerMessage::BattleSeeking),
        }
    }

    pub(super) fn cancel_battle_seek(&self, player: Uuid) {
        lock(&self.inner.battle_seeks).retain(|seek| seek.player.id != player);
        self.send(player, ServerMessage::BattleSeekCancelled);
    }

    /// The player's answer, passed to their battle.
    pub(super) async fn battle_answer(&self, player: &Player, battle_id: Uuid, solved: bool) {
        let commands = lock(&self.inner.battles)
            .get(&battle_id)
            .filter(|entry| entry.players.contains(&player.id))
            .map(|entry| entry.commands.clone());
        if let Some(commands) = commands {
            // A finished battle takes no more answers; the player already has its result.
            let _ = commands
                .send(BattleCommand::Answer {
                    player: player.id,
                    solved,
                })
                .await;
        }
    }

    /// The player gives up their battle: it ends, and the other player wins.
    pub(super) async fn resign_battle(&self, player: &Player, battle_id: Uuid) {
        let commands = lock(&self.inner.battles)
            .get(&battle_id)
            .filter(|entry| entry.players.contains(&player.id))
            .map(|entry| entry.commands.clone());
        if let Some(commands) = commands {
            // A battle already over has its result; resigning changes nothing.
            let _ = commands
                .send(BattleCommand::Resign { player: player.id })
                .await;
        }
    }

    /// The player's battle, if any, resigned for them.
    pub(super) fn resign_battles_of(&self, player: Uuid) {
        if let Some(commands) = self.battle_of(player) {
            // A battle already over has its result.
            let _ = commands.try_send(BattleCommand::Resign { player });
        }
    }

    /// The battle is over: it takes no more answers.
    pub fn battle_over(&self, battle_id: Uuid) {
        lock(&self.inner.battles).remove(&battle_id);
    }

    /// The battle the player is in, as it stands, sent to them; for a socket just connected.
    pub(super) fn resend_battle(&self, player: Uuid) {
        if let Some(commands) = self.battle_of(player) {
            // A full or finished battle has its own way to reach the player.
            let _ = commands.try_send(BattleCommand::Snapshot { player });
        }
    }

    /// Invites an online friend to a battle for a minute, withdrawing any earlier invitation.
    pub(super) async fn challenge_battle(&self, player: &Player, to: Uuid, rated: bool) {
        if self.is_battling(player.id) {
            return self.refuse(player.id, "Finish your battle first");
        }
        let opponent = match self.challengeable(player, to).await {
            Ok(opponent) => opponent,
            Err(reason) => return self.refuse(player.id, &reason),
        };
        if self.is_battling(opponent.id) {
            let reason = format!("{} is in a battle", opponent.username);
            return self.refuse(player.id, &reason);
        }
        let rated = rated && !player.guest && !opponent.guest;
        let pending = PendingBattle {
            model: BattleChallengeModel {
                id: Uuid::now_v7(),
                from: user_ref(player),
                to: user_ref(&opponent),
                rated,
            },
            from: player.clone(),
            to: opponent,
        };
        let (id, from, to) = (pending.model.id, pending.from.id, pending.to.id);
        {
            let mut pendings = lock(&self.inner.battle_challenges);
            pendings.retain(|old| old.from.id != player.id);
            pendings.push(pending);
        }
        self.send_battle_challenges(from);
        self.send_battle_challenges(to);
        self.expire_battle_later(id);
    }

    /// The invited player says yes: the battle starts for both.
    pub(super) async fn accept_battle(&self, player: &Player, challenge_id: Uuid) {
        let taken = self.take_battle_challenge(|pending| {
            pending.model.id == challenge_id && pending.to.id == player.id
        });
        let Some(pending) = taken else {
            return self.refuse(player.id, "That invitation is no longer open");
        };
        self.send_battle_challenges(pending.from.id);
        self.send_battle_challenges(pending.to.id);
        if !self
            .are_friends(pending.from.id, pending.to.id)
            .await
            .unwrap_or(false)
        {
            return self.refuse(player.id, "That invitation is no longer open");
        }
        if self.is_battling(pending.from.id) || self.is_battling(pending.to.id) {
            return self.refuse(player.id, "One of you is already in a battle");
        }
        self.start_battle([pending.from, pending.to], pending.model.rated)
            .await;
    }

    /// Either player calls it off: the invited one declining, or the one who sent it.
    pub(super) fn decline_battle(&self, player: &Player, challenge_id: Uuid) {
        let taken = self.take_battle_challenge(|pending| {
            pending.model.id == challenge_id && pending.involves(player.id)
        });
        if let Some(pending) = taken {
            self.send_battle_challenges(pending.from.id);
            self.send_battle_challenges(pending.to.id);
        }
    }

    /// The battle invitations waiting, each way, sent to the player.
    pub(super) fn send_battle_challenges(&self, player: Uuid) {
        let (sent, received) = {
            let pendings = lock(&self.inner.battle_challenges);
            let models = |mine: fn(&PendingBattle, Uuid) -> bool| {
                pendings
                    .iter()
                    .filter(|pending| mine(pending, player))
                    .map(|pending| pending.model.clone())
                    .collect::<Vec<_>>()
            };
            (
                models(|pending, player| pending.from.id == player),
                models(|pending, player| pending.to.id == player),
            )
        };
        self.send(player, ServerMessage::BattleChallenges { sent, received });
    }

    /// The player left: their battle seek and invitations go.
    pub(super) fn drop_battle_wishes_of(&self, player: Uuid) {
        lock(&self.inner.battle_seeks).retain(|seek| seek.player.id != player);
        self.drop_invitations_of(player);
    }

    /// Picks the puzzles, stores the battle and starts it; both players hear it began.
    async fn start_battle(&self, players: [Player; 2], rated: bool) {
        let ids = [players[0].id, players[1].id];
        // Checked and reserved in one step: two requests at once cannot both take a player.
        let refusal = {
            let mut starting = lock(&self.inner.battles_starting);
            let refusal = if ids.iter().any(|id| self.is_closed(*id)) {
                Some("That battle could not start: your opponent has left")
            } else if ids
                .iter()
                .any(|id| starting.contains(id) || self.battle_of(*id).is_some())
            {
                Some("That battle could not start: one of you is already in one")
            } else {
                None
            };
            if refusal.is_none() {
                starting.extend(ids);
            }
            refusal
        };
        if let Some(refusal) = refusal {
            for id in ids {
                self.refuse(id, refusal);
                self.send(id, ServerMessage::BattleSeekCancelled);
            }
            return;
        }
        for id in ids {
            lock(&self.inner.battle_seeks).retain(|seek| seek.player.id != id);
            self.drop_invitations_of(id);
        }
        let started = self.set_up_battle(players, rated).await;
        let mut starting = lock(&self.inner.battles_starting);
        for id in ids {
            starting.remove(&id);
        }
        drop(starting);
        if let Err(reason) = started {
            for id in ids {
                self.refuse(id, reason);
                self.send(id, ServerMessage::BattleSeekCancelled);
            }
        }
    }

    /// The battle's puzzles, its row and its task; why not, if one could not be had.
    async fn set_up_battle(&self, players: [Player; 2], rated: bool) -> Result<(), &'static str> {
        let puzzles = match battle_ladder(&self.inner.db).await {
            Ok(puzzles) => puzzles,
            Err(error) => {
                tracing::error!(?error, "picking a battle's puzzles failed");
                return Err("Puzzle Battle needs the puzzle database imported");
            }
        };
        let [first, second] = players;
        let new = NewBattle {
            id: Uuid::now_v7(),
            first_id: first.id,
            second_id: second.id,
            rated,
        };
        if let Err(error) = battle::insert(&self.inner.db, &new).await {
            tracing::error!(?error, "starting a battle failed");
            return Err("The battle could not start: try again");
        }
        let start = BattleStart {
            id: new.id,
            players: [
                self.battler(first, rated).await,
                self.battler(second, rated).await,
            ],
            rated,
            puzzles,
        };
        let players = [start.players[0].id, start.players[1].id];
        let commands = spawn_battle(start, self.clone(), self.inner.db.clone());
        lock(&self.inner.battles).insert(new.id, BattleEntry { players, commands });
        // A player whose account closed while it was being set up resigns it at once.
        for player in players {
            if self.is_closed(player) {
                self.resign_battles_of(player);
            }
        }
        Ok(())
    }

    /// In a battle, or about to be.
    fn is_battling(&self, player: Uuid) -> bool {
        self.battle_of(player).is_some() || lock(&self.inner.battles_starting).contains(&player)
    }

    /// Battle invitations between the two, either way, withdrawn; both hear.
    pub fn drop_battle_invitations_between(&self, first: Uuid, second: Uuid) {
        lock(&self.inner.battle_challenges)
            .retain(|pending| !(pending.involves(first) && pending.involves(second)));
        for player in [first, second] {
            self.send_battle_challenges(player);
        }
    }

    /// The player's battle invitations, either way, withdrawn; whoever else was in them hears.
    fn drop_invitations_of(&self, player: Uuid) {
        let others: Vec<Uuid> = {
            let mut pendings = lock(&self.inner.battle_challenges);
            let others = pendings
                .iter()
                .filter(|pending| pending.involves(player))
                .map(|pending| {
                    if pending.from.id == player {
                        pending.to.id
                    } else {
                        pending.from.id
                    }
                })
                .collect();
            pendings.retain(|pending| !pending.involves(player));
            others
        };
        for other in others {
            self.send_battle_challenges(other);
        }
    }

    /// The player as their opponent sees them: with their battle rating, in a rated battle.
    async fn battler(&self, player: Player, rated: bool) -> BattlePlayer {
        let stored = match rating::get(&self.inner.db, player.id, BATTLE_CATEGORY).await {
            Ok(stored) => stored,
            Err(error) => {
                tracing::error!(?error, "reading a battle rating failed");
                None
            }
        };
        let at_start = stored.map_or(Rating::NEW, from_row);
        BattlePlayer {
            id: player.id,
            username: player.username,
            rating: rated.then(|| whole(at_start.rating)),
            at_start,
        }
    }

    fn battle_of(&self, player: Uuid) -> Option<mpsc::Sender<BattleCommand>> {
        lock(&self.inner.battles)
            .values()
            .find(|entry| entry.players.contains(&player))
            .map(|entry| entry.commands.clone())
    }

    fn take_battle_challenge(
        &self,
        matches: impl Fn(&PendingBattle) -> bool,
    ) -> Option<PendingBattle> {
        let mut pendings = lock(&self.inner.battle_challenges);
        let found = pendings.iter().position(matches)?;
        Some(pendings.remove(found))
    }

    fn expire_battle_later(&self, challenge_id: Uuid) {
        let hub = self.clone();
        tokio::spawn(async move {
            tokio::time::sleep(INVITATION_WAIT).await;
            let expired = hub.take_battle_challenge(|pending| pending.model.id == challenge_id);
            if let Some(pending) = expired {
                hub.send_battle_challenges(pending.from.id);
                hub.send_battle_challenges(pending.to.id);
            }
        });
    }
}
