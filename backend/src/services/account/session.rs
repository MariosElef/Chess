use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
use sha2::{Digest as _, Sha256};
use uuid::Uuid;

use super::password;
use crate::{
    errors::{AppError, ChessCode},
    models::account::{CreateSessionModel, GetSessionModel, NewSession, SessionUser},
    repositories::{
        account::{session, user},
        database::Db,
    },
    services::play::PlayHub,
};

/// How long a sign-in lasts before the app has to sign in again: thirty days.
const SESSION_HOURS: i32 = 30 * 24;
/// A guest is held this long at most, so one left signed in frees itself.
const GUEST_SESSION_HOURS: i32 = 12;

/// Signing in and out, and recognising a signed-in request by its token.
#[derive(Clone)]
pub struct SessionService {
    db: Db,
    /// Says which guests are in use: those connected or at a board.
    play: PlayHub,
}

impl SessionService {
    pub fn new(db: Db, play: PlayHub) -> Self {
        Self { db, play }
    }

    /// A new session for the account whose email or username and password match. Which of
    /// the two was wrong is never said.
    pub async fn sign_in(
        &self,
        body: CreateSessionModel,
        user_agent: Option<String>,
    ) -> Result<GetSessionModel, AppError> {
        let account = user::find_credentials(&self.db, body.login.trim()).await?;
        let stored_hash = account
            .as_ref()
            .map(|account| account.password_hash.clone());
        let matches = password::verify(body.password, stored_hash).await?;
        let Some(account) = account.filter(|_| matches) else {
            return Err(ChessCode::InvalidCredentials.into());
        };
        let token = new_token();
        let new_session = NewSession {
            id: Uuid::now_v7(),
            user_id: account.id,
            token_hash: token_hash(&token),
            user_agent,
        };
        let expires_at = session::insert(&self.db, &new_session, SESSION_HOURS).await?;
        let response = GetSessionModel {
            token,
            expires_at,
            user: account.into_user(),
        };
        Ok(response)
    }

    /// A session for the lowest guest nobody is using, Guest before Guest2 and so on; with all
    /// ten in use, the player is asked to sign up. A guest left signed in on a closed app is
    /// free again, and its old sessions end, so that app signs in afresh next time. Choosing
    /// the guest and starting its session is one transaction, so two sign-ins at once never
    /// share a guest.
    pub async fn sign_in_as_guest(
        &self,
        user_agent: Option<String>,
    ) -> Result<GetSessionModel, AppError> {
        let mut tx = self.db.begin().await?;
        let guest = user::claim_free_guest(&mut *tx, &self.play.players_present())
            .await?
            .ok_or(ChessCode::AllGuestsBusy)?;
        session::revoke_all_of_user(&mut *tx, guest.id).await?;
        let token = new_token();
        let new_session = NewSession {
            id: Uuid::now_v7(),
            user_id: guest.id,
            token_hash: token_hash(&token),
            user_agent,
        };
        let expires_at = session::insert(&mut *tx, &new_session, GUEST_SESSION_HOURS).await?;
        tx.commit().await?;
        let response = GetSessionModel {
            token,
            expires_at,
            user: guest,
        };
        Ok(response)
    }

    /// The live session `token` names; anything else is a request to sign in.
    pub async fn authenticate(&self, token: &str) -> Result<SessionUser, AppError> {
        let response = session::authenticate(&self.db, &token_hash(token))
            .await?
            .ok_or(ChessCode::SignInRequired)?;
        Ok(response)
    }

    pub async fn sign_out(&self, session_id: Uuid) -> Result<(), AppError> {
        session::revoke(&self.db, session_id).await?;
        Ok(())
    }
}

/// 32 random bytes, URL-safe: what the client keeps and sends back as its bearer token.
fn new_token() -> String {
    let bytes: [u8; 32] = rand::random();
    URL_SAFE_NO_PAD.encode(bytes)
}

/// Only this is stored, so a copy of the database signs no one in.
fn token_hash(token: &str) -> Vec<u8> {
    Sha256::digest(token.as_bytes()).to_vec()
}

#[cfg(test)]
mod tests {
    use super::{new_token, token_hash};

    #[test]
    fn tokens_are_long_random_and_hashed_the_same_each_time() {
        let (first, second) = (new_token(), new_token());
        assert_eq!(first.len(), 43, "32 bytes in unpadded base64");
        assert_ne!(first, second);
        assert_eq!(token_hash(&first), token_hash(&first));
        assert_eq!(token_hash(&first).len(), 32);
    }
}
