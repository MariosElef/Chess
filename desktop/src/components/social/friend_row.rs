use std::rc::Rc;

use gpui_kit::{
    assets::IconName,
    component::{
        ActiveTheme as _, Disableable as _, Sizable as _,
        button::{Button, ButtonVariants},
        h_flex,
        popover::{Popover, PopoverState},
        v_flex,
    },
    prelude::FluentBuilder as _,
    *,
};

use super::avatar::pawn_avatar;

type OnClick = Rc<dyn Fn(&mut Window, &mut App)>;

/// How big a friend's picture is.
const PICTURE: Pixels = px(72.);

/// A friend's row: their picture, when they were last online, and Challenge and Message.
#[derive(IntoElement)]
pub struct FriendRow {
    id: SharedString,
    name: SharedString,
    online: bool,
    seen: SharedString,
    on_challenge: OnClick,
    on_message: OnClick,
    on_remove: OnClick,
}

impl FriendRow {
    pub fn new(
        (id, name): (impl Into<SharedString>, impl Into<SharedString>),
        (online, seen): (bool, impl Into<SharedString>),
        on_challenge: impl Fn(&mut Window, &mut App) + 'static,
        (on_message, on_remove): (
            impl Fn(&mut Window, &mut App) + 'static,
            impl Fn(&mut Window, &mut App) + 'static,
        ),
    ) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            online,
            seen: seen.into(),
            on_challenge: Rc::new(on_challenge),
            on_message: Rc::new(on_message),
            on_remove: Rc::new(on_remove),
        }
    }
}

impl RenderOnce for FriendRow {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = cx.theme();
        let (muted, green, ground) = (theme.muted_foreground, theme.success, theme.muted);
        let picture = div()
            .relative()
            .child(pawn_avatar(PICTURE))
            .when(self.online, |this| {
                this.child(
                    div()
                        .absolute()
                        .right(px(-3.))
                        .bottom(px(-3.))
                        .size_4()
                        .rounded_full()
                        .border_2()
                        .border_color(ground)
                        .bg(green),
                )
            });
        let seen_colour = if self.online { green } else { muted };
        let (challenge, message) = (Rc::clone(&self.on_challenge), Rc::clone(&self.on_message));
        h_flex()
            .w_full()
            .gap_4()
            .py_2()
            .child(picture)
            .child(
                v_flex()
                    .flex_1()
                    .min_w_0()
                    .gap_0p5()
                    .child(
                        div()
                            .text_base()
                            .font_weight(FontWeight::BOLD)
                            .child(self.name.clone()),
                    )
                    .child(div().text_sm().text_color(seen_colour).child(self.seen)),
            )
            .child(
                h_flex()
                    .flex_shrink_0()
                    .gap_1()
                    .child(
                        Button::new(SharedString::from(format!("challenge-{}", self.id)))
                            .ghost()
                            .icon(IconName::Swords)
                            .tooltip("Challenge")
                            .disabled(!self.online)
                            .on_click(move |_, window, cx| challenge(window, cx)),
                    )
                    .child(
                        Button::new(SharedString::from(format!("message-{}", self.id)))
                            .ghost()
                            .icon(IconName::Mail)
                            .tooltip("Message")
                            .on_click(move |_, window, cx| message(window, cx)),
                    )
                    .child(more_menu(&self.id, self.on_remove)),
            )
    }
}

/// The "…" menu: Remove friend.
fn more_menu(id: &SharedString, on_remove: OnClick) -> Popover {
    let trigger = Button::new(SharedString::from(format!("more-{id}")))
        .ghost()
        .icon(IconName::Ellipsis);
    let item_id = SharedString::from(format!("unfriend-{id}"));
    Popover::new(SharedString::from(format!("more-menu-{id}")))
        .anchor(Anchor::TopRight)
        .trigger(trigger)
        .p_1()
        .content(move |_, _, cx| {
            let menu = cx.entity().downgrade();
            let remove = Rc::clone(&on_remove);
            Button::new(item_id.clone())
                .ghost()
                .small()
                .icon(IconName::UserMinus)
                .label("Remove friend")
                .on_click(move |_, window, cx| {
                    remove(window, cx);
                    // Gone already only if the window is closing.
                    let _ = menu.update(cx, |menu: &mut PopoverState, cx| menu.dismiss(window, cx));
                })
        })
}
