use std::collections::HashMap;

use uuid::Uuid;

use crate::{
    enums::{chat::ConversationKind, social::FriendshipStatus},
    errors::{AppError, ChessCode},
    models::{
        chat::{
            conversation::{ConversationModel, ConversationRow, MemberRow, NewConversation},
            message::{ChatMessageModel, MessageRow, NewMessage},
        },
        play::messages::ServerMessage,
        social::friend::UserRef,
    },
    repositories::{
        chat::{conversation, message},
        database::Db,
        social::friendship,
    },
    services::play::PlayHub,
};

/// The longest message and group name.
const LONGEST_MESSAGE: usize = 2000;
const LONGEST_TITLE: usize = 60;
/// Messages sent per page.
const PAGE: i64 = 50;

/// Conversations between friends, one to one or in groups, and each online game's chat. Only
/// members read or write one; a message reaches every member online at once.
#[derive(Clone)]
pub struct ChatService {
    db: Db,
    play: PlayHub,
}

impl ChatService {
    pub fn new(db: Db, play: PlayHub) -> Self {
        Self { db, play }
    }

    /// The player's conversations, most recently active first; games' chats are with the game.
    pub async fn list(&self, user_id: Uuid) -> Result<Vec<ConversationModel>, AppError> {
        let rows = conversation::of_user(&self.db, user_id).await?;
        self.models(user_id, rows).await
    }

    /// The conversation with a friend, started if it is the first time.
    pub async fn open_direct(
        &self,
        user_id: Uuid,
        other: Uuid,
    ) -> Result<ConversationModel, AppError> {
        self.refuse_unless_friends(user_id, other).await?;
        let key = direct_key(user_id, other);
        let id = match conversation::find_direct(&self.db, &key).await? {
            Some(id) => id,
            None => {
                let id = Uuid::now_v7();
                let mut tx = self.db.begin().await?;
                let new = NewConversation {
                    id,
                    kind: ConversationKind::Direct,
                    title: None,
                    direct_key: Some(key),
                    created_by_id: Some(user_id),
                };
                conversation::insert(&mut *tx, &new).await?;
                for member in [user_id, other] {
                    conversation::add_member(&mut *tx, id, member, "member").await?;
                }
                tx.commit().await?;
                id
            }
        };
        self.one(user_id, id).await
    }

    /// A group of the player and some of their friends, with a name.
    pub async fn create_group(
        &self,
        user_id: Uuid,
        title: &str,
        members: &[Uuid],
    ) -> Result<ConversationModel, AppError> {
        let title = title.trim();
        if title.is_empty() || title.chars().count() > LONGEST_TITLE {
            return Err(ChessCode::GroupTitle.into());
        }
        for &member in members {
            self.refuse_unless_friends(user_id, member).await?;
        }
        let id = Uuid::now_v7();
        let mut tx = self.db.begin().await?;
        let new = NewConversation {
            id,
            kind: ConversationKind::Group,
            title: Some(title.to_owned()),
            direct_key: None,
            created_by_id: Some(user_id),
        };
        conversation::insert(&mut *tx, &new).await?;
        conversation::add_member(&mut *tx, id, user_id, "owner").await?;
        for &member in members {
            conversation::add_member(&mut *tx, id, member, "member").await?;
        }
        tx.commit().await?;
        self.tell_members(id, ServerMessage::ConversationsChanged)
            .await;
        self.one(user_id, id).await
    }

    /// A member adds one of their friends to a group.
    pub async fn add_member(
        &self,
        user_id: Uuid,
        conversation_id: Uuid,
        other: Uuid,
    ) -> Result<ConversationModel, AppError> {
        self.refuse_unless_group(user_id, conversation_id).await?;
        self.refuse_unless_friends(user_id, other).await?;
        conversation::add_member(&self.db, conversation_id, other, "member").await?;
        self.tell_members(conversation_id, ServerMessage::ConversationsChanged)
            .await;
        self.one(user_id, conversation_id).await
    }

    /// The player leaves a group.
    pub async fn leave(&self, user_id: Uuid, conversation_id: Uuid) -> Result<(), AppError> {
        self.refuse_unless_group(user_id, conversation_id).await?;
        conversation::remove_member(&self.db, conversation_id, user_id).await?;
        self.tell_members(conversation_id, ServerMessage::ConversationsChanged)
            .await;
        self.play.send(user_id, ServerMessage::ConversationsChanged);
        Ok(())
    }

    /// A page of messages, oldest first: the newest, or those before `before`.
    pub async fn messages(
        &self,
        user_id: Uuid,
        conversation_id: Uuid,
        before: Option<Uuid>,
    ) -> Result<Vec<ChatMessageModel>, AppError> {
        self.refuse_unless_member(user_id, conversation_id).await?;
        let mut rows = message::page(&self.db, conversation_id, before, PAGE).await?;
        rows.reverse();
        Ok(rows.into_iter().map(message_model).collect())
    }

    /// Sends a message; every member online has it at once.
    pub async fn send(
        &self,
        user_id: Uuid,
        conversation_id: Uuid,
        body: &str,
    ) -> Result<ChatMessageModel, AppError> {
        self.refuse_unless_member(user_id, conversation_id).await?;
        let body = body.trim();
        if body.is_empty() || body.chars().count() > LONGEST_MESSAGE {
            return Err(ChessCode::MessageLength.into());
        }
        let new = NewMessage {
            id: Uuid::now_v7(),
            conversation_id,
            sender_id: user_id,
            body: body.to_owned(),
        };
        let sent_at = message::insert(&self.db, &new).await?;
        // The sender has read their own message, and all before it.
        conversation::mark_read(&self.db, conversation_id, user_id).await?;
        let sender_name = self
            .member_rows(&[conversation_id])
            .await?
            .into_iter()
            .find(|member| member.user_id == user_id)
            .map(|member| member.username)
            .unwrap_or_default();
        let sent = ChatMessageModel {
            id: new.id,
            conversation_id,
            sender: UserRef {
                id: user_id,
                username: sender_name,
            },
            body: new.body,
            sent_at,
        };
        self.tell_members(
            conversation_id,
            ServerMessage::ChatMessage {
                message: sent.clone(),
            },
        )
        .await;
        Ok(sent)
    }

    pub async fn mark_read(&self, user_id: Uuid, conversation_id: Uuid) -> Result<(), AppError> {
        self.refuse_unless_member(user_id, conversation_id).await?;
        conversation::mark_read(&self.db, conversation_id, user_id).await?;
        Ok(())
    }

    async fn one(&self, user_id: Uuid, id: Uuid) -> Result<ConversationModel, AppError> {
        let row = conversation::one_of_user(&self.db, user_id, id)
            .await?
            .ok_or(ChessCode::ConversationNotFound)?;
        let mut models = self.models(user_id, vec![row]).await?;
        models
            .pop()
            .ok_or_else(|| ChessCode::ConversationNotFound.into())
    }

    /// Each conversation with its members, its name for the player and its last message.
    async fn models(
        &self,
        user_id: Uuid,
        rows: Vec<ConversationRow>,
    ) -> Result<Vec<ConversationModel>, AppError> {
        let ids: Vec<Uuid> = rows.iter().map(|row| row.id).collect();
        let mut members: HashMap<Uuid, Vec<UserRef>> = HashMap::new();
        for member in self.member_rows(&ids).await? {
            members
                .entry(member.conversation_id)
                .or_default()
                .push(UserRef {
                    id: member.user_id,
                    username: member.username,
                });
        }
        let mut last: HashMap<Uuid, ChatMessageModel> = message::last_of(&self.db, &ids)
            .await?
            .into_iter()
            .map(|row| (row.conversation_id, message_model(row)))
            .collect();
        let models = rows
            .into_iter()
            .filter_map(|row| {
                let kind = ConversationKind::from_key(&row.kind)?;
                let members = members.remove(&row.id).unwrap_or_default();
                Some(ConversationModel {
                    id: row.id,
                    kind,
                    title: title_for(user_id, kind, row.title, &members),
                    members,
                    last_message: last.remove(&row.id),
                    unread: row.unread,
                })
            })
            .collect();
        Ok(models)
    }

    async fn member_rows(&self, ids: &[Uuid]) -> Result<Vec<MemberRow>, AppError> {
        Ok(conversation::members_of(&self.db, ids).await?)
    }

    async fn refuse_unless_member(
        &self,
        user_id: Uuid,
        conversation_id: Uuid,
    ) -> Result<ConversationRow, AppError> {
        let row = conversation::one_of_user(&self.db, user_id, conversation_id)
            .await?
            .ok_or(ChessCode::ConversationNotFound)?;
        Ok(row)
    }

    async fn refuse_unless_group(
        &self,
        user_id: Uuid,
        conversation_id: Uuid,
    ) -> Result<(), AppError> {
        let row = self.refuse_unless_member(user_id, conversation_id).await?;
        if ConversationKind::from_key(&row.kind) != Some(ConversationKind::Group) {
            return Err(ChessCode::NotAGroup.into());
        }
        Ok(())
    }

    async fn refuse_unless_friends(&self, user_id: Uuid, other: Uuid) -> Result<(), AppError> {
        let pair = (user_id.min(other), user_id.max(other));
        let friends = friendship::between(&self.db, pair)
            .await?
            .is_some_and(|row| {
                FriendshipStatus::from_key(&row.status) == Some(FriendshipStatus::Accepted)
            });
        if !friends || user_id == other {
            return Err(ChessCode::OnlyFriends.into());
        }
        Ok(())
    }

    /// Sends `message` to every member of the conversation who is online.
    async fn tell_members(&self, conversation_id: Uuid, message: ServerMessage) {
        match conversation::member_ids(&self.db, conversation_id).await {
            Ok(members) => {
                for member in members {
                    self.play.send(member, message.clone());
                }
            }
            Err(error) => tracing::error!(?error, "finding a conversation's members failed"),
        }
    }
}

/// A pair's one direct conversation, whoever starts it.
fn direct_key(first: Uuid, second: Uuid) -> String {
    format!("{}:{}", first.min(second), first.max(second))
}

/// What the player sees a conversation called: a group's name, the other player in a direct
/// chat, "Game chat" for a game's.
fn title_for(
    user_id: Uuid,
    kind: ConversationKind,
    stored: Option<String>,
    members: &[UserRef],
) -> String {
    match kind {
        ConversationKind::Group => stored.unwrap_or_default(),
        ConversationKind::Direct => members
            .iter()
            .find(|member| member.id != user_id)
            .map(|member| member.username.clone())
            .unwrap_or_default(),
        ConversationKind::Game => "Game chat".to_owned(),
    }
}

fn message_model(row: MessageRow) -> ChatMessageModel {
    ChatMessageModel {
        id: row.id,
        conversation_id: row.conversation_id,
        sender: UserRef {
            id: row.sender_id,
            username: row.sender_name,
        },
        body: row.body,
        sent_at: row.sent_at,
    }
}

#[cfg(test)]
mod tests {
    use uuid::Uuid;

    use super::{direct_key, title_for};
    use crate::{enums::chat::ConversationKind, models::social::friend::UserRef};

    #[test]
    fn a_pair_has_one_key_whoever_starts() {
        let (a, b) = (Uuid::from_u128(1), Uuid::from_u128(2));
        assert_eq!(direct_key(a, b), direct_key(b, a));
    }

    #[test]
    fn a_direct_chat_is_called_after_the_other_player() {
        let (me, them) = (Uuid::from_u128(1), Uuid::from_u128(2));
        let members = [
            UserRef {
                id: me,
                username: "ann".to_owned(),
            },
            UserRef {
                id: them,
                username: "bob".to_owned(),
            },
        ];
        assert_eq!(
            title_for(me, ConversationKind::Direct, None, &members),
            "bob"
        );
        let group = title_for(
            me,
            ConversationKind::Group,
            Some("Club".to_owned()),
            &members,
        );
        assert_eq!(group, "Club");
    }
}
