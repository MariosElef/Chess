use std::collections::HashSet;

use gpui_kit::{
    component::{
        Sizable as _,
        button::{Button, ButtonVariants},
    },
    *,
};

use crate::{
    components::{panel::StartButton, shell::Toast},
    models::{
        play::{Challenge, ClientMessage, LiveEvent, ServerMessage},
        puzzles::BattleChallenge,
        social::UserRef,
    },
    services::{
        play::{COUNTDOWN_TICK, Countdown, challenge_terms, friend_challenges},
        social::face_of,
    },
    views::{live::LiveConnection, ticker::start_ticker},
};

/// An invitation put to the player: a friend's game, or a Puzzle Battle.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Invite {
    Game(Challenge),
    Battle(BattleChallenge),
}

/// Which kind of invitation the toast shows, so the page that lists that kind can hide it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InviteKind {
    Game,
    Battle,
}

impl Invite {
    fn id(&self) -> &str {
        match self {
            Invite::Game(challenge) => &challenge.id,
            Invite::Battle(challenge) => &challenge.id,
        }
    }

    fn from(&self) -> &UserRef {
        match self {
            Invite::Game(challenge) => &challenge.from,
            Invite::Battle(challenge) => &challenge.from,
        }
    }

    fn kind(&self) -> InviteKind {
        match self {
            Invite::Game(_) => InviteKind::Game,
            Invite::Battle(_) => InviteKind::Battle,
        }
    }

    /// The toast's title: who asks, and for what.
    fn title(&self) -> String {
        let name = &self.from().username;
        match self {
            Invite::Game(_) => format!("{name} challenges you"),
            Invite::Battle(_) => format!("{name} invites you to a Puzzle Battle"),
        }
    }

    /// The terms under the title: the clock and rating for a game, the rating for a battle.
    fn terms(&self) -> String {
        match self {
            Invite::Game(challenge) => challenge_terms(challenge),
            Invite::Battle(challenge) if challenge.rated => "Rated".to_owned(),
            Invite::Battle(_) => "Casual".to_owned(),
        }
    }

    /// The minute a friend's challenge waits; a battle invitation waits without a clock.
    fn countdown(&self) -> Option<Countdown> {
        match self {
            Invite::Game(challenge) => Some(Countdown::from_server(
                challenge.wait_s,
                challenge.seconds_left,
            )),
            Invite::Battle(_) => None,
        }
    }

    fn answer(&self, accept: bool) -> ClientMessage {
        let challenge_id = self.id().to_owned();
        match (self, accept) {
            (Invite::Game(_), true) => ClientMessage::AcceptChallenge { challenge_id },
            (Invite::Game(_), false) => ClientMessage::DeclineChallenge { challenge_id },
            (Invite::Battle(_), true) => ClientMessage::AcceptBattle { challenge_id },
            (Invite::Battle(_), false) => ClientMessage::DeclineBattle { challenge_id },
        }
    }
}

/// The invitation on show, the time it has left, and why accepting it failed, if it did.
struct Shown {
    invite: Invite,
    countdown: Option<Countdown>,
    problem: Option<String>,
    /// It left the lists after its refusal: only the reason and the X are left.
    gone: bool,
}

/// Friends' challenges and battle invitations, newest first, as a toast over any page.
pub struct InviteToast {
    connection: Entity<LiveConnection>,
    _live: Subscription,
    /// The friends' challenges waiting, as the server last listed them; rematches have their own.
    games: Vec<Challenge>,
    /// The battle invitations waiting, as the server last listed them.
    battles: Vec<BattleChallenge>,
    /// Every invitation listed, in the order it came, so the newest shows.
    arrived: Vec<String>,
    /// The invitations answered or put away here, hidden while still listed.
    answered: HashSet<String>,
    shown: Option<Shown>,
    /// The invitation accepted here, until its game or battle starts or its refusal comes.
    accepting: Option<Invite>,
    _ticker: Option<Task<()>>,
}

/// The invitation shown came, went, or changed: the shell decides again whether to draw it.
pub struct InviteChanged;

impl EventEmitter<InviteChanged> for InviteToast {}

impl InviteToast {
    pub fn new(connection: Entity<LiveConnection>, cx: &mut Context<Self>) -> Self {
        let _live = cx.subscribe(&connection, |toast, _, event: &LiveEvent, cx| {
            match event {
                LiveEvent::Message(message) => toast.on_message(message, cx),
                // The lists come again on reconnecting, and the toast starts afresh from them.
                LiveEvent::Disconnected => toast.forget(cx),
                LiveEvent::Connected => {}
            }
        });
        Self {
            connection,
            _live,
            games: Vec::new(),
            battles: Vec::new(),
            arrived: Vec::new(),
            answered: HashSet::new(),
            shown: None,
            accepting: None,
            _ticker: None,
        }
    }

    /// Which kind of invitation is on show, if any.
    pub fn shown_kind(&self) -> Option<InviteKind> {
        self.shown.as_ref().map(|shown| shown.invite.kind())
    }

    fn on_message(&mut self, message: &ServerMessage, cx: &mut Context<Self>) {
        match message {
            ServerMessage::Challenges { received, .. } => {
                self.games = friend_challenges(received);
                self.follow(cx);
            }
            ServerMessage::InviteRefused {
                challenge_id,
                message,
            } => self.refused(challenge_id, message, cx),
            ServerMessage::Game { .. } | ServerMessage::Battle { .. } => self.accepting = None,
            ServerMessage::BattleChallenges { received, .. } => {
                self.battles = received.clone();
                self.follow(cx);
            }
            ServerMessage::Seeking { .. }
            | ServerMessage::SeekCancelled
            | ServerMessage::Error { .. }
            | ServerMessage::FriendsChanged
            | ServerMessage::ChatMessage { .. }
            | ServerMessage::ConversationsChanged
            | ServerMessage::ProfileChanged
            | ServerMessage::RematchRefused { .. }
            | ServerMessage::BattleSeeking
            | ServerMessage::BattleSeekCancelled => {}
        }
    }

    /// Every invitation listed now, friends' challenges then battle invitations.
    fn listed(&self) -> Vec<Invite> {
        let games = self.games.iter().cloned().map(Invite::Game);
        let battles = self.battles.iter().cloned().map(Invite::Battle);
        games.chain(battles).collect()
    }

    /// Shows the newest invitation not answered here, as the server now lists them.
    fn follow(&mut self, cx: &mut Context<Self>) {
        let listed = self.listed();
        let is_listed = |id: &String| listed.iter().any(|invite| invite.id() == id);
        self.arrived.retain(is_listed);
        self.answered.retain(is_listed);
        for invite in &listed {
            if !self.arrived.iter().any(|id| id == invite.id()) {
                self.arrived.push(invite.id().to_owned());
            }
        }
        let newest = self
            .arrived
            .iter()
            .rev()
            .find(|id| !self.answered.contains(*id))
            .and_then(|id| listed.iter().find(|invite| invite.id() == id));
        // A refusal stays to be read, though its invitation has gone, until put away.
        if let Some(shown) = self.shown.as_mut()
            && shown.problem.is_some()
            && newest.is_none_or(|invite| invite.id() != shown.invite.id())
        {
            shown.gone = !is_listed(&shown.invite.id().to_owned());
            return cx.notify();
        }
        // The same one, listed afresh: its sender's picture or name may have changed.
        if let Some(shown) = self.shown.as_mut()
            && let Some(invite) = newest.filter(|invite| invite.id() == shown.invite.id())
        {
            shown.invite = invite.clone();
            return cx.notify();
        }
        let shown = newest.map(|invite| Shown {
            countdown: invite.countdown(),
            invite: invite.clone(),
            problem: None,
            gone: false,
        });
        self.show(shown, cx);
    }

    /// Shows `shown`, or nothing; the shell hears only when the invitation shown changes.
    fn show(&mut self, shown: Option<Shown>, cx: &mut Context<Self>) {
        let id = |shown: Option<&Shown>| shown.map(|shown| shown.invite.id().to_owned());
        let changed = id(self.shown.as_ref()) != id(shown.as_ref());
        self._ticker = shown
            .as_ref()
            .and_then(|shown| shown.countdown)
            .map(|_| start_ticker(COUNTDOWN_TICK, cx, InviteToast::tick));
        self.shown = shown;
        if changed {
            cx.emit(InviteChanged);
        }
        cx.notify();
    }

    /// The bar moves on; once a challenge's minute is over, the next invitation shows.
    fn tick(&mut self, cx: &mut Context<Self>) -> bool {
        let over = self
            .shown
            .as_ref()
            .and_then(|shown| shown.countdown)
            .is_none_or(|countdown| countdown.is_over());
        if over {
            self.put_away(cx);
        } else {
            cx.notify();
        }
        !over
    }

    /// The connection dropped: the lists come again, and the toast starts afresh from them.
    fn forget(&mut self, cx: &mut Context<Self>) {
        self.accepting = None;
        self.games.clear();
        self.battles.clear();
        self.arrived.clear();
        self.answered.clear();
        self.show(None, cx);
    }

    /// Accepts or declines the invitation shown; the next one, if any, takes its place.
    fn answer(&mut self, accept: bool, cx: &mut Context<Self>) {
        let Some(shown) = &self.shown else {
            return;
        };
        let message = shown.invite.answer(accept);
        if accept {
            self.accepting = Some(shown.invite.clone());
        }
        self.connection.read(cx).send(message);
        self.put_away(cx);
    }

    /// Accepting here was refused: the invitation comes back saying why.
    fn refused(&mut self, challenge_id: &str, message: &str, cx: &mut Context<Self>) {
        let Some(invite) = self
            .accepting
            .take()
            .filter(|invite| invite.id() == challenge_id)
        else {
            return;
        };
        self.answered.remove(challenge_id);
        let listed = self
            .listed()
            .iter()
            .any(|listed| listed.id() == challenge_id);
        let shown = Shown {
            countdown: invite.countdown(),
            invite,
            problem: Some(message.to_owned()),
            gone: !listed,
        };
        self.show(Some(shown), cx);
    }

    /// The invitation shown goes from the toast; it stays listed on its own page.
    fn put_away(&mut self, cx: &mut Context<Self>) {
        if let Some(shown) = &self.shown {
            self.answered.insert(shown.invite.id().to_owned());
        }
        self.follow(cx);
    }
}

impl Render for InviteToast {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let Some(shown) = &self.shown else {
            return div().into_any_element();
        };
        let from = shown.invite.from();
        let dismissed = cx.entity().downgrade();
        let toast = Toast::new("invite-toast", shown.invite.title())
            .person(face_of(&from.id, &from.username, from.avatar.as_deref()))
            .description(shown.invite.terms())
            .problem(shown.problem.clone())
            .time_left(shown.countdown.map(|countdown| countdown.left()))
            .on_dismiss(move |_, cx| {
                // The toast outlives its buttons unless the window is closing.
                let _ = dismissed.update(cx, |toast, cx| toast.put_away(cx));
            });
        if shown.gone {
            return toast.into_any_element();
        }
        toast
            .action(
                StartButton::new("invite-accept", "Accept", {
                    let toast = cx.entity().downgrade();
                    move |_, cx| {
                        // The toast outlives its buttons unless the window is closing.
                        let _ = toast.update(cx, |toast, cx| toast.answer(true, cx));
                    }
                })
                .small(),
            )
            .action(
                Button::new("invite-decline")
                    .ghost()
                    .small()
                    .label("Decline")
                    .on_click(cx.listener(|toast, _, _, cx| toast.answer(false, cx))),
            )
            .into_any_element()
    }
}

#[cfg(test)]
mod tests {
    use std::{rc::Rc, sync::Arc};

    use axum::{Router, extract::ws::WebSocketUpgrade, routing::get};
    use gpui_kit::{AppContext as _, Entity, TestAppContext, VisualTestContext};
    use serde_json::json;

    use super::{InviteKind, InviteToast};
    use crate::{
        clients::server::ServerClient,
        models::play::{LiveEvent, ServerMessage},
        testing::{Heard, ann, challenge_json, record_socket, wait_until, was_heard},
        views::live::LiveConnection,
    };

    /// Ann's invite toast, connected to a server that only listens.
    fn toast(cx: &mut TestAppContext) -> (Entity<InviteToast>, Heard, &mut VisualTestContext) {
        cx.executor().allow_parking();
        cx.update(gpui_kit::init);
        let heard = Heard::default();
        let socket_heard = Arc::clone(&heard);
        let client = ServerClient::serving(Router::new().route(
            "/api/ws",
            get(move |upgrade: WebSocketUpgrade| async move {
                upgrade.on_upgrade(move |socket| record_socket(socket, None, socket_heard))
            }),
        ));
        let session = ann(false);
        let connected = Rc::new(std::cell::Cell::new(false));
        let told = Rc::clone(&connected);
        let (toast, cx) = cx.add_window_view(move |_, cx| {
            let live =
                cx.new(|_| LiveConnection::new(client, Rc::new(move |_| Some(session.clone()))));
            cx.subscribe(&live, move |_, _, event: &LiveEvent, _| {
                if matches!(event, LiveEvent::Connected) {
                    told.set(true);
                }
            })
            .detach();
            live.update(cx, |live, cx| live.connect(cx));
            InviteToast::new(live, cx)
        });
        wait_until(cx, |_| connected.get());
        (toast, heard, cx)
    }

    /// The server's lists of friends' challenges to Ann: `ids`, and a rematch she has too.
    fn challenges(ids: &[&str]) -> ServerMessage {
        let mut received: Vec<_> = ids
            .iter()
            .map(|id| challenge_json(id, "bob", None))
            .collect();
        received.push(challenge_json("r1", "bob", Some("g1")));
        serde_json::from_value(json!({ "type": "challenges", "sent": [], "received": received }))
            .expect("the lists")
    }

    /// The server's list of battle invitations to Ann.
    fn battles(ids: &[&str]) -> ServerMessage {
        let received: Vec<_> = ids
            .iter()
            .map(|id| {
                json!({
                    "id": id, "from": { "id": "cat", "username": "cat" },
                    "to": { "id": "ann", "username": "ann" }, "rated": true
                })
            })
            .collect();
        serde_json::from_value(
            json!({ "type": "battle_challenges", "sent": [], "received": received }),
        )
        .expect("the lists")
    }

    fn shown_id(toast: &Entity<InviteToast>, cx: &mut VisualTestContext) -> Option<String> {
        toast.read_with(cx, |toast, _| {
            toast
                .shown
                .as_ref()
                .map(|shown| shown.invite.id().to_owned())
        })
    }

    #[gpui_kit::test]
    fn a_friends_challenge_shows_and_is_accepted_but_a_rematch_is_left_to_its_own(
        cx: &mut TestAppContext,
    ) {
        let (toast, heard, cx) = toast(cx);
        toast.update(cx, |toast, cx| toast.on_message(&challenges(&["c1"]), cx));
        assert_eq!(shown_id(&toast, cx).as_deref(), Some("c1"));
        assert_eq!(
            toast.read_with(cx, |toast, _| toast.shown_kind()),
            Some(InviteKind::Game)
        );
        toast.update(cx, |toast, cx| toast.answer(true, cx));
        wait_until(cx, |_| was_heard(&heard, "accept_challenge c1"));
        assert_eq!(
            shown_id(&toast, cx),
            None,
            "the rematch r1 is not one of its own"
        );
    }

    #[gpui_kit::test]
    fn the_newest_invitation_shows_and_a_declined_battle_gives_way(cx: &mut TestAppContext) {
        let (toast, heard, cx) = toast(cx);
        toast.update(cx, |toast, cx| toast.on_message(&challenges(&["c1"]), cx));
        toast.update(cx, |toast, cx| toast.on_message(&battles(&["b1"]), cx));
        assert_eq!(shown_id(&toast, cx).as_deref(), Some("b1"), "the newest");
        toast.update(cx, |toast, cx| toast.answer(false, cx));
        wait_until(cx, |_| was_heard(&heard, "decline_battle b1"));
        assert_eq!(
            shown_id(&toast, cx).as_deref(),
            Some("c1"),
            "the one before it"
        );
    }

    #[gpui_kit::test]
    fn an_invitation_put_away_stays_away_until_the_connection_drops(cx: &mut TestAppContext) {
        let (toast, heard, cx) = toast(cx);
        toast.update(cx, |toast, cx| toast.on_message(&challenges(&["c1"]), cx));
        toast.update(cx, |toast, cx| toast.put_away(cx));
        toast.update(cx, |toast, cx| toast.on_message(&challenges(&["c1"]), cx));
        assert_eq!(shown_id(&toast, cx), None);
        cx.run_until_parked();
        assert!(
            !was_heard(&heard, "decline_challenge c1"),
            "the X answers nothing"
        );
        toast.update(cx, |toast, cx| toast.forget(cx));
        toast.update(cx, |toast, cx| toast.on_message(&challenges(&["c1"]), cx));
        assert_eq!(
            shown_id(&toast, cx).as_deref(),
            Some("c1"),
            "the lists start afresh"
        );
    }

    #[gpui_kit::test]
    fn an_accept_refused_brings_the_invitation_back_saying_why(cx: &mut TestAppContext) {
        let (toast, heard, cx) = toast(cx);
        toast.update(cx, |toast, cx| toast.on_message(&battles(&["b1"]), cx));
        toast.update(cx, |toast, cx| toast.answer(true, cx));
        wait_until(cx, |_| was_heard(&heard, "accept_battle b1"));
        let refusal = ServerMessage::InviteRefused {
            challenge_id: "b1".to_owned(),
            message: "One of you is already in a battle".to_owned(),
        };
        toast.update(cx, |toast, cx| toast.on_message(&refusal, cx));
        toast.read_with(cx, |toast, _| {
            let shown = toast.shown.as_ref().expect("back");
            assert_eq!(
                shown.problem.as_deref(),
                Some("One of you is already in a battle")
            );
            assert!(!shown.gone, "still listed: it can be accepted again");
        });
        toast.update(cx, |toast, cx| toast.on_message(&battles(&[]), cx));
        assert!(
            toast.read_with(cx, |toast, _| toast
                .shown
                .as_ref()
                .is_some_and(|shown| shown.gone)),
            "gone from the lists, the reason stays with only the X"
        );
    }

    #[gpui_kit::test]
    fn a_challenge_gives_way_when_its_minute_runs_out(cx: &mut TestAppContext) {
        let (toast, _heard, cx) = toast(cx);
        toast.update(cx, |toast, cx| toast.on_message(&challenges(&["c1"]), cx));
        // Its wait is told by the real clock: the challenge is made a minute older.
        toast.update(cx, |toast, cx| {
            let countdown = toast
                .shown
                .as_mut()
                .and_then(|shown| shown.countdown.as_mut())
                .expect("a minute's bar");
            countdown.began = countdown
                .began
                .checked_sub(countdown.wait)
                .expect("a minute ago");
            toast.tick(cx);
        });
        assert_eq!(shown_id(&toast, cx), None);
    }
}
