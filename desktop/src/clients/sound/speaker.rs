use std::io::Cursor;

use rodio::{Decoder, DeviceSinkBuilder, MixerDeviceSink, Player};

use super::sound_files::sound_bytes;
use crate::enums::sound::Sound;

/// The computer's audio output. Separate sounds overlap rather than wait for each other;
/// `play_in_turn` plays a few one after another.
pub struct Speaker {
    output: MixerDeviceSink,
}

impl Speaker {
    /// The default output, or `None` on a computer without one: the app then plays silently.
    pub fn open() -> Option<Self> {
        let mut output = DeviceSinkBuilder::open_default_sink().ok()?;
        output.log_on_drop(false);
        Some(Self { output })
    }

    /// Starts `sound` and returns at once.
    pub fn play(&self, sound: Sound) {
        if let Some(decoded) = decode(sound) {
            self.output.mixer().add(decoded);
        }
    }

    /// Starts `sounds` one after another and returns at once, as a mate's check and then the
    /// game's end.
    pub fn play_in_turn(&self, sounds: &[Sound]) {
        let player = Player::connect_new(self.output.mixer());
        for decoded in sounds.iter().filter_map(|&sound| decode(sound)) {
            player.append(decoded);
        }
        player.detach();
    }
}

/// A file that will not decode is skipped: a missed sound is not worth interrupting the game.
fn decode(sound: Sound) -> Option<Decoder<Cursor<&'static [u8]>>> {
    Decoder::new(Cursor::new(sound_bytes(sound))).ok()
}

#[cfg(test)]
mod tests {
    use std::io::Cursor;

    use rodio::Decoder;

    use crate::{clients::sound::sound_files::sound_bytes, enums::sound::Sound};

    #[test]
    fn every_sound_decodes() {
        for sound in [
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
        ] {
            assert!(
                Decoder::new(Cursor::new(sound_bytes(sound))).is_ok(),
                "{sound:?} decodes"
            );
        }
    }
}
