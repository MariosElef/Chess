use std::{
    cell::{Cell, RefCell},
    io::Cursor,
};

use rodio::{Decoder, DeviceSinkBuilder, MixerDeviceSink, Player};

use super::sound_files::sound_bytes;
use crate::enums::{settings::SoundSet, sound::Sound};

/// The computer's audio output; separate sounds overlap rather than wait for each other.
pub struct Speaker {
    output: MixerDeviceSink,
    /// The set the sounds are played from, as Settings picks it.
    set: Cell<SoundSet>,
    /// The set's sounds being played through after it was picked; a new pick stops it.
    tour: RefCell<Option<Player>>,
}

impl Speaker {
    /// The default output, or `None` on a computer without one: the app then plays silently.
    pub fn open() -> Option<Self> {
        let mut output = DeviceSinkBuilder::open_default_sink().ok()?;
        output.log_on_drop(false);
        Some(Self {
            output,
            set: Cell::new(SoundSet::default()),
            tour: RefCell::new(None),
        })
    }

    /// From now on, sounds come from `set`.
    pub fn use_set(&self, set: SoundSet) {
        self.set.set(set);
    }

    /// Starts `sound` and returns at once.
    pub fn play(&self, sound: Sound) {
        if let Some(decoded) = decode(sound, self.set.get()) {
            self.output.mixer().add(decoded);
        }
    }

    /// Plays every sound of the set one after another, stopping a tour already playing.
    pub fn play_tour(&self, sounds: &[Sound]) {
        let player = Player::connect_new(self.output.mixer());
        let set = self.set.get();
        for decoded in sounds.iter().filter_map(|&sound| decode(sound, set)) {
            player.append(decoded);
        }
        if let Some(previous) = self.tour.replace(Some(player)) {
            previous.stop();
        }
    }

    /// Starts `sounds` one after another and returns at once, as a mate's check then the end.
    pub fn play_in_turn(&self, sounds: &[Sound]) {
        let player = Player::connect_new(self.output.mixer());
        let set = self.set.get();
        for decoded in sounds.iter().filter_map(|&sound| decode(sound, set)) {
            player.append(decoded);
        }
        player.detach();
    }
}

/// A file that will not decode is skipped: a missed sound is not worth interrupting the game.
fn decode(sound: Sound, set: SoundSet) -> Option<Decoder<Cursor<&'static [u8]>>> {
    Decoder::new(Cursor::new(sound_bytes(sound, set))).ok()
}

#[cfg(test)]
mod tests {
    use std::io::Cursor;

    use rodio::Decoder;

    use crate::{
        clients::sound::sound_files::sound_bytes,
        enums::{settings::SoundSet, sound::Sound},
    };

    #[test]
    fn every_sound_of_every_set_decodes() {
        for (set, sound) in SoundSet::ALL.into_iter().flat_map(|set| {
            [
                Sound::MoveSelf,
                Sound::MoveOpponent,
                Sound::Capture,
                Sound::Castle,
                Sound::Check,
                Sound::Promote,
                Sound::Premove,
                Sound::Illegal,
                Sound::GameStart,
                Sound::GameEnd,
                Sound::Notify,
                Sound::LowTime,
            ]
            .map(move |sound| (set, sound))
        }) {
            assert!(
                Decoder::new(Cursor::new(sound_bytes(sound, set))).is_ok(),
                "{set:?} {sound:?} decodes"
            );
        }
    }
}
