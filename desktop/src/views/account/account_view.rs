use gpui_kit::{
    component::{
        ActiveTheme as _,
        input::{InputEvent, InputState},
        label::Label,
        v_flex,
    },
    *,
};

use crate::{
    clients::{server::ServerClient, settings::SessionTokenFile},
    components::account::{
        AccountCard, FormField, GuestCard, SignedInCard, StatsPanel, WaitingCard,
    },
    enums::account::AccountForm,
    models::{
        account::{AccountChanged, FormProblems, Session, SignIn, SignUp, Stats, User},
        server::ServerError,
    },
    services::account::form_problems,
};

/// Signing in, creating an account, and signing out; it keeps the session for the whole app.
pub struct AccountView {
    client: ServerClient,
    tokens: SessionTokenFile,
    session: Option<Session>,
    form: AccountForm,
    login: Entity<InputState>,
    email: Entity<InputState>,
    username: Entity<InputState>,
    password: Entity<InputState>,
    problems: FormProblems,
    /// A request is on its way; the button shows it and a second press does nothing.
    busy: bool,
    /// The saved token is being checked at launch: a moment with no form to fill.
    restoring: bool,
    /// Enter in any field submits the form.
    _enter_presses: Vec<Subscription>,
    /// The signed-in player's ratings and results, once fetched.
    stats: Option<Stats>,
}

impl EventEmitter<AccountChanged> for AccountView {}

impl AccountView {
    /// Starts signed out, then signs back in with the saved token if there is one.
    pub fn new(
        client: ServerClient,
        tokens: SessionTokenFile,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let login = text_field("Email or username", false, window, cx);
        let email = text_field("you@example.com", false, window, cx);
        let username = text_field("3 to 20 letters, digits or _", false, window, cx);
        let password = text_field("At least 8 characters", true, window, cx);
        let _enter_presses = [&login, &email, &username, &password]
            .map(|input| cx.subscribe_in(input, window, submit_on_enter))
            .into();
        let mut view = Self {
            client,
            tokens,
            session: None,
            form: AccountForm::default(),
            login,
            email,
            username,
            password,
            problems: FormProblems::default(),
            busy: false,
            restoring: false,
            _enter_presses,
            stats: None,
        };
        view.restore(window, cx);
        view
    }

    /// Signed in, with an account or as a guest: either way the app opens.
    pub fn is_admitted(&self) -> bool {
        self.session.is_some()
    }

    /// The signed-in session: token and account, for live play.
    pub fn session(&self) -> Option<Session> {
        self.session.clone()
    }

    /// The token the server knows this app by, for requests made while signed in.
    pub fn token(&self) -> Option<String> {
        self.session.as_ref().map(|session| session.token.clone())
    }

    /// The name the sidebar shows: the username, Guest3 for a guest.
    pub fn player_name(&self) -> Option<String> {
        self.user().map(|user| user.username.clone())
    }

    /// The account signed in on this app, if any.
    pub fn user(&self) -> Option<&User> {
        self.session.as_ref().map(|session| &session.user)
    }

    /// Asks the server who the saved token belongs to. A token it refuses is forgotten; an
    /// unreachable server keeps it for next time.
    fn restore(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(token) = self.tokens.load() else {
            return;
        };
        self.busy = true;
        self.restoring = true;
        let request = self.client.me(&token);
        cx.spawn_in(window, async move |view, cx| {
            let result = request.await;
            // The view outlives the request unless the window is closing.
            let _ = view.update_in(cx, |view, _, cx| view.restored(token, result, cx));
        })
        .detach();
    }

    fn restored(
        &mut self,
        token: String,
        result: Result<User, ServerError>,
        cx: &mut Context<Self>,
    ) {
        self.busy = false;
        self.restoring = false;
        match result {
            Ok(user) => self.signed_in(Session { token, user }, cx),
            Err(ServerError::Refused(_)) => {
                // A failed delete only means the next launch asks again.
                let _ = self.tokens.forget();
            }
            Err(error) => self.problems = form_problems(&error),
        }
        cx.notify();
    }

    fn submit(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let request = self.session_request(cx);
        self.request_session(request, window, cx);
    }

    /// Takes the first free guest account; with all ten in use, the form says so.
    pub fn continue_as_guest(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let request = self.client.sign_in_as_guest();
        self.request_session(request, window, cx);
    }

    /// Sends a sign-in of some kind; its answer signs in or fills the form's problems.
    fn request_session(
        &mut self,
        request: impl std::future::Future<Output = Result<Session, ServerError>> + 'static,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.busy {
            return;
        }
        self.busy = true;
        self.problems = FormProblems::default();
        cx.spawn_in(window, async move |view, cx| {
            let result = request.await;
            let _ = view.update_in(cx, |view, window, cx| view.submitted(result, window, cx));
        })
        .detach();
        cx.notify();
    }

    /// Signs in; a new account is created first and then signed in with what was typed.
    fn session_request(
        &self,
        cx: &App,
    ) -> impl std::future::Future<Output = Result<Session, ServerError>> + use<> {
        let client = self.client.clone();
        let password = value(&self.password, cx);
        let sign_up = (self.form == AccountForm::SignUp).then(|| SignUp {
            email: value(&self.email, cx),
            username: value(&self.username, cx),
            password: password.clone(),
        });
        let login = match &sign_up {
            Some(sign_up) => sign_up.username.clone(),
            None => value(&self.login, cx),
        };
        async move {
            if let Some(sign_up) = sign_up {
                client.sign_up(&sign_up).await?;
            }
            client.sign_in(&SignIn { login, password }).await
        }
    }

    fn submitted(
        &mut self,
        result: Result<Session, ServerError>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.busy = false;
        match result {
            Ok(session) => {
                // A failed save only means the next launch starts signed out.
                let _ = self.tokens.save(&session.token);
                self.clear_fields(window, cx);
                self.signed_in(session, cx);
            }
            Err(error) => self.problems = form_problems(&error),
        }
        cx.notify();
    }

    fn signed_in(&mut self, session: Session, cx: &mut Context<Self>) {
        self.session = Some(session);
        self.problems = FormProblems::default();
        self.stats = None;
        self.opened(cx);
        cx.emit(AccountChanged);
    }

    /// The page came on screen: the stats are fetched afresh, as games and puzzles move them.
    pub fn opened(&mut self, cx: &mut Context<Self>) {
        let Some(token) = self.token() else {
            return;
        };
        let request = self.client.stats(&token);
        cx.spawn(async move |view, cx| {
            // Without them the page shows the account alone.
            if let Ok(stats) = request.await {
                let _ = view.update(cx, |view, cx| {
                    view.stats = Some(stats);
                    cx.notify();
                });
            }
        })
        .detach();
    }

    /// Signs out here at once; the server is told too, and if it cannot be reached the token
    /// simply expires there.
    pub fn sign_out(&mut self, cx: &mut Context<Self>) {
        let Some(session) = self.session.take() else {
            return;
        };
        self.stats = None;
        let _ = self.tokens.forget();
        let request = self.client.sign_out(&session.token);
        cx.spawn(async move |_, _| {
            // Signed out here already; the server's answer changes nothing.
            let _ = request.await;
        })
        .detach();
        self.form = AccountForm::SignIn;
        cx.emit(AccountChanged);
        cx.notify();
    }

    /// From a guest to the sign-in screen, on `form`: the guest is signed out, which frees it.
    fn leave_guest(&mut self, form: AccountForm, cx: &mut Context<Self>) {
        self.sign_out(cx);
        self.form = form;
    }

    fn switch_form(&mut self, cx: &mut Context<Self>) {
        self.form = self.form.other();
        self.problems = FormProblems::default();
        cx.notify();
    }

    fn clear_fields(&self, window: &mut Window, cx: &mut Context<Self>) {
        for input in [&self.login, &self.email, &self.username, &self.password] {
            input.update(cx, |input, cx| input.set_value("", window, cx));
        }
    }

    fn fields(&self) -> Vec<FormField> {
        let problems = self.problems.clone();
        match self.form {
            AccountForm::SignIn => vec![
                FormField::new("Email or username", self.login.clone(), problems.login),
                FormField::new("Password", self.password.clone(), problems.password).password(),
            ],
            AccountForm::SignUp => vec![
                FormField::new("Email", self.email.clone(), problems.email),
                FormField::new("Username", self.username.clone(), problems.username),
                FormField::new("Password", self.password.clone(), problems.password).password(),
            ],
        }
    }

    fn card(&self, cx: &mut Context<Self>) -> AnyElement {
        if self.restoring {
            return WaitingCard::new("Signing you in…").into_any_element();
        }
        let view = cx.entity().downgrade();
        if self.user().is_some_and(|user| user.guest) {
            let sign_up_view = view.clone();
            return GuestCard::new(
                move |_, cx| {
                    let _ = view.update(cx, |view, cx| view.leave_guest(AccountForm::SignIn, cx));
                },
                move |_, cx| {
                    let _ = sign_up_view
                        .update(cx, |view, cx| view.leave_guest(AccountForm::SignUp, cx));
                },
            )
            .into_any_element();
        }
        match &self.session {
            Some(session) => SignedInCard::new(
                session.user.username.clone(),
                session.user.email.clone(),
                move |_, cx| {
                    let _ = view.update(cx, |view, cx| view.sign_out(cx));
                },
            )
            .into_any_element(),
            None => {
                let (switch_view, guest_view) = (view.clone(), view.clone());
                AccountCard::new(
                    self.form,
                    self.fields(),
                    self.problems.form.clone(),
                    self.busy,
                )
                .on_submit(move |window, cx| {
                    let _ = view.update(cx, |view, cx| view.submit(window, cx));
                })
                .on_switch(move |_, cx| {
                    let _ = switch_view.update(cx, |view, cx| view.switch_form(cx));
                })
                .on_guest(move |window, cx| {
                    let _ = guest_view.update(cx, |view, cx| view.continue_as_guest(window, cx));
                })
                .into_any_element()
            }
        }
    }
}

impl AccountView {
    /// The Account page inside the app; before it opens, a welcome above the forms.
    fn heading(&self, cx: &App) -> Div {
        if self.is_admitted() {
            return v_flex().child(Label::new("Account").text_2xl());
        }
        v_flex()
            .items_center()
            .gap_1()
            .pt_12()
            .child(Label::new("Chess").text_3xl())
            .child(
                Label::new("Sign in, create an account, or play as a guest")
                    .text_color(cx.theme().muted_foreground),
            )
    }
}

impl Render for AccountView {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .size_full()
            .p_6()
            .gap_4()
            .child(self.heading(cx))
            .child(div().flex().justify_center().child(self.card(cx)))
            .children(
                self.stats
                    .clone()
                    .filter(|_| self.is_admitted())
                    .map(|stats| div().flex().justify_center().child(StatsPanel::new(stats))),
            )
    }
}

/// A one-line field; a password one hides what is typed.
fn text_field(
    placeholder: &'static str,
    masked: bool,
    window: &mut Window,
    cx: &mut Context<AccountView>,
) -> Entity<InputState> {
    cx.new(|cx| {
        InputState::new(window, cx)
            .placeholder(placeholder)
            .masked(masked)
    })
}

fn value(input: &Entity<InputState>, cx: &App) -> String {
    input.read(cx).value().to_string()
}

fn submit_on_enter(
    view: &mut AccountView,
    _: &Entity<InputState>,
    event: &InputEvent,
    window: &mut Window,
    cx: &mut Context<AccountView>,
) {
    if let InputEvent::PressEnter { .. } = event {
        view.submit(window, cx);
    }
}

#[cfg(test)]
mod tests {
    use std::{env, process, thread, time::Duration};

    use axum::{
        Json, Router,
        http::StatusCode,
        routing::{get, post},
    };
    use gpui_kit::{Entity, TestAppContext, VisualTestContext};
    use serde_json::{Value, json};

    use super::AccountView;
    use crate::{
        clients::{server::ServerClient, settings::SessionTokenFile},
        enums::account::AccountForm,
    };

    /// A new session's answer, as the server sends it.
    fn session(token: &str, username: &str, guest: bool) -> Value {
        json!({
            "token": token,
            "expires_at": "2026-12-08T00:00:00Z",
            "user": { "id": "1", "email": "m@example.com", "username": username, "guest": guest }
        })
    }

    /// A server that signs anyone in as Mario with the token `abc`, and hands out Guest3.
    fn fake_server() -> ServerClient {
        let sign_in = session("abc", "Mario", false);
        let guest = session("guest-token", "Guest3", true);
        ServerClient::serving(
            Router::new()
                .route(
                    "/api/sessions",
                    post(move || async move { (StatusCode::CREATED, Json(sign_in)) }),
                )
                .route(
                    "/api/sessions/guest",
                    post(move || async move { (StatusCode::CREATED, Json(guest)) }),
                )
                .route(
                    "/api/users/me/stats",
                    get(|| async {
                        let mode = |category: &str, rating: Option<i32>| {
                            json!({
                                "category": category, "rating": rating, "provisional": false,
                                "best": rating, "games": 3, "wins": 2, "losses": 1, "draws": 0
                            })
                        };
                        Json(json!({
                            "modes": [
                                mode("bullet", None), mode("blitz", Some(1521)),
                                mode("rapid", None), mode("classical", None)
                            ],
                            "puzzles": {
                                "rating": 1650, "provisional": true, "best": 1700,
                                "attempted": 4, "solved": 3,
                                "best_rush": { "three_minutes": 12, "five_minutes": null, "survival": null }
                            }
                        }))
                    }),
                )
                .route(
                    "/api/users/me",
                    get(|| async {
                        // A real server takes a moment: long enough to see the wait.
                        tokio::time::sleep(Duration::from_millis(300)).await;
                        Json(json!({
                            "id": "1", "email": "m@example.com",
                            "username": "Mario", "guest": false
                        }))
                    }),
                ),
        )
    }

    /// An account view on the fake server, with a token file of the test's own.
    fn opened<'a>(
        cx: &'a mut TestAppContext,
        name: &str,
    ) -> (
        Entity<AccountView>,
        SessionTokenFile,
        &'a mut VisualTestContext,
    ) {
        cx.executor().allow_parking();
        cx.update(gpui_kit::init);
        let path = env::temp_dir().join(format!("chess-{name}-{}", process::id()));
        let tokens = SessionTokenFile::at(path);
        let view_tokens = tokens.clone();
        let (view, cx) = cx
            .add_window_view(|window, cx| AccountView::new(fake_server(), view_tokens, window, cx));
        (view, tokens, cx)
    }

    /// The request runs on another thread: let the app run until its answer has arrived.
    fn wait_for(
        view: &Entity<AccountView>,
        cx: &mut VisualTestContext,
        done: impl Fn(&AccountView) -> bool,
    ) {
        for _ in 0..300 {
            cx.run_until_parked();
            if view.update(cx, |view, _| done(view)) {
                return;
            }
            thread::sleep(Duration::from_millis(10));
        }
        panic!("the request never finished");
    }

    #[gpui_kit::test]
    fn signing_in_shows_the_account_and_keeps_the_token(cx: &mut TestAppContext) {
        let (view, tokens, cx) = opened(cx, "account-sign-in");
        view.update_in(cx, |view, window, cx| {
            view.login
                .update(cx, |input, cx| input.set_value("mario", window, cx));
            view.password
                .update(cx, |input, cx| input.set_value("correct horse", window, cx));
            view.submit(window, cx);
        });
        wait_for(&view, cx, AccountView::is_admitted);
        let name = view.update(cx, |view, _| view.player_name());
        assert_eq!(name.as_deref(), Some("Mario"));
        assert_eq!(tokens.load().as_deref(), Some("abc"));
        wait_for(&view, cx, |view| view.stats.is_some());
        let (blitz, puzzles) = view.update(cx, |view, _| {
            let stats = view.stats.clone().expect("the stats");
            (stats.modes[1].rating, stats.puzzles.rating)
        });
        assert_eq!((blitz, puzzles), (Some(1521), Some(1650)));

        view.update(cx, |view, cx| view.sign_out(cx));
        assert!(
            view.update(cx, |view, _| view.stats.is_none()),
            "nor do their stats stay"
        );
        assert!(!view.update(cx, |view, _| view.is_admitted()));
        assert_eq!(tokens.load(), None, "signing out forgets the token");
    }

    #[gpui_kit::test]
    fn a_guest_gets_a_numbered_guest_and_frees_it_to_sign_up(cx: &mut TestAppContext) {
        let (view, tokens, cx) = opened(cx, "account-guest");
        view.update_in(cx, |view, window, cx| view.continue_as_guest(window, cx));
        wait_for(&view, cx, AccountView::is_admitted);
        let name = view.update(cx, |view, _| view.player_name());
        assert_eq!(name.as_deref(), Some("Guest3"));
        assert_eq!(tokens.load().as_deref(), Some("guest-token"));

        view.update(cx, |view, cx| view.leave_guest(AccountForm::SignUp, cx));
        let (admitted, form) = view.update(cx, |view, _| (view.is_admitted(), view.form));
        assert!(!admitted, "back to the account screen");
        assert_eq!(form, AccountForm::SignUp);
        assert_eq!(tokens.load(), None);
    }

    #[gpui_kit::test]
    fn a_saved_session_signs_back_in_without_showing_the_form(cx: &mut TestAppContext) {
        cx.executor().allow_parking();
        cx.update(gpui_kit::init);
        let path = env::temp_dir().join(format!("chess-account-restore-{}", process::id()));
        let tokens = SessionTokenFile::at(path);
        tokens.save("abc").expect("saved");
        let view_tokens = tokens.clone();
        let (view, cx) = cx
            .add_window_view(|window, cx| AccountView::new(fake_server(), view_tokens, window, cx));
        assert!(
            view.update(cx, |view, _| view.restoring),
            "a moment of \"Signing you in…\" first"
        );
        wait_for(&view, cx, AccountView::is_admitted);
        assert!(!view.update(cx, |view, _| view.restoring));
        assert_eq!(
            view.update(cx, |view, _| view.player_name()).as_deref(),
            Some("Mario")
        );
        tokens.forget().expect("cleaned up");
    }
}
