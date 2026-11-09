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
        database::{Db, Tx},
        social::{block, friendship},
    },
    services::play::PlayHub,
};

/// The longest message and group name.
const LONGEST_MESSAGE: usize = 2000;
const LONGEST_TITLE: usize = 60;
/// Messages sent per page.
const PAGE: i64 = 50;

/// Chats between friends, one to one or in groups, and in online games; only members take part.
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
        let mut everyone = members.to_vec();
        everyone.push(user_id);
        let id = Uuid::now_v7();
        let mut tx = self.db.begin().await?;
        // Under the pair locks a block takes too, so no block slips in while the group is made.
        lock_pairs(&mut tx, pairs_among(&everyone)).await?;
        if block::any_among(&mut *tx, &everyone).await? {
            return Err(ChessCode::CannotJoinGroup.into());
        }
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
        let mut tx = self.db.begin().await?;
        conversation::lock(&mut *tx, conversation_id).await?;
        // Checked again under the locks: still a member, and no block between the newcomer and
        // anyone in it; a block between those already in it lets the group go on.
        let members = conversation::member_ids(&mut *tx, conversation_id).await?;
        if !members.contains(&user_id) {
            return Err(ChessCode::ConversationNotFound.into());
        }
        if members.contains(&other) {
            // Already in it: nothing to add.
            drop(tx);
            return self.one(user_id, conversation_id).await;
        }
        let pairs = members.iter().map(|&member| (member, other)).collect();
        lock_pairs(&mut tx, pairs).await?;
        if block::with_any(&mut *tx, other, &members).await? {
            return Err(ChessCode::CannotJoinGroup.into());
        }
        conversation::add_member(&mut *tx, conversation_id, other, "member").await?;
        tx.commit().await?;
        self.tell_members(conversation_id, ServerMessage::ConversationsChanged)
            .await;
        self.one(user_id, conversation_id).await
    }

    /// The player leaves a group; if they owned it, it passes to the longest-standing member.
    pub async fn leave(&self, user_id: Uuid, conversation_id: Uuid) -> Result<(), AppError> {
        self.refuse_unless_group(user_id, conversation_id).await?;
        self.take_out(conversation_id, (user_id, None)).await
    }

    /// The owner renames their group.
    pub async fn rename(
        &self,
        user_id: Uuid,
        conversation_id: Uuid,
        title: &str,
    ) -> Result<ConversationModel, AppError> {
        self.refuse_unless_group(user_id, conversation_id).await?;
        let title = title.trim();
        if title.is_empty() || title.chars().count() > LONGEST_TITLE {
            return Err(ChessCode::GroupTitle.into());
        }
        let mut tx = self.db.begin().await?;
        conversation::lock(&mut *tx, conversation_id).await?;
        refuse_unless_owner(&mut tx, conversation_id, user_id).await?;
        conversation::rename(&mut *tx, conversation_id, title).await?;
        tx.commit().await?;
        self.tell_members(conversation_id, ServerMessage::ConversationsChanged)
            .await;
        self.one(user_id, conversation_id).await
    }

    /// The owner takes a member out of their group; anyone taking themselves out is leaving.
    pub async fn remove_member(
        &self,
        user_id: Uuid,
        conversation_id: Uuid,
        other: Uuid,
    ) -> Result<(), AppError> {
        if other == user_id {
            return self.leave(user_id, conversation_id).await;
        }
        self.refuse_unless_group(user_id, conversation_id).await?;
        self.take_out(conversation_id, (other, Some(user_id))).await
    }

    /// `member` is out of the group, by `owner` if given, and the group keeps an owner.
    async fn take_out(
        &self,
        conversation_id: Uuid,
        (member, owner): (Uuid, Option<Uuid>),
    ) -> Result<(), AppError> {
        let mut tx = self.db.begin().await?;
        conversation::lock(&mut *tx, conversation_id).await?;
        if let Some(owner) = owner {
            refuse_unless_owner(&mut tx, conversation_id, owner).await?;
        }
        conversation::remove_member(&mut *tx, conversation_id, member).await?;
        conversation::pass_ownership(&mut *tx, conversation_id).await?;
        tx.commit().await?;
        self.tell_members(conversation_id, ServerMessage::ConversationsChanged)
            .await;
        self.play.send(member, ServerMessage::ConversationsChanged);
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
        let conversation = self.refuse_unless_member(user_id, conversation_id).await?;
        if ConversationKind::from_key(&conversation.kind) == Some(ConversationKind::Direct) {
            self.refuse_unless_still_friends(user_id, conversation_id)
                .await?;
        }
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
        let mut owners: HashMap<Uuid, Uuid> = HashMap::new();
        for member in self.member_rows(&ids).await? {
            if member.owner {
                owners.insert(member.conversation_id, member.user_id);
            }
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
                    owner_id: owners.remove(&row.id),
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

    /// A direct chat's other member is still a friend: no one unfriended or blocked is messaged.
    async fn refuse_unless_still_friends(
        &self,
        user_id: Uuid,
        conversation_id: Uuid,
    ) -> Result<(), AppError> {
        let other = self
            .member_rows(&[conversation_id])
            .await?
            .into_iter()
            .find(|member| member.user_id != user_id)
            .map(|member| member.user_id)
            .ok_or(ChessCode::OnlyFriends)?;
        self.refuse_unless_friends(user_id, other).await
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

/// Every pair among `players`, each as many times as it appears.
fn pairs_among(players: &[Uuid]) -> Vec<(Uuid, Uuid)> {
    players
        .iter()
        .enumerate()
        .flat_map(|(index, &first)| {
            players[index + 1..]
                .iter()
                .map(move |&second| (first, second))
        })
        .collect()
}

/// Takes the pairs' locks in one fixed order, so two takers never wait on each other.
async fn lock_pairs(tx: &mut Tx, pairs: Vec<(Uuid, Uuid)>) -> Result<(), AppError> {
    let mut pairs: Vec<(Uuid, Uuid)> = pairs
        .into_iter()
        .filter(|(first, second)| first != second)
        .map(|(first, second)| (first.min(second), first.max(second)))
        .collect();
    pairs.sort_unstable();
    pairs.dedup();
    for pair in pairs {
        friendship::lock_pair(&mut **tx, pair).await?;
    }
    Ok(())
}

/// The player owns the group, as the transaction holding it sees.
async fn refuse_unless_owner(
    tx: &mut Tx,
    conversation_id: Uuid,
    user_id: Uuid,
) -> Result<(), AppError> {
    if !conversation::is_owner(&mut **tx, conversation_id, user_id).await? {
        return Err(ChessCode::NotGroupOwner.into());
    }
    Ok(())
}

/// A pair's one direct conversation, whoever starts it.
fn direct_key(first: Uuid, second: Uuid) -> String {
    format!("{}:{}", first.min(second), first.max(second))
}

/// What the player sees a conversation called: a group's name, the other player, or "Game chat".
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
