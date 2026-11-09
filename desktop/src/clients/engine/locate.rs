use std::{
    env,
    ffi::OsString,
    path::{Path, PathBuf},
};

/// Stockfish's program name on this system.
const BINARY: &str = if cfg!(windows) {
    "stockfish.exe"
} else {
    "stockfish"
};
/// Where Homebrew installs it on Apple silicon and Intel, tried even when `PATH` lacks them.
const HOMEBREW: [&str; 2] = ["/opt/homebrew/bin/stockfish", "/usr/local/bin/stockfish"];

/// The Stockfish to run: `$STOCKFISH_PATH` if set, else the one shipped with the app, else the
/// Homebrew locations, else `PATH`.
pub fn locate_stockfish() -> Option<PathBuf> {
    let shipped = env::current_exe()
        .ok()
        .map(|app| shipped_with(&app))
        .unwrap_or_default();
    candidates(env::var_os("STOCKFISH_PATH"), shipped, env::var_os("PATH"))
        .into_iter()
        .find(|path| path.is_file())
}

/// Where an installed app keeps its own Stockfish: beside the program, as the Windows and Linux
/// downloads have it, or in a macOS app's Resources.
fn shipped_with(app: &Path) -> Vec<PathBuf> {
    let Some(folder) = app.parent() else {
        return Vec::new();
    };
    vec![
        folder.join(BINARY),
        folder.join("../Resources").join(BINARY),
    ]
}

/// Every place to look, in order of preference.
fn candidates(
    chosen: Option<OsString>,
    shipped: Vec<PathBuf>,
    path_var: Option<OsString>,
) -> Vec<PathBuf> {
    let chosen = chosen.map(PathBuf::from);
    let homebrew = HOMEBREW.iter().map(PathBuf::from);
    let on_path = path_var
        .iter()
        .flat_map(env::split_paths)
        .map(|folder| folder.join(BINARY));
    chosen
        .into_iter()
        .chain(shipped)
        .chain(homebrew)
        .chain(on_path)
        .collect()
}

#[cfg(test)]
mod tests {
    use std::{ffi::OsString, path::PathBuf};

    use super::{candidates, shipped_with};

    #[test]
    fn a_chosen_path_comes_first_then_the_shipped_one_then_homebrew_then_path() {
        let shipped = shipped_with(&PathBuf::from(
            "/Applications/Chess.app/Contents/MacOS/chess",
        ));
        let found = candidates(
            Some(OsString::from("/opt/custom/stockfish")),
            shipped,
            Some(OsString::from("/usr/bin:/bin")),
        );
        assert_eq!(
            found,
            [
                "/opt/custom/stockfish",
                "/Applications/Chess.app/Contents/MacOS/stockfish",
                "/Applications/Chess.app/Contents/MacOS/../Resources/stockfish",
                "/opt/homebrew/bin/stockfish",
                "/usr/local/bin/stockfish",
                "/usr/bin/stockfish",
                "/bin/stockfish",
            ]
            .map(PathBuf::from)
        );
    }

    #[test]
    fn without_a_choice_or_path_only_homebrew_is_tried() {
        assert_eq!(candidates(None, Vec::new(), None).len(), 2);
    }
}
