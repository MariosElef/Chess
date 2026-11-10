use std::rc::Rc;

use gpui_kit::{
    component::{ActiveTheme as _, h_flex, v_flex},
    prelude::FluentBuilder as _,
    *,
};

use super::{
    invite_toast::{InviteChanged, InviteKind, InviteToast},
    rematch_toast::{RematchToast, ToastChanged},
};
use crate::{
    assets::{load_pictures_from, retry_missed_pictures},
    clients::{
        engine::device_cores,
        http::HttpRuntime,
        server::{ServerClient, server_url},
        settings::{
            SessionTokenFile, load_appearance, load_board_theme, load_engine_settings,
            load_language, load_piece_set, load_sound_set, save_appearance, save_board_theme,
            save_engine_settings, save_language, save_piece_set, save_sound_set,
        },
    },
    components::{
        board::BoardStyle,
        settings::{SettingsChoices, SettingsPage},
        shell::{AccountMenu, AppHeader, MESSAGES_PICTURE, MenuPerson, NavRail},
    },
    enums::{
        play::LiveStatus,
        puzzles::BattleStatus,
        settings::{Appearance, Language, SoundSet},
        shell::Page,
    },
    i18n::{self, tr},
    models::{
        account::AccountChanged,
        engine::EngineSettings,
        play::{AnalyzeGame, LiveEvent, ServerMessage},
    },
    services::social::avatar_path,
    views::{
        account::AccountView,
        analysis::AnalysisPage,
        chat::ChatsView,
        live::{LiveConnection, SessionSource},
        play::{OnlineView, PlayPage},
        puzzles::PuzzlesView,
        settings::apply_appearance,
        social::{LeaderboardView, OpenMessages, SocialView},
        sound::{play_sound_tour, use_sound_set},
    },
};

/// What the shell talks to: the chess server and the saved session; a test hands in its own.
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
    /// A game picked in the Account page's history opens on the analysis board.
    _account_games: Subscription,
    _account_redraws: Subscription,
    /// Puzzles from the server, fetched the first time the page opens.
    puzzles: Entity<PuzzlesView>,
    /// The live connection Play's online games, Puzzle Battle and Social share.
    live: Entity<LiveConnection>,
    social: Entity<SocialView>,
    /// The player's chats: the Messages page.
    messages: Entity<ChatsView>,
    /// Everyone's ratings, ranked: the Leaderboard page under Social.
    leaderboard: Entity<LeaderboardView>,
    /// An opponent's rematch, over every page, the game's own card included.
    rematch_toast: Entity<RematchToast>,
    _rematch_changes: Subscription,
    /// A friend's challenge or battle invitation, over every page but the one listing it.
    invite_toast: Entity<InviteToast>,
    _invite_changes: Subscription,
    /// A challenge's game opens Play, a battle Puzzles; Social's waiting count redraws the rail.
    _live_events: Subscription,
    _social_changes: Subscription,
    _message_changes: Subscription,
    _message_asks: Subscription,
    /// Battle invitations redraw the rail's count on Puzzles.
    _puzzle_changes: Subscription,
    /// The last battle that opened Puzzles: each one opens it once, when it begins.
    last_battle: Option<String>,
    language: Language,
    appearance: Appearance,
    /// The board theme and pieces every board draws with; also kept as a global for them.
    board_style: BoardStyle,
    sound_set: SoundSet,
    /// Stockfish's threads, memory and depth, shared by Settings and the analysis board.
    engine_settings: EngineSettings,
    _appearance_changes: Subscription,
    /// Play's Game Review hands its game over here.
    _play_events: Subscription,
}

impl AppShell {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let setup = ShellSetup::from_system();
        i18n::set_current(load_language().unwrap_or_default());
        // Players' pictures come from the same server as everything else.
        load_pictures_from(setup.client.clone());
        Self::with(setup, window, cx)
    }

    pub fn with(setup: ShellSetup, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let language = i18n::current();
        let appearance = load_appearance().unwrap_or_default();
        apply_appearance(appearance, window, cx);
        let _appearance_changes = cx.observe_window_appearance(window, |shell, window, cx| {
            if shell.appearance == Appearance::System {
                apply_appearance(Appearance::System, window, cx);
            }
        });

        let board_style = saved_board_style();
        cx.set_global(board_style);
        let sound_set = load_sound_set().unwrap_or_default();
        use_sound_set(sound_set, cx);
        let engine_settings = load_engine_settings();
        let ShellSetup { client, tokens } = setup;
        let account = cx.new(|cx| AccountView::new(client.clone(), tokens, window, cx));
        let _account_changes = cx.subscribe_in(&account, window, account_changed);
        let _account_games = cx.subscribe_in(&account, window, analyze_history_game);
        // A new picture shows in the header's menu at once.
        let _account_redraws = cx.observe(&account, |_, _, cx| cx.notify());
        let pages = Pages::new(&client, &account, engine_settings, window, cx);
        Self {
            page: Page::Play,
            client,
            play: pages.play,
            analyze: pages.analyze,
            account,
            _account_changes,
            _account_games,
            _account_redraws,
            puzzles: pages.puzzles,
            live: pages.live,
            social: pages.social,
            messages: pages.messages,
            leaderboard: pages.leaderboard,
            rematch_toast: pages.rematch_toast,
            _rematch_changes: pages.rematch_changes,
            invite_toast: pages.invite_toast,
            _invite_changes: pages.invite_changes,
            _message_changes: pages.message_changes,
            _message_asks: pages.message_asks,
            _live_events: pages.live_events,
            _social_changes: pages.social_changes,
            _puzzle_changes: pages.puzzle_changes,
            last_battle: None,
            language,
            appearance,
            board_style,
            sound_set,
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
        if page == Page::Social {
            self.social.update(cx, |social, cx| social.opened(cx));
        }
        self.messages.update(cx, |messages, cx| {
            messages.set_shown(page == Page::Messages, cx);
            if page == Page::Messages {
                messages.refresh(cx);
            }
        });
        if page == Page::Account {
            self.account.update(cx, |account, cx| account.opened(cx));
        }
        if page == Page::Leaderboard {
            self.leaderboard
                .update(cx, |leaderboard, cx| leaderboard.opened(cx));
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
            Page::Puzzles
            | Page::Social
            | Page::Messages
            | Page::Leaderboard
            | Page::Settings
            | Page::Account => None,
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

    /// Saves the new sound set, plays from it from now on, and plays all of it to hear it.
    fn set_sound_set(&mut self, set: SoundSet, cx: &mut Context<Self>) {
        self.sound_set = set;
        // A failed save only means the next launch starts on the standard sounds.
        let _ = save_sound_set(set);
        use_sound_set(set, cx);
        play_sound_tour(cx);
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
        self.live = pages.live;
        self.social = pages.social;
        self.messages = pages.messages;
        self.leaderboard = pages.leaderboard;
        self.rematch_toast = pages.rematch_toast;
        self._rematch_changes = pages.rematch_changes;
        self.invite_toast = pages.invite_toast;
        self._invite_changes = pages.invite_changes;
        self._message_changes = pages.message_changes;
        self._message_asks = pages.message_asks;
        self._live_events = pages.live_events;
        self._social_changes = pages.social_changes;
        self._puzzle_changes = pages.puzzle_changes;
        self._play_events = pages.play_events;
    }

    /// Every page draws again in `language`, and the next launch starts in it.
    fn set_language(&mut self, language: Language, window: &mut Window, cx: &mut Context<Self>) {
        self.language = language;
        i18n::set_current(language);
        // A failed save only means the next launch starts in English again.
        let _ = save_language(language);
        window.refresh();
        cx.notify();
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
            // The header still holds the window's buttons and moves it.
            return v_flex()
                .size_full()
                .bg(cx.theme().background)
                .child(AppHeader::new())
                .child(div().flex_1().min_h_0().child(self.account.clone()))
                .into_any_element();
        }
        let person = account.user().map(|user| MenuPerson {
            name: user.username.clone(),
            picture: avatar_path(&user.id, user.avatar.as_deref()),
            email: user.email.clone().filter(|_| !user.guest),
            guest: user.guest,
        });
        let waiting = self.social.read(cx).waiting_for_me();
        let unread = self.messages.read(cx).unread();
        let invitations = self.puzzles.read(cx).invitations(cx);
        let shell = cx.entity().downgrade();
        let header = person.into_iter().fold(AppHeader::new(), |header, person| {
            header.action(self.account_menu(person, cx))
        });
        // The header and the rail are one L of the chrome's colour; the page is a panel set
        // into it, rounded where it meets the corner.
        let rematch = self.shows_rematch_toast(cx);
        let listing = (
            self.page == Page::Social,
            self.page == Page::Puzzles && self.puzzles.read(cx).shows_battles(),
        );
        let invite = invite_toast_shows(self.invite_toast.read(cx).shown_kind(), listing);
        v_flex()
            .relative()
            .size_full()
            .bg(cx.theme().sidebar)
            .child(header)
            .child(
                h_flex()
                    .flex_1()
                    .min_h_0()
                    .items_stretch()
                    .child(
                        div()
                            .debug_selector(|| "nav-rail".to_owned())
                            .h_full()
                            .child(
                                NavRail::new(self.page, open_page(shell.clone()))
                                    .count(Page::Social, waiting)
                                    .count(Page::Messages, unread)
                                    .count(Page::Puzzles, invitations),
                            ),
                    )
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .rounded_tl(px(10.))
                            .overflow_hidden()
                            .bg(cx.theme().background)
                            .child(self.content(shell)),
                    ),
            )
            .when(rematch || invite, |this| {
                this.child(
                    v_flex()
                        .absolute()
                        .bottom_4()
                        .right_4()
                        .gap_3()
                        .when(rematch, |toasts| toasts.child(self.rematch_toast.clone()))
                        .when(invite, |toasts| toasts.child(self.invite_toast.clone())),
                )
            })
            .into_any_element()
    }
}

impl AppShell {
    /// A rematch is waiting and no game card on screen already offers to answer it.
    fn shows_rematch_toast(&self, cx: &App) -> bool {
        self.rematch_toast.read(cx).shown().is_some()
    }

    /// The avatar's menu: Account and Settings open here, signing out goes to the account.
    fn account_menu(&self, person: MenuPerson, cx: &mut Context<Self>) -> AccountMenu {
        let shell = cx.entity().downgrade();
        let (sign_out, leave) = (self.account.downgrade(), self.account.downgrade());
        AccountMenu::new(
            person,
            open_page(shell),
            move |_, cx| {
                let _ = sign_out.update(cx, |account, cx| account.sign_out(cx));
            },
            move |form, _, cx| {
                let _ = leave.update(cx, |account, cx| account.leave_guest(form, cx));
            },
        )
    }

    fn content(&self, shell: WeakEntity<Self>) -> AnyElement {
        match self.page {
            Page::Play => self.play.clone().into_any_element(),
            Page::Puzzles => self.puzzles.clone().into_any_element(),
            Page::Analyze => self.analyze.clone().into_any_element(),
            Page::Social => self.social.clone().into_any_element(),
            Page::Messages => messages_page(self.messages.clone()).into_any_element(),
            Page::Leaderboard => self.leaderboard.clone().into_any_element(),
            Page::Account => self.account.clone().into_any_element(),
            Page::Settings => SettingsPage::new(
                SettingsChoices {
                    language: self.language,
                    appearance: self.appearance,
                    board_style: self.board_style,
                    sound_set: self.sound_set,
                    engine: self.engine_settings,
                    cores: device_cores(),
                },
                change_language(shell.clone()),
                change_appearance(shell.clone()),
                change_board_style(shell.clone()),
                change_sound_set(shell.clone()),
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

fn change_sound_set(
    shell: WeakEntity<AppShell>,
) -> impl Fn(SoundSet, &mut Window, &mut App) + 'static {
    move |set, _, cx| {
        let _ = shell.update(cx, |shell, cx| shell.set_sound_set(set, cx));
    }
}

fn change_board_style(
    shell: WeakEntity<AppShell>,
) -> impl Fn(BoardStyle, &mut Window, &mut App) + 'static {
    move |style, _, cx| {
        let _ = shell.update(cx, |shell, cx| shell.set_board_style(style, cx));
    }
}

fn change_language(
    shell: WeakEntity<AppShell>,
) -> impl Fn(Language, &mut Window, &mut App) + 'static {
    move |language, window, cx| {
        let _ = shell.update(cx, |shell, cx| shell.set_language(language, window, cx));
    }
}

fn change_appearance(
    shell: WeakEntity<AppShell>,
) -> impl Fn(Appearance, &mut Window, &mut App) + 'static {
    move |appearance, window, cx| {
        let _ = shell.update(cx, |shell, cx| shell.set_appearance(appearance, window, cx));
    }
}

/// A game picked in the Account page's history, if the player is still there to see it.
fn analyze_history_game(
    shell: &mut AppShell,
    account: &Entity<AccountView>,
    event: &AnalyzeGame,
    window: &mut Window,
    cx: &mut Context<AppShell>,
) {
    if shell.page == Page::Account {
        analyze_game(shell, account, event, window, cx);
    }
}

/// Opens a finished game, from Play or the Account page's history, on the analysis board.
fn analyze_game<Source: 'static>(
    shell: &mut AppShell,
    _: &Entity<Source>,
    event: &AnalyzeGame,
    window: &mut Window,
    cx: &mut Context<AppShell>,
) {
    shell.analyze.update(cx, |analysis, cx| {
        analysis.load_game(&event.game, (event.viewing, event.outcome), cx);
        if event.review {
            analysis.start_review(cx);
        }
    });
    shell.open(Page::Analyze, window, cx);
}

/// Signing in opens Play; signing out shows the account screen and resets the pages.
fn account_changed(
    shell: &mut AppShell,
    account: &Entity<AccountView>,
    _: &AccountChanged,
    window: &mut Window,
    cx: &mut Context<AppShell>,
) {
    if account.read(cx).is_admitted() {
        shell.page = Page::Play;
        shell.live.update(cx, |live, cx| live.connect(cx));
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
    live: Entity<LiveConnection>,
    social: Entity<SocialView>,
    messages: Entity<ChatsView>,
    leaderboard: Entity<LeaderboardView>,
    rematch_toast: Entity<RematchToast>,
    /// The toast's changes redraw the shell, not each tick of its bar.
    rematch_changes: Subscription,
    invite_toast: Entity<InviteToast>,
    /// The invitation shown changes whether the toast shows.
    invite_changes: Subscription,
    live_events: Subscription,
    /// Unread messages change the rail's count.
    message_changes: Subscription,
    /// A friend's Message button opens the Messages page.
    message_asks: Subscription,
    social_changes: Subscription,
    puzzle_changes: Subscription,
    /// Play's Game Review hands its game over to the shell.
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
        let session = session_of(account);
        let live = cx.new(|_| LiveConnection::new(client.clone(), Rc::clone(&session)));
        let live_events = cx.subscribe_in(&live, window, follow_live);
        let play = new_play_page(&live, Rc::clone(&session), window, cx);
        let play_events = cx.subscribe_in(&play, window, analyze_game);
        let puzzles = new_puzzles_view(
            client.clone(),
            account,
            (live.clone(), Rc::clone(&session)),
            cx,
        );
        let messages =
            cx.new(|cx| ChatsView::new(client.clone(), (Rc::clone(&session), live.clone()), cx));
        let message_changes = cx.observe(&messages, |_, _, cx| cx.notify());
        let leaderboard = cx.new(|_| LeaderboardView::new(client.clone(), Rc::clone(&session)));
        let social = cx.new(|cx| {
            SocialView::new(
                client.clone(),
                (session, live.clone()),
                messages.clone(),
                window,
                cx,
            )
        });
        let message_asks = cx.subscribe_in(&social, window, open_messages);
        let social_changes = cx.observe(&social, |_, _, cx| cx.notify());
        let puzzle_changes = cx.observe(&puzzles, |_, _, cx| cx.notify());
        let rematch_toast = cx.new(|cx| RematchToast::new(live.clone(), cx));
        // The toast's bar ticks redraw the toast alone; this hears only what it shows.
        let rematch_changes =
            cx.subscribe(&rematch_toast, |_, _, _: &ToastChanged, cx| cx.notify());
        let invite_toast = cx.new(|cx| InviteToast::new(live.clone(), cx));
        let invite_changes = cx.subscribe(&invite_toast, |_, _, _: &InviteChanged, cx| cx.notify());
        Self {
            play,
            analyze: cx.new(|cx| AnalysisPage::new(engine_settings, window, cx)),
            puzzles,
            live,
            social,
            messages,
            leaderboard,
            rematch_toast,
            rematch_changes,
            invite_toast,
            invite_changes,
            live_events,
            message_changes,
            message_asks,
            social_changes,
            puzzle_changes,
            play_events,
        }
    }
}

/// Starts with keyboard focus, so the arrow keys work before anything is clicked.
fn new_play_page(
    live: &Entity<LiveConnection>,
    session: SessionSource,
    window: &mut Window,
    cx: &mut Context<AppShell>,
) -> Entity<PlayPage> {
    let online = cx.new(|cx| OnlineView::new(live.clone(), Rc::clone(&session), cx));
    let play = cx.new(|cx| PlayPage::new(online, session, cx));
    let focus = play.read(cx).focus_handle();
    focus.focus(window, cx);
    play
}

/// The Messages page: its title, then the chats.
fn messages_page(chats: Entity<ChatsView>) -> Div {
    v_flex()
        .size_full()
        .p_6()
        .gap_5()
        .child(
            h_flex()
                .gap_3()
                .child(img(MESSAGES_PICTURE).size_10())
                .child(
                    div()
                        .text_3xl()
                        .font_weight(FontWeight::BOLD)
                        .child(tr!("shell.pages.messages")),
                ),
        )
        .child(div().flex_1().min_h_0().child(chats))
}

/// A friend's Message button: the Messages page opens, with their chat.
fn open_messages(
    shell: &mut AppShell,
    _: &Entity<SocialView>,
    _: &OpenMessages,
    window: &mut Window,
    cx: &mut Context<AppShell>,
) {
    shell.open(Page::Messages, window, cx);
}

/// Reads the signed-in session from the account, for the pages that need who is playing.
fn session_of(account: &Entity<AccountView>) -> SessionSource {
    let account = account.downgrade();
    Rc::new(move |cx| {
        account
            .upgrade()
            .and_then(|account| account.read(cx).session())
    })
}

/// The live connection's word for the shell: new games, missed pictures and account changes.
fn follow_live(
    shell: &mut AppShell,
    _: &Entity<LiveConnection>,
    event: &LiveEvent,
    window: &mut Window,
    cx: &mut Context<AppShell>,
) {
    if let LiveEvent::SignedOut = event {
        shell
            .account
            .update(cx, |account, cx| account.session_ended(cx));
        return;
    }
    if let LiveEvent::Connected = event {
        // Back in touch with the server: pictures that failed while it was away come now, and
        // the account is fetched again, as it may have changed on another device meanwhile.
        retry_missed_pictures(cx);
        shell
            .account
            .update(cx, |account, cx| account.reload_user(cx));
    }
    let LiveEvent::Message(message) = event else {
        return;
    };
    match message.as_ref() {
        ServerMessage::ProfileChanged => {
            shell
                .account
                .update(cx, |account, cx| account.reload_user(cx));
        }
        // The lists are fetched again with fresh pictures: missed ones get another try too.
        ServerMessage::FriendsChanged | ServerMessage::ConversationsChanged => {
            retry_missed_pictures(cx);
        }
        ServerMessage::Seeking { .. }
        | ServerMessage::SeekCancelled
        | ServerMessage::Game { .. }
        | ServerMessage::Error { .. }
        | ServerMessage::Challenges { .. }
        | ServerMessage::ChatMessage { .. }
        | ServerMessage::BattleSeeking
        | ServerMessage::BattleSeekCancelled
        | ServerMessage::Battle { .. }
        | ServerMessage::BattleChallenges { .. }
        | ServerMessage::RematchRefused { .. }
        | ServerMessage::InviteRefused { .. } => {}
    }
    if let ServerMessage::Game { game } = message.as_ref()
        && game.status == LiveStatus::Ongoing
        && game.moves.is_empty()
        && shell.page != Page::Play
    {
        shell.open(Page::Play, window, cx);
    }
    if let ServerMessage::Battle { battle } = message.as_ref()
        && battle.status == BattleStatus::Ongoing
        && shell.last_battle.as_deref() != Some(battle.id.as_str())
    {
        shell.last_battle = Some(battle.id.clone());
        if shell.page != Page::Puzzles {
            shell.open(Page::Puzzles, window, cx);
        }
    }
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
    live: (Entity<LiveConnection>, SessionSource),
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
            live,
            cx,
        )
    })
}

/// An invitation shows as a toast unless the screen listing its kind is open: Social, or battles.
fn invite_toast_shows(kind: Option<InviteKind>, (social, battles): (bool, bool)) -> bool {
    match kind {
        Some(InviteKind::Game) => !social,
        Some(InviteKind::Battle) => !battles,
        None => false,
    }
}

#[cfg(test)]
mod tests {
    use std::{env, process, time::Duration};

    use axum::{
        Json, Router,
        extract::ws::{Message, WebSocket, WebSocketUpgrade},
        http::StatusCode,
        routing::{delete, get, post},
    };
    use gpui_kit::{Bounds, Entity, Modifiers, Pixels, TestAppContext, VisualTestContext};
    use serde_json::json;

    use super::{AppShell, ShellSetup};
    use crate::{
        clients::{server::ServerClient, settings::SessionTokenFile},
        enums::shell::Page,
        models::play::ClientMessage,
        testing::{centre_of, click, wait_until},
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
                )
                .route(
                    "/api/ws",
                    get(
                        |upgrade: WebSocketUpgrade| async move { upgrade.on_upgrade(start_a_game) },
                    ),
                ),
        )
    }

    /// Answers the app's first message with a new game, as an accepted challenge starts one.
    async fn start_a_game(mut socket: WebSocket) {
        if !matches!(socket.recv().await, Some(Ok(Message::Text(_)))) {
            return;
        }
        let game = json!({ "type": "game", "game": {
            "id": "g1",
            "white": { "id": "3", "username": "Guest3", "rating": null },
            "black": { "id": "4", "username": "Guest4", "rating": null },
            "rated": false, "initial_s": 180, "increment_s": 2, "moves": [],
            "white_ms": 180_000, "black_ms": 180_000, "running": null,
            "status": "ongoing", "result": null, "winner": null, "draw_offer": null,
            "rating_changes": null, "chat_id": "c1"
        }});
        let _ = socket.send(Message::Text(game.to_string().into())).await;
        // Kept open, as the server keeps it, until the app goes.
        while let Some(Ok(_)) = socket.recv().await {}
    }

    fn rail_shown(cx: &mut VisualTestContext) -> bool {
        cx.run_until_parked();
        cx.debug_bounds("nav-rail").is_some()
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
        assert!(!rail_shown(cx), "only the account screen before signing in");

        let account = shell.update(cx, |shell, _| shell.account.clone());
        account.update_in(cx, |account, window, cx| {
            account.continue_as_guest(window, cx)
        });
        wait_until(cx, |cx| {
            account.update(cx, |account, _| account.is_admitted())
        });
        assert!(rail_shown(cx), "the app opens");
        let play_before = shell.update(cx, |shell, _| shell.play.entity_id());

        account.update(cx, |account, cx| account.sign_out(cx));
        assert!(!rail_shown(cx), "back to the account screen");
        let play_after = shell.update(cx, |shell, _| shell.play.entity_id());
        assert_ne!(
            play_before, play_after,
            "the next player gets a fresh Play page"
        );
    }

    /// Where one square is drawn on `page`, and how big, to check every page's board matches.
    fn square_on(
        shell: &Entity<AppShell>,
        page: Page,
        cx: &mut VisualTestContext,
    ) -> Bounds<Pixels> {
        shell.update_in(cx, |shell, window, cx| shell.open(page, window, cx));
        cx.run_until_parked();
        cx.debug_bounds("square-a1").expect("the board is drawn")
    }

    #[gpui_kit::test]
    fn the_board_is_the_same_size_and_in_the_same_place_on_every_page(cx: &mut TestAppContext) {
        cx.executor().allow_parking();
        cx.update(gpui_kit::init);
        let path = env::temp_dir().join(format!("chess-shell-board-test-{}", process::id()));
        let setup = ShellSetup {
            client: guest_server(),
            tokens: SessionTokenFile::at(path),
        };
        let (shell, cx) = cx.add_window_view(|window, cx| AppShell::with(setup, window, cx));
        let account = shell.update(cx, |shell, _| shell.account.clone());
        account.update_in(cx, |account, window, cx| {
            account.continue_as_guest(window, cx)
        });
        wait_until(cx, |cx| {
            account.update(cx, |account, _| account.is_admitted())
        });
        let play = square_on(&shell, Page::Play, cx);
        assert_eq!(square_on(&shell, Page::Puzzles, cx), play, "Puzzles");
        assert_eq!(square_on(&shell, Page::Analyze, cx), play, "Analysis");
    }

    #[gpui_kit::test]
    fn hovering_social_offers_friends_and_messages(cx: &mut TestAppContext) {
        cx.executor().allow_parking();
        cx.update(gpui_kit::init);
        let path = env::temp_dir().join(format!("chess-shell-menu-test-{}", process::id()));
        let setup = ShellSetup {
            client: guest_server(),
            tokens: SessionTokenFile::at(path),
        };
        let (shell, cx) = cx.add_window_view(|window, cx| AppShell::with(setup, window, cx));
        let account = shell.update(cx, |shell, _| shell.account.clone());
        account.update_in(cx, |account, window, cx| {
            account.continue_as_guest(window, cx)
        });
        wait_until(cx, |cx| {
            account.update(cx, |account, _| account.is_admitted())
        });
        let social = centre_of(cx, "rail-Social");
        cx.simulate_mouse_move(social, None, Modifiers::none());
        wait_until(cx, |cx| {
            cx.executor().advance_clock(Duration::from_millis(200));
            cx.debug_bounds("rail-menu-Messages").is_some()
        });
        click(cx, "rail-menu-Messages");
        assert_eq!(shell.read_with(cx, |shell, _| shell.page), Page::Messages);
    }

    #[gpui_kit::test]
    fn clicking_social_itself_opens_nothing(cx: &mut TestAppContext) {
        cx.executor().allow_parking();
        cx.update(gpui_kit::init);
        let path = env::temp_dir().join(format!("chess-shell-social-test-{}", process::id()));
        let setup = ShellSetup {
            client: guest_server(),
            tokens: SessionTokenFile::at(path),
        };
        let (shell, cx) = cx.add_window_view(|window, cx| AppShell::with(setup, window, cx));
        let account = shell.update(cx, |shell, _| shell.account.clone());
        account.update_in(cx, |account, window, cx| {
            account.continue_as_guest(window, cx)
        });
        wait_until(cx, |cx| {
            account.update(cx, |account, _| account.is_admitted())
        });
        let social = centre_of(cx, "rail-Social");
        cx.simulate_mouse_move(social, None, Modifiers::none());
        wait_until(cx, |cx| {
            cx.executor().advance_clock(Duration::from_millis(200));
            cx.debug_bounds("rail-menu-Messages").is_some()
        });
        click(cx, "rail-Social");
        assert_eq!(
            shell.read_with(cx, |shell, _| shell.page),
            Page::Play,
            "Friends and Messages are picked from the menu"
        );
    }

    #[gpui_kit::test]
    fn a_game_starting_elsewhere_opens_play(cx: &mut TestAppContext) {
        cx.executor().allow_parking();
        cx.update(gpui_kit::init);
        let path = env::temp_dir().join(format!("chess-shell-game-test-{}", process::id()));
        let setup = ShellSetup {
            client: guest_server(),
            tokens: SessionTokenFile::at(path),
        };
        let (shell, cx) = cx.add_window_view(|window, cx| AppShell::with(setup, window, cx));
        let account = shell.update(cx, |shell, _| shell.account.clone());
        account.update_in(cx, |account, window, cx| {
            account.continue_as_guest(window, cx)
        });
        wait_until(cx, |cx| {
            account.update(cx, |account, _| account.is_admitted())
        });
        shell.update_in(cx, |shell, window, cx| {
            shell.open(Page::Social, window, cx);
            shell.live.read(cx).send(ClientMessage::CancelSeek);
        });
        wait_until(cx, |cx| {
            shell.update(cx, |shell, _| shell.page == Page::Play)
        });
        let in_game = shell.update(cx, |shell, cx| shell.play.read(cx).shows_online(cx));
        assert!(in_game, "Play shows the online game");
    }

    #[test]
    fn an_invite_toast_steps_aside_on_the_screen_listing_its_kind() {
        use super::{InviteKind, invite_toast_shows};
        let (on_social, on_battles, elsewhere) = ((true, false), (false, true), (false, false));
        assert!(invite_toast_shows(Some(InviteKind::Game), elsewhere));
        assert!(!invite_toast_shows(Some(InviteKind::Game), on_social));
        assert!(invite_toast_shows(Some(InviteKind::Battle), on_social));
        assert!(!invite_toast_shows(Some(InviteKind::Battle), on_battles));
        assert!(!invite_toast_shows(None, elsewhere));
    }
}
