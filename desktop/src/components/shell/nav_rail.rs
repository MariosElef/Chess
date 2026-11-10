use std::{rc::Rc, time::Duration};

use gpui_kit::{
    component::{ActiveTheme as _, h_flex, hover_card::HoverCard, v_flex},
    prelude::FluentBuilder as _,
    *,
};

use super::nav_pictures::page_picture;
use crate::enums::shell::Page;

type OnSelect = Rc<dyn Fn(Page, &mut Window, &mut App)>;

/// The rail's width, and an item's tile, the gap under it and its name's height.
const RAIL_WIDTH: Pixels = px(72.);
const TILE: Pixels = px(36.);
const LABEL_GAP: Pixels = px(6.);
const LABEL_HEIGHT: Pixels = px(12.);
/// The space between the rail's edge and the tip of a menu's point.
const MENU_GAP: Pixels = px(6.);
/// How far the menu's point sticks out to the left.
const CARET_WIDTH: Pixels = px(8.);

/// How long the pointer rests on an item before its menu opens.
const MENU_DELAY: Duration = Duration::from_millis(150);

/// The rail down the left: a tile per page with its picture and name, the current one filled.
#[derive(IntoElement)]
pub struct NavRail {
    active: Page,
    /// What waits on each page, as a red count on its tile.
    counts: Vec<(Page, usize)>,
    on_select: OnSelect,
}

impl NavRail {
    pub fn new(active: Page, on_select: impl Fn(Page, &mut Window, &mut App) + 'static) -> Self {
        Self {
            active,
            counts: Vec::new(),
            on_select: Rc::new(on_select),
        }
    }

    pub fn count(mut self, page: Page, count: usize) -> Self {
        self.counts.push((page, count));
        self
    }
}

impl RenderOnce for NavRail {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let items: Vec<AnyElement> = Page::NAV
            .into_iter()
            .map(|page| self.with_menu(page, cx))
            .collect();
        v_flex()
            .h_full()
            .w(RAIL_WIDTH)
            .flex_shrink_0()
            .items_center()
            .gap_3()
            .pt_5()
            .bg(cx.theme().sidebar)
            .children(items)
    }
}

impl NavRail {
    /// What waits on `page` itself, for its row in a menu.
    fn own_count(&self, page: Page) -> usize {
        self.counts
            .iter()
            .filter(|(counted, _)| *counted == page)
            .map(|(_, count)| count)
            .sum()
    }

    /// What waits on `page`, counting the pages in its menu too, for its tile.
    fn waiting(&self, page: Page) -> usize {
        self.counts
            .iter()
            .filter(|(counted, _)| *counted == page || page.menu().contains(counted))
            .map(|(_, count)| count)
            .sum()
    }

    /// The item, and for one with a menu, the menu beside it while hovered, as chess.com's.
    fn with_menu(&self, page: Page, cx: &App) -> AnyElement {
        let item = self.item(page, cx);
        if page.menu().is_empty() {
            return item.into_any_element();
        }
        let entries: Vec<(Page, usize)> = page
            .menu()
            .iter()
            .map(|entry| (*entry, self.own_count(*entry)))
            .collect();
        let (on_select, active) = (Rc::clone(&self.on_select), self.active);
        HoverCard::new(SharedString::from(format!("rail-menu-{}", page.key())))
            .anchor(Anchor::LeftCenter)
            .open_delay(MENU_DELAY)
            .appearance(false)
            .trigger(item)
            .content(move |_, _, cx| {
                let rows: Vec<Stateful<Div>> = entries
                    .iter()
                    .map(|(entry, count)| {
                        menu_row(
                            *entry,
                            (*entry == active, *count),
                            Rc::clone(&on_select),
                            cx,
                        )
                    })
                    .collect();
                menu_card(rows, cx)
            })
            .into_any_element()
    }

    fn item(&self, page: Page, cx: &App) -> Stateful<Div> {
        let on_select = Rc::clone(&self.on_select);
        let active = page == self.active.rail_item();
        let waiting = Some(self.waiting(page)).filter(|count| *count > 0);
        let theme = cx.theme();
        // mc2's active tile: a card on the light chrome, the accent on the dark.
        let filled = if theme.is_dark() {
            theme.accent
        } else {
            theme.popover
        };
        let tile = div()
            .relative()
            .size(TILE)
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
            .id(page.key())
            .debug_selector(move || format!("rail-{}", page.key()))
            // As wide as the rail, so a menu beside it starts past the rail's edge.
            .w(RAIL_WIDTH)
            .items_center()
            .gap(LABEL_GAP)
            .text_color(theme.sidebar_foreground)
            .child(tile)
            .child(
                div()
                    .text_xs()
                    .line_height(LABEL_HEIGHT)
                    .child(page.label()),
            )
            // An item with a menu opens nothing itself: its pages are picked from the menu.
            .when(page.menu().is_empty(), |this| {
                this.cursor_pointer()
                    .on_click(move |_, window, cx| on_select(page, window, cx))
            })
    }
}

/// The menu's card, with a small point on its left edge at the item it came out of.
fn menu_card(rows: Vec<Stateful<Div>>, cx: &App) -> Div {
    let theme = cx.theme();
    let (ground, edge) = (theme.popover, theme.border);
    // Clear of the rail, and lifted so the point meets the middle of the tile, not of the tile
    // and its name together.
    div()
        .pl(MENU_GAP + CARET_WIDTH)
        .pb(LABEL_GAP + LABEL_HEIGHT)
        .child(
            v_flex()
                .relative()
                .w(px(200.))
                .p_1()
                .rounded(px(8.))
                .border_1()
                .border_color(edge)
                .bg(ground)
                .shadow_lg()
                .children(rows)
                .child(
                    div()
                        .absolute()
                        .left(-CARET_WIDTH)
                        .top_0()
                        .bottom_0()
                        .flex()
                        .items_center()
                        .child(caret(ground, edge)),
                ),
        )
}

/// The point: a triangle in the card's colour and outline, its base over the card's edge.
fn caret(ground: Hsla, edge: Hsla) -> Canvas<()> {
    canvas(
        |_, _, _| {},
        move |bounds, (), window, _| {
            paint_triangle(bounds, px(0.), edge, window);
            paint_triangle(bounds, px(1.5), ground, window);
        },
    )
    .w(CARET_WIDTH + px(1.))
    .h(CARET_WIDTH * 2.)
}

/// A triangle pointing left, its tip `inset` in from the left of `bounds`.
fn paint_triangle(bounds: Bounds<Pixels>, inset: Pixels, colour: Hsla, window: &mut Window) {
    let (left, right) = (bounds.left() + inset, bounds.right());
    let middle = bounds.center().y;
    let rise = (right - left) * (bounds.size.height / bounds.size.width);
    let mut path = PathBuilder::fill();
    path.move_to(point(left, middle));
    path.line_to(point(right, middle - rise / 2.));
    path.line_to(point(right, middle + rise / 2.));
    path.close();
    if let Ok(path) = path.build() {
        window.paint_path(path, colour);
    }
}

/// A page in a rail item's menu: its icon, its name, and its count; the open one filled.
fn menu_row(
    page: Page,
    (open, count): (bool, usize),
    on_select: OnSelect,
    cx: &App,
) -> Stateful<Div> {
    let theme = cx.theme();
    let hover = theme.accent;
    h_flex()
        .id(SharedString::from(format!("rail-menu-{}", page.menu_key())))
        .debug_selector(move || format!("rail-menu-{}", page.menu_key()))
        .gap_3()
        .px_3()
        .py_2()
        .rounded(px(6.))
        .cursor_pointer()
        .font_weight(FontWeight::SEMIBOLD)
        .when(open, |this| this.bg(hover))
        .hover(move |this| this.bg(hover))
        .when_some(page_picture(page), |this, path| {
            this.child(img(path).size_5())
        })
        .child(div().flex_1().child(page.menu_label()))
        .when(count > 0, |this| {
            this.child(
                div()
                    .px_1p5()
                    .rounded_full()
                    .bg(theme.red)
                    .text_color(white())
                    .text_xs()
                    .child(count.to_string()),
            )
        })
        .on_click(move |_, window, cx| on_select(page, window, cx))
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

#[cfg(test)]
mod tests {
    use super::NavRail;
    use crate::enums::shell::Page;

    #[test]
    fn a_tile_counts_its_menu_and_each_row_only_its_own() {
        let rail = NavRail::new(Page::Play, |_, _, _| {})
            .count(Page::Social, 2)
            .count(Page::Messages, 3);
        assert_eq!(rail.waiting(Page::Social), 5, "the Social tile");
        assert_eq!(rail.own_count(Page::Social), 2, "Friends in the menu");
        assert_eq!(rail.own_count(Page::Messages), 3, "Messages in the menu");
    }
}
