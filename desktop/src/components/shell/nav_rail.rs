use std::rc::Rc;

use gpui_kit::{
    component::{ActiveTheme as _, v_flex},
    prelude::FluentBuilder as _,
    *,
};

use super::nav_pictures::page_picture;
use crate::enums::shell::Page;

type OnSelect = Rc<dyn Fn(Page, &mut Window, &mut App)>;

/// The rail down the left: each page a tile with its picture and its name under it; the current
/// page's tile is filled. It shares the header's colour, the two making one L around the page.
#[derive(IntoElement)]
pub struct NavRail {
    active: Page,
    /// Challenges, friend requests and unread messages, shown on Social.
    waiting: usize,
    on_select: OnSelect,
}

impl NavRail {
    pub fn new(active: Page, on_select: impl Fn(Page, &mut Window, &mut App) + 'static) -> Self {
        Self {
            active,
            waiting: 0,
            on_select: Rc::new(on_select),
        }
    }

    pub fn waiting(mut self, waiting: usize) -> Self {
        self.waiting = waiting;
        self
    }
}

impl RenderOnce for NavRail {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let items: Vec<Stateful<Div>> = Page::NAV
            .into_iter()
            .map(|page| self.item(page, cx))
            .collect();
        v_flex()
            .h_full()
            .w(px(72.))
            .flex_shrink_0()
            .items_center()
            .gap_3()
            .pt_5()
            .bg(cx.theme().sidebar)
            .children(items)
    }
}

impl NavRail {
    fn item(&self, page: Page, cx: &App) -> Stateful<Div> {
        let on_select = Rc::clone(&self.on_select);
        let active = page == self.active;
        let waiting = (page == Page::Social && self.waiting > 0).then_some(self.waiting);
        let theme = cx.theme();
        // mc2's active tile: a card on the light chrome, the accent on the dark.
        let filled = if theme.is_dark() {
            theme.accent
        } else {
            theme.popover
        };
        let tile = div()
            .relative()
            .size_9()
            .flex()
            .items_center()
            .justify_center()
            .rounded(px(12.))
            .when(active, |this| this.bg(filled))
            .when_some(page_picture(page), |this, path| {
                this.child(img(path).size_5())
            })
            .when_some(waiting, |this, waiting| {
                this.child(count_badge(waiting, cx))
            });
        v_flex()
            .id(page.label())
            .w_full()
            .items_center()
            .gap_1p5()
            .cursor_pointer()
            .text_color(theme.sidebar_foreground)
            .child(tile)
            .child(div().text_xs().line_height(px(12.)).child(page.label()))
            .on_click(move |_, window, cx| on_select(page, window, cx))
    }
}

/// A small red count on a tile's corner.
fn count_badge(count: usize, cx: &App) -> Div {
    div()
        .absolute()
        .top(px(-4.))
        .right(px(-6.))
        .px_1()
        .min_w(px(16.))
        .flex()
        .justify_center()
        .rounded_full()
        .bg(cx.theme().red)
        .text_color(white())
        .text_xs()
        .line_height(px(16.))
        .child(count.to_string())
}
