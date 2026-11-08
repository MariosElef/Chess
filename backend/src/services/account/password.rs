//! Argon2id hashing. Hashing is deliberately slow, so it runs off the async runtime.
use std::sync::OnceLock;

use anyhow::anyhow;
use argon2::{
    Argon2,
    password_hash::{PasswordHasher, PasswordVerifier, phc::PasswordHash},
};
use tokio::task;

/// The password's PHC string (`$argon2id$v=19$…`), salt included.
pub async fn hash(password: String) -> anyhow::Result<String> {
    task::spawn_blocking(move || hash_now(&password)).await?
}

/// Whether `password` matches `hash`. With no account to check, a stand-in hash is checked so
/// an unknown name takes as long as a wrong password and the two cannot be told apart.
pub async fn verify(password: String, hash: Option<String>) -> anyhow::Result<bool> {
    task::spawn_blocking(move || {
        let matches = verify_now(&password, hash.as_deref().unwrap_or(stand_in_hash()?))?;
        Ok(matches && hash.is_some())
    })
    .await?
}

fn hash_now(password: &str) -> anyhow::Result<String> {
    let hash = Argon2::default()
        .hash_password(password.as_bytes())
        .map_err(|error| anyhow!("hashing the password failed: {error}"))?;
    Ok(hash.to_string())
}

fn verify_now(password: &str, hash: &str) -> anyhow::Result<bool> {
    let parsed =
        PasswordHash::new(hash).map_err(|error| anyhow!("a stored hash is malformed: {error}"))?;
    Ok(Argon2::default()
        .verify_password(password.as_bytes(), &parsed)
        .is_ok())
}

/// A hash no password is checked against for real, made once.
fn stand_in_hash() -> anyhow::Result<&'static str> {
    static STAND_IN: OnceLock<String> = OnceLock::new();
    if let Some(hash) = STAND_IN.get() {
        return Ok(hash);
    }
    let hash = hash_now("the stand-in for an unknown account")?;
    Ok(STAND_IN.get_or_init(|| hash))
}

#[cfg(test)]
mod tests {
    use super::{hash, verify};

    #[tokio::test]
    async fn a_password_matches_its_own_hash_only() {
        let stored = hash("correct horse".to_owned()).await.expect("hashes");
        assert!(stored.starts_with("$argon2id$"));
        let right = verify("correct horse".to_owned(), Some(stored.clone()));
        assert!(right.await.expect("verifies"));
        let wrong = verify("battery staple".to_owned(), Some(stored));
        assert!(!wrong.await.expect("verifies"));
    }

    #[tokio::test]
    async fn no_account_never_matches() {
        let missing = verify("anything".to_owned(), None);
        assert!(!missing.await.expect("verifies"));
    }
}
