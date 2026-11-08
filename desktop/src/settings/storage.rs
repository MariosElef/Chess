//! The settings file, kept between launches: `~/Library/Application Support/Chess/appearance` on macOS.
use std::{fs, io, path::PathBuf};

use super::appearance::Appearance;

const APP_FOLDER: &str = "Chess";
const APPEARANCE_FILE: &str = "appearance";

/// The saved choice, or `None` on a first launch or when the file is missing or unreadable.
pub fn load_appearance() -> Option<Appearance> {
    let text = fs::read_to_string(appearance_path()?).ok()?;
    Appearance::from_key(text.trim())
}

pub fn save_appearance(appearance: Appearance) -> io::Result<()> {
    let path = appearance_path().ok_or_else(no_config_folder)?;
    if let Some(folder) = path.parent() {
        fs::create_dir_all(folder)?;
    }
    fs::write(path, appearance.key())
}

fn appearance_path() -> Option<PathBuf> {
    dirs::config_dir().map(|folder| folder.join(APP_FOLDER).join(APPEARANCE_FILE))
}

fn no_config_folder() -> io::Error {
    io::Error::new(
        io::ErrorKind::NotFound,
        "this system has no settings folder",
    )
}
