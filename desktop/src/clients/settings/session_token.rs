use std::{fs, io, path::PathBuf};

/// The file in the settings folder that keeps the session token between launches.
const SESSION_FILE: &str = "session";
const APP_FOLDER: &str = "Chess";

/// Where the session token is kept, readable only by this user, so the app stays signed in.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionTokenFile {
    path: Option<PathBuf>,
}

impl SessionTokenFile {
    /// `~/Library/Application Support/Chess/session` on macOS.
    pub fn in_settings_folder() -> Self {
        Self {
            path: dirs::config_dir().map(|folder| folder.join(APP_FOLDER).join(SESSION_FILE)),
        }
    }

    /// A file of the test's own, never the real settings.
    #[cfg(test)]
    pub fn at(path: PathBuf) -> Self {
        Self { path: Some(path) }
    }

    /// The saved token, or `None` when signed out or the file cannot be read.
    pub fn load(&self) -> Option<String> {
        let token = fs::read_to_string(self.path.as_ref()?).ok()?;
        let token = token.trim();
        (!token.is_empty()).then(|| token.to_owned())
    }

    pub fn save(&self, token: &str) -> io::Result<()> {
        let path = self.path.as_ref().ok_or_else(no_settings_folder)?;
        if let Some(folder) = path.parent() {
            fs::create_dir_all(folder)?;
        }
        fs::write(path, token)?;
        owner_only(path)
    }

    /// Forgets the saved token only if it is `token`: another app may have saved its own since.
    pub fn forget_if(&self, token: &str) -> io::Result<()> {
        if self.load().as_deref() == Some(token) {
            return self.forget();
        }
        Ok(())
    }

    /// Signing out: the token is gone from disk, whether or not the server heard about it.
    pub fn forget(&self) -> io::Result<()> {
        match &self.path {
            Some(path) if path.exists() => fs::remove_file(path),
            Some(_) | None => Ok(()),
        }
    }
}

#[cfg(unix)]
fn owner_only(path: &PathBuf) -> io::Result<()> {
    use std::os::unix::fs::PermissionsExt as _;
    fs::set_permissions(path, fs::Permissions::from_mode(0o600))
}

#[cfg(not(unix))]
fn owner_only(_: &PathBuf) -> io::Result<()> {
    Ok(())
}

fn no_settings_folder() -> io::Error {
    io::Error::new(
        io::ErrorKind::NotFound,
        "this system has no settings folder",
    )
}

#[cfg(test)]
mod tests {
    use std::{env, process};

    use super::SessionTokenFile;

    #[test]
    fn a_token_is_saved_read_back_and_forgotten() {
        let path = env::temp_dir().join(format!("chess-session-test-{}", process::id()));
        let file = SessionTokenFile::at(path.clone());
        assert_eq!(file.load(), None);
        file.save("abc").expect("saved");
        assert_eq!(file.load().as_deref(), Some("abc"));
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt as _;
            let mode = std::fs::metadata(&path)
                .expect("exists")
                .permissions()
                .mode();
            assert_eq!(mode & 0o777, 0o600, "only its owner can read it");
        }
        file.forget().expect("forgotten");
        assert_eq!(file.load(), None);
    }

    #[test]
    fn only_the_token_named_is_forgotten() {
        let path = env::temp_dir().join(format!("chess-session-if-{}", process::id()));
        let file = SessionTokenFile::at(path);
        file.save("newer").expect("saved");
        file.forget_if("older").expect("left be");
        assert_eq!(file.load().as_deref(), Some("newer"), "another app's stays");
        file.forget_if("newer").expect("forgotten");
        assert_eq!(file.load(), None);
    }
}
