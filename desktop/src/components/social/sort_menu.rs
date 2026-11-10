use std::rc::Rc;

use gpui_kit::{
    assets::IconName,
    component::{
        Sizable as _,
        button::{Button, ButtonVariants},
        popover::{Popover, PopoverState},
        v_flex,
    },
    *,
};

use crate::enums::social::FriendSort;

type OnSort = Rc<dyn Fn(FriendSort, &mut Window, &mut App)>;

/// The friends' order, as a small menu at the right of their heading: "Most Recently Online".
#[derive(IntoElement)]
pub struct SortMenu {
    sort: FriendSort,
    on_sort: OnSort,
}

impl SortMenu {
    pub fn new(
        sort: FriendSort,
        on_sort: impl Fn(FriendSort, &mut Window, &mut App) + 'static,
    ) -> Self {
        Self {
            sort,
            on_sort: Rc::new(on_sort),
        }
    }
}

impl RenderOnce for SortMenu {
    fn render(self, _: &mut Window, _: &mut App) -> impl IntoElement {
        let trigger = Button::new("friend-sort")
            .ghost()
            .small()
            .label(self.sort.label())
            .icon(IconName::ChevronDown);
        let on_sort = self.on_sort;
        Popover::new("friend-sort-menu")
            .anchor(Anchor::TopRight)
            .trigger(trigger)
            .p_1()
            .content(move |_, _, cx| {
                let menu = cx.entity().downgrade();
                let items: Vec<Button> = FriendSort::ALL
                    .into_iter()
                    .map(|sort| {
                        let (on_sort, menu) = (Rc::clone(&on_sort), menu.clone());
                        Button::new(SharedString::from(sort.key()))
                            .ghost()
                            .small()
                            .w_full()
                            .justify_start()
                            .label(sort.label())
                            .on_click(move |_, window, cx| {
                                on_sort(sort, window, cx);
                                // Gone already only if the window is closing.
                                let _ = menu.update(cx, |menu: &mut PopoverState, cx| {
                                    menu.dismiss(window, cx)
                                });
                            })
                    })
                    .collect();
                v_flex().children(items)
            })
    }
}
