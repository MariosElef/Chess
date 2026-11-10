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
    enums::sound::Sound,
    i18n::tr,
    models::play::{Challenge, ClientMessage, LiveEvent, ServerMessage},
    services::{
        play::{COUNTDOWN_TICK, Countdown, challenge_terms},
        social::face_of,
    },
    views::{live::LiveConnection, sound::play_sound, ticker::start_ticker},
};

/// A rematch the opponent asked for, when it was posted, and why accepting it failed, if it did.
#[derive(Clone)]
struct Offer {
    challenge: Challenge,
    /// The minute it waits, from when the server posted it.
    countdown: Countdown,
    problem: Option<String>,
    /// The rematch left the lists after its refusal: only the reason and the X are left.
    gone: bool,
}

impl Offer {
    fn new(challenge: &Challenge) -> Self {
        Self {
            challenge: challenge.clone(),
            countdown: Countdown::from_server(challenge.wait_s, challenge.seconds_left),
            problem: None,
            gone: false,
        }
    }

    fn is_over(&self) -> bool {
        self.countdown.is_over()
    }
}

/// The rematch an opponent asks for, as a toast over whatever page is showing.
pub struct RematchToast {
    connection: Entity<LiveConnection>,
    _live: Subscription,
    offer: Option<Offer>,
    /// The rematches answered or put away here, hidden even while still listed.
    answered: HashSet<String>,
    /// The rematches that rang, so one seen again, as after a reconnect, rings no more.
    rung: HashSet<String>,
    /// The rematch accepted here, until a game starts or its refusal brings it back.
    accepting: Option<Offer>,
    _ticker: Option<Task<()>>,
}

/// The toast came, went, or shows another rematch: the shell decides again whether to draw it.
pub struct ToastChanged;

impl EventEmitter<ToastChanged> for RematchToast {}

impl RematchToast {
    pub fn new(connection: Entity<LiveConnection>, cx: &mut Context<Self>) -> Self {
        let _live = cx.subscribe(&connection, |toast, _, event: &LiveEvent, cx| {
            match event {
                LiveEvent::Message(message) => toast.on_message(message, cx),
                // Gone with the connection: the lists come again on reconnecting, if it does.
                LiveEvent::Disconnected | LiveEvent::SignedOut => toast.forget(cx),
                LiveEvent::Connected => {}
            }
        });
        Self {
            connection,
            _live,
            offer: None,
            answered: HashSet::new(),
            rung: HashSet::new(),
            accepting: None,
            _ticker: None,
        }
    }

    /// The game the rematch shown would replay, and whether it says why an accept was refused.
    pub fn shown(&self) -> Option<(&str, bool)> {
        let offer = self.offer.as_ref()?;
        let game_id = offer.challenge.rematch_of.as_deref()?;
        Some((game_id, offer.problem.is_some()))
    }

    fn on_message(&mut self, message: &ServerMessage, cx: &mut Context<Self>) {
        match message {
            ServerMessage::Challenges { received, .. } => self.follow(received, cx),
            ServerMessage::RematchRefused { game_id, message } => {
                self.refused(game_id, message, cx)
            }
            ServerMessage::Game { .. } => self.accepting = None,
            ServerMessage::Seeking { .. }
            | ServerMessage::SeekCancelled
            | ServerMessage::Error { .. }
            | ServerMessage::FriendsChanged
            | ServerMessage::ChatMessage { .. }
            | ServerMessage::ConversationsChanged
            | ServerMessage::ProfileChanged
            | ServerMessage::BattleSeeking
            | ServerMessage::BattleSeekCancelled
            | ServerMessage::Battle { .. }
            | ServerMessage::BattleChallenges { .. }
            | ServerMessage::InviteRefused { .. } => {}
        }
    }

    /// The newest rematch waiting for the player, as the server now has it: a new one rings.
    fn follow(&mut self, received: &[Challenge], cx: &mut Context<Self>) {
        let still_there = |offer: &Offer| received.iter().any(|new| new.id == offer.challenge.id);
        if !self.accepting.as_ref().is_some_and(still_there) {
            self.accepting = None;
        }
        // Only rematches still listed need hiding, or remembering as rung.
        let listed = |id: &String| received.iter().any(|challenge| &challenge.id == id);
        self.answered.retain(listed);
        self.rung.retain(listed);
        let open: Vec<&Challenge> = received
            .iter()
            .filter(|challenge| challenge.is_rematch() && !self.answered.contains(&challenge.id))
            .collect();
        // A new rematch rings once.
        let mut new_one = false;
        for challenge in &open {
            new_one |= self.rung.insert(challenge.id.clone());
        }
        if new_one {
            play_sound(Sound::Notify, cx);
        }
        let waiting = open.into_iter().next_back();
        // A refusal stays to be read, though its rematch has gone, until its bar runs out.
        let refused = |offer: &&mut Offer| waiting.is_none() && offer.problem.is_some();
        if let Some(offer) = self.offer.as_mut().filter(refused) {
            offer.gone = true;
            return cx.notify();
        }
        // The same one, listed afresh: its sender's picture or name may have changed.
        if let Some(offer) = self.offer.as_mut()
            && let Some(new) = waiting.filter(|new| new.id == offer.challenge.id)
        {
            offer.challenge = new.clone();
            return cx.notify();
        }
        let offer = waiting.map(Offer::new).filter(|offer| !offer.is_over());
        self.show(offer, cx);
    }

    /// Accepting the rematch here was refused: the toast comes back saying why.
    fn refused(&mut self, game_id: &str, message: &str, cx: &mut Context<Self>) {
        let of_game = |offer: &Offer| offer.challenge.rematch_of.as_deref() == Some(game_id);
        let Some(mut offer) = self.accepting.take().filter(of_game) else {
            return;
        };
        offer.problem = Some(message.to_owned());
        // Back in play: the next list must not hide it as answered.
        self.answered.remove(&offer.challenge.id);
        self.show(Some(offer), cx);
    }

    /// Shows `offer`, or nothing; the shell hears only when what it shows changes, not each tick.
    fn show(&mut self, offer: Option<Offer>, cx: &mut Context<Self>) {
        let id = |offer: Option<&Offer>| offer.map(|offer| offer.challenge.id.clone());
        let changed = id(self.offer.as_ref()) != id(offer.as_ref());
        self._ticker = offer
            .is_some()
            .then(|| start_ticker(COUNTDOWN_TICK, cx, RematchToast::tick));
        self.offer = offer;
        if changed {
            cx.emit(ToastChanged);
        }
        cx.notify();
    }

    /// The connection dropped: what was shown, answered or accepted here starts afresh.
    fn forget(&mut self, cx: &mut Context<Self>) {
        self.answered.clear();
        self.accepting = None;
        self.show(None, cx);
    }

    /// The bar moves on; once the wait is over, the toast goes.
    fn tick(&mut self, cx: &mut Context<Self>) -> bool {
        let over = self.offer.as_ref().is_none_or(Offer::is_over);
        if over {
            self.show(None, cx);
        } else {
            cx.notify();
        }
        !over
    }

    /// Accept goes by game, so a refusal comes as that game's; Decline turns the rematch down.
    fn answer(&mut self, accept: bool, cx: &mut Context<Self>) {
        let Some(offer) = self.offer.clone() else {
            return;
        };
        let challenge_id = offer.challenge.id.clone();
        self.answered.insert(challenge_id.clone());
        let message = match offer.challenge.rematch_of.clone() {
            Some(game_id) if accept => {
                self.accepting = Some(offer);
                ClientMessage::AcceptRematch { game_id }
            }
            Some(_) | None => ClientMessage::DeclineChallenge { challenge_id },
        };
        self.connection.read(cx).send(message);
        self.show(None, cx);
    }

    /// The X: the toast goes, and the rematch is left to the game's card or to run out.
    fn hide(&mut self, cx: &mut Context<Self>) {
        if let Some(offer) = &self.offer {
            self.answered.insert(offer.challenge.id.clone());
        }
        self.show(None, cx);
    }
}

impl Render for RematchToast {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let Some(offer) = &self.offer else {
            return div().into_any_element();
        };
        let from = &offer.challenge.from;
        let left = offer.countdown.left();
        let terms = challenge_terms(&offer.challenge);
        let dismissed = cx.entity().downgrade();
        let toast = Toast::new(
            "rematch-toast",
            tr!("shell.rematch.wants_rematch", name = from.username),
        )
        .person(face_of(&from.id, &from.username, from.avatar.as_deref()))
        .description(tr!("shell.rematch.terms", terms = terms))
        .problem(offer.problem.clone())
        .time_left(Some(left))
        .on_dismiss(move |_, cx| {
            // The toast outlives its buttons unless the window is closing.
            let _ = dismissed.update(cx, |toast, cx| toast.hide(cx));
        });
        if offer.gone {
            return toast.into_any_element();
        }
        toast
            .action(
                StartButton::new("rematch-accept", tr!("common.accept"), {
                    let toast = cx.entity().downgrade();
                    move |_, cx| {
                        // The toast outlives its buttons unless the window is closing.
                        let _ = toast.update(cx, |toast, cx| toast.answer(true, cx));
                    }
                })
                .small(),
            )
            .action(
                Button::new("rematch-decline")
                    .ghost()
                    .small()
                    .label(tr!("common.decline"))
                    .on_click(cx.listener(|toast, _, _, cx| toast.answer(false, cx))),
            )
            .into_any_element()
    }
}

#[cfg(test)]
mod tests {
    use std::{rc::Rc, sync::Arc};

    use axum::{Router, extract::ws::WebSocketUpgrade, routing::get};
    use gpui_kit::{AppContext as _, TestAppContext};
    use serde_json::{Value, json};

    use super::RematchToast;
    use crate::{
        clients::server::ServerClient,
        enums::sound::Sound,
        models::play::{Challenge, ServerMessage},
        testing::{Heard, ann, record_socket, wait_until, was_heard},
        views::{
            live::LiveConnection,
            sound::{played_sounds, record_sounds},
        },
    };

    /// Bob's rematch of `g1`, waiting for Ann as she connects.
    fn bob_asked_lists() -> Value {
        json!({ "type": "challenges", "sent": [], "received": [{
            "id": "r1",
            "from": { "id": "bob", "username": "bob" },
            "to": { "id": "ann", "username": "ann" },
            "initial_s": 180, "increment_s": 2, "rated": true, "rematch_of": "g1",
            "wait_s": 60, "seconds_left": 60
        }]})
    }

    /// Ann's toast, on a server where Bob asks for a rematch of `g1` as soon as she connects.
    fn bob_asked(
        cx: &mut TestAppContext,
    ) -> (
        gpui_kit::Entity<RematchToast>,
        Heard,
        &mut gpui_kit::VisualTestContext,
    ) {
        cx.executor().allow_parking();
        cx.update(|cx| {
            gpui_kit::init(cx);
            record_sounds(cx);
        });
        let asked = Heard::default();
        let socket_asked = Arc::clone(&asked);
        let client = ServerClient::serving(Router::new().route(
            "/api/ws",
            get(move |upgrade: WebSocketUpgrade| async move {
                upgrade.on_upgrade(move |socket| {
                    record_socket(socket, Some(bob_asked_lists()), socket_asked)
                })
            }),
        ));
        let session = ann(false);
        let (toast, cx) = cx.add_window_view(|_, cx| {
            let live =
                cx.new(|_| LiveConnection::new(client, Rc::new(move |_| Some(session.clone()))));
            live.update(cx, |live, cx| live.connect(cx));
            RematchToast::new(live, cx)
        });
        wait_until(cx, |cx| {
            toast.read_with(cx, |toast, _| toast.shown().is_some())
        });
        (toast, asked, cx)
    }

    /// Bob's rematch of `g1` with the id `id`, a full minute to go.
    fn offer(id: &str) -> Challenge {
        serde_json::from_value(json!({
            "id": id, "from": { "id": "bob", "username": "bob" },
            "to": { "id": "ann", "username": "ann" },
            "initial_s": 180, "increment_s": 2, "rated": true, "rematch_of": "g1",
            "wait_s": 60, "seconds_left": 60
        }))
        .expect("a challenge")
    }

    fn showing(
        toast: &gpui_kit::Entity<RematchToast>,
        cx: &mut gpui_kit::VisualTestContext,
    ) -> bool {
        toast.read_with(cx, |toast, _| toast.shown().is_some())
    }

    #[gpui_kit::test]
    fn a_rematch_rings_shows_and_is_declined(cx: &mut TestAppContext) {
        let (toast, asked, cx) = bob_asked(cx);
        assert!(
            cx.update(|_, cx| played_sounds(cx))
                .contains(&Sound::Notify)
        );
        toast.update(cx, |toast, cx| toast.answer(false, cx));
        assert!(!showing(&toast, cx), "it goes at once");
        wait_until(cx, |_| was_heard(&asked, "decline_challenge r1"));
    }

    #[gpui_kit::test]
    fn an_accept_refused_brings_the_toast_back_saying_why(cx: &mut TestAppContext) {
        let (toast, asked, cx) = bob_asked(cx);
        toast.update(cx, |toast, cx| toast.answer(true, cx));
        assert!(!showing(&toast, cx));
        wait_until(cx, |_| was_heard(&asked, "accept_rematch g1"));
        let refusal = ServerMessage::RematchRefused {
            game_id: "g1".to_owned(),
            message: "bob is playing a game".to_owned(),
        };
        toast.update(cx, |toast, cx| toast.on_message(&refusal, cx));
        toast.read_with(cx, |toast, _| {
            let offer = toast.offer.as_ref().expect("back");
            assert_eq!(offer.problem.as_deref(), Some("bob is playing a game"));
        });
    }

    #[gpui_kit::test]
    fn the_x_hides_the_toast_and_answers_nothing(cx: &mut TestAppContext) {
        let (toast, asked, cx) = bob_asked(cx);
        toast.update(cx, |toast, cx| toast.hide(cx));
        assert!(!showing(&toast, cx));
        cx.run_until_parked();
        assert!(
            !was_heard(&asked, "decline_challenge r1"),
            "left to the card or to run out"
        );
    }

    #[gpui_kit::test]
    fn a_dropped_connection_forgets_what_was_answered(cx: &mut TestAppContext) {
        let (toast, _asked, cx) = bob_asked(cx);
        toast.update(cx, |toast, cx| toast.hide(cx));
        toast.update(cx, |toast, cx| toast.forget(cx));
        toast.read_with(cx, |toast, _| {
            assert!(toast.answered.is_empty() && toast.accepting.is_none());
        });
    }

    #[gpui_kit::test]
    fn the_newest_rematch_shows_and_one_seen_again_rings_no_more(cx: &mut TestAppContext) {
        let (toast, _asked, cx) = bob_asked(cx);
        let rings = |cx: &mut gpui_kit::VisualTestContext| {
            cx.update(|_, cx| played_sounds(cx))
                .iter()
                .filter(|sound| **sound == Sound::Notify)
                .count()
        };
        let lists = |ids: &[&str]| ServerMessage::Challenges {
            sent: Vec::new(),
            received: ids.iter().map(|id| offer(id)).collect(),
        };
        toast.update(cx, |toast, cx| toast.on_message(&lists(&["r1", "r2"]), cx));
        assert_eq!(
            toast.read_with(cx, |toast, _| toast
                .offer
                .as_ref()
                .map(|offer| offer.challenge.id.clone())),
            Some("r2".to_owned())
        );
        let before = rings(cx);
        toast.update(cx, |toast, cx| toast.forget(cx));
        toast.update(cx, |toast, cx| toast.on_message(&lists(&["r1", "r2"]), cx));
        assert!(showing(&toast, cx), "back after reconnecting");
        assert_eq!(rings(cx), before, "it rang once already");
    }

    #[gpui_kit::test]
    fn a_refusal_stays_to_be_read_after_its_rematch_has_gone(cx: &mut TestAppContext) {
        let (toast, _asked, cx) = bob_asked(cx);
        toast.update(cx, |toast, cx| toast.answer(true, cx));
        let refusal = ServerMessage::RematchRefused {
            game_id: "g1".to_owned(),
            message: "The game could not start: try again".to_owned(),
        };
        toast.update(cx, |toast, cx| toast.on_message(&refusal, cx));
        let gone = ServerMessage::Challenges {
            sent: Vec::new(),
            received: Vec::new(),
        };
        toast.update(cx, |toast, cx| toast.on_message(&gone, cx));
        assert_eq!(
            toast.read_with(cx, |toast, _| toast
                .shown()
                .map(|(game, refused)| (game.to_owned(), refused))),
            Some(("g1".to_owned(), true))
        );
        assert!(
            toast.read_with(cx, |toast, _| toast
                .offer
                .as_ref()
                .is_some_and(|offer| offer.gone)),
            "only the reason and the X are left"
        );
    }

    #[gpui_kit::test]
    fn the_shell_hears_only_when_the_rematch_shown_changes(cx: &mut TestAppContext) {
        let (toast, _asked, cx) = bob_asked(cx);
        let changes = Rc::new(std::cell::Cell::new(0));
        let counted = Rc::clone(&changes);
        let _heard = cx.update(|_, cx| {
            cx.subscribe(&toast, move |_, _: &super::ToastChanged, _| {
                counted.set(counted.get() + 1)
            })
        });
        cx.executor()
            .advance_clock(std::time::Duration::from_secs(2));
        cx.run_until_parked();
        assert_eq!(changes.get(), 0, "the bar's ticks are the toast's alone");
        toast.update(cx, |toast, cx| toast.hide(cx));
        assert_eq!(changes.get(), 1);
    }

    #[gpui_kit::test]
    fn the_toast_goes_when_its_minute_runs_out(cx: &mut TestAppContext) {
        let (toast, _asked, cx) = bob_asked(cx);
        let changes = Rc::new(std::cell::Cell::new(0));
        let counted = Rc::clone(&changes);
        let _heard = cx.update(|_, cx| {
            cx.subscribe(&toast, move |_, _: &super::ToastChanged, _| {
                counted.set(counted.get() + 1)
            })
        });
        // Its wait is told by the real clock: the offer is made a minute older.
        toast.update(cx, |toast, cx| {
            let offer = toast.offer.as_mut().expect("shown");
            let countdown = &mut offer.countdown;
            countdown.began = countdown
                .began
                .checked_sub(countdown.wait)
                .expect("a minute ago");
            toast.tick(cx);
        });
        assert!(!showing(&toast, cx));
        assert_eq!(changes.get(), 1, "the shell hears it go");
    }

    #[gpui_kit::test]
    fn a_refusal_of_another_games_rematch_leaves_the_toast_be(cx: &mut TestAppContext) {
        let (toast, _asked, cx) = bob_asked(cx);
        toast.update(cx, |toast, cx| toast.answer(true, cx));
        let other = ServerMessage::RematchRefused {
            game_id: "g9".to_owned(),
            message: "no".to_owned(),
        };
        toast.update(cx, |toast, cx| toast.on_message(&other, cx));
        assert!(!showing(&toast, cx), "still waiting on g1's answer");
    }

    #[gpui_kit::test]
    fn each_rematch_put_away_stays_away_and_rings_once(cx: &mut TestAppContext) {
        let (toast, _asked, cx) = bob_asked(cx);
        let lists = || ServerMessage::Challenges {
            sent: Vec::new(),
            received: vec![offer("r1"), offer("r2")],
        };
        let shown = |cx: &mut gpui_kit::VisualTestContext| {
            toast.read_with(cx, |toast, _| {
                toast.offer.as_ref().map(|offer| offer.challenge.id.clone())
            })
        };
        toast.update(cx, |toast, cx| toast.on_message(&lists(), cx));
        assert_eq!(shown(cx).as_deref(), Some("r2"), "the newest");
        let rings = |cx: &mut gpui_kit::VisualTestContext| {
            cx.update(|_, cx| played_sounds(cx))
                .iter()
                .filter(|sound| **sound == Sound::Notify)
                .count()
        };
        toast.update(cx, |toast, cx| toast.hide(cx));
        toast.update(cx, |toast, cx| toast.on_message(&lists(), cx));
        assert_eq!(
            shown(cx).as_deref(),
            Some("r1"),
            "the other, not the one put away"
        );
        let before = rings(cx);
        toast.update(cx, |toast, cx| toast.hide(cx));
        toast.update(cx, |toast, cx| toast.on_message(&lists(), cx));
        assert_eq!(shown(cx), None, "both put away");
        assert_eq!(rings(cx), before);
    }
}
