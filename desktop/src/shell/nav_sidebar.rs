use std::rc::Rc;

use gpui_kit::{
    assets::IconName,
    component::{
        Icon, Selectable,
        button::{Button, ButtonVariants},
        h_flex,
        popover::Popover,
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
        let items = Page::NAV.map(|page| {
            let on_select = Rc::clone(&self.on_select);
            SidebarMenuItem::new(page.label())
                .icon(page.icon())
                .active(page == self.active)
                .on_click(move |_, window, cx| on_select(page, window, cx))
        });

        let on_select = self.on_select;
        let profile = Popover::new("profile-menu")
            .anchor(Anchor::BottomLeft)
            .trigger(
                // Matches SidebarMenuItem's h_7 with p_2: the footer adds the p_2 around this row.
                SidebarFooter::new()
                    .h_3()
                    .text_sm()
                    .selected(self.active == Page::Settings)
                    .child(Icon::from(IconName::CircleUser))
                    .child("Player"),
            )
            .trigger_style(StyleRefinement::default().w_full())
            .w_48()
            .content(move |_, _, cx| {
                let on_select = Rc::clone(&on_select);
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
            });

        Sidebar::new("nav-sidebar")
            .collapsible(false)
            .header(
                SidebarHeader::new().child(
                    h_flex()
                        .gap_2()
                        .child(Icon::from(IconName::ChessKnight))
                        .child("Chess"),
                ),
            )
            .child(SidebarMenu::new().children(items))
            .footer(profile)
    }
}
