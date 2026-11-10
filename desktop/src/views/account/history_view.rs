use std::{future::Future, rc::Rc};

use gpui_kit::*;

use crate::{
    clients::server::ServerClient,
    components::account::{HistoryPanel, OnOpen, attempt_row, battle_row, game_row},
    enums::account::HistoryTab,
    models::{
        account::AccountProblem,
        play::{AnalyzeGame, GameRecord, PlayedGame},
        puzzles::{PlayedAttempt, PlayedBattle},
        server::ServerError,
    },
    services::{account::problem_text, play::opened_game},
};

/// How many rows the server sends a page; a shorter page is the last.
const PAGE: usize = 10;

/// One tab's rows so far, whether older ones are left, and its page on the way.
struct Listed<T> {
    rows: Vec<T>,
    more: bool,
    /// Fetched at least once since the page was opened.
    fetched: bool,
    /// A page is on its way; another tab's fetch leaves it be.
    loading: bool,
    _fetch: Option<Task<()>>,
}

impl<T> Default for Listed<T> {
    fn default() -> Self {
        Self {
            rows: Vec::new(),
            more: false,
            fetched: false,
            loading: false,
            _fetch: None,
        }
    }
}

impl<T> Listed<T> {
    /// A page is on its way; a newer one for this tab replaces it.
    fn start(&mut self, fetch: Task<()>) {
        self.loading = true;
        self._fetch = Some(fetch);
    }

    /// A page arrived: the newest replaces the rows, an older one joins them.
    fn add(&mut self, page: Vec<T>, newest: bool) {
        self.more = page.len() == PAGE;
        // Only the newest page makes the tab current; an older one leaves a failed refresh due.
        if newest {
            self.fetched = true;
            self.rows.clear();
        }
        self.rows.extend(page);
    }

    /// The page is opened again: what shows stays until the newest page replaces it.
    fn refresh(&mut self) {
        self.fetched = false;
    }
}

/// The Account page's History: games, battles and puzzles by the page; games open to review.
pub struct HistoryView {
    client: ServerClient,
    /// The signed-in token the history was fetched with; none once signed out.
    token: Option<String>,
    tab: HistoryTab,
    games: Listed<PlayedGame>,
    battles: Listed<PlayedBattle>,
    attempts: Listed<PlayedAttempt>,
    /// What went wrong, and on which tab, so it shows only there.
    problem: Option<(HistoryTab, AccountProblem)>,
    /// The game picked, on its way; a newer pick or signing out drops it.
    _opening: Option<Task<()>>,
}

impl EventEmitter<AnalyzeGame> for HistoryView {}

impl HistoryView {
    pub fn new(client: ServerClient) -> Self {
        Self {
            client,
            token: None,
            tab: HistoryTab::default(),
            games: Listed::default(),
            battles: Listed::default(),
            attempts: Listed::default(),
            problem: None,
            _opening: None,
        }
    }

    /// The page came on screen: each tab is fetched afresh when shown, this one now.
    pub fn open(&mut self, token: String, cx: &mut Context<Self>) {
        self.token = Some(token);
        self.problem = None;
        self.games.refresh();
        self.battles.refresh();
        self.attempts.refresh();
        self.fetch(None, cx);
    }

    /// Signed out: nothing of the last account is kept.
    pub fn clear(&mut self) {
        *self = Self::new(self.client.clone());
    }

    fn pick_tab(&mut self, tab: HistoryTab, cx: &mut Context<Self>) {
        self.tab = tab;
        if !self.fetched(tab) {
            self.fetch(None, cx);
        }
        cx.notify();
    }

    fn loading(&self, tab: HistoryTab) -> bool {
        match tab {
            HistoryTab::Games => self.games.loading,
            HistoryTab::Battles => self.battles.loading,
            HistoryTab::Puzzles => self.attempts.loading,
        }
    }

    fn fetched(&self, tab: HistoryTab) -> bool {
        match tab {
            HistoryTab::Games => self.games.fetched,
            HistoryTab::Battles => self.battles.fetched,
            HistoryTab::Puzzles => self.attempts.fetched,
        }
    }

    /// The oldest row shown on the tab, which the next page comes before.
    fn last_id(&self) -> Option<String> {
        match self.tab {
            HistoryTab::Games => self.games.rows.last().map(|game| game.id.clone()),
            HistoryTab::Battles => self.battles.rows.last().map(|battle| battle.id.clone()),
            HistoryTab::Puzzles => self.attempts.rows.last().map(|attempt| attempt.id.clone()),
        }
    }

    fn load_more(&mut self, cx: &mut Context<Self>) {
        let before = self.last_id();
        self.fetch(before, cx);
    }

    /// Asks for the tab's page before `before`; a newer request replaces one on its way.
    fn fetch(&mut self, before: Option<String>, cx: &mut Context<Self>) {
        let Some(token) = self.token.clone() else {
            return;
        };
        if self
            .problem
            .as_ref()
            .is_some_and(|(tab, _)| *tab == self.tab)
        {
            self.problem = None;
        }
        let newest = before.is_none();
        let before = before.as_deref();
        match self.tab {
            HistoryTab::Games => {
                let request = self.client.games(&token, before);
                let fetch = fetch_page(
                    request,
                    (HistoryTab::Games, newest),
                    |view| &mut view.games,
                    cx,
                );
                self.games.start(fetch);
            }
            HistoryTab::Battles => {
                let request = self.client.battles(&token, before);
                let fetch = fetch_page(
                    request,
                    (HistoryTab::Battles, newest),
                    |view| &mut view.battles,
                    cx,
                );
                self.battles.start(fetch);
            }
            HistoryTab::Puzzles => {
                let request = self.client.attempts(&token, before);
                let fetch = fetch_page(
                    request,
                    (HistoryTab::Puzzles, newest),
                    |view| &mut view.attempts,
                    cx,
                );
                self.attempts.start(fetch);
            }
        }
        cx.notify();
    }

    /// A page for `tab`, whose list `list` picks out, arrived, or the server said why not.
    fn arrived<T>(
        &mut self,
        result: Result<Vec<T>, ServerError>,
        ((tab, newest), cx): ((HistoryTab, bool), &mut Context<Self>),
        list: fn(&mut Self) -> &mut Listed<T>,
    ) {
        list(self).loading = false;
        match result {
            Ok(page) => list(self).add(page, newest),
            Err(error) => self.problem = Some((tab, AccountProblem::Server(error))),
        }
        cx.notify();
    }

    /// Fetches the game's moves, then hands it to the analysis board.
    fn open_game(&mut self, id: String, review: bool, cx: &mut Context<Self>) {
        let Some(token) = self.token.clone() else {
            return;
        };
        let request = self.client.game(&token, &id);
        self._opening = Some(cx.spawn(async move |view, cx| {
            let result = request.await;
            // The view outlives the request unless the window is closing.
            let _ = view.update(cx, |view, cx| view.game_arrived(result, review, cx));
        }));
    }

    fn game_arrived(
        &mut self,
        result: Result<GameRecord, ServerError>,
        review: bool,
        cx: &mut Context<Self>,
    ) {
        let record = match result {
            Ok(record) => record,
            Err(error) => {
                self.problem = Some((HistoryTab::Games, AccountProblem::Server(error)));
                cx.notify();
                return;
            }
        };
        let Some((game, outcome)) = opened_game(&record) else {
            self.problem = Some((HistoryTab::Games, AccountProblem::GameUnreadable));
            cx.notify();
            return;
        };
        let viewing = game.moves().len();
        cx.emit(AnalyzeGame {
            game,
            viewing,
            review,
            outcome,
        });
    }

    /// The chosen tab's rows, and whether older ones are left.
    fn rows(&self, cx: &mut Context<Self>) -> (Vec<AnyElement>, bool) {
        match self.tab {
            HistoryTab::Games => {
                let view = cx.entity().downgrade();
                let on_open: OnOpen = Rc::new(move |id, review, _, cx| {
                    // The view outlives its buttons unless the window is closing.
                    let _ = view.update(cx, |view, cx| view.open_game(id, review, cx));
                });
                let rows = self
                    .games
                    .rows
                    .iter()
                    .map(|game| game_row(game, Rc::clone(&on_open), cx).into_any_element())
                    .collect();
                (rows, self.games.more)
            }
            HistoryTab::Battles => {
                let rows = self
                    .battles
                    .rows
                    .iter()
                    .map(|battle| battle_row(battle, cx).into_any_element())
                    .collect();
                (rows, self.battles.more)
            }
            HistoryTab::Puzzles => {
                let rows = self
                    .attempts
                    .rows
                    .iter()
                    .map(|attempt| attempt_row(attempt, cx).into_any_element())
                    .collect();
                (rows, self.attempts.more)
            }
        }
    }
}

/// Waits for a page of `tab`, newest or older, then adds it to the list `list` picks out.
fn fetch_page<T: 'static>(
    request: impl Future<Output = Result<Vec<T>, ServerError>> + 'static,
    page: (HistoryTab, bool),
    list: fn(&mut HistoryView) -> &mut Listed<T>,
    cx: &mut Context<HistoryView>,
) -> Task<()> {
    cx.spawn(async move |view, cx| {
        let result = request.await;
        // The view outlives the request unless the window is closing.
        let _ = view.update(cx, |view, cx| view.arrived(result, (page, cx), list));
    })
}

impl Render for HistoryView {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let (rows, more) = self.rows(cx);
        let view = cx.entity().downgrade();
        let pick = view.clone();
        let loading = self.loading(self.tab);
        let problem = self
            .problem
            .clone()
            .filter(|(tab, _)| *tab == self.tab)
            .map(|(_, problem)| problem_text(&problem));
        HistoryPanel::new(
            (self.tab, rows),
            (more, loading, problem),
            move |tab, _, cx| {
                // The view outlives its buttons unless the window is closing.
                let _ = pick.update(cx, |view, cx| view.pick_tab(tab, cx));
            },
            move |_, cx| {
                // The view outlives its buttons unless the window is closing.
                let _ = view.update(cx, |view, cx| view.load_more(cx));
            },
        )
    }
}

#[cfg(test)]
mod tests {
    use std::{cell::RefCell, rc::Rc};

    use axum::{Json, Router, routing::get};
    use gpui_kit::TestAppContext;
    use serde_json::{Value, json};

    use super::HistoryView;
    use crate::{
        clients::server::ServerClient,
        models::play::AnalyzeGame,
        testing::{click, wait_until},
    };

    fn game() -> Value {
        json!({
            "id": "g1", "color": "white", "opponent": "bob", "result": "lost",
            "outcome": "checkmate", "category": "blitz", "initial_s": 180, "increment_s": 2,
            "rated": true, "plies": 4, "ago_s": 3600, "rating": 1490, "rating_change": -10
        })
    }

    fn server() -> ServerClient {
        ServerClient::serving(
            Router::new()
                .route("/api/games", get(|| async { Json(json!([game()])) }))
                .route(
                    "/api/games/g1",
                    get(|| async {
                        let mut record = game();
                        record["moves"] = json!(["f2f3", "e7e5", "g2g4", "d8h4"]);
                        Json(record)
                    }),
                ),
        )
    }

    #[gpui_kit::test]
    fn a_game_from_the_history_opens_for_review(cx: &mut TestAppContext) {
        cx.executor().allow_parking();
        cx.update(gpui_kit::init);
        let (view, cx) = cx.add_window_view(|_, _| HistoryView::new(server()));
        let opened: Rc<RefCell<Option<AnalyzeGame>>> = Rc::default();
        let heard = Rc::clone(&opened);
        let _listening = cx.update(|_, cx| {
            cx.subscribe(&view, move |_, event: &AnalyzeGame, _| {
                *heard.borrow_mut() = Some(event.clone());
            })
        });
        view.update(cx, |view, cx| view.open("abc".to_owned(), cx));
        wait_until(cx, |cx| {
            view.read_with(cx, |view, _| !view.games.rows.is_empty())
        });
        assert!(
            !view.read_with(cx, |view, _| view.games.more),
            "a short page is the last"
        );
        view.update(cx, |view, cx| view.open_game("g1".to_owned(), true, cx));
        wait_until(cx, |_| opened.borrow().is_some());
        let event = opened.borrow().clone().expect("the game");
        assert!(event.review, "Review asks for Game Review");
        assert_eq!(event.game.moves().len(), 4);
        assert_eq!(event.viewing, 4, "it opens on the final position");
    }

    #[gpui_kit::test]
    fn each_tab_fetches_its_own_history_when_first_shown(cx: &mut TestAppContext) {
        cx.executor().allow_parking();
        cx.update(gpui_kit::init);
        let client = ServerClient::serving(
            Router::new()
                .route("/api/games", get(|| async { Json(json!([game()])) }))
                .route(
                    "/api/puzzles/battles",
                    get(|| async {
                        Json(json!([{
                            "id": "b1", "opponent": "bob", "my_score": 7, "their_score": 5,
                            "result": "won", "rated": true, "ago_s": 60,
                            "rating": 1512, "rating_change": 12
                        }]))
                    }),
                )
                .route("/api/puzzles/attempts", get(|| async { Json(json!([])) })),
        );
        let (view, cx) = cx.add_window_view(|_, _| HistoryView::new(client));
        view.update(cx, |view, cx| view.open("abc".to_owned(), cx));
        wait_until(cx, |cx| view.read_with(cx, |view, _| view.games.fetched));
        assert!(
            !view.read_with(cx, |view, _| view.battles.fetched),
            "not until shown"
        );
        click(cx, "history-tab-battles");
        wait_until(cx, |cx| view.read_with(cx, |view, _| view.battles.fetched));
        let battle = view.read_with(cx, |view, _| view.battles.rows[0].clone());
        assert_eq!((battle.my_score, battle.their_score), (7, 5));
        click(cx, "history-tab-puzzles");
        wait_until(cx, |cx| view.read_with(cx, |view, _| view.attempts.fetched));
    }
}
