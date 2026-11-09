use std::{future::Future, time::Duration};

use chess_domain::TimeControl;
use gpui_kit::{
    assets::IconName,
    component::{
        ActiveTheme as _, Icon, Sizable as _,
        button::{Button, ButtonVariants},
        h_flex,
        input::{Input, InputEvent, InputState},
        scroll::ScrollableElement as _,
        v_flex,
    },
    prelude::FluentBuilder as _,
    *,
};

use super::challenge_form::ChallengeForm;
use crate::{
    clients::server::ServerClient,
    components::{
        panel::StartButton,
        shell::SOCIAL_PICTURE,
        social::{ActionTile, EmptyState, FriendRow, Leaderboard, PeopleList, PersonRow, SortMenu},
    },
    enums::{
        social::{FriendSort, LeaderBoard, Relation, SocialStage},
        sound::Sound,
    },
    models::{
        play::{Challenge, ClientMessage, LiveEvent, ServerMessage},
        server::ServerError,
        social::{FoundUser, Friend, Friends, UserRef},
    },
    services::{
        play::{DEFAULT_TIME_CONTROL, challenge_terms, friend_challenges, whole_seconds},
        server::error_message,
        social::{avatar_path, last_online_text, leaderboard, sorted_friends},
    },
    views::{
        chat::ChatsView,
        live::{LiveConnection, SessionSource, my_face},
        sound::play_sound,
    },
};

/// How long typing pauses before the search goes out.
const SEARCH_PAUSE: Duration = Duration::from_millis(250);
/// The fewest letters worth searching for.
const SHORTEST_SEARCH: usize = 2;

/// A friend's Message button was pressed: their chat is open on the Messages page.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OpenMessages;

/// The Friends page: friends and last online, requests, search, challenges and the leaderboard.
pub struct SocialView {
    client: ServerClient,
    session: SessionSource,
    connection: Entity<LiveConnection>,
    _live: Subscription,
    friends: Friends,
    /// The friends have arrived once; until then the page says it is loading.
    loaded: bool,
    problem: Option<String>,
    search: Entity<InputState>,
    _search_changes: Subscription,
    found: Vec<FoundUser>,
    _search_task: Option<Task<()>>,
    sent: Vec<Challenge>,
    received: Vec<Challenge>,
    /// The time control the last challenge used, which the modal starts from.
    choice: TimeControl,
    /// Whether the player last wanted a challenge rated; a guest's is sent unrated all the same.
    rated: bool,
    /// A challenge went out or was answered, and the server's word on it is still to come.
    awaiting: bool,
    /// The friends, or the requests each way.
    stage: SocialStage,
    sort: FriendSort,
    /// The player's chats, shown on the Messages page: a friend's Message button opens theirs.
    chats: Entity<ChatsView>,
}

impl EventEmitter<OpenMessages> for SocialView {}

impl SocialView {
    pub fn new(
        client: ServerClient,
        (session, connection): (SessionSource, Entity<LiveConnection>),
        chats: Entity<ChatsView>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let _live = cx.subscribe(&connection, |view, _, event: &LiveEvent, cx| {
            view.on_event(event, cx);
        });
        let search =
            cx.new(|cx| InputState::new(window, cx).placeholder("Search by name or username"));
        let _search_changes = cx.subscribe(&search, |view, _, event: &InputEvent, cx| {
            if let InputEvent::Change = event {
                view.search_changed(cx);
            }
        });
        Self {
            client,
            session,
            connection,
            _live,
            friends: Friends::default(),
            loaded: false,
            problem: None,
            search,
            _search_changes,
            found: Vec::new(),
            _search_task: None,
            sent: Vec::new(),
            received: Vec::new(),
            choice: DEFAULT_TIME_CONTROL.unwrap_or(TimeControl::new(3, 2)),
            rated: true,
            awaiting: false,
            stage: SocialStage::default(),
            sort: FriendSort::default(),
            chats,
        }
    }

    /// The page came on screen: its lists are fetched afresh.
    pub fn opened(&mut self, cx: &mut Context<Self>) {
        self.refresh(cx);
    }

    /// What waits for the player here: challenges and friend requests.
    pub fn waiting_for_me(&self) -> usize {
        self.received.len() + self.friends.incoming.len()
    }

    /// The signed-in player's name, for their place on the leaderboard.
    fn my_name(&self, cx: &App) -> String {
        (self.session)(cx).map_or_else(String::new, |session| session.user.username)
    }

    /// Signed in with an account: guests play unrated only.
    fn can_rate(&self, cx: &App) -> bool {
        (self.session)(cx).is_some_and(|session| !session.user.guest)
    }

    /// The signed-in token, unless signed out.
    fn account_token(&self, cx: &App) -> Option<String> {
        (self.session)(cx).map(|session| session.token)
    }

    fn refresh(&mut self, cx: &mut Context<Self>) {
        let Some(token) = self.account_token(cx) else {
            return;
        };
        let request = self.client.friends(&token);
        cx.spawn(async move |view, cx| {
            let result = request.await;
            // The page outlives the request unless the window is closing.
            let _ = view.update(cx, |view, cx| view.friends_arrived(result, cx));
        })
        .detach();
    }

    fn friends_arrived(&mut self, result: Result<Friends, ServerError>, cx: &mut Context<Self>) {
        match result {
            Ok(friends) => {
                let listed = friends.friends.clone();
                self.chats
                    .update(cx, |chats, cx| chats.set_friends(listed, cx));
                self.friends = friends;
                self.loaded = true;
            }
            Err(error) => self.problem = Some(error_message(&error)),
        }
        cx.notify();
    }

    /// Searches once typing pauses; fewer than two letters clear the results.
    fn search_changed(&mut self, cx: &mut Context<Self>) {
        let query = self.search.read(cx).value().trim().to_owned();
        let Some(token) = self.account_token(cx) else {
            return;
        };
        if query.chars().count() < SHORTEST_SEARCH {
            self.found.clear();
            self._search_task = None;
            cx.notify();
            return;
        }
        let client = self.client.clone();
        // A newer search replaces this task, which stops the older one.
        self._search_task = Some(cx.spawn(async move |view, cx| {
            cx.background_executor().timer(SEARCH_PAUSE).await;
            let result = client.search_players(&token, &query).await;
            let _ = view.update(cx, |view, cx| {
                match result {
                    Ok(found) => view.found = found,
                    Err(error) => view.problem = Some(error_message(&error)),
                }
                cx.notify();
            });
        }));
    }

    /// Sends a change to a friendship; once done, the lists and the search follow it.
    fn change_friendship(
        &mut self,
        request: impl Future<Output = Result<(), ServerError>> + 'static,
        cx: &mut Context<Self>,
    ) {
        self.problem = None;
        cx.spawn(async move |view, cx| {
            let result = request.await;
            let _ = view.update(cx, |view, cx| match result {
                Ok(()) => {
                    view.refresh(cx);
                    view.search_changed(cx);
                }
                Err(error) => {
                    view.problem = Some(error_message(&error));
                    cx.notify();
                }
            });
        })
        .detach();
    }

    fn add_friend(&mut self, user_id: &str, cx: &mut Context<Self>) {
        if let Some(token) = self.account_token(cx) {
            let request = self.client.request_friend(&token, user_id);
            self.change_friendship(request, cx);
        }
    }

    fn accept_friend(&mut self, user_id: &str, cx: &mut Context<Self>) {
        if let Some(token) = self.account_token(cx) {
            let request = self.client.accept_friend(&token, user_id);
            self.change_friendship(request, cx);
        }
    }

    /// Unfriends, declines a request, or withdraws one.
    fn remove_friend(&mut self, user_id: &str, cx: &mut Context<Self>) {
        if let Some(token) = self.account_token(cx) {
            let request = self.client.remove_friend(&token, user_id);
            self.change_friendship(request, cx);
        }
    }

    fn block(&mut self, user_id: &str, cx: &mut Context<Self>) {
        if let Some(token) = self.account_token(cx) {
            let request = self.client.block(&token, user_id);
            self.change_friendship(request, cx);
        }
    }

    fn unblock(&mut self, user_id: &str, cx: &mut Context<Self>) {
        if let Some(token) = self.account_token(cx) {
            let request = self.client.unblock(&token, user_id);
            self.change_friendship(request, cx);
        }
    }

    fn on_event(&mut self, event: &LiveEvent, cx: &mut Context<Self>) {
        match event {
            LiveEvent::Connected => self.refresh(cx),
            LiveEvent::Disconnected | LiveEvent::SignedOut => self.awaiting = false,
            LiveEvent::Message(message) => self.on_message(message, cx),
        }
        cx.notify();
    }

    fn on_message(&mut self, message: &ServerMessage, cx: &mut Context<Self>) {
        match message {
            // The search results show where the player stands with each, so they follow too.
            ServerMessage::FriendsChanged => {
                self.refresh(cx);
                self.search_changed(cx);
            }
            ServerMessage::Challenges { sent, received } => {
                let received = friend_challenges(received);
                let new_one = received
                    .iter()
                    .any(|challenge| !self.received.iter().any(|old| old.id == challenge.id));
                if new_one {
                    play_sound(Sound::Notify, cx);
                }
                self.sent = friend_challenges(sent);
                self.received = received;
                self.awaiting = false;
            }
            ServerMessage::Game { .. } => self.awaiting = false,
            // Only a refusal of this page's own challenge or answer is its to show.
            ServerMessage::Error { message } | ServerMessage::InviteRefused { message, .. }
                if self.awaiting =>
            {
                self.awaiting = false;
                self.problem = Some(message.clone());
            }
            ServerMessage::Error { .. }
            | ServerMessage::InviteRefused { .. }
            | ServerMessage::Seeking { .. }
            | ServerMessage::SeekCancelled
            | ServerMessage::ChatMessage { .. }
            | ServerMessage::ConversationsChanged
            | ServerMessage::ProfileChanged
            | ServerMessage::RematchRefused { .. }
            | ServerMessage::BattleSeeking
            | ServerMessage::BattleSeekCancelled
            | ServerMessage::Battle { .. }
            | ServerMessage::BattleChallenges { .. } => {}
        }
    }

    fn send_live(&mut self, message: ClientMessage, cx: &mut Context<Self>) {
        self.problem = None;
        self.awaiting = true;
        self.connection.update(cx, |connection, cx| {
            connection.connect(cx);
            connection.send(message);
        });
        cx.notify();
    }

    /// Opens the challenge modal for `friend`, starting from the last time control picked.
    fn open_challenge(&mut self, friend: Friend, window: &mut Window, cx: &mut Context<Self>) {
        let view = cx.entity().downgrade();
        let friend_id = friend.id.clone();
        ChallengeForm::open(
            friend,
            (self.choice, self.rated, self.can_rate(cx)),
            move |choice, rated, _, cx| {
                // The view outlives the modal unless the window is closing.
                let _ = view.update(cx, |view, cx| {
                    view.send_challenge((friend_id.clone(), choice, rated), cx);
                });
            },
            window,
            cx,
        );
    }

    /// Sends the challenge, rated only for an account, and keeps the choices for the next one.
    fn send_challenge(
        &mut self,
        (friend_id, choice, rated): (String, TimeControl, bool),
        cx: &mut Context<Self>,
    ) {
        self.choice = choice;
        self.rated = rated;
        let message = ClientMessage::Challenge {
            user_id: friend_id,
            initial_s: whole_seconds(choice.initial),
            increment_s: whole_seconds(choice.increment),
            rated: rated && self.can_rate(cx),
        };
        self.send_live(message, cx);
    }

    fn pick_stage(&mut self, stage: SocialStage, cx: &mut Context<Self>) {
        self.stage = stage;
        cx.notify();
    }

    /// The requests, in place of anything a search had found.
    fn open_requests(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.search
            .update(cx, |input, cx| input.set_value("", window, cx));
        self.pick_stage(SocialStage::Requests, cx);
    }

    fn pick_sort(&mut self, sort: FriendSort, cx: &mut Context<Self>) {
        self.sort = sort;
        cx.notify();
    }

    /// A search long enough to go out is in the box: its results take the friends' place.
    fn searching(&self, cx: &App) -> bool {
        self.search.read(cx).value().trim().chars().count() >= SHORTEST_SEARCH
    }

    /// Opens the friend's chat and asks for the Messages page.
    fn message(&mut self, friend_id: &str, window: &mut Window, cx: &mut Context<Self>) {
        self.chats
            .update(cx, |chats, cx| chats.message_friend(friend_id, window, cx));
        cx.emit(OpenMessages);
    }

    /// Find Friends and Friend Requests, side by side, as chess.com's tiles.
    fn tiles(&self, cx: &mut Context<Self>) -> Div {
        let view = cx.entity().downgrade();
        let search = self.search.clone();
        let (find, requests) = (view.clone(), view);
        // Side by side while they fit, one under the other in a narrow window.
        h_flex()
            .flex_wrap()
            .gap_3()
            .child(ActionTile::new(
                "find-friends",
                (IconName::Search, "Find Friends"),
                move |window, cx| {
                    search.update(cx, |input, cx| input.focus(window, cx));
                    let _ = find.update(cx, |view, cx| view.pick_stage(SocialStage::Friends, cx));
                },
            ))
            .child(
                ActionTile::new(
                    "friend-requests",
                    (IconName::UserPlus, "Friend Requests"),
                    move |window, cx| {
                        let _ = requests.update(cx, |view, cx| view.open_requests(window, cx));
                    },
                )
                .count(self.friends.incoming.len()),
            )
    }

    /// Challenges waiting either way, above the friends.
    fn challenges(&self, cx: &mut Context<Self>) -> Option<PeopleList> {
        if self.sent.is_empty() && self.received.is_empty() {
            return None;
        }
        let mut rows: Vec<PersonRow> = Vec::new();
        for challenge in &self.received {
            let (accept, decline) = (challenge.id.clone(), challenge.id.clone());
            let row = PersonRow::new(challenge.from.username.clone())
                .picture(picture_of(&challenge.from))
                .detail(format!("Challenges you · {}", challenge_terms(challenge)))
                .action({
                    let view = cx.entity().downgrade();
                    let id = format!("accept-challenge-{accept}");
                    StartButton::new(id, "Accept", move |_, cx| {
                        let message = ClientMessage::AcceptChallenge {
                            challenge_id: accept.clone(),
                        };
                        // The page outlives its buttons unless the window is closing.
                        let _ = view.update(cx, |view, cx| view.send_live(message, cx));
                    })
                    .small()
                })
                .action(
                    icon_button(("decline-challenge", &decline), IconName::Close, "Decline")
                        .on_click(cx.listener(move |view, _, _, cx| {
                            let message = ClientMessage::DeclineChallenge {
                                challenge_id: decline.clone(),
                            };
                            view.send_live(message, cx);
                        })),
                );
            rows.push(row);
        }
        for challenge in &self.sent {
            let id = challenge.id.clone();
            let row = PersonRow::new(challenge.to.username.clone())
                .picture(picture_of(&challenge.to))
                .detail(format!(
                    "Waiting for an answer · {}",
                    challenge_terms(challenge)
                ))
                .action(
                    icon_button(("cancel-challenge", &id), IconName::Close, "Cancel").on_click(
                        cx.listener(move |view, _, _, cx| {
                            let message = ClientMessage::CancelChallenge {
                                challenge_id: id.clone(),
                            };
                            view.send_live(message, cx);
                        }),
                    ),
                );
            rows.push(row);
        }
        Some(PeopleList::new().heading("Challenges").children(rows))
    }

    /// Under the search box: what it found, or else the friends or the requests.
    fn listing(&self, cx: &mut Context<Self>) -> AnyElement {
        if self.searching(cx) {
            return self.found_list(cx).into_any_element();
        }
        match self.stage {
            SocialStage::Friends => self.friends_list(cx).into_any_element(),
            SocialStage::Requests => self.requests_list(cx).into_any_element(),
        }
    }

    /// "Friends 45" and the order they are in, then each friend.
    fn friends_list(&self, cx: &mut Context<Self>) -> Div {
        let view = cx.entity().downgrade();
        let heading = h_flex()
            .justify_between()
            .child(
                h_flex()
                    .gap_2()
                    .child(
                        div()
                            .text_lg()
                            .font_weight(FontWeight::BOLD)
                            .child("Friends"),
                    )
                    .child(count_chip(self.friends.friends.len(), cx)),
            )
            .child(SortMenu::new(self.sort, move |sort, _, cx| {
                let _ = view.update(cx, |view, cx| view.pick_sort(sort, cx));
            }));
        if !self.loaded {
            return v_flex()
                .gap_2()
                .child(heading)
                .child(muted_line("Loading…", cx));
        }
        if self.friends.friends.is_empty() {
            return v_flex().gap_2().child(heading).child(EmptyState::new(
                IconName::Users,
                "No friends yet",
                "Search for players by name above and add them as friends.",
            ));
        }
        let rows: Vec<FriendRow> = sorted_friends(&self.friends.friends, self.sort)
            .into_iter()
            .map(|friend| self.friend_row(friend, cx))
            .collect();
        v_flex().gap_2().child(heading).children(rows)
    }

    /// A friend: Challenge opens the challenge modal, Message their chat.
    fn friend_row(&self, friend: &Friend, cx: &mut Context<Self>) -> FriendRow {
        let view = cx.entity().downgrade();
        let (challenge, message, remove, block) = (view.clone(), view.clone(), view.clone(), view);
        let (to_challenge, to_message, to_remove, to_block) = (
            friend.clone(),
            friend.id.clone(),
            friend.id.clone(),
            friend.id.clone(),
        );
        FriendRow::new(
            (friend.id.clone(), friend.username.clone()),
            (
                friend.online,
                last_online_text(friend.online, friend.away_s),
            ),
            move |window, cx| {
                let _ = challenge.update(cx, |view, cx| {
                    view.open_challenge(to_challenge.clone(), window, cx);
                });
            },
            (
                move |window, cx| {
                    let _ = message.update(cx, |view, cx| view.message(&to_message, window, cx));
                },
                move |_, cx| {
                    let _ = remove.update(cx, |view, cx| view.remove_friend(&to_remove, cx));
                },
                move |_, cx| {
                    let _ = block.update(cx, |view, cx| view.block(&to_block, cx));
                },
            ),
        )
        .picture(avatar_path(&friend.id, friend.avatar.as_deref()))
    }

    /// Requests to accept or decline, and ones the player sent, to call off; with a way back.
    fn requests_list(&self, cx: &mut Context<Self>) -> Div {
        let back = Button::new("requests-back")
            .ghost()
            .small()
            .icon(IconName::ArrowLeft)
            .on_click(cx.listener(|view, _, _, cx| view.pick_stage(SocialStage::Friends, cx)));
        let heading = h_flex().gap_2().child(back).child(
            div()
                .text_lg()
                .font_weight(FontWeight::BOLD)
                .child("Friend Requests"),
        );
        let Friends {
            incoming,
            outgoing,
            blocked,
            ..
        } = &self.friends;
        if incoming.is_empty() && outgoing.is_empty() && blocked.is_empty() {
            return v_flex().gap_2().child(heading).child(EmptyState::new(
                IconName::UserPlus,
                "No requests",
                "Requests to and from you show up here.",
            ));
        }
        let received: Vec<PersonRow> = incoming
            .iter()
            .map(|person| {
                PersonRow::new(person.username.clone())
                    .picture(picture_of(person))
                    .detail("Wants to be friends")
                    .action(self.accept_button("accept-friend", person, cx))
                    .action(self.remove_button(
                        ("decline-friend", IconName::UserX, "Decline"),
                        person,
                        cx,
                    ))
            })
            .collect();
        let sent: Vec<PersonRow> = outgoing
            .iter()
            .map(|person| {
                PersonRow::new(person.username.clone())
                    .picture(picture_of(person))
                    .detail("Request sent")
                    .action(self.remove_button(
                        ("withdraw-friend", IconName::Close, "Cancel request"),
                        person,
                        cx,
                    ))
            })
            .collect();
        let blocked: Vec<PersonRow> = blocked
            .iter()
            .map(|person| {
                PersonRow::new(person.username.clone())
                    .picture(picture_of(person))
                    .detail("Blocked")
                    .action(self.unblock_button(person, cx))
            })
            .collect();
        v_flex()
            .gap_4()
            .child(heading)
            .when(!received.is_empty(), |this| {
                this.child(PeopleList::new().heading("Received").children(received))
            })
            .when(!sent.is_empty(), |this| {
                this.child(PeopleList::new().heading("Sent").children(sent))
            })
            .when(!blocked.is_empty(), |this| {
                this.child(PeopleList::new().heading("Blocked").children(blocked))
            })
    }

    /// The players the search found.
    fn found_list(&self, cx: &mut Context<Self>) -> Div {
        let heading = div()
            .text_lg()
            .font_weight(FontWeight::BOLD)
            .child("Players");
        if self.found.is_empty() {
            return v_flex().gap_2().child(heading).child(EmptyState::new(
                IconName::Search,
                "No one found",
                "No player has a name like that.",
            ));
        }
        let rows: Vec<PersonRow> = self
            .found
            .iter()
            .map(|found| self.found_row(found, cx))
            .collect();
        v_flex()
            .gap_2()
            .child(heading)
            .child(PeopleList::new().children(rows))
    }

    /// A player found, with what can be done next with them.
    fn found_row(&self, found: &FoundUser, cx: &mut Context<Self>) -> PersonRow {
        let person = UserRef {
            id: found.id.clone(),
            username: found.username.clone(),
            avatar: found.avatar.clone(),
        };
        let row = PersonRow::new(found.username.clone()).picture(picture_of(&person));
        match found.relation {
            Relation::None => row
                .action(self.add_button(&person, cx))
                .action(self.block_button(&person, cx)),
            Relation::RequestReceived => row
                .detail("Wants to be friends")
                .action(self.accept_button("accept-found", &person, cx))
                .action(self.block_button(&person, cx)),
            Relation::RequestSent => row
                .detail("Request sent")
                .action(self.block_button(&person, cx)),
            Relation::Friends => row.detail("Friends"),
            Relation::Blocked => row
                .detail("Blocked")
                .action(self.unblock_button(&person, cx)),
        }
    }

    /// Asks to be friends.
    fn add_button(&self, person: &UserRef, cx: &mut Context<Self>) -> Button {
        let user_id = person.id.clone();
        Button::new(SharedString::from(format!("add-friend-{user_id}")))
            .primary()
            .small()
            .icon(IconName::UserPlus)
            .label("Add friend")
            .on_click(cx.listener(move |view, _, _, cx| view.add_friend(&user_id, cx)))
    }

    /// Blocks them: no request, challenge or message from them reaches the player.
    fn block_button(&self, person: &UserRef, cx: &mut Context<Self>) -> Button {
        let user_id = person.id.clone();
        icon_button(("block-found", &user_id), IconName::Ban, "Block")
            .on_click(cx.listener(move |view, _, _, cx| view.block(&user_id, cx)))
    }

    /// Lifts the player's block.
    fn unblock_button(&self, person: &UserRef, cx: &mut Context<Self>) -> Button {
        let user_id = person.id.clone();
        Button::new(SharedString::from(format!("unblock-{user_id}")))
            .ghost()
            .small()
            .label("Unblock")
            .on_click(cx.listener(move |view, _, _, cx| view.unblock(&user_id, cx)))
    }

    /// Accepts a request they sent.
    fn accept_button(&self, id: &str, person: &UserRef, cx: &mut Context<Self>) -> StartButton {
        let user_id = person.id.clone();
        let view = cx.entity().downgrade();
        StartButton::new(format!("{id}-{user_id}"), "Accept", move |_, cx| {
            // The page outlives its buttons unless the window is closing.
            let _ = view.update(cx, |view, cx| view.accept_friend(&user_id, cx));
        })
        .small()
    }

    /// Ends the request, whichever way it went.
    fn remove_button(
        &self,
        (id, icon, tooltip): (&str, IconName, &'static str),
        person: &UserRef,
        cx: &mut Context<Self>,
    ) -> Button {
        let user_id = person.id.clone();
        icon_button((id, &user_id), icon, tooltip)
            .on_click(cx.listener(move |view, _, _, cx| view.remove_friend(&user_id, cx)))
    }

    /// The friends' leaderboard: each board's best three among them and the player.
    fn leaderboard(&self, cx: &App) -> Leaderboard {
        let name = self.my_name(cx);
        let mine = my_face(&self.session, cx).and_then(|face| face.picture);
        let boards = LeaderBoard::ALL
            .into_iter()
            .map(|board| {
                let rows = leaderboard(
                    &self.friends.friends,
                    ((&name, mine.clone()), &self.friends.my_ratings),
                    board,
                );
                (board, rows)
            })
            .collect();
        Leaderboard::new(boards)
    }
}

impl Render for SocialView {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let card = v_flex()
            .gap_5()
            .p_5()
            .rounded(px(8.))
            .bg(cx.theme().muted)
            .child(self.tiles(cx))
            .child(
                Input::new(&self.search)
                    .large()
                    .prefix(Icon::from(IconName::Search).size_5()),
            )
            .when_some(self.problem.clone(), |this, problem| {
                this.child(div().text_sm().text_color(cx.theme().danger).child(problem))
            })
            .children(self.challenges(cx))
            .child(self.listing(cx));
        let main = v_flex()
            .flex_1()
            .min_w_0()
            .max_w(px(820.))
            .gap_5()
            .child(heading())
            .child(card);
        // The page scrolls with a scrollbar, so the leaderboard's end is found in a short window.
        div()
            .size_full()
            .child(
                h_flex()
                    .w_full()
                    .items_start()
                    .justify_center()
                    .gap_6()
                    .p_6()
                    .child(main)
                    .child(div().pt(px(64.)).child(self.leaderboard(cx))),
            )
            .overflow_y_scrollbar()
    }
}

/// The page's title: its picture and "Friends", large.
fn heading() -> Div {
    h_flex().gap_3().child(img(SOCIAL_PICTURE).size_10()).child(
        div()
            .text_3xl()
            .font_weight(FontWeight::BOLD)
            .child("Friends"),
    )
}

/// A small grey count beside a heading.
fn count_chip(count: usize, cx: &App) -> Div {
    div()
        .px_2()
        .rounded(px(4.))
        .bg(cx.theme().secondary)
        .text_sm()
        .text_color(cx.theme().muted_foreground)
        .child(count.to_string())
}

/// A round icon button with its name on hover: the row actions.
fn icon_button((id, key): (&str, &str), icon: IconName, tooltip: &'static str) -> Button {
    Button::new(SharedString::from(format!("{id}-{key}")))
        .ghost()
        .small()
        .icon(icon)
        .tooltip(tooltip)
}

fn muted_line(text: &'static str, cx: &App) -> Div {
    div()
        .text_sm()
        .text_color(cx.theme().muted_foreground)
        .child(text)
}

/// The person's picture, as an asset path; none shows their initial.
fn picture_of(person: &UserRef) -> Option<String> {
    avatar_path(&person.id, person.avatar.as_deref())
}

#[cfg(test)]
mod tests {
    use std::{rc::Rc, sync::Arc};

    use axum::{
        Json, Router,
        extract::{Path, ws::WebSocketUpgrade},
        http::StatusCode,
        routing::{get, post},
    };
    use gpui_kit::{AppContext as _, Entity, TestAppContext, VisualTestContext};
    use serde_json::{Value, json};

    use chess_domain::TimeControl;

    use super::{OpenMessages, SocialStage, SocialView};
    use crate::{
        clients::server::ServerClient,
        enums::sound::Sound,
        testing::{Heard, ann, click, heard_message, record_socket, wait_until, was_heard},
        views::{
            chat::ChatsView,
            live::{LiveConnection, SessionSource},
            sound::{played_sounds, record_sounds},
        },
    };

    /// Ann has Bob online and a request from Cat; Bob challenges her, and accepting is recorded.
    fn server(asked: Heard) -> ServerClient {
        let accepted = Arc::clone(&asked);
        let blocked = Arc::clone(&asked);
        let unblocked = Arc::clone(&asked);
        let socket_asked = Arc::clone(&asked);
        ServerClient::serving(
            Router::new()
                .route(
                    "/api/social/friends",
                    get(|| async {
                        Json(json!({
                            "friends": [{ "id": "bob", "username": "bob", "online": true }],
                            "incoming": [{ "id": "cat", "username": "cat" }],
                            "outgoing": [],
                            "blocked": [{ "id": "dan", "username": "dan" }]
                        }))
                    }),
                )
                .route(
                    "/api/social/blocks/{user_id}",
                    post(move |Path(user_id): Path<String>| async move {
                        blocked
                            .lock()
                            .expect("the list")
                            .push(format!("block {user_id}"));
                        StatusCode::NO_CONTENT
                    })
                    .delete(move |Path(user_id): Path<String>| async move {
                        unblocked
                            .lock()
                            .expect("the list")
                            .push(format!("unblock {user_id}"));
                        StatusCode::NO_CONTENT
                    }),
                )
                .route(
                    "/api/social/friends/requests/{user_id}/accept",
                    post(move |Path(user_id): Path<String>| async move {
                        let mut asked = accepted.lock().expect("the list");
                        asked.push(format!("accept {user_id}"));
                        StatusCode::NO_CONTENT
                    }),
                )
                .route(
                    "/api/ws",
                    get(move |upgrade: WebSocketUpgrade| async move {
                        upgrade.on_upgrade(move |socket| {
                            record_socket(socket, Some(bob_challenge_lists()), socket_asked)
                        })
                    }),
                ),
        )
    }

    /// Bob's challenge, waiting for Ann as she connects.
    fn bob_challenge_lists() -> Value {
        json!({ "type": "challenges", "sent": [], "received": [{
            "id": "c1",
            "from": { "id": "bob", "username": "bob" },
            "to": { "id": "ann", "username": "ann" },
            "initial_s": 300, "increment_s": 3, "rated": true, "wait_s": 60, "seconds_left": 60
        }]})
    }

    fn signed_in(guest: bool) -> SessionSource {
        let session = ann(guest);
        Rc::new(move |_| Some(session.clone()))
    }

    fn social(
        cx: &mut TestAppContext,
        guest: bool,
    ) -> (Entity<SocialView>, Heard, &mut VisualTestContext) {
        cx.executor().allow_parking();
        cx.update(|cx| {
            gpui_kit::init(cx);
            record_sounds(cx);
        });
        let asked = Heard::default();
        let client = server(Arc::clone(&asked));
        let (view, cx) = cx.add_window_view(|window, cx| {
            let session = signed_in(guest);
            let live = cx.new(|_| LiveConnection::new(client.clone(), Rc::clone(&session)));
            let chats = cx
                .new(|cx| ChatsView::new(client.clone(), (Rc::clone(&session), live.clone()), cx));
            SocialView::new(client, (session, live), chats, window, cx)
        });
        (view, asked, cx)
    }

    #[gpui_kit::test]
    fn friends_and_requests_load_and_a_request_is_accepted(cx: &mut TestAppContext) {
        let (view, asked, cx) = social(cx, false);
        view.update(cx, |view, cx| view.opened(cx));
        wait_until(cx, |cx| view.read_with(cx, |view, _| view.loaded));
        view.read_with(cx, |view, _| {
            assert_eq!(view.friends.friends[0].username, "bob");
            assert!(view.friends.friends[0].online);
            assert_eq!(view.waiting_for_me(), 1, "Cat's request waits");
        });
        view.update(cx, |view, cx| view.accept_friend("cat", cx));
        wait_until(cx, |_| was_heard(&asked, "accept cat"));
    }

    #[gpui_kit::test]
    fn a_challenge_rings_and_accepting_it_is_sent(cx: &mut TestAppContext) {
        let (view, asked, cx) = social(cx, false);
        view.update(cx, |view, cx| {
            view.connection.update(cx, |live, cx| live.connect(cx));
        });
        wait_until(cx, |cx| {
            view.read_with(cx, |view, _| !view.received.is_empty())
        });
        assert!(
            cx.update(|_, cx| played_sounds(cx))
                .contains(&Sound::Notify)
        );
        view.update(cx, |view, cx| {
            let message = super::ClientMessage::AcceptChallenge {
                challenge_id: "c1".to_owned(),
            };
            view.send_live(message, cx);
        });
        wait_until(cx, |_| was_heard(&asked, "accept_challenge c1"));
    }

    #[gpui_kit::test]
    fn a_guest_has_friends_like_anyone(cx: &mut TestAppContext) {
        let (view, _, cx) = social(cx, true);
        view.update(cx, |view, cx| view.opened(cx));
        wait_until(cx, |cx| view.read_with(cx, |view, _| view.loaded));
    }

    #[gpui_kit::test]
    fn friend_requests_open_from_their_tile_and_go_back(cx: &mut TestAppContext) {
        let (view, _, cx) = social(cx, false);
        view.update(cx, |view, cx| view.opened(cx));
        wait_until(cx, |cx| view.read_with(cx, |view, _| view.loaded));
        let stage = |cx: &mut VisualTestContext| view.read_with(cx, |view, _| view.stage);
        assert_eq!(stage(cx), SocialStage::Friends, "friends come first");
        view.update_in(cx, |view, window, cx| {
            view.search
                .update(cx, |input, cx| input.set_value("bo", window, cx));
        });
        click(cx, "friend-requests");
        assert_eq!(stage(cx), SocialStage::Requests);
        assert!(
            !view.read_with(cx, |view, cx| view.searching(cx)),
            "the requests show, not what the search found"
        );
        click(cx, "find-friends");
        assert_eq!(stage(cx), SocialStage::Friends);
    }

    #[gpui_kit::test]
    fn messaging_a_friend_asks_for_the_messages_page(cx: &mut TestAppContext) {
        let (view, _, cx) = social(cx, false);
        let asked = Rc::new(std::cell::Cell::new(false));
        let heard = Rc::clone(&asked);
        let _listening =
            cx.update(|_, cx| cx.subscribe(&view, move |_, _: &OpenMessages, _| heard.set(true)));
        view.update_in(cx, |view, window, cx| view.message("bob", window, cx));
        assert!(asked.get(), "the shell is asked to open Messages");
    }

    #[gpui_kit::test]
    fn blocking_and_unblocking_are_sent_and_the_blocked_are_listed(cx: &mut TestAppContext) {
        let (view, asked, cx) = social(cx, false);
        view.update(cx, |view, cx| view.opened(cx));
        wait_until(cx, |cx| view.read_with(cx, |view, _| view.loaded));
        assert_eq!(
            view.read_with(cx, |view, _| view.friends.blocked[0].username.clone()),
            "dan"
        );
        view.update(cx, |view, cx| view.block("bob", cx));
        wait_until(cx, |_| was_heard(&asked, "block bob"));
        view.update(cx, |view, cx| view.unblock("dan", cx));
        wait_until(cx, |_| was_heard(&asked, "unblock dan"));
    }

    #[test]
    fn rematches_stay_out_of_the_friends_challenges() {
        let challenge = |id: &str, rematch_of: Option<&str>| -> super::Challenge {
            serde_json::from_value(serde_json::json!({
                "id": id, "from": { "id": "bob", "username": "Bob" },
                "to": { "id": "ann", "username": "Ann" },
                "initial_s": 300, "increment_s": 0, "rated": false, "rematch_of": rematch_of,
                "wait_s": 60, "seconds_left": 60,
            }))
            .expect("a challenge")
        };
        let both = [challenge("c1", None), challenge("r1", Some("g1"))];
        let kept = crate::services::play::friend_challenges(&both);
        assert_eq!(kept.len(), 1);
        assert_eq!(kept[0].id, "c1");
    }

    #[gpui_kit::test]
    fn the_last_challenge_sets_the_next_and_a_guest_asks_unrated(cx: &mut TestAppContext) {
        let (view, asked, cx) = social(cx, true);
        view.update(cx, |view, cx| {
            view.connection.update(cx, |live, cx| live.connect(cx));
        });
        let blitz = TimeControl::new(5, 0);
        view.update(cx, |view, cx| {
            view.send_challenge(("bob".to_owned(), blitz, true), cx)
        });
        wait_until(cx, |_| heard_message(&asked, "challenge").is_some());
        let sent = heard_message(&asked, "challenge").expect("the challenge");
        assert_eq!(sent["rated"], false, "a guest's goes unrated all the same");
        assert_eq!(sent["initial_s"], 300);
        view.read_with(cx, |view, _| {
            assert_eq!(
                (view.choice, view.rated),
                (blitz, true),
                "kept for the next modal"
            );
        });
    }
}
