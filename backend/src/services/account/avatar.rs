use std::sync::Arc;

use image::ImageError;

use tokio::sync::Semaphore;
use uuid::Uuid;

use super::avatar_image::{picture_hash, square_jpeg};
use crate::{
    errors::{AppError, ChessCode},
    models::play::messages::ServerMessage,
    repositories::{
        account::{avatar, closure},
        database::Db,
        social::{block, friendship},
    },
    services::play::PlayHub,
};

/// Pictures decoded at once: each may take up to its whole memory budget for a moment.
const DECODES_AT_ONCE: usize = 1;

/// Profile pictures: set, removed, and handed out by their hash.
#[derive(Clone)]
pub struct AvatarService {
    db: Db,
    /// Players' live connections, which carry their picture to challenges and invitations.
    play: PlayHub,
    decodes: Arc<Semaphore>,
}

impl AvatarService {
    pub fn new(db: Db, play: PlayHub) -> Self {
        Self {
            db,
            play,
            decodes: Arc::new(Semaphore::new(DECODES_AT_ONCE)),
        }
    }

    /// Makes `bytes` the player's picture, as a square JPEG; returns the hash that names it.
    pub async fn set(
        &self,
        (user_id, guest): (Uuid, bool),
        bytes: Vec<u8>,
    ) -> Result<String, AppError> {
        if guest {
            return Err(ChessCode::GuestCannotHaveAvatar.into());
        }
        let jpeg = self.decode(bytes).await?;
        let hash = picture_hash(&jpeg);
        let mut tx = self.db.begin().await?;
        refuse_unless_open(avatar::lock_owner(&mut *tx, user_id).await?)?;
        avatar::save(&mut *tx, user_id, (&hash, &jpeg)).await?;
        avatar::set_hash(&mut *tx, user_id, Some(&hash)).await?;
        tx.commit().await?;
        // Only once it is saved, so challenges never carry a picture the server lacks.
        self.play.picture_changed(user_id, Some(hash.clone()));
        self.tell_acquaintances(user_id).await;
        Ok(hash)
    }

    /// The player goes back to their initial.
    pub async fn remove(&self, user_id: Uuid) -> Result<(), AppError> {
        let mut tx = self.db.begin().await?;
        refuse_unless_open(avatar::lock_owner(&mut *tx, user_id).await?)?;
        avatar::delete(&mut *tx, user_id).await?;
        avatar::set_hash(&mut *tx, user_id, None).await?;
        tx.commit().await?;
        // Only once it is saved, so challenges never carry a picture the server lacks.
        self.play.picture_changed(user_id, None);
        self.tell_acquaintances(user_id).await;
        Ok(())
    }

    /// Everyone who shows the player's picture fetches it again: friends, requests, chats, blocks.
    async fn tell_acquaintances(&self, user_id: Uuid) {
        let friends = friendship::friend_ids(&self.db, user_id).await;
        let members = closure::fellow_members(&self.db, user_id).await;
        let asking = closure::request_partners(&self.db, user_id).await;
        let blockers = block::blockers_of(&self.db, user_id).await;
        let (Ok(mut told), Ok(members), Ok(asking), Ok(blockers)) =
            (friends, members, asking, blockers)
        else {
            // Their lists catch up at their next refresh.
            tracing::warn!("finding who to tell about a new picture failed");
            return;
        };
        told.extend(members);
        told.extend(asking);
        told.extend(blockers);
        // The player's other devices show it too: in their header, and in their groups' members.
        self.play.send(user_id, ServerMessage::ProfileChanged);
        self.play.send(user_id, ServerMessage::ConversationsChanged);
        told.sort_unstable();
        told.dedup();
        for other in told {
            self.play.send(other, ServerMessage::FriendsChanged);
            self.play.send(other, ServerMessage::ConversationsChanged);
        }
    }

    /// The picture as a square JPEG, decoded off the async threads and a few at a time.
    async fn decode(&self, bytes: Vec<u8>) -> Result<Vec<u8>, AppError> {
        // The turn goes with the decode, so a request dropped mid-way still holds it until done.
        let turn = Arc::clone(&self.decodes)
            .acquire_owned()
            .await
            .map_err(|_| AppError::from(ChessCode::InternalError))?;
        tokio::task::spawn_blocking(move || {
            let _turn = turn;
            square_jpeg(&bytes)
        })
        .await
        .map_err(|_| AppError::from(ChessCode::InternalError))?
        .map_err(|error| AppError::from(refusal_for(&error)))
    }

    /// The picture `hash` names, for anyone who was handed the hash.
    pub async fn image(&self, user_id: Uuid, hash: &str) -> Result<Vec<u8>, AppError> {
        avatar::image(&self.db, user_id, hash)
            .await?
            .ok_or_else(|| ChessCode::AvatarNotFound.into())
    }
}

/// The account is still open: a closed one's request is one to sign in.
fn refuse_unless_open(open: bool) -> Result<(), AppError> {
    if open {
        Ok(())
    } else {
        Err(ChessCode::SignInRequired.into())
    }
}

/// Why a picture was refused: too large to take on, or not a picture at all.
fn refusal_for(error: &ImageError) -> ChessCode {
    match error {
        ImageError::Limits(_) => ChessCode::AvatarTooLarge,
        ImageError::Decoding(_)
        | ImageError::Encoding(_)
        | ImageError::Parameter(_)
        | ImageError::Unsupported(_)
        | ImageError::IoError(_) => ChessCode::AvatarUnreadable,
    }
}
