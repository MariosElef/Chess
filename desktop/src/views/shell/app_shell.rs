use gpui_kit::{component::h_flex, *};

use crate::{
    clients::{
        engine::device_cores,
        settings::{load_appearance, load_engine_settings, save_appearance, save_engine_settings},
    },
    components::{puzzles::PuzzlesPage, settings::SettingsPage, shell::NavSidebar},
    enums::{settings::Appearance, shell::Page},
    models::{engine::EngineSettings, play::AnalyzeGame},
    views::{analysis::AnalysisPage, play::PlayPage, settings::apply_appearance},
};

pub struct AppShell {
    page: Page,
    play: Entity<PlayPage>,
    analyze: Entity<AnalysisPage>,
    appearance: Appearance,
    /// Stockfish's threads, memory and depth, shared by Settings and the analysis board.
    engine_settings: EngineSettings,
    _appearance_changes: Subscription,
    /// Play's "Analyze" button hands its game over here.
    _play_events: Subscription,
}

impl AppShell {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let appearance = load_appearance().unwrap_or_default();
        apply_appearance(appearance, window, cx);
        let _appearance_changes = cx.observe_window_appearance(window, |shell, window, cx| {
            if shell.appearance == Appearance::System {
                apply_appearance(Appearance::System, window, cx);
            }
        });

        let engine_settings = load_engine_settings();
        let play = new_play_page(window, cx);
        let _play_events = cx.subscribe_in(&play, window, analyze_game);
        Self {
            page: Page::Play,
            play,
            analyze: cx.new(|cx| AnalysisPage::new(engine_settings, window, cx)),
            appearance,
            engine_settings,
            _appearance_changes,
            _play_events,
        }
    }

    /// Shows `page`, gives it the keyboard, and lets the analysis board know if it is on screen.
    fn open(&mut self, page: Page, window: &mut Window, cx: &mut Context<Self>) {
        self.page = page;
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
            Page::Puzzles | Page::Settings => None,
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
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let shell = cx.entity().downgrade();
        h_flex()
            .size_full()
            .items_stretch()
            .child(NavSidebar::new(self.page, open_page(shell.clone())))
            .child(div().flex_1().min_w_0().child(self.content(shell)))
    }
}

impl AppShell {
    fn content(&self, shell: WeakEntity<Self>) -> AnyElement {
        match self.page {
            Page::Play => self.play.clone().into_any_element(),
            Page::Puzzles => PuzzlesPage.into_any_element(),
            Page::Analyze => self.analyze.clone().into_any_element(),
            Page::Settings => SettingsPage::new(
                self.appearance,
                self.engine_settings,
                device_cores(),
                change_appearance(shell.clone()),
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

/// Starts with keyboard focus, so the arrow keys work before anything is clicked.
fn new_play_page(window: &mut Window, cx: &mut Context<AppShell>) -> Entity<PlayPage> {
    let play = cx.new(PlayPage::new);
    let focus = play.read(cx).focus_handle();
    focus.focus(window, cx);
    play
}
