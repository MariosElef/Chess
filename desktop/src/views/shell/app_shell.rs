use gpui_kit::{component::h_flex, *};

use crate::{
    clients::{
        engine::device_cores,
        http::HttpRuntime,
        server::{ServerClient, server_url},
        settings::{
            SessionTokenFile, load_appearance, load_board_theme, load_engine_settings,
            load_piece_set, save_appearance, save_board_theme, save_engine_settings,
            save_piece_set,
        },
    },
    components::{
        board::BoardStyle,
        settings::{SettingsChoices, SettingsPage},
        shell::NavSidebar,
    },
    enums::{settings::Appearance, shell::Page},
    models::{account::AccountChanged, engine::EngineSettings, play::AnalyzeGame},
    views::{
        account::AccountView, analysis::AnalysisPage, play::PlayPage, puzzles::PuzzlesView,
        settings::apply_appearance,
    },
};

/// What the shell talks to: the chess server, and the file that keeps the session between
/// launches. The app uses the real ones; a test hands in its own.
pub struct ShellSetup {
    pub client: ServerClient,
    pub tokens: SessionTokenFile,
}

impl ShellSetup {
    /// The server named by `CHESS_SERVER_URL` (or the local one), and the settings folder.
    pub fn from_system() -> Self {
        let http = HttpRuntime::new().expect("the network runtime starts");
        Self {
            client: ServerClient::new(http, &server_url()),
            tokens: SessionTokenFile::in_settings_folder(),
        }
    }
}

pub struct AppShell {
    page: Page,
    client: ServerClient,
    play: Entity<PlayPage>,
    analyze: Entity<AnalysisPage>,
    /// Signing in and out; it keeps the session the other pages will use.
    account: Entity<AccountView>,
    _account_changes: Subscription,
    /// Puzzles from the server, fetched the first time the page opens.
    puzzles: Entity<PuzzlesView>,
    appearance: Appearance,
    /// The board theme and pieces every board draws with; also kept as a global for them.
    board_style: BoardStyle,
    /// Stockfish's threads, memory and depth, shared by Settings and the analysis board.
    engine_settings: EngineSettings,
    _appearance_changes: Subscription,
    /// Play's "Analyze" button hands its game over here.
    _play_events: Subscription,
}

impl AppShell {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        Self::with(ShellSetup::from_system(), window, cx)
    }

    pub fn with(setup: ShellSetup, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let appearance = load_appearance().unwrap_or_default();
        apply_appearance(appearance, window, cx);
        let _appearance_changes = cx.observe_window_appearance(window, |shell, window, cx| {
            if shell.appearance == Appearance::System {
                apply_appearance(Appearance::System, window, cx);
            }
        });

        let board_style = saved_board_style();
        cx.set_global(board_style);
        let engine_settings = load_engine_settings();
        let ShellSetup { client, tokens } = setup;
        let account = cx.new(|cx| AccountView::new(client.clone(), tokens, window, cx));
        let _account_changes = cx.subscribe_in(&account, window, account_changed);
        let pages = Pages::new(&client, &account, engine_settings, window, cx);
        Self {
            page: Page::Play,
            client,
            play: pages.play,
            analyze: pages.analyze,
            account,
            _account_changes,
            puzzles: pages.puzzles,
            appearance,
            board_style,
            engine_settings,
            _appearance_changes,
            _play_events: pages.play_events,
        }
    }

    /// Shows `page`, gives it the keyboard, and lets the analysis board know if it is on screen.
    fn open(&mut self, page: Page, window: &mut Window, cx: &mut Context<Self>) {
        self.page = page;
        if page == Page::Puzzles {
            self.puzzles.update(cx, |puzzles, cx| puzzles.opened(cx));
        }
        self.analyze.update(cx, |analysis, cx| {
            analysis.set_shown(page == Page::Analyze, cx)
        });
        if let Some(focus) = self.focus_handle_of(page, cx) {
            focus.focus(window, cx);
        }
        cx.notify();
    }

    /// The page that takes keyboard focus when opened; Puzzles and Settings have no shortcuts.
    fn focus_handle_of(&self, page: Page, cx: &App) -> Option<FocusHandle> {
        match page {
            Page::Play => Some(self.play.read(cx).focus_handle()),
            Page::Analyze => Some(self.analyze.read(cx).focus_handle()),
            Page::Puzzles | Page::Settings | Page::Account => None,
        }
    }

    /// Saves the new engine settings and restarts the analysis engine with them.
    fn set_engine_settings(&mut self, settings: EngineSettings, cx: &mut Context<Self>) {
        self.engine_settings = settings;
        // A failed save only means the next launch starts on this device's defaults.
        let _ = save_engine_settings(settings);
        self.analyze.update(cx, |analysis, cx| {
            analysis.set_engine_settings(settings, cx)
        });
        cx.notify();
    }

    /// Saves the new board style and redraws every board with it.
    fn set_board_style(&mut self, style: BoardStyle, cx: &mut Context<Self>) {
        self.board_style = style;
        // A failed save only means the next launch starts on the default board.
        let _ = save_board_theme(style.theme);
        let _ = save_piece_set(style.pieces);
        cx.set_global(style);
        cx.refresh_windows();
    }

    /// Every page as new, so nothing of the account that just left is shown to the next.
    fn fresh_pages(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let pages = Pages::new(
            &self.client,
            &self.account,
            self.engine_settings,
            window,
            cx,
        );
        self.play = pages.play;
        self.analyze = pages.analyze;
        self.puzzles = pages.puzzles;
        self._play_events = pages.play_events;
    }

    fn set_appearance(
        &mut self,
        appearance: Appearance,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.appearance = appearance;
        apply_appearance(appearance, window, cx);
        // A failed save only means the next launch starts on System again.
        let _ = save_appearance(appearance);
        cx.notify();
    }
}

impl Render for AppShell {
    /// Nothing but the account screen until the player signs in or chooses to play as a guest.
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let account = self.account.read(cx);
        if !account.is_admitted() {
            return div()
                .size_full()
                .child(self.account.clone())
                .into_any_element();
        }
        let player = account.player_name();
        let shell = cx.entity().downgrade();
        h_flex()
            .size_full()
            .items_stretch()
            .child(
                div()
                    .debug_selector(|| "nav-sidebar".to_owned())
                    .child(NavSidebar::new(self.page, player, open_page(shell.clone()))),
            )
            .child(div().flex_1().min_w_0().child(self.content(shell)))
            .into_any_element()
    }
}

impl AppShell {
    fn content(&self, shell: WeakEntity<Self>) -> AnyElement {
        match self.page {
            Page::Play => self.play.clone().into_any_element(),
            Page::Puzzles => self.puzzles.clone().into_any_element(),
            Page::Analyze => self.analyze.clone().into_any_element(),
            Page::Account => self.account.clone().into_any_element(),
            Page::Settings => SettingsPage::new(
                SettingsChoices {
                    appearance: self.appearance,
                    board_style: self.board_style,
                    engine: self.engine_settings,
                    cores: device_cores(),
                },
                change_appearance(shell.clone()),
                change_board_style(shell.clone()),
                change_engine_settings(shell),
            )
            .into_any_element(),
        }
    }
}

// The shell outlives the elements it renders; a failed update means the window is closing.

fn open_page(shell: WeakEntity<AppShell>) -> impl Fn(Page, &mut Window, &mut App) + 'static {
    move |page, window, cx| {
        let _ = shell.update(cx, |shell, cx| shell.open(page, window, cx));
    }
}

fn change_engine_settings(
    shell: WeakEntity<AppShell>,
) -> impl Fn(EngineSettings, &mut Window, &mut App) + 'static {
    move |settings, _, cx| {
        let _ = shell.update(cx, |shell, cx| shell.set_engine_settings(settings, cx));
    }
}

fn change_board_style(
    shell: WeakEntity<AppShell>,
) -> impl Fn(BoardStyle, &mut Window, &mut App) + 'static {
    move |style, _, cx| {
        let _ = shell.update(cx, |shell, cx| shell.set_board_style(style, cx));
    }
}

fn change_appearance(
    shell: WeakEntity<AppShell>,
) -> impl Fn(Appearance, &mut Window, &mut App) + 'static {
    move |appearance, window, cx| {
        let _ = shell.update(cx, |shell, cx| shell.set_appearance(appearance, window, cx));
    }
}

/// Opens the finished game from Play on the analysis board, at the position Play was showing.
fn analyze_game(
    shell: &mut AppShell,
    _: &Entity<PlayPage>,
    event: &AnalyzeGame,
    window: &mut Window,
    cx: &mut Context<AppShell>,
) {
    shell.analyze.update(cx, |analysis, cx| {
        analysis.load_game(&event.game, event.viewing, cx)
    });
    shell.open(Page::Analyze, window, cx);
}

/// Coming in, the app opens on Play; going out, the account screen takes over and the pages
/// start afresh for whoever comes next.
fn account_changed(
    shell: &mut AppShell,
    account: &Entity<AccountView>,
    _: &AccountChanged,
    window: &mut Window,
    cx: &mut Context<AppShell>,
) {
    if account.read(cx).is_admitted() {
        shell.page = Page::Play;
    } else {
        shell.fresh_pages(window, cx);
    }
    cx.notify();
}

/// The pages that keep what a player did: games, analysis and puzzles.
struct Pages {
    play: Entity<PlayPage>,
    analyze: Entity<AnalysisPage>,
    puzzles: Entity<PuzzlesView>,
    /// Play's "Analyze" button hands its game over to the shell.
    play_events: Subscription,
}

impl Pages {
    fn new(
        client: &ServerClient,
        account: &Entity<AccountView>,
        engine_settings: EngineSettings,
        window: &mut Window,
        cx: &mut Context<AppShell>,
    ) -> Self {
        let play = new_play_page(window, cx);
        let play_events = cx.subscribe_in(&play, window, analyze_game);
        Self {
            play,
            analyze: cx.new(|cx| AnalysisPage::new(engine_settings, window, cx)),
            puzzles: new_puzzles_view(client.clone(), account, cx),
            play_events,
        }
    }
}

/// Starts with keyboard focus, so the arrow keys work before anything is clicked.
fn new_play_page(window: &mut Window, cx: &mut Context<AppShell>) -> Entity<PlayPage> {
    let play = cx.new(PlayPage::new);
    let focus = play.read(cx).focus_handle();
    focus.focus(window, cx);
    play
}

/// The board style saved last time, or the defaults on a first launch.
fn saved_board_style() -> BoardStyle {
    BoardStyle {
        theme: load_board_theme().unwrap_or_default(),
        pieces: load_piece_set().unwrap_or_default(),
    }
}

/// The puzzles page asks the account for the signed-in token each time it fetches.
fn new_puzzles_view(
    client: ServerClient,
    account: &Entity<AccountView>,
    cx: &mut Context<AppShell>,
) -> Entity<PuzzlesView> {
    let account = account.downgrade();
    cx.new(|cx| {
        PuzzlesView::new(
            client,
            move |cx| {
                account
                    .upgrade()
                    .and_then(|account| account.read(cx).token())
            },
            cx,
        )
    })
}

#[cfg(test)]
mod tests {
    use std::{env, process};

    use axum::{
        Json, Router,
        http::StatusCode,
        routing::{delete, post},
    };
    use gpui_kit::{TestAppContext, VisualTestContext};
    use serde_json::json;

    use super::{AppShell, ShellSetup};
    use crate::{
        clients::{server::ServerClient, settings::SessionTokenFile},
        testing::wait_until,
    };

    /// A server that hands out Guest3 and takes sign-outs.
    fn guest_server() -> ServerClient {
        ServerClient::serving(
            Router::new()
                .route(
                    "/api/sessions/guest",
                    post(|| async {
                        (
                            StatusCode::CREATED,
                            Json(json!({
                                "token": "guest-token",
                                "expires_at": "2026-11-09T00:00:00Z",
                                "user": {
                                    "id": "3", "email": "guest3@guest.invalid",
                                    "username": "Guest3", "guest": true
                                }
                            })),
                        )
                    }),
                )
                .route(
                    "/api/sessions/current",
                    delete(|| async { Json(json!({ "message": "Signed out successfully" })) }),
                ),
        )
    }

    fn sidebar_shown(cx: &mut VisualTestContext) -> bool {
        cx.run_until_parked();
        cx.debug_bounds("nav-sidebar").is_some()
    }

    #[gpui_kit::test]
    fn the_app_opens_only_once_signed_in_and_starts_afresh_after(cx: &mut TestAppContext) {
        cx.executor().allow_parking();
        cx.update(gpui_kit::init);
        let path = env::temp_dir().join(format!("chess-shell-test-{}", process::id()));
        let setup = ShellSetup {
            client: guest_server(),
            tokens: SessionTokenFile::at(path),
        };
        let (shell, cx) = cx.add_window_view(|window, cx| AppShell::with(setup, window, cx));
        assert!(
            !sidebar_shown(cx),
            "only the account screen before signing in"
        );

        let account = shell.update(cx, |shell, _| shell.account.clone());
        account.update_in(cx, |account, window, cx| {
            account.continue_as_guest(window, cx)
        });
        wait_until(cx, |cx| {
            account.update(cx, |account, _| account.is_admitted())
        });
        assert!(sidebar_shown(cx), "the app opens");
        let play_before = shell.update(cx, |shell, _| shell.play.entity_id());

        account.update(cx, |account, cx| account.sign_out(cx));
        assert!(!sidebar_shown(cx), "back to the account screen");
        let play_after = shell.update(cx, |shell, _| shell.play.entity_id());
        assert_ne!(
            play_before, play_after,
            "the next player gets a fresh Play page"
        );
    }
}
