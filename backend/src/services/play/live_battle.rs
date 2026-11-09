use std::time::Duration;

use chess_domain::{Rating, Side};
use tokio::{
    sync::mpsc,
    time::{Instant, sleep_until},
};
use uuid::Uuid;

use super::{PlayHub, game_rating::rate_pair};
use crate::{
    enums::puzzles::{BattleResult, BattleStatus},
    models::{
        play::messages::ServerMessage,
        puzzles::{
            battle::{BattleSideModel, BattleSnapshot, FinishedBattle},
            puzzle::GetPuzzleModel,
        },
        rating::RatingSource,
    },
    repositories::{database::Db, puzzles::battle},
    services::puzzles::rating::BATTLE_CATEGORY,
};

/// How long a battle lasts, and how many mistakes end a player's run.
pub const BATTLE_TIME: Duration = Duration::from_secs(180);
pub const STRIKES: u32 = 3;
/// Answers queued for a battle before the sender waits.
const COMMANDS_AHEAD: usize = 32;

/// A player in a battle: who, their battle rating before it if rated, and their full rating.
#[derive(Debug, Clone, PartialEq)]
pub struct BattlePlayer {
    pub id: Uuid,
    pub username: String,
    pub avatar: Option<String>,
    pub rating: Option<i32>,
    /// Their battle rating as the battle began, for rating it if they close meanwhile.
    pub at_start: Rating,
}

/// What a battle starts with: its players, whether it counts, and the puzzles they share.
pub struct BattleStart {
    pub id: Uuid,
    pub players: [BattlePlayer; 2],
    pub rated: bool,
    pub puzzles: Vec<GetPuzzleModel>,
}

/// What a battle's task is told.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BattleCommand {
    /// The player solved their current puzzle, or made a mistake on it.
    Answer { player: Uuid, solved: bool },
    /// Send the player the battle as it stands, as when they reconnect.
    Snapshot { player: Uuid },
    /// The player gives up: the battle ends, and the other player wins.
    Resign { player: Uuid },
}

/// Runs the battle on a task of its own, which alone holds both players' progress.
pub fn spawn_battle(start: BattleStart, hub: PlayHub, db: Db) -> mpsc::Sender<BattleCommand> {
    let (sender, commands) = mpsc::channel(COMMANDS_AHEAD);
    tokio::spawn(run(LiveBattle::new(start, hub, db), commands));
    sender
}

/// Takes answers until the time is up or both players are out, then ends it.
async fn run(mut live: LiveBattle, mut commands: mpsc::Receiver<BattleCommand>) {
    live.broadcast();
    let deadline = live.began + BATTLE_TIME;
    while !live.both_out() && live.resigned.is_none() {
        tokio::select! {
            command = commands.recv() => match command {
                Some(command) => live.handle(command),
                None => return,
            },
            () = sleep_until(deadline) => break,
        }
    }
    live.finish().await;
}

/// One player's run: puzzles solved, mistakes made, and when they last scored, to break a tie.
#[derive(Debug, Clone, Copy, Default)]
struct Progress {
    score: u32,
    mistakes: u32,
    scored_at: Option<Instant>,
}

struct LiveBattle {
    start: BattleStart,
    hub: PlayHub,
    db: Db,
    began: Instant,
    progress: [Progress; 2],
    /// Set once it is over: the winner's place, `None` for a draw.
    ended: Option<Option<usize>>,
    /// The place of a player who gave up.
    resigned: Option<usize>,
    rating_changes: Option<[i32; 2]>,
}

impl LiveBattle {
    fn new(start: BattleStart, hub: PlayHub, db: Db) -> Self {
        Self {
            start,
            hub,
            db,
            began: Instant::now(),
            progress: [Progress::default(); 2],
            ended: None,
            resigned: None,
            rating_changes: None,
        }
    }

    fn place_of(&self, player: Uuid) -> Option<usize> {
        self.start.players.iter().position(|seat| seat.id == player)
    }

    fn handle(&mut self, command: BattleCommand) {
        match command {
            BattleCommand::Answer { player, solved } => {
                let Some(place) = self.place_of(player) else {
                    return;
                };
                if self.is_out(place) {
                    return;
                }
                let progress = &mut self.progress[place];
                if solved {
                    progress.score += 1;
                    progress.scored_at = Some(Instant::now());
                } else {
                    progress.mistakes += 1;
                }
                self.broadcast();
            }
            BattleCommand::Resign { player } => self.resigned = self.place_of(player),
            BattleCommand::Snapshot { player } => {
                if let Some(place) = self.place_of(player) {
                    let battle = self.snapshot(place);
                    self.hub.send(player, ServerMessage::Battle { battle });
                }
            }
        }
    }

    /// Out of mistakes, or out of puzzles.
    fn is_out(&self, place: usize) -> bool {
        let progress = self.progress[place];
        let answered = usize::try_from(progress.score + progress.mistakes).unwrap_or(usize::MAX);
        progress.mistakes >= STRIKES || answered >= self.start.puzzles.len()
    }

    fn both_out(&self) -> bool {
        self.is_out(0) && self.is_out(1)
    }

    /// Stores the result, moves the ratings of a rated battle, and tells both players.
    async fn finish(&mut self) {
        // Out of the hub at once: answers sent from now on are refused, not lost unread, and
        // its players may look for another battle while this one is stored.
        self.hub.battle_over(self.start.id);
        let winner = match self.resigned {
            Some(loser) => Some(1 - loser),
            None => winner_of(self.progress),
        };
        self.ended = Some(winner);
        let [first, second] = &self.start.players;
        let finished = FinishedBattle {
            id: self.start.id,
            first_score: i32::try_from(self.progress[0].score).unwrap_or(i32::MAX),
            second_score: i32::try_from(self.progress[1].score).unwrap_or(i32::MAX),
            winner_id: winner.map(|place| self.start.players[place].id),
        };
        if let Err(error) = battle::finish(&self.db, &finished).await {
            tracing::error!(?error, "storing a battle's result failed");
        }
        if self.start.rated {
            let source = RatingSource::Battle(self.start.id);
            let side = winner.map(|place| if place == 0 { Side::White } else { Side::Black });
            match rate_pair(
                &self.db,
                source,
                ((first.id, first.at_start), (second.id, second.at_start)),
                BATTLE_CATEGORY,
                side,
            )
            .await
            {
                Ok(changes) => self.rating_changes = Some([changes.white, changes.black]),
                Err(error) => tracing::error!(?error, "rating a battle failed"),
            }
        }
        self.broadcast();
    }

    fn broadcast(&self) {
        for (place, player) in self.start.players.iter().enumerate() {
            let battle = self.snapshot(place);
            self.hub.send(player.id, ServerMessage::Battle { battle });
        }
    }

    /// The battle as the player in `place` sees it.
    fn snapshot(&self, place: usize) -> BattleSnapshot {
        let other = 1 - place;
        let left = match self.ended {
            Some(_) => Duration::ZERO,
            None => BATTLE_TIME.saturating_sub(self.began.elapsed()),
        };
        BattleSnapshot {
            id: self.start.id,
            me: self.side(place),
            opponent: self.side(other),
            rated: self.start.rated,
            puzzles: self.start.puzzles.clone(),
            duration_ms: millis(BATTLE_TIME),
            left_ms: millis(left),
            status: if self.ended.is_some() {
                BattleStatus::Finished
            } else {
                BattleStatus::Ongoing
            },
            result: self.ended.map(|winner| result_for(place, winner)),
            rating_change: self.rating_changes.map(|changes| changes[place]),
        }
    }

    fn side(&self, place: usize) -> BattleSideModel {
        let player = &self.start.players[place];
        BattleSideModel {
            id: player.id,
            username: player.username.clone(),
            avatar: player.avatar.clone(),
            rating: player.rating,
            score: self.progress[place].score,
            mistakes: self.progress[place].mistakes,
        }
    }
}

/// More solved wins; the same score goes to whoever reached it first; nobody scoring is a draw.
fn winner_of(progress: [Progress; 2]) -> Option<usize> {
    let [first, second] = progress;
    match first.score.cmp(&second.score) {
        std::cmp::Ordering::Greater => Some(0),
        std::cmp::Ordering::Less => Some(1),
        std::cmp::Ordering::Equal => match (first.scored_at, second.scored_at) {
            (Some(first_at), Some(second_at)) if first_at < second_at => Some(0),
            (Some(first_at), Some(second_at)) if second_at < first_at => Some(1),
            _ => None,
        },
    }
}

fn result_for(place: usize, winner: Option<usize>) -> BattleResult {
    match winner {
        Some(winner) if winner == place => BattleResult::Won,
        Some(_) => BattleResult::Lost,
        None => BattleResult::Drawn,
    }
}

fn millis(duration: Duration) -> u64 {
    u64::try_from(duration.as_millis()).unwrap_or(u64::MAX)
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use tokio::time::Instant;

    use super::{Progress, winner_of};

    fn progress(score: u32, scored_at: Option<Instant>) -> Progress {
        Progress {
            score,
            mistakes: 0,
            scored_at,
        }
    }

    #[test]
    fn more_solved_wins_and_a_tie_goes_to_whoever_got_there_first() {
        let now = Instant::now();
        let later = now + Duration::from_secs(1);
        assert_eq!(
            winner_of([progress(5, Some(later)), progress(4, Some(now))]),
            Some(0)
        );
        assert_eq!(
            winner_of([progress(5, Some(later)), progress(5, Some(now))]),
            Some(1)
        );
        assert_eq!(winner_of([progress(0, None), progress(0, None)]), None);
    }
}
