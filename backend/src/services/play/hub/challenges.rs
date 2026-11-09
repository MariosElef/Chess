use std::time::{Duration, Instant};

use chess_domain::TimeControl;
use uuid::Uuid;

use super::{PlayHub, Players, lock, seek_refusal, whole_seconds, whole_seconds_up};
use crate::{
    enums::{
        play::{ChallengeStatus, Color},
        social::FriendshipStatus,
    },
    errors::AppError,
    models::play::{
        challenge::{ChallengeModel, ListedChallenge, NewChallenge},
        messages::ServerMessage,
        player::Player,
    },
    repositories::{
        account::user,
        play::challenge,
        social::{block, friendship},
    },
};

/// How long a challenge, or a rematch, waits for an answer.
const CHALLENGE_WAIT: Duration = Duration::from_secs(60);

/// A challenge waiting for its answer, with what starting the game needs.
pub(super) struct PendingChallenge {
    model: ChallengeModel,
    control: TimeControl,
    from: Player,
    to: Player,
    /// Who plays White, for a rematch; a challenge leaves it to chance.
    white: Option<Uuid>,
    /// When it was put to them, for the time it has left.
    posted: Instant,
}

/// What a challenge asks for: its clock and rating, and for a rematch the game and colours.
pub(super) struct Terms {
    pub(super) initial_s: u32,
    pub(super) increment_s: u32,
    pub(super) rated: bool,
    pub(super) rematch_of: Option<Uuid>,
    pub(super) white: Option<Uuid>,
}

/// What starting a challenged game needs, read off it before it is taken.
struct Accepting {
    from: Player,
    control: TimeControl,
    rated: bool,
    white: Option<Uuid>,
    rematch: bool,
}

impl PendingChallenge {
    fn involves(&self, player: Uuid) -> bool {
        self.from.id == player || self.to.id == player
    }

    /// Whether this is `from`'s rematch of the game.
    pub(super) fn is_rematch_by(&self, from: Uuid, game_id: Uuid) -> bool {
        self.from.id == from && self.model.rematch_of == Some(game_id)
    }

    /// Whether this is a rematch of the game put to `to`.
    fn is_rematch_for(&self, to: Uuid, game_id: Uuid) -> bool {
        self.to.id == to && self.model.rematch_of == Some(game_id)
    }
}

impl PlayHub {
    /// Invites an online, free friend to a game for a minute, withdrawing an earlier challenge.
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
        let terms = Terms {
            initial_s,
            increment_s,
            rated,
            rematch_of: None,
            white: None,
        };
        let (new, pending) = self.draw_up((player, opponent), terms);
        self.post_challenge(new, pending).await;
    }

    /// The challenge as it is stored, and as it waits for its answer.
    pub(super) fn draw_up(
        &self,
        (player, opponent): (&Player, Player),
        terms: Terms,
    ) -> (NewChallenge, PendingChallenge) {
        let challenger_colour = terms.white.map(|white| {
            if white == player.id {
                Color::White
            } else {
                Color::Black
            }
        });
        let new = NewChallenge {
            id: Uuid::now_v7(),
            challenger_id: player.id,
            challenged_id: opponent.id,
            initial_s: i32::try_from(terms.initial_s).unwrap_or(i32::MAX),
            increment_s: i32::try_from(terms.increment_s).unwrap_or(i32::MAX),
            rated: terms.rated,
            challenger_colour,
            rematch_of: terms.rematch_of,
            expires_in_s: i32::try_from(CHALLENGE_WAIT.as_secs()).unwrap_or(i32::MAX),
        };
        let pending = PendingChallenge {
            model: ChallengeModel {
                id: new.id,
                from: self.user_ref(player),
                to: self.user_ref(&opponent),
                initial_s: terms.initial_s,
                increment_s: terms.increment_s,
                rated: terms.rated,
                rematch_of: terms.rematch_of,
            },
            control: TimeControl {
                initial: Duration::from_secs(terms.initial_s.into()),
                increment: Duration::from_secs(terms.increment_s.into()),
            },
            from: player.clone(),
            to: opponent,
            white: terms.white,
            posted: Instant::now(),
        };
        (new, pending)
    }

    /// Stores the challenge and puts it to both for a minute, the sender's earlier one withdrawn.
    pub(super) async fn post_challenge(&self, new: NewChallenge, mut pending: PendingChallenge) {
        let (from, to, game_id) = (
            pending.from.clone(),
            pending.to.clone(),
            pending.model.rematch_of,
        );
        if let Err(error) = challenge::insert(&self.inner.db, &new).await {
            tracing::error!(?error, "storing a challenge failed");
            let message = if game_id.is_some() {
                "The rematch could not be sent: try again"
            } else {
                "The challenge could not be sent: try again"
            };
            return self.refuse_for(from.id, game_id, message);
        }
        loop {
            // Placed under one lock: two rematches sent at the same moment find each other.
            let placed = place(&mut lock(&self.inner.challenges), pending);
            match placed {
                Placed::Crossing(theirs, mine) => match self.try_accept(&from, theirs).await {
                    // Theirs went just now: the player's own is placed after all.
                    Err(Unaccepted::Gone) => pending = *mine,
                    Err(Unaccepted::Refused(reason)) => {
                        self.record(new.id, ChallengeStatus::Cancelled, None);
                        return self.refuse_for(from.id, game_id, &reason);
                    }
                    Ok(()) => return self.record(new.id, ChallengeStatus::Cancelled, None),
                },
                Placed::Already => {
                    self.record(new.id, ChallengeStatus::Cancelled, None);
                    // Only a rematch repeats one waiting.
                    if let Some(game_id) = game_id {
                        self.asked_elsewhere(from.id, game_id);
                    }
                    return;
                }
                Placed::Posted(earlier) => {
                    return self
                        .put_or_take_back((&from, &to), (new.id, game_id), earlier)
                        .await;
                }
            }
        }
    }

    /// Puts the placed challenge to both, unless either has gone or sat down while it was stored.
    async fn put_or_take_back(
        &self,
        (from, to): (&Player, &Player),
        (id, game_id): (Uuid, Option<Uuid>),
        earlier: Vec<Uuid>,
    ) {
        // Checked once placed: a game, block or unfriending from now on calls it off itself.
        let sender_unready = self.sender_unready(from, to);
        let reason = match sender_unready.clone() {
            Some(reason) => Some(reason),
            None => self.opponent_unready((from, to), game_id.is_some()).await,
        };
        // Put, or the sender is away: the earlier ones go. Only the opponent missing it keeps them.
        if reason.is_none() || sender_unready.is_some() {
            for earlier in earlier {
                let taken = self.take_challenge(|pending| pending.model.id == earlier);
                self.settle(taken, ChallengeStatus::Cancelled);
            }
        }
        let Some(reason) = reason else {
            self.send_challenges(from.id);
            self.send_challenges(to.id);
            return self.expire_later(id);
        };
        // Not there to take back means accepted meanwhile: nothing to refuse.
        if let Some(placed) = self.take_challenge(|pending| pending.model.id == id) {
            self.refuse_for(from.id, game_id, &reason);
            self.settle(Some(placed), ChallengeStatus::Cancelled);
        }
    }

    /// The player asked for a rematch another of their devices asked for first: that one stands.
    pub(super) fn asked_elsewhere(&self, player: Uuid, game_id: Uuid) {
        self.refuse_for(
            player,
            Some(game_id),
            "You asked for this rematch on another device",
        );
        self.send_challenges(player);
    }

    /// The id of the first waiting challenge `matches` picks.
    pub(super) fn find_challenge(
        &self,
        matches: impl Fn(&PendingChallenge) -> bool,
    ) -> Option<Uuid> {
        lock(&self.inner.challenges)
            .iter()
            .find(|pending| matches(pending))
            .map(|pending| pending.model.id)
    }

    /// Why the sender cannot put a placed challenge now: offline, or playing.
    fn sender_unready(&self, from: &Player, to: &Player) -> Option<String> {
        if !self.is_online(from.id) {
            return Some("You went offline".to_owned());
        }
        self.is_playing(from.id)
            .then(|| self.busy_refusal(from.id, to))
            .flatten()
    }

    /// Why a placed challenge cannot be put to the opponent now: playing, offline, blocked.
    async fn opponent_unready(
        &self,
        (from, to): (&Player, &Player),
        rematch: bool,
    ) -> Option<String> {
        if self.is_playing(to.id) {
            return Some(format!("{} is playing a game", to.username));
        }
        let allowed = if rematch {
            self.unblocked(from.id, to.id).await
        } else {
            self.are_friends(from.id, to.id).await.unwrap_or(false)
        };
        if self.is_online(to.id) && allowed {
            return None;
        }
        Some(if rematch {
            format!("{} is not taking a rematch", to.username)
        } else {
            format!("{} is offline", to.username)
        })
    }

    /// Why the two cannot start a game now: either already playing, or being seated.
    pub(super) fn busy_refusal(&self, player: Uuid, opponent: &Player) -> Option<String> {
        if self.is_playing(player) {
            return Some("Finish your game first".to_owned());
        }
        self.is_playing(opponent.id)
            .then(|| format!("{} is playing a game", opponent.username))
    }

    /// The player's waiting challenges show their new picture, to both sides.
    pub(super) fn repicture_challenges(&self, player: Uuid, avatar: Option<&str>) {
        let mut told = Vec::new();
        for pending in lock(&self.inner.challenges).iter_mut() {
            for side in [&mut pending.model.from, &mut pending.model.to] {
                if side.id == player {
                    side.avatar = avatar.map(str::to_owned);
                    told.extend([pending.from.id, pending.to.id]);
                }
            }
        }
        told.sort_unstable();
        told.dedup();
        for other in told {
            self.send_challenges(other);
        }
    }

    /// Neither of the two has blocked the other; an error counts as a block, to be safe.
    pub(super) async fn unblocked(&self, first: Uuid, second: Uuid) -> bool {
        block::blockers_between(&self.inner.db, first, second)
            .await
            .is_ok_and(|blockers| blockers.is_empty())
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
        let opponent = Player::from(found);
        if !self.is_online(to) {
            return Err(format!("{} is offline", opponent.username));
        }
        match self.busy_refusal(player.id, &opponent) {
            Some(busy) => Err(busy),
            None => Ok(opponent),
        }
    }

    /// Starts the challenged game, calling off the rest; a rematch's refusal is said of it.
    pub(super) async fn accept_challenge(&self, player: &Player, challenge_id: Uuid) {
        let game_id = lock(&self.inner.challenges)
            .iter()
            .find(|pending| pending.model.id == challenge_id)
            .and_then(|pending| pending.model.rematch_of);
        if let Err(unaccepted) = self.try_accept(player, challenge_id).await {
            let reason = unaccepted.message(game_id.is_some());
            self.refuse_accept(player.id, (challenge_id, game_id), &reason);
        }
    }

    /// Starts the challenged game; Err says why it cannot, a game failing to start is told here.
    pub(super) async fn try_accept(
        &self,
        player: &Player,
        challenge_id: Uuid,
    ) -> Result<(), Unaccepted> {
        let accepting = lock(&self.inner.challenges)
            .iter()
            .find(|pending| pending.model.id == challenge_id && pending.to.id == player.id)
            .map(|pending| Accepting {
                from: pending.from.clone(),
                control: pending.control,
                rated: pending.model.rated,
                white: pending.white,
                rematch: pending.model.rematch_of.is_some(),
            })
            .ok_or(Unaccepted::Gone)?;
        let from = &accepting.from;
        // A rematch is between any two who just played; a challenge, between friends.
        if accepting.rematch {
            if !self.unblocked(from.id, player.id).await {
                self.drop_challenges_between(from.id, player.id);
                return Err(Unaccepted::Gone);
            }
        } else if !self.are_friends(from.id, player.id).await.unwrap_or(false) {
            self.drop_friend_challenges_between(from.id, player.id);
            return Err(Unaccepted::Gone);
        }
        if let Some(busy) = self.busy_refusal(player.id, from) {
            return Err(Unaccepted::Refused(busy));
        }
        let pending = self
            .take_challenge(|pending| pending.model.id == challenge_id)
            .ok_or(Unaccepted::Gone)?;
        let players = match accepting.white {
            Some(white) if white == player.id => Players::Fixed(player, from),
            Some(_) => Players::Fixed(from, player),
            None => Players::EitherWay(from, player),
        };
        let started = self
            .start_game(players, accepting.control, accepting.rated)
            .await;
        let status = if started.is_ok() {
            ChallengeStatus::Accepted
        } else {
            ChallengeStatus::Cancelled
        };
        self.record(pending.model.id, status, started.ok());
        // Told before the lists, which no longer hold it, so the apps still know what failed.
        if let Err(refusal) = started {
            let accepted = (pending.model.id, pending.model.rematch_of);
            self.refuse_accept(player.id, accepted, refusal.message());
            self.refuse_for(from.id, pending.model.rematch_of, refusal.message());
        }
        self.send_challenges(from.id);
        self.send_challenges(player.id);
        Ok(())
    }

    /// Accepts the opponent's rematch of the game, or says why not, as that game's.
    pub(super) async fn accept_rematch(&self, player: &Player, game_id: Uuid) {
        let theirs = self.find_challenge(|pending| pending.is_rematch_for(player.id, game_id));
        let accepted = match theirs {
            Some(challenge_id) => self.try_accept(player, challenge_id).await,
            None => Err(Unaccepted::Gone),
        };
        if let Err(unaccepted) = accepted {
            self.refuse_for(player.id, Some(game_id), &unaccepted.message(true));
        }
    }

    /// Refuses an accept: as that game's rematch, or as that invitation's, for whoever pressed it.
    pub(super) fn refuse_accept(
        &self,
        player: Uuid,
        (challenge_id, rematch_of): (Uuid, Option<Uuid>),
        message: &str,
    ) {
        if rematch_of.is_some() {
            return self.refuse_for(player, rematch_of, message);
        }
        let refused = ServerMessage::InviteRefused {
            challenge_id,
            message: message.to_owned(),
        };
        self.reply(player, refused);
    }

    /// Refuses the player, of a rematch if the refusal is of one, so only that rematch is told.
    pub(super) fn refuse_for(&self, player: Uuid, rematch_of: Option<Uuid>, message: &str) {
        match rematch_of {
            Some(game_id) => self.reply(
                player,
                ServerMessage::RematchRefused {
                    game_id,
                    message: message.to_owned(),
                },
            ),
            None => self.refuse(player, message),
        }
    }

    pub(super) fn decline_challenge(&self, player: &Player, challenge_id: Uuid) {
        let declined = self.take_challenge(|pending| {
            pending.model.id == challenge_id && pending.to.id == player.id
        });
        self.settle(declined, ChallengeStatus::Declined);
    }

    /// Withdraws the player's rematch of the game, if it is still waiting.
    pub(super) fn cancel_rematch(&self, player: &Player, game_id: Uuid) {
        let cancelled = self.take_challenge(|pending| pending.is_rematch_by(player.id, game_id));
        self.settle(cancelled, ChallengeStatus::Cancelled);
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

    /// Calls off friend challenges between the two but not a rematch, which needs no friendship.
    pub fn drop_friend_challenges_between(&self, first: Uuid, second: Uuid) {
        self.drop_between((first, second), |pending| {
            pending.model.rematch_of.is_none()
        });
    }

    /// Calls off any challenge between the two, a rematch too: one blocked the other.
    pub fn drop_challenges_between(&self, first: Uuid, second: Uuid) {
        self.drop_between((first, second), |_| true);
    }

    /// Calls off the challenges between the two that `which` picks.
    fn drop_between(
        &self,
        (first, second): (Uuid, Uuid),
        which: impl Fn(&PendingChallenge) -> bool,
    ) {
        let between = |pending: &PendingChallenge| {
            pending.involves(first) && pending.involves(second) && which(pending)
        };
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
                    .map(sent_model)
                    .collect::<Vec<_>>()
            };
            (
                models_where(&|pending| pending.from.id == player),
                models_where(&|pending| pending.to.id == player),
            )
        };
        self.send(player, ServerMessage::Challenges { sent, received });
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

/// Why a challenge was not accepted.
pub(super) enum Unaccepted {
    /// It is no longer waiting: answered, withdrawn, run out, or called off by a block.
    Gone,
    /// It waits, but cannot start now, in a sentence.
    Refused(String),
}

impl Unaccepted {
    /// The refusal in a sentence, naming a rematch or a challenge.
    pub(super) fn message(self, rematch: bool) -> String {
        match self {
            Unaccepted::Gone if rematch => "That rematch is no longer waiting".to_owned(),
            Unaccepted::Gone => "That challenge is no longer open".to_owned(),
            Unaccepted::Refused(reason) => reason,
        }
    }
}

/// Where a challenge just stored went.
enum Placed {
    /// The opponent's rematch of the same game was waiting: it is accepted instead of this one.
    Crossing(Uuid, Box<PendingChallenge>),
    /// The sender's rematch of the same game was waiting already.
    Already,
    /// Added; the sender's earlier challenges of its kind go once it is put to the opponent.
    Posted(Vec<Uuid>),
}

/// Adds the challenge to the list, unless it crosses a rematch or repeats the sender's own.
fn place(challenges: &mut Vec<PendingChallenge>, pending: PendingChallenge) -> Placed {
    if let Some(theirs) = challenges.iter().find(|theirs| crosses(theirs, &pending)) {
        return Placed::Crossing(theirs.model.id, Box::new(pending));
    }
    if challenges.iter().any(|mine| repeats(mine, &pending)) {
        return Placed::Already;
    }
    // Left in place until the new one is put to its opponent, so they still run out on time.
    let earlier = challenges
        .iter()
        .filter(|other| replaced_by(other, &pending))
        .map(|other| other.model.id)
        .collect();
    challenges.push(pending);
    Placed::Posted(earlier)
}

/// Whether `mine` is the same sender's rematch of the same game as `new`.
fn repeats(mine: &PendingChallenge, new: &PendingChallenge) -> bool {
    new.model
        .rematch_of
        .is_some_and(|game_id| mine.is_rematch_by(new.from.id, game_id))
}

/// Whether `other` is the sender's earlier challenge of the same kind as `new`: one waits each.
fn replaced_by(other: &PendingChallenge, new: &PendingChallenge) -> bool {
    other.from.id == new.from.id
        && other.model.rematch_of.is_some() == new.model.rematch_of.is_some()
}

/// Whether `theirs` is the other player's rematch of the same game as `mine`.
fn crosses(theirs: &PendingChallenge, mine: &PendingChallenge) -> bool {
    mine.model.rematch_of.is_some_and(|game_id| {
        theirs.is_rematch_by(mine.to.id, game_id) && theirs.to.id == mine.from.id
    })
}

/// The challenge as it is sent, with the time it has left now.
fn sent_model(pending: &PendingChallenge) -> ListedChallenge {
    let left = CHALLENGE_WAIT.saturating_sub(pending.posted.elapsed());
    ListedChallenge {
        challenge: pending.model.clone(),
        wait_s: whole_seconds(CHALLENGE_WAIT),
        seconds_left: whole_seconds_up(left),
    }
}

#[cfg(test)]
mod tests {
    use std::time::{Duration, Instant};

    use chess_domain::TimeControl;
    use uuid::Uuid;

    use super::{PendingChallenge, Placed, crosses, place};
    use crate::models::{
        play::{challenge::ChallengeModel, player::Player},
        social::friend::UserRef,
    };

    fn player(id: u128) -> Player {
        Player {
            id: Uuid::from_u128(id),
            username: id.to_string(),
            guest: false,
            avatar: None,
        }
    }

    fn rematch(from: u128, to: u128, of: Option<Uuid>) -> PendingChallenge {
        let (from, to) = (player(from), player(to));
        let side = |player: &Player| UserRef {
            id: player.id,
            username: player.username.clone(),
            avatar: None,
        };
        PendingChallenge {
            model: ChallengeModel {
                id: Uuid::now_v7(),
                from: side(&from),
                to: side(&to),
                initial_s: 180,
                increment_s: 0,
                rated: false,
                rematch_of: of,
            },
            control: TimeControl {
                initial: Duration::from_secs(180),
                increment: Duration::ZERO,
            },
            from,
            to,
            white: None,
            posted: Instant::now(),
        }
    }

    #[test]
    fn two_rematches_of_one_game_the_other_way_cross() {
        let game = Some(Uuid::from_u128(9));
        assert!(crosses(&rematch(2, 1, game), &rematch(1, 2, game)));
        assert!(
            !crosses(&rematch(1, 2, game), &rematch(1, 2, game)),
            "the same way round"
        );
        assert!(!crosses(
            &rematch(2, 1, Some(Uuid::from_u128(8))),
            &rematch(1, 2, game)
        ));
        assert!(
            !crosses(&rematch(2, 1, None), &rematch(1, 2, None)),
            "friends' challenges"
        );
    }

    #[test]
    fn a_challenge_is_placed_once_and_names_the_senders_earlier_one_of_its_kind() {
        let game = Some(Uuid::from_u128(9));
        let mut list = vec![rematch(1, 2, game), rematch(1, 3, None)];
        assert!(matches!(
            place(&mut list, rematch(1, 2, game)),
            Placed::Already
        ));
        assert!(matches!(
            place(&mut list, rematch(2, 1, game)),
            Placed::Crossing(..)
        ));
        assert_eq!(list.len(), 2, "neither was added");
        let Placed::Posted(replaced) = place(&mut list, rematch(1, 4, None)) else {
            panic!("a new friend's challenge is posted");
        };
        assert_eq!(
            replaced,
            [list[1].model.id],
            "the earlier friend's challenge, not the rematch"
        );
        assert_eq!(list.len(), 3, "it goes only once the new one is put");
    }
}
