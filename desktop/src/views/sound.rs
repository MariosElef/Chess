mod sound_output;

pub use sound_output::{install_speaker, play_move_sound, play_sound};
#[cfg(test)]
pub use sound_output::{played_sounds, record_sounds};
