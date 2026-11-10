//! The settings files, kept between launches in `~/Library/Application Support/Chess/` on macOS.
use std::{fs, io, path::PathBuf};

use crate::{
    clients::engine::device_cores,
    enums::settings::{Appearance, BoardTheme, Language, PieceSet, SoundSet},
    models::engine::EngineSettings,
};

const APP_FOLDER: &str = "Chess";
const LANGUAGE_FILE: &str = "language";
const APPEARANCE_FILE: &str = "appearance";
const ENGINE_FILE: &str = "engine";
const BOARD_THEME_FILE: &str = "board_theme";
const PIECE_SET_FILE: &str = "pieces";
const SOUND_SET_FILE: &str = "sounds";

/// The saved language, or `None` on a first launch or when the file is missing or unreadable.
pub fn load_language() -> Option<Language> {
    Language::from_key(read_setting(LANGUAGE_FILE)?.trim())
}

pub fn save_language(language: Language) -> io::Result<()> {
    write_setting(LANGUAGE_FILE, language.key())
}

/// The saved choice, or `None` on a first launch or when the file is missing or unreadable.
pub fn load_appearance() -> Option<Appearance> {
    Appearance::from_key(read_setting(APPEARANCE_FILE)?.trim())
}

pub fn save_appearance(appearance: Appearance) -> io::Result<()> {
    write_setting(APPEARANCE_FILE, appearance.key())
}

/// The saved board theme, or `None` on a first launch or when the file is missing or unreadable.
pub fn load_board_theme() -> Option<BoardTheme> {
    BoardTheme::from_key(read_setting(BOARD_THEME_FILE)?.trim())
}

pub fn save_board_theme(theme: BoardTheme) -> io::Result<()> {
    write_setting(BOARD_THEME_FILE, theme.key())
}

/// The saved piece set, or `None` on a first launch or when the file is missing or unreadable.
pub fn load_piece_set() -> Option<PieceSet> {
    PieceSet::from_key(read_setting(PIECE_SET_FILE)?.trim())
}

pub fn save_piece_set(set: PieceSet) -> io::Result<()> {
    write_setting(PIECE_SET_FILE, set.key())
}

/// The saved sound set, or `None` on a first launch or when the file is missing or unreadable.
pub fn load_sound_set() -> Option<SoundSet> {
    SoundSet::from_key(read_setting(SOUND_SET_FILE)?.trim())
}

pub fn save_sound_set(set: SoundSet) -> io::Result<()> {
    write_setting(SOUND_SET_FILE, set.key())
}

/// The saved engine settings, fitted to this device; its defaults on a first launch.
pub fn load_engine_settings() -> EngineSettings {
    let cores = device_cores();
    read_setting(ENGINE_FILE).map_or_else(
        || EngineSettings::for_cores(cores),
        |text| EngineSettings::from_text(&text, cores),
    )
}

pub fn save_engine_settings(settings: EngineSettings) -> io::Result<()> {
    write_setting(ENGINE_FILE, &settings.to_text())
}

fn read_setting(file: &str) -> Option<String> {
    fs::read_to_string(setting_path(file)?).ok()
}

fn write_setting(file: &str, text: &str) -> io::Result<()> {
    let path = setting_path(file).ok_or_else(no_config_folder)?;
    if let Some(folder) = path.parent() {
        fs::create_dir_all(folder)?;
    }
    fs::write(path, text)
}

fn setting_path(file: &str) -> Option<PathBuf> {
    dirs::config_dir().map(|folder| folder.join(APP_FOLDER).join(file))
}

fn no_config_folder() -> io::Error {
    io::Error::new(
        io::ErrorKind::NotFound,
        "this system has no settings folder",
    )
}
