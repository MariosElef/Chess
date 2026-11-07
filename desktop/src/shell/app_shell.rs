use gpui_kit::{component::h_flex, *};

use super::{nav_sidebar::NavSidebar, page::Page};
use crate::{
    play::PlayPage,
    puzzles::PuzzlesPage,
    settings::{Appearance, SettingsPage},
};

pub struct AppShell {
    page: Page,
    appearance: Appearance,
    _appearance_changes: Subscription,
}

impl AppShell {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let appearance = Appearance::default();
        appearance.apply(window, cx);
        let _appearance_changes = cx.observe_window_appearance(window, |shell, window, cx| {
            if shell.appearance == Appearance::System {
                Appearance::System.apply(window, cx);
            }
        });

        Self {
            page: Page::Play,
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
        cx.notify();
    }
}

impl Render for AppShell {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        // The shell outlives the elements it renders; a failed update means the window is closing.
        let shell = cx.entity().downgrade();
        let open = {
            let shell = shell.clone();
            move |page, _: &mut Window, cx: &mut App| {
                let _ = shell.update(cx, |shell, cx| shell.open(page, cx));
            }
        };

        let content = match self.page {
            Page::Play => PlayPage.into_any_element(),
            Page::Puzzles => PuzzlesPage.into_any_element(),
            Page::Settings => SettingsPage::new(self.appearance, move |appearance, window, cx| {
                let _ = shell.update(cx, |shell, cx| shell.set_appearance(appearance, window, cx));
            })
            .into_any_element(),
        };

        h_flex()
            .size_full()
            .items_stretch()
            .child(NavSidebar::new(self.page, open))
            .child(div().flex_1().min_w_0().child(content))
    }
}
