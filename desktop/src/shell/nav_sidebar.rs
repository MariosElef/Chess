use std::rc::Rc;

use gpui_kit::{
    assets::IconName,
    component::{
        Icon, Selectable,
        button::{Button, ButtonVariants},
        h_flex,
        popover::{Popover, PopoverState},
        sidebar::{Sidebar, SidebarFooter, SidebarHeader, SidebarMenu, SidebarMenuItem},
    },
    *,
};

use super::page::Page;

type OnSelect = Rc<dyn Fn(Page, &mut Window, &mut App)>;

#[derive(IntoElement)]
pub struct NavSidebar {
    active: Page,
    on_select: OnSelect,
}

impl NavSidebar {
    pub fn new(active: Page, on_select: impl Fn(Page, &mut Window, &mut App) + 'static) -> Self {
        Self {
            active,
            on_select: Rc::new(on_select),
        }
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
    fn nav_items(&self) -> [SidebarMenuItem; 3] {
        Page::NAV.map(|page| {
            let on_select = Rc::clone(&self.on_select);
            SidebarMenuItem::new(page.label())
                .icon(page.icon())
                .active(page == self.active)
                .on_click(move |_, window, cx| on_select(page, window, cx))
        })
    }

    /// The profile row at the bottom; it opens a small menu upward with Settings.
    fn profile_menu(self) -> Popover {
        let on_select = self.on_select;
        Popover::new("profile-menu")
            .anchor(Anchor::BottomLeft)
            .trigger(profile_row(self.active == Page::Settings))
            .trigger_style(StyleRefinement::default().w_full())
            .w_48()
            .content(move |_, _, cx| settings_button(Rc::clone(&on_select), cx))
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
fn profile_row(selected: bool) -> SidebarFooter {
    SidebarFooter::new()
        .h_3()
        .text_sm()
        .selected(selected)
        .child(Icon::from(IconName::CircleUser))
        .child("Player")
}

/// Opens Settings and closes the profile menu it sits in.
fn settings_button(on_select: OnSelect, cx: &mut Context<PopoverState>) -> Button {
    let popover = cx.entity().downgrade();
    Button::new("open-settings")
        .ghost()
        .w_full()
        .justify_start()
        .icon(Page::Settings.icon())
        .label(Page::Settings.label())
        .on_click(move |_, window, cx| {
            on_select(Page::Settings, window, cx);
            let _ = popover.update(cx, |popover, cx| popover.dismiss(window, cx));
        })
}
