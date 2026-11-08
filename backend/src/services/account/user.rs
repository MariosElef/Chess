use uuid::Uuid;

use super::{
    credentials::{checked_email, checked_password, checked_username},
    password,
};
use crate::{
    errors::{AppError, ChessCode, Fields},
    models::account::{CreateUserModel, GetUserModel, NewUser},
    repositories::{account::user, database::Db},
};

/// Accounts: signing up and reading one's own.
#[derive(Clone)]
pub struct UserService {
    db: Db,
}

impl UserService {
    pub fn new(db: Db) -> Self {
        Self { db }
    }

    /// Checks every field, refuses a taken email or username, then stores the account.
    pub async fn register(&self, body: CreateUserModel) -> Result<Uuid, AppError> {
        let mut fields = Fields::default();
        let email = fields.check(checked_email(&body.email));
        let username = fields.check(checked_username(&body.username));
        let password = fields.check(checked_password(&body.password));
        fields.finish()?;
        self.refuse_taken(&email, &username).await?;
        let new_user = NewUser {
            id: Uuid::now_v7(),
            email,
            username,
            password_hash: password::hash(password).await?,
        };
        if !user::insert(&self.db, &new_user).await? {
            // Another sign-up took the email or username after the check.
            self.refuse_taken(&new_user.email, &new_user.username)
                .await?;
            return Err(ChessCode::UsernameTaken.into());
        }
        let response = new_user.id;
        Ok(response)
    }

    pub async fn get(&self, user_id: Uuid) -> Result<GetUserModel, AppError> {
        let response = user::get(&self.db, user_id)
            .await?
            .ok_or(ChessCode::SignInRequired)?;
        Ok(response)
    }

    async fn refuse_taken(&self, email: &str, username: &str) -> Result<(), AppError> {
        if user::email_taken(&self.db, email).await? {
            return Err(ChessCode::EmailTaken.into());
        }
        if user::username_taken(&self.db, username).await? {
            return Err(ChessCode::UsernameTaken.into());
        }
        Ok(())
    }
}
