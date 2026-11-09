use uuid::Uuid;

use super::{
    credentials::{checked_optional_email, checked_password, checked_username},
    password,
};
use crate::{
    errors::{AppError, ChessCode, Fields},
    models::{
        account::{CreateUserModel, GetUserModel, NewUser},
        play::messages::ServerMessage,
    },
    repositories::{
        account::{avatar, closure, session, user},
        chat::conversation,
        database::Db,
        social::friendship,
    },
    services::play::PlayHub,
};

/// Accounts: signing up, reading one's own, and closing it.
#[derive(Clone)]
pub struct UserService {
    db: Db,
    play: PlayHub,
}

impl UserService {
    pub fn new(db: Db, play: PlayHub) -> Self {
        Self { db, play }
    }

    /// Checks every field, refuses a taken email or username, then stores the account. The
    /// email is optional.
    pub async fn register(&self, body: CreateUserModel) -> Result<Uuid, AppError> {
        let mut fields = Fields::default();
        let email = fields.check(checked_optional_email(body.email.as_deref()));
        let username = fields.check(checked_username(&body.username));
        let password = fields.check(checked_password(&body.password));
        fields.finish()?;
        self.refuse_taken(email.as_deref(), &username).await?;
        let new_user = NewUser {
            id: Uuid::now_v7(),
            email,
            username,
            password_hash: password::hash(password).await?,
        };
        if !user::insert(&self.db, &new_user).await? {
            // Another sign-up took the email or username after the check.
            self.refuse_taken(new_user.email.as_deref(), &new_user.username)
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

    /// Another player's account, open; a closed or unknown one is not found.
    pub async fn player(&self, user_id: Uuid) -> Result<GetUserModel, AppError> {
        let found = user::get(&self.db, user_id)
            .await?
            .ok_or(ChessCode::UserNotFound)?;
        Ok(found)
    }

    /// Closes the account once its password is typed again; its games stay for the others.
    pub async fn close(&self, user_id: Uuid, password: String) -> Result<(), AppError> {
        let check = closure::password_hash(&self.db, user_id)
            .await?
            .ok_or(ChessCode::SignInRequired)?;
        if check.guest {
            return Err(ChessCode::GuestCannotClose.into());
        }
        if !password::verify(password, Some(check.password_hash)).await? {
            return Err(ChessCode::InvalidCredentials.into());
        }
        let mut told = friendship::friend_ids(&self.db, user_id).await?;
        told.extend(closure::request_partners(&self.db, user_id).await?);
        told.extend(closure::fellow_members(&self.db, user_id).await?);
        told.sort_unstable();
        told.dedup();
        let mut tx = self.db.begin().await?;
        // Their conversations first, then their user row: the order adding a member takes them in.
        let held = closure::held_conversations(&mut tx, user_id).await?;
        closure::lock_user(&mut tx, user_id).await?;
        closure::forget_friendships(&mut tx, user_id).await?;
        closure::forget_blocks(&mut tx, user_id).await?;
        avatar::delete(&mut *tx, user_id).await?;
        avatar::set_hash(&mut *tx, user_id, None).await?;
        closure::forget_challenges(&mut tx, user_id).await?;
        closure::forget_direct_chats(&mut tx, user_id).await?;
        closure::forget_messages(&mut tx, user_id).await?;
        closure::forget_memberships(&mut tx, user_id).await?;
        for conversation_id in held {
            // A group they owned keeps an owner: its longest-standing member left.
            conversation::pass_ownership(&mut *tx, conversation_id).await?;
        }
        closure::forget_empty_groups(&mut tx).await?;
        closure::forget_conversations_started(&mut tx, user_id).await?;
        closure::forget_ratings(&mut tx, user_id).await?;
        closure::forget_rating_history(&mut tx, user_id).await?;
        closure::forget_puzzle_attempts(&mut tx, user_id).await?;
        closure::forget_rush_runs(&mut tx, user_id).await?;
        session::revoke_all_of_user(&mut *tx, user_id).await?;
        closure::close(&mut tx, user_id).await?;
        tx.commit().await?;
        // The live side lets go of them too: searches, invitations, games, battles and sockets.
        self.play.forget_player(user_id);
        // Their friends' lists and everyone's chats lost them: they fetch both again.
        for other in told {
            self.play.send(other, ServerMessage::FriendsChanged);
            self.play.send(other, ServerMessage::ConversationsChanged);
        }
        Ok(())
    }

    /// A taken email, when one was given, or a taken username, is refused.
    async fn refuse_taken(&self, email: Option<&str>, username: &str) -> Result<(), AppError> {
        if let Some(email) = email
            && user::email_taken(&self.db, email).await?
        {
            return Err(ChessCode::EmailTaken.into());
        }
        if user::username_taken(&self.db, username).await? {
            return Err(ChessCode::UsernameTaken.into());
        }
        Ok(())
    }
}
