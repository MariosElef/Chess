use std::{future::Future, time::Duration};

use chess_domain::TimeControl;
use gpui_kit::{
    assets::IconName,
    component::{
        ActiveTheme as _, Icon, Sizable as _,
        button::{Button, ButtonVariants},
        h_flex,
        input::{Input, InputEvent, InputState},
        v_flex,
    },
    prelude::FluentBuilder as _,
    *,
};

use crate::{
    clients::server::ServerClient,
    components::{
        shell::SOCIAL_PICTURE,
        social::{
            ActionTile, ChallengePicker, EmptyState, FriendRow, Leaderboard, PeopleList, PersonRow,
            SortMenu,
        },
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
        play::{DEFAULT_TIME_CONTROL, challenge_terms},
        server::error_message,
        social::{last_online_text, leaderboard, sorted_friends},
    },
    views::{
        chat::ChatsView,
        live::{LiveConnection, SessionSource},
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
    /// The friend whose challenge picker is open.
    picking_for: Option<String>,
    choice: TimeControl,
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
            picking_for: None,
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

    fn on_event(&mut self, event: &LiveEvent, cx: &mut Context<Self>) {
        match event {
            LiveEvent::Connected => self.refresh(cx),
            LiveEvent::Disconnected => self.awaiting = false,
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
                let new_one = received
                    .iter()
                    .any(|challenge| !self.received.iter().any(|old| old.id == challenge.id));
                if new_one {
                    play_sound(Sound::Notify, cx);
                }
                self.sent = sent.clone();
                self.received = received.clone();
                self.awaiting = false;
            }
            ServerMessage::Game { .. } => self.awaiting = false,
            // Only a refusal of this page's own challenge or answer is its to show.
            ServerMessage::Error { message } if self.awaiting => {
                self.awaiting = false;
                self.problem = Some(message.clone());
            }
            ServerMessage::Error { .. }
            | ServerMessage::Seeking { .. }
            | ServerMessage::SeekCancelled
            | ServerMessage::ChatMessage { .. }
            | ServerMessage::ConversationsChanged
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

    fn send_challenge(&mut self, friend_id: String, cx: &mut Context<Self>) {
        self.picking_for = None;
        let message = ClientMessage::Challenge {
            user_id: friend_id,
            initial_s: seconds(self.choice.initial),
            increment_s: seconds(self.choice.increment),
            rated: self.rated && self.can_rate(cx),
        };
        self.send_live(message, cx);
    }

    fn pick_stage(&mut self, stage: SocialStage, cx: &mut Context<Self>) {
        self.stage = stage;
        self.picking_for = None;
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
        h_flex()
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
                .detail(format!("Challenges you · {}", challenge_terms(challenge)))
                .action(
                    Button::new(SharedString::from(format!("accept-challenge-{accept}")))
                        .success()
                        .small()
                        .icon(IconName::Check)
                        .label("Accept")
                        .on_click(cx.listener(move |view, _, _, cx| {
                            let message = ClientMessage::AcceptChallenge {
                                challenge_id: accept.clone(),
                            };
                            view.send_live(message, cx);
                        })),
                )
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
        let rows: Vec<Div> = sorted_friends(&self.friends.friends, self.sort)
            .into_iter()
            .map(|friend| self.friend_row(friend, cx))
            .collect();
        v_flex().gap_2().child(heading).children(rows)
    }

    /// A friend, and the challenge picker under them once Challenge is pressed.
    fn friend_row(&self, friend: &Friend, cx: &mut Context<Self>) -> Div {
        let picking = self.picking_for.as_deref() == Some(friend.id.as_str());
        let view = cx.entity().downgrade();
        let (challenge, message, remove) = (view.clone(), view.clone(), view);
        let (to_challenge, to_message, to_remove) =
            (friend.id.clone(), friend.id.clone(), friend.id.clone());
        let row = FriendRow::new(
            (friend.id.clone(), friend.username.clone()),
            (
                friend.online,
                last_online_text(friend.online, friend.away_s),
            ),
            move |_, cx| {
                let _ = challenge.update(cx, |view, cx| {
                    view.picking_for = Some(to_challenge.clone());
                    cx.notify();
                });
            },
            (
                move |window, cx| {
                    let _ = message.update(cx, |view, cx| view.message(&to_message, window, cx));
                },
                move |_, cx| {
                    let _ = remove.update(cx, |view, cx| view.remove_friend(&to_remove, cx));
                },
            ),
        );
        v_flex().child(row).when(picking, |this| {
            this.child(
                div()
                    .pl(px(88.))
                    .pb_2()
                    .child(self.picker(friend.id.clone(), cx)),
            )
        })
    }

    fn picker(&self, friend_id: String, cx: &mut Context<Self>) -> ChallengePicker {
        let view = cx.entity().downgrade();
        let (choose, rated, send, close) = (view.clone(), view.clone(), view.clone(), view);
        ChallengePicker::new(
            (self.choice, self.rated, self.can_rate(cx)),
            move |choice, _, cx| {
                let _ = choose.update(cx, |view, cx| {
                    view.choice = choice;
                    cx.notify();
                });
            },
            move |on, _, cx| {
                let _ = rated.update(cx, |view, cx| {
                    view.rated = on;
                    cx.notify();
                });
            },
            (
                move |_, cx: &mut App| {
                    let _ = send.update(cx, |view, cx| view.send_challenge(friend_id.clone(), cx));
                },
                move |_, cx: &mut App| {
                    let _ = close.update(cx, |view, cx| {
                        view.picking_for = None;
                        cx.notify();
                    });
                },
            ),
        )
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
            incoming, outgoing, ..
        } = &self.friends;
        if incoming.is_empty() && outgoing.is_empty() {
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
                    .detail("Request sent")
                    .action(self.remove_button(
                        ("withdraw-friend", IconName::Close, "Cancel request"),
                        person,
                        cx,
                    ))
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
        };
        let row = PersonRow::new(found.username.clone());
        match found.relation {
            Relation::None => row.action(self.add_button(&person, cx)),
            Relation::RequestReceived => row
                .detail("Wants to be friends")
                .action(self.accept_button("accept-found", &person, cx)),
            Relation::RequestSent => row.detail("Request sent"),
            Relation::Friends => row.detail("Friends"),
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

    /// Accepts a request they sent.
    fn accept_button(&self, id: &str, person: &UserRef, cx: &mut Context<Self>) -> Button {
        let user_id = person.id.clone();
        Button::new(SharedString::from(format!("{id}-{user_id}")))
            .success()
            .small()
            .icon(IconName::UserCheck)
            .label("Accept")
            .on_click(cx.listener(move |view, _, _, cx| view.accept_friend(&user_id, cx)))
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
        let boards = LeaderBoard::ALL
            .into_iter()
            .map(|board| {
                let rows = leaderboard(
                    &self.friends.friends,
                    (&name, &self.friends.my_ratings),
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
        div()
            .id("social-page")
            .size_full()
            .overflow_y_scroll()
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

fn seconds(duration: Duration) -> u32 {
    u32::try_from(duration.as_secs()).unwrap_or(u32::MAX)
}

#[cfg(test)]
mod tests {
    use std::{
        rc::Rc,
        sync::{Arc, Mutex},
    };

    use axum::{
        Json, Router,
        extract::{
            Path,
            ws::{Message, WebSocket, WebSocketUpgrade},
        },
        http::StatusCode,
        routing::{get, post},
    };
    use gpui_kit::{AppContext as _, Entity, TestAppContext, VisualTestContext};
    use serde_json::{Value, json};

    use super::{OpenMessages, SocialStage, SocialView};
    use crate::{
        clients::server::ServerClient,
        enums::sound::Sound,
        models::account::{Session, User},
        testing::{click, wait_until},
        views::{
            chat::ChatsView,
            live::{LiveConnection, SessionSource},
            sound::{played_sounds, record_sounds},
        },
    };

    /// What the fake server was asked, in order.
    type Asked = Arc<Mutex<Vec<String>>>;

    /// Ann has Bob online and a request from Cat; Bob challenges her, and accepting is recorded.
    fn server(asked: Asked) -> ServerClient {
        let accepted = Arc::clone(&asked);
        let socket_asked = Arc::clone(&asked);
        ServerClient::serving(
            Router::new()
                .route(
                    "/api/social/friends",
                    get(|| async {
                        Json(json!({
                            "friends": [{ "id": "bob", "username": "bob", "online": true }],
                            "incoming": [{ "id": "cat", "username": "cat" }],
                            "outgoing": []
                        }))
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
                        upgrade.on_upgrade(move |socket| bob_challenges(socket, socket_asked))
                    }),
                ),
        )
    }

    async fn bob_challenges(mut socket: WebSocket, asked: Asked) {
        let challenge = json!({ "type": "challenges", "sent": [], "received": [{
            "id": "c1",
            "from": { "id": "bob", "username": "bob" },
            "to": { "id": "ann", "username": "ann" },
            "initial_s": 300, "increment_s": 3, "rated": true
        }]});
        if socket
            .send(Message::Text(challenge.to_string().into()))
            .await
            .is_err()
        {
            return;
        }
        while let Some(Ok(Message::Text(text))) = socket.recv().await {
            let message: Value = serde_json::from_str(text.as_str()).expect("json");
            let kind = message["type"].as_str().unwrap_or_default().to_owned();
            let id = message["challenge_id"]
                .as_str()
                .unwrap_or_default()
                .to_owned();
            asked.lock().expect("the list").push(format!("{kind} {id}"));
        }
    }

    fn signed_in(guest: bool) -> SessionSource {
        let session = Session {
            token: "abc".to_owned(),
            user: User {
                id: "ann".to_owned(),
                email: "ann@example.com".to_owned(),
                username: "ann".to_owned(),
                guest,
            },
        };
        Rc::new(move |_| Some(session.clone()))
    }

    fn social(
        cx: &mut TestAppContext,
        guest: bool,
    ) -> (Entity<SocialView>, Asked, &mut VisualTestContext) {
        cx.executor().allow_parking();
        cx.update(|cx| {
            gpui_kit::init(cx);
            record_sounds(cx);
        });
        let asked = Asked::default();
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

    fn asked_for(asked: &Asked, what: &str) -> bool {
        asked
            .lock()
            .expect("the list")
            .iter()
            .any(|line| line == what)
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
        wait_until(cx, |_| asked_for(&asked, "accept cat"));
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
        wait_until(cx, |_| asked_for(&asked, "accept_challenge c1"));
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
}
