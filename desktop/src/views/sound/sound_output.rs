#[cfg(test)]
use std::cell::RefCell;

use chess_domain::{Move, Position};
use gpui_kit::{App, Global};

use crate::{
    clients::sound::Speaker,
    enums::{settings::SoundSet, sound::Sound},
    services::sound::move_sound,
};

/// Where the pages' sounds go: the speaker, or in tests a list of what would have played.
enum SoundOutput {
    Speaker(Speaker),
    #[cfg(test)]
    Recorder(RefCell<Vec<Sound>>),
}

impl Global for SoundOutput {}

/// Opens the computer's audio output for the whole app; without one, the app stays silent.
pub fn install_speaker(cx: &mut App) {
    if let Some(speaker) = Speaker::open() {
        cx.set_global(SoundOutput::Speaker(speaker));
    }
}

/// Plays the app's sounds from `set` from now on.
pub fn use_sound_set(set: SoundSet, cx: &App) {
    match cx.try_global::<SoundOutput>() {
        Some(SoundOutput::Speaker(speaker)) => speaker.use_set(set),
        #[cfg(test)]
        Some(SoundOutput::Recorder(_)) => {}
        None => {}
    }
}

/// Plays `sound`, if the app has somewhere to play it.
pub fn play_sound(sound: Sound, cx: &App) {
    match cx.try_global::<SoundOutput>() {
        Some(SoundOutput::Speaker(speaker)) => speaker.play(sound),
        #[cfg(test)]
        Some(SoundOutput::Recorder(played)) => played.borrow_mut().push(sound),
        None => {}
    }
}

/// Plays `sounds` one after another.
fn play_in_turn(sounds: &[Sound], cx: &App) {
    match cx.try_global::<SoundOutput>() {
        Some(SoundOutput::Speaker(speaker)) => speaker.play_in_turn(sounds),
        #[cfg(test)]
        Some(SoundOutput::Recorder(played)) => played.borrow_mut().extend_from_slice(sounds),
        None => {}
    }
}

/// The sound `mv` makes from `before` to `after`, `plain` if quiet, then the end if it `ends_game`.
pub fn play_move_sound(
    before: &Position,
    after: &Position,
    mv: Move,
    plain: Sound,
    ends_game: bool,
    cx: &App,
) {
    let sound = move_sound(before, after, mv, plain);
    if ends_game {
        play_in_turn(&[sound, Sound::GameEnd], cx);
    } else {
        play_sound(sound, cx);
    }
}

/// Swaps the speaker for a recorder, so a test can see which sounds play.
#[cfg(test)]
pub fn record_sounds(cx: &mut App) {
    cx.set_global(SoundOutput::Recorder(RefCell::new(Vec::new())));
}

/// Everything played since `record_sounds`.
#[cfg(test)]
pub fn played_sounds(cx: &App) -> Vec<Sound> {
    match cx.try_global::<SoundOutput>() {
        Some(SoundOutput::Recorder(played)) => played.borrow().clone(),
        Some(SoundOutput::Speaker(_)) | None => Vec::new(),
    }
}
