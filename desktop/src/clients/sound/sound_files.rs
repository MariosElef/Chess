//! The sound files of every set, compiled into the binary.
use crate::enums::{settings::SoundSet, sound::Sound};

pub fn sound_bytes(sound: Sound, set: SoundSet) -> &'static [u8] {
    match set {
        SoundSet::Standard => standard(sound),
        SoundSet::Lisp => lisp(sound),
    }
}

fn standard(sound: Sound) -> &'static [u8] {
    match sound {
        Sound::MoveSelf => include_bytes!("../../../assets/sounds/standard/move-self.mp3"),
        Sound::MoveOpponent => include_bytes!("../../../assets/sounds/standard/move-opponent.mp3"),
        Sound::Capture => include_bytes!("../../../assets/sounds/standard/capture.mp3"),
        Sound::Castle => include_bytes!("../../../assets/sounds/standard/castle.mp3"),
        Sound::Check => include_bytes!("../../../assets/sounds/standard/move-check.mp3"),
        Sound::Promote => include_bytes!("../../../assets/sounds/standard/promote.mp3"),
        Sound::Premove => include_bytes!("../../../assets/sounds/standard/premove.mp3"),
        Sound::Illegal => include_bytes!("../../../assets/sounds/standard/illegal.mp3"),
        Sound::GameStart => include_bytes!("../../../assets/sounds/standard/game-start.mp3"),
        Sound::GameEnd => include_bytes!("../../../assets/sounds/standard/game-end.mp3"),
        Sound::Notify => include_bytes!("../../../assets/sounds/standard/notify.mp3"),
        Sound::LowTime => include_bytes!("../../../assets/sounds/standard/tenseconds.mp3"),
    }
}

fn lisp(sound: Sound) -> &'static [u8] {
    match sound {
        Sound::MoveSelf => include_bytes!("../../../assets/sounds/lisp/move-self.mp3"),
        Sound::MoveOpponent => include_bytes!("../../../assets/sounds/lisp/move-opponent.mp3"),
        Sound::Capture => include_bytes!("../../../assets/sounds/lisp/capture.mp3"),
        Sound::Castle => include_bytes!("../../../assets/sounds/lisp/castle.mp3"),
        Sound::Check => include_bytes!("../../../assets/sounds/lisp/move-check.mp3"),
        Sound::Promote => include_bytes!("../../../assets/sounds/lisp/promote.mp3"),
        Sound::Premove => include_bytes!("../../../assets/sounds/lisp/premove.mp3"),
        Sound::Illegal => include_bytes!("../../../assets/sounds/lisp/illegal.mp3"),
        Sound::GameStart => include_bytes!("../../../assets/sounds/lisp/game-start.mp3"),
        Sound::GameEnd => include_bytes!("../../../assets/sounds/lisp/game-end.mp3"),
        Sound::Notify => include_bytes!("../../../assets/sounds/lisp/notify.mp3"),
        Sound::LowTime => include_bytes!("../../../assets/sounds/lisp/tenseconds.mp3"),
    }
}
