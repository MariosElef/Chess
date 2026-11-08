use std::{env, ffi::OsString, path::PathBuf};

const BINARY: &str = "stockfish";
/// Where Homebrew installs it on Apple silicon and on Intel Macs. An app opened from Finder
/// does not get the shell's `PATH`, so these are tried even when `PATH` lacks them.
const HOMEBREW: [&str; 2] = ["/opt/homebrew/bin/stockfish", "/usr/local/bin/stockfish"];

/// The Stockfish to run: `$STOCKFISH_PATH` if set, else the Homebrew locations, else `PATH`.
pub fn locate_stockfish() -> Option<PathBuf> {
    candidates(env::var_os("STOCKFISH_PATH"), env::var_os("PATH"))
        .into_iter()
        .find(|path| path.is_file())
}

/// Every place to look, in order of preference.
fn candidates(chosen: Option<OsString>, path_var: Option<OsString>) -> Vec<PathBuf> {
    let chosen = chosen.map(PathBuf::from);
    let homebrew = HOMEBREW.iter().map(PathBuf::from);
    let on_path = path_var
        .iter()
        .flat_map(env::split_paths)
        .map(|folder| folder.join(BINARY));
    chosen.into_iter().chain(homebrew).chain(on_path).collect()
}

#[cfg(test)]
mod tests {
    use std::{ffi::OsString, path::PathBuf};

    use super::candidates;

    #[test]
    fn a_chosen_path_comes_first_then_homebrew_then_path() {
        let found = candidates(
            Some(OsString::from("/opt/custom/stockfish")),
            Some(OsString::from("/usr/bin:/bin")),
        );
        assert_eq!(
            found,
            [
                "/opt/custom/stockfish",
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
        assert_eq!(candidates(None, None).len(), 2);
    }
}
