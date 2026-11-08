use std::rc::Rc;

use gpui_kit::{
    assets::IconName,
    component::{
        ActiveTheme as _,
        button::{Button, ButtonRounded, ButtonVariants},
        h_flex,
        popover::{Popover, PopoverState},
        v_flex,
    },
    *,
};

use crate::enums::{account::AccountForm, shell::Page};

type OnPage = Rc<dyn Fn(Page, &mut Window, &mut App)>;
type OnClick = Rc<dyn Fn(&mut Window, &mut App)>;
type OnLeaveGuest = Rc<dyn Fn(AccountForm, &mut Window, &mut App)>;

/// Who is signed in, as the menu shows them.
pub struct MenuPerson {
    pub name: String,
    /// `None` for a guest, who has no email.
    pub email: Option<String>,
    pub guest: bool,
}

/// The avatar at the right of the header and the panel it opens: who is signed in, Account and
/// Settings, and Sign out; a guest gets Create account and Sign in instead.
#[derive(IntoElement)]
pub struct AccountMenu {
    person: MenuPerson,
    on_page: OnPage,
    on_sign_out: OnClick,
    on_leave_guest: OnLeaveGuest,
}

impl AccountMenu {
    pub fn new(
        person: MenuPerson,
        on_page: impl Fn(Page, &mut Window, &mut App) + 'static,
        on_sign_out: impl Fn(&mut Window, &mut App) + 'static,
        on_leave_guest: impl Fn(AccountForm, &mut Window, &mut App) + 'static,
    ) -> Self {
        Self {
            person,
            on_page: Rc::new(on_page),
            on_sign_out: Rc::new(on_sign_out),
            on_leave_guest: Rc::new(on_leave_guest),
        }
    }
}

impl RenderOnce for AccountMenu {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let initial = initial_of(&self.person.name);
        let trigger = Button::new("account-menu")
            .ghost()
            .rounded(ButtonRounded::Size(px(16.)))
            .p_0()
            .child(avatar(&initial, px(32.), cx));
        let Self {
            person,
            on_page,
            on_sign_out,
            on_leave_guest,
        } = self;
        let person = Rc::new(person);
        Popover::new("account-menu-panel")
            .anchor(Anchor::TopRight)
            .trigger(trigger)
            .w(px(288.))
            .p_0()
            .content(move |_, _, cx| {
                let close = closer(cx);
                let mut items = v_flex().p_1();
                for page in [Page::Account, Page::Settings] {
                    let (on_page, close) = (Rc::clone(&on_page), Rc::clone(&close));
                    items = items.child(item(page.label(), page_icon(page)).on_click(
                        move |_, window, cx| {
                            on_page(page, window, cx);
                            close(window, cx);
                        },
                    ));
                }
                let ending: Vec<Button> = if person.guest {
                    [
                        (AccountForm::SignUp, "Create account", IconName::UserPlus),
                        (AccountForm::SignIn, "Sign in", IconName::LogIn),
                    ]
                    .into_iter()
                    .map(|(form, label, icon)| {
                        let (leave, close) = (Rc::clone(&on_leave_guest), Rc::clone(&close));
                        item(label, icon).on_click(move |_, window, cx| {
                            close(window, cx);
                            leave(form, window, cx);
                        })
                    })
                    .collect()
                } else {
                    let (sign_out, close) = (Rc::clone(&on_sign_out), Rc::clone(&close));
                    vec![
                        item("Sign out", IconName::LogOut).on_click(move |_, window, cx| {
                            close(window, cx);
                            sign_out(window, cx);
                        }),
                    ]
                };
                v_flex()
                    .child(who(&person, cx))
                    .child(rule(cx))
                    .child(items)
                    .child(rule(cx))
                    .child(v_flex().p_1().children(ending))
            })
    }
}

/// The panel's head: a large avatar, the name and the email.
fn who(person: &MenuPerson, cx: &App) -> Div {
    let detail = person
        .email
        .clone()
        .unwrap_or_else(|| "Playing as a guest".to_owned());
    h_flex()
        .gap_3()
        .p_3()
        .child(avatar(&initial_of(&person.name), px(48.), cx))
        .child(
            v_flex()
                .min_w_0()
                .child(
                    div()
                        .font_weight(FontWeight::SEMIBOLD)
                        .truncate()
                        .child(person.name.clone()),
                )
                .child(
                    div()
                        .text_sm()
                        .truncate()
                        .text_color(cx.theme().muted_foreground)
                        .child(detail),
                ),
        )
}

/// A circle with the person's initial, tinted with the theme's primary colour.
fn avatar(initial: &str, size: Pixels, cx: &App) -> Div {
    let primary = cx.theme().primary;
    div()
        .size(size)
        .flex_shrink_0()
        .flex()
        .items_center()
        .justify_center()
        .rounded_full()
        .bg(primary.opacity(0.15))
        .text_color(primary)
        .text_size(size * 0.45)
        .font_weight(FontWeight::SEMIBOLD)
        .child(initial.to_owned())
}

fn item(label: &'static str, icon: IconName) -> Button {
    Button::new(label)
        .ghost()
        .w_full()
        .justify_start()
        .icon(icon)
        .label(label)
}

fn rule(cx: &App) -> Div {
    div().h_px().w_full().bg(cx.theme().border)
}

/// Closes the panel, after one of its items is chosen.
fn closer(cx: &mut Context<PopoverState>) -> OnClick {
    let panel = cx.entity().downgrade();
    Rc::new(move |window, cx| {
        // Gone already only if the window is closing.
        let _ = panel.update(cx, |panel, cx| panel.dismiss(window, cx));
    })
}

/// The first letter of the name, capital.
fn initial_of(name: &str) -> String {
    name.trim()
        .chars()
        .next()
        .map(|first| first.to_uppercase().collect())
        .unwrap_or_default()
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
