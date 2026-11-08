use gpui_kit::{component::h_flex, *};

use super::{nav_sidebar::NavSidebar, page::Page};
use crate::{
    play::PlayPage,
    puzzles::PuzzlesPage,
    settings::{Appearance, SettingsPage, load_appearance, save_appearance},
};

pub struct AppShell {
    page: Page,
    play: Entity<PlayPage>,
    appearance: Appearance,
    _appearance_changes: Subscription,
}

impl AppShell {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let appearance = load_appearance().unwrap_or_default();
        appearance.apply(window, cx);
        let _appearance_changes = cx.observe_window_appearance(window, |shell, window, cx| {
            if shell.appearance == Appearance::System {
                Appearance::System.apply(window, cx);
            }
        });

        Self {
            page: Page::Play,
            play: new_play_page(window, cx),
            appearance,
            _appearance_changes,
        }
    }

    fn open(&mut self, page: Page, cx: &mut Context<Self>) {
        self.page = page;
        cx.notify();
    }

    fn set_appearance(
        &mut self,
        appearance: Appearance,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.appearance = appearance;
        appearance.apply(window, cx);
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
            Page::Settings => {
                SettingsPage::new(self.appearance, change_appearance(shell)).into_any_element()
            }
        }
    }
}

// The shell outlives the elements it renders; a failed update means the window is closing.

fn open_page(shell: WeakEntity<AppShell>) -> impl Fn(Page, &mut Window, &mut App) + 'static {
    move |page, _, cx| {
        let _ = shell.update(cx, |shell, cx| shell.open(page, cx));
    }
}

fn change_appearance(
    shell: WeakEntity<AppShell>,
) -> impl Fn(Appearance, &mut Window, &mut App) + 'static {
    move |appearance, window, cx| {
        let _ = shell.update(cx, |shell, cx| shell.set_appearance(appearance, window, cx));
    }
}

/// Starts with keyboard focus, so the arrow keys work before anything is clicked.
fn new_play_page(window: &mut Window, cx: &mut Context<AppShell>) -> Entity<PlayPage> {
    let play = cx.new(PlayPage::new);
    let focus = play.read(cx).focus_handle();
    focus.focus(window, cx);
    play
}
