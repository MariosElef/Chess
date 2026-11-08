//! The sound files, compiled into the binary.
use crate::enums::sound::Sound;

pub fn sound_bytes(sound: Sound) -> &'static [u8] {
    match sound {
        Sound::MoveSelf => include_bytes!("../../../assets/sounds/move-self.mp3"),
        Sound::MoveOpponent => include_bytes!("../../../assets/sounds/move-opponent.mp3"),
        Sound::Capture => include_bytes!("../../../assets/sounds/capture.mp3"),
        Sound::Castle => include_bytes!("../../../assets/sounds/castle.mp3"),
        Sound::Check => include_bytes!("../../../assets/sounds/move-check.mp3"),
        Sound::Promote => include_bytes!("../../../assets/sounds/promote.mp3"),
        Sound::Premove => include_bytes!("../../../assets/sounds/premove.mp3"),
        Sound::Illegal => include_bytes!("../../../assets/sounds/illegal.mp3"),
        Sound::GameStart => include_bytes!("../../../assets/sounds/game-start.mp3"),
        Sound::GameEnd => include_bytes!("../../../assets/sounds/game-end.mp3"),
        Sound::Notify => include_bytes!("../../../assets/sounds/notify.mp3"),
        Sound::LowTime => include_bytes!("../../../assets/sounds/tenseconds.mp3"),
    }
}
