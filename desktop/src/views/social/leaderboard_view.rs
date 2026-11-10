use std::rc::Rc;

use gpui_kit::{
    component::{h_flex, v_flex},
    *,
};

use crate::{
    clients::server::ServerClient,
    components::{shell::LEADERBOARD_PICTURE, social::RankingPanel},
    enums::social::RankingBoard,
    i18n::tr,
    models::{
        account::Face,
        server::ServerError,
        social::{RankedPlayer, RankingPage},
    },
    services::social::{face_of, problem_text},
    views::{live::SessionSource, social::profile_view::ProfileView},
};

/// Everyone's ratings, a board per category: the Leaderboard page under Social.
pub struct LeaderboardView {
    client: ServerClient,
    session: SessionSource,
    board: RankingBoard,
    rows: Vec<RankedPlayer>,
    /// The player's own place, if they are on the board.
    me: Option<RankedPlayer>,
    more: bool,
    loading: bool,
    problem: Option<String>,
    /// The page on its way; choosing another board drops it.
    _fetch: Option<Task<()>>,
}

impl LeaderboardView {
    pub fn new(client: ServerClient, session: SessionSource) -> Self {
        Self {
            client,
            session,
            board: RankingBoard::default(),
            rows: Vec::new(),
            me: None,
            more: false,
            loading: false,
            problem: None,
            _fetch: None,
        }
    }

    /// The page came on screen: the board is fetched afresh.
    pub fn opened(&mut self, cx: &mut Context<Self>) {
        self.fetch(0, cx);
    }

    fn choose(&mut self, board: RankingBoard, cx: &mut Context<Self>) {
        if board == self.board {
            return;
        }
        self.board = board;
        self.rows.clear();
        self.me = None;
        self.more = false;
        self.fetch(0, cx);
    }

    fn load_more(&mut self, cx: &mut Context<Self>) {
        self.fetch(self.rows.len(), cx);
    }

    /// Asks for the board from place `offset` on: the first page replaces the list, later ones add.
    fn fetch(&mut self, offset: usize, cx: &mut Context<Self>) {
        // Signed out, the shell builds the pages afresh, so there is nothing to show meanwhile.
        let Some(token) = (self.session)(cx).map(|session| session.token) else {
            return;
        };
        self.loading = true;
        self.problem = None;
        let board = self.board;
        let request = self.client.leaderboard(&token, (board.key(), offset));
        self._fetch = Some(cx.spawn(async move |view, cx| {
            let result = request.await;
            // The view outlives the request unless its page is gone.
            let _ = view.update(cx, |view, cx| view.fetched(offset, result, cx));
        }));
        cx.notify();
    }

    fn fetched(
        &mut self,
        offset: usize,
        result: Result<RankingPage, ServerError>,
        cx: &mut Context<Self>,
    ) {
        self.loading = false;
        match result {
            Ok(page) => {
                if offset == 0 {
                    self.rows.clear();
                }
                // A rating moving between pages could list a player twice: they stay once.
                let fresh: Vec<RankedPlayer> = page
                    .rows
                    .into_iter()
                    .filter(|row| !self.rows.iter().any(|listed| listed.id == row.id))
                    .collect();
                self.rows.extend(fresh);
                self.me = page.me;
                self.more = page.more;
            }
            Err(error) => self.problem = Some(problem_text(&error)),
        }
        cx.notify();
    }
}

/// A place with how its player looks: their picture, or their initial.
fn with_face(player: &RankedPlayer) -> (RankedPlayer, Face) {
    let face = face_of(&player.id, &player.username, player.avatar.as_deref());
    (player.clone(), face)
}

impl Render for LeaderboardView {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let (chooser, more) = (cx.entity().downgrade(), cx.entity().downgrade());
        let panel = RankingPanel::new(
            self.board,
            move |board, _, cx| {
                // The page outlives its tabs unless the window is closing.
                let _ = chooser.update(cx, |view, cx| view.choose(board, cx));
            },
            move |_, cx| {
                let _ = more.update(cx, |view, cx| view.load_more(cx));
            },
        )
        .on_profile({
            let (client, session) = (self.client.clone(), Rc::clone(&self.session));
            move |player, window, cx| {
                ProfileView::open((client.clone(), Rc::clone(&session)), player, window, cx);
            }
        })
        .rows(self.rows.iter().map(with_face).collect())
        .me(self.me.as_ref().map(with_face))
        .more(self.more)
        .loading(self.loading)
        .problem(self.problem.clone());
        // One column down the middle of the page, however wide the window.
        let column = v_flex()
            .w_full()
            .max_w(px(860.))
            .gap_5()
            .child(
                h_flex()
                    .gap_3()
                    .child(img(LEADERBOARD_PICTURE).size_10())
                    .child(
                        div()
                            .text_3xl()
                            .font_weight(FontWeight::BOLD)
                            .child(tr!("social.leaderboard.page_title")),
                    ),
            )
            .child(panel);
        v_flex()
            .id("leaderboard-page")
            .size_full()
            .items_center()
            .p_6()
            .overflow_y_scroll()
            .child(column)
    }
}

#[cfg(test)]
mod tests {
    use std::{
        rc::Rc,
        sync::{Arc, Mutex},
    };

    use axum::{
        Json, Router,
        extract::{Path, Query},
        routing::get,
    };
    use gpui_kit::{Entity, TestAppContext, VisualTestContext};
    use serde_json::{Value, json};

    use super::LeaderboardView;
    use crate::{
        clients::server::ServerClient,
        enums::social::RankingBoard,
        testing::{ann, wait_until},
        views::live::SessionSource,
    };

    type Asked = Arc<Mutex<Vec<String>>>;

    /// A board of two places a page, three pages deep; every request is written down.
    fn server(asked: Asked) -> ServerClient {
        ServerClient::serving(Router::new().route(
            "/api/leaderboard/{board}",
            get(
                move |Path(board): Path<String>, Query(query): Query<Value>| async move {
                    let offset: usize = query["offset"]
                        .as_str()
                        .and_then(|offset| offset.parse().ok())
                        .unwrap_or_default();
                    asked
                        .lock()
                        .expect("the list")
                        .push(format!("{board} {offset}"));
                    let row = |place: usize| {
                        json!({
                            "place": place, "id": format!("p{place}"),
                            "username": format!("player{place}"), "rating": 2000 - place,
                            "provisional": false, "played": 10
                        })
                    };
                    Json(json!({
                        "rows": [row(offset + 1), row(offset + 2)],
                        "me": row(5),
                        "more": offset < 4,
                    }))
                },
            ),
        ))
    }

    fn opened(cx: &mut TestAppContext) -> (Entity<LeaderboardView>, Asked, &mut VisualTestContext) {
        cx.executor().allow_parking();
        cx.update(gpui_kit::init);
        let asked = Asked::default();
        let client = server(Arc::clone(&asked));
        let session = ann(false);
        let (view, cx) = cx.add_window_view(move |_, _| {
            let session: SessionSource = Rc::new(move |_| Some(session.clone()));
            LeaderboardView::new(client, session)
        });
        view.update(cx, |view, cx| view.opened(cx));
        wait_until(cx, |cx| view.read_with(cx, |view, _| !view.rows.is_empty()));
        (view, asked, cx)
    }

    fn names(view: &Entity<LeaderboardView>, cx: &mut VisualTestContext) -> Vec<String> {
        view.read_with(cx, |view, _| {
            view.rows.iter().map(|row| row.username.clone()).collect()
        })
    }

    #[gpui_kit::test]
    fn the_board_loads_more_and_switches_boards(cx: &mut TestAppContext) {
        let (view, asked, cx) = opened(cx);
        assert_eq!(names(&view, cx), ["player1", "player2"]);
        assert_eq!(
            view.read_with(cx, |view, _| view.me.as_ref().map(|me| me.place)),
            Some(5),
            "the player's own place"
        );
        view.update(cx, |view, cx| view.load_more(cx));
        wait_until(cx, |cx| view.read_with(cx, |view, _| view.rows.len() == 4));
        assert_eq!(names(&view, cx)[2], "player3", "the next page is added");

        view.update(cx, |view, cx| view.choose(RankingBoard::Bullet, cx));
        wait_until(cx, |cx| view.read_with(cx, |view, _| view.rows.len() == 2));
        let asked = asked.lock().expect("the list").clone();
        assert_eq!(asked, ["average 0", "average 2", "bullet 0"]);
    }

    #[gpui_kit::test]
    fn a_page_asked_for_another_board_is_dropped(cx: &mut TestAppContext) {
        let (view, asked, cx) = opened(cx);
        // Bullet is asked for, then Rapid before Bullet's answer is read.
        view.update(cx, |view, cx| {
            view.choose(RankingBoard::Bullet, cx);
            view.choose(RankingBoard::Rapid, cx);
        });
        wait_until(cx, |cx| view.read_with(cx, |view, _| !view.loading));
        assert_eq!(
            view.read_with(cx, |view, _| view.board),
            RankingBoard::Rapid
        );
        assert_eq!(
            names(&view, cx),
            ["player1", "player2"],
            "one page, not two"
        );
        assert!(
            asked
                .lock()
                .expect("the list")
                .contains(&"rapid 0".to_owned())
        );
    }
}
