use std::rc::Rc;

use gpui_kit::{
    assets::IconName,
    component::{
        ActiveTheme as _, Icon, Selectable,
        button::{Button, ButtonVariants},
        h_flex,
        popover::{Popover, PopoverState},
        sidebar::{Sidebar, SidebarFooter, SidebarHeader, SidebarMenu, SidebarMenuItem},
        v_flex,
    },
    prelude::FluentBuilder as _,
    *,
};

use crate::enums::shell::Page;

type OnSelect = Rc<dyn Fn(Page, &mut Window, &mut App)>;

#[derive(IntoElement)]
pub struct NavSidebar {
    active: Page,
    /// The signed-in username or "Guest", shown on the profile row; `None` shows "Player".
    player: Option<String>,
    /// Challenges and friend requests waiting, shown on Social.
    waiting: usize,
    on_select: OnSelect,
}

impl NavSidebar {
    pub fn new(
        active: Page,
        player: Option<String>,
        on_select: impl Fn(Page, &mut Window, &mut App) + 'static,
    ) -> Self {
        Self {
            active,
            player,
            waiting: 0,
            on_select: Rc::new(on_select),
        }
    }

    pub fn waiting(mut self, waiting: usize) -> Self {
        self.waiting = waiting;
        self
    }
}

impl RenderOnce for NavSidebar {
    fn render(self, _: &mut Window, _: &mut App) -> impl IntoElement {
        Sidebar::new("nav-sidebar")
            .collapsible(false)
            .header(app_header())
            .child(SidebarMenu::new().children(self.nav_items()))
            .footer(self.profile_menu())
    }
}

impl NavSidebar {
    fn nav_items(&self) -> [SidebarMenuItem; 4] {
        let waiting = self.waiting;
        Page::NAV.map(|page| {
            let on_select = Rc::clone(&self.on_select);
            SidebarMenuItem::new(page.label())
                .icon(page_icon(page))
                .active(page == self.active)
                .when(page == Page::Social && waiting > 0, |item| {
                    item.suffix(move |_, cx| count_pill(waiting, cx))
                })
                .on_click(move |_, window, cx| on_select(page, window, cx))
        })
    }

    /// The profile row at the bottom; it opens a small menu upward with Account and Settings.
    fn profile_menu(self) -> Popover {
        let on_select = self.on_select;
        let selected = matches!(self.active, Page::Settings | Page::Account);
        Popover::new("profile-menu")
            .anchor(Anchor::BottomLeft)
            .trigger(profile_row(self.player, selected))
            .trigger_style(StyleRefinement::default().w_full())
            .w_48()
            .content(move |_, _, cx| {
                v_flex()
                    .child(menu_button(Page::Account, Rc::clone(&on_select), cx))
                    .child(menu_button(Page::Settings, Rc::clone(&on_select), cx))
            })
    }
}

fn app_header() -> SidebarHeader {
    SidebarHeader::new().child(
        h_flex()
            .gap_2()
            .child(Icon::from(IconName::ChessKnight))
            .child("Chess"),
    )
}

/// Matches SidebarMenuItem's h_7 with p_2: the footer adds the p_2 around this row.
fn profile_row(player: Option<String>, selected: bool) -> SidebarFooter {
    SidebarFooter::new()
        .h_3()
        .text_sm()
        .selected(selected)
        .child(Icon::from(IconName::CircleUser))
        .child(player.unwrap_or_else(|| "Player".to_owned()))
}

/// Opens `page` and closes the profile menu it sits in.
fn menu_button(page: Page, on_select: OnSelect, cx: &mut Context<PopoverState>) -> Button {
    let popover = cx.entity().downgrade();
    Button::new(page.label())
        .ghost()
        .w_full()
        .justify_start()
        .icon(page_icon(page))
        .label(page.label())
        .on_click(move |_, window, cx| {
            on_select(page, window, cx);
            let _ = popover.update(cx, |popover, cx| popover.dismiss(window, cx));
        })
}

/// A small red count, for what waits on a page.
fn count_pill(count: usize, cx: &App) -> Div {
    div()
        .px_1p5()
        .rounded_full()
        .bg(cx.theme().red)
        .text_color(white())
        .text_xs()
        .child(count.to_string())
}

fn page_icon(page: Page) -> IconName {
    match page {
        Page::Play => IconName::ChessKnight,
        Page::Puzzles => IconName::Puzzle,
        Page::Analyze => IconName::Microscope,
        Page::Social => IconName::Users,
        Page::Settings => IconName::Settings,
        Page::Account => IconName::CircleUser,
    }
}
