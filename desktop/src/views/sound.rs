mod sound_output;

pub use sound_output::{
    install_speaker, play_move_sound, play_sound, play_sound_tour, use_sound_set,
};
#[cfg(test)]
pub use sound_output::{played_sounds, record_sounds};
