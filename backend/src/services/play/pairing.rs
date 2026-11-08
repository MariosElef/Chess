use chess_domain::TimeControl;

use crate::models::play::player::Player;

/// A player waiting for an opponent.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Seek {
    pub player: Player,
    pub control: TimeControl,
    pub rated: bool,
}

/// The waiting seek `new` pairs with: someone else's, for the same time control, rated alike.
/// It leaves the queue; the oldest such seek is taken first.
pub fn take_match(seeks: &mut Vec<Seek>, new: &Seek) -> Option<Seek> {
    let index = seeks.iter().position(|waiting| {
        waiting.player.id != new.player.id
            && waiting.control == new.control
            && waiting.rated == new.rated
    })?;
    Some(seeks.remove(index))
}

/// Queues `new`; a player has one seek at a time, so an earlier one of theirs is dropped.
pub fn queue(seeks: &mut Vec<Seek>, new: Seek) {
    cancel(seeks, &new.player);
    seeks.push(new);
}

/// Whether the player had a seek, now gone.
pub fn cancel(seeks: &mut Vec<Seek>, player: &Player) -> bool {
    let before = seeks.len();
    seeks.retain(|waiting| waiting.player.id != player.id);
    seeks.len() != before
}

#[cfg(test)]
mod tests {
    use chess_domain::TimeControl;
    use uuid::Uuid;

    use super::{Seek, cancel, queue, take_match};
    use crate::models::play::player::Player;

    fn seek(name: &str, control: TimeControl, rated: bool) -> Seek {
        Seek {
            player: Player {
                id: Uuid::from_u128(name.bytes().map(u128::from).sum()),
                username: name.to_owned(),
                guest: false,
            },
            control,
            rated,
        }
    }

    #[test]
    fn the_same_time_control_rated_alike_pairs() {
        let blitz = TimeControl::new(3, 2);
        let mut seeks = vec![
            seek("ann", TimeControl::new(1, 0), false),
            seek("bob", blitz, true),
            seek("cat", blitz, false),
        ];
        let paired = take_match(&mut seeks, &seek("dan", blitz, false));
        assert_eq!(
            paired.map(|seek| seek.player.username),
            Some("cat".to_owned())
        );
        assert_eq!(seeks.len(), 2);
    }

    #[test]
    fn nobody_is_paired_with_themselves() {
        let blitz = TimeControl::new(3, 2);
        let mut seeks = vec![seek("ann", blitz, false)];
        assert_eq!(take_match(&mut seeks, &seek("ann", blitz, false)), None);
    }

    #[test]
    fn a_new_seek_replaces_the_players_old_one() {
        let mut seeks = Vec::new();
        queue(&mut seeks, seek("ann", TimeControl::new(3, 2), false));
        queue(&mut seeks, seek("ann", TimeControl::new(10, 0), false));
        assert_eq!(seeks.len(), 1);
        assert_eq!(seeks[0].control, TimeControl::new(10, 0));
        let ann = seeks[0].player.clone();
        assert!(cancel(&mut seeks, &ann));
        assert!(seeks.is_empty());
    }
}
