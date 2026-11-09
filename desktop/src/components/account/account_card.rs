use std::rc::Rc;

use gpui_kit::{
    assets::IconName,
    component::{
        ActiveTheme as _, Sizable as _,
        button::{Button, ButtonVariants},
        h_flex,
        label::Label,
        v_flex,
    },
    prelude::FluentBuilder as _,
    *,
};

use super::form_field::FormField;
use crate::{
    components::{
        panel::{PanelHeader, StartButton},
        shell::PLAY_PICTURE,
        social::avatar,
    },
    enums::account::AccountForm,
};

type OnClick = Rc<dyn Fn(&mut Window, &mut App)>;

/// The sign-in or create-account card: its fields, its complaint, the button, and the other form.
#[derive(IntoElement)]
pub struct AccountCard {
    form: AccountForm,
    fields: Vec<FormField>,
    problem: Option<String>,
    busy: bool,
    on_submit: OnClick,
    on_switch: OnClick,
    on_guest: OnClick,
}

impl AccountCard {
    pub fn new(
        form: AccountForm,
        fields: Vec<FormField>,
        problem: Option<String>,
        busy: bool,
    ) -> Self {
        Self {
            form,
            fields,
            problem,
            busy,
            on_submit: Rc::new(|_, _| {}),
            on_switch: Rc::new(|_, _| {}),
            on_guest: Rc::new(|_, _| {}),
        }
    }

    pub fn on_submit(mut self, handler: impl Fn(&mut Window, &mut App) + 'static) -> Self {
        self.on_submit = Rc::new(handler);
        self
    }

    pub fn on_switch(mut self, handler: impl Fn(&mut Window, &mut App) + 'static) -> Self {
        self.on_switch = Rc::new(handler);
        self
    }

    pub fn on_guest(mut self, handler: impl Fn(&mut Window, &mut App) + 'static) -> Self {
        self.on_guest = Rc::new(handler);
        self
    }
}

impl RenderOnce for AccountCard {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let (on_submit, on_switch, on_guest) = (self.on_submit, self.on_switch, self.on_guest);
        let body = v_flex()
            .gap_4()
            .px_6()
            .py_5()
            .children(self.fields)
            .when_some(self.problem, |this, problem| {
                this.child(div().text_sm().text_color(cx.theme().danger).child(problem))
            })
            .child(
                StartButton::new("account-submit", self.form.title(), move |window, cx| {
                    on_submit(window, cx)
                })
                .busy(self.busy),
            )
            .child(
                Button::new("account-switch")
                    .ghost()
                    .w_full()
                    .label(self.form.switch_label())
                    .on_click(move |_, window, cx| on_switch(window, cx)),
            )
            .child(or_rule(cx))
            .child(
                Button::new("continue-as-guest")
                    .w_full()
                    .large()
                    .icon(IconName::User)
                    .label("Continue as guest")
                    .on_click(move |_, window, cx| on_guest(window, cx)),
            );
        titled_card(self.form.title(), cx).child(body)
    }
}

/// A guest plays on this computer only; an account is a click away.
#[derive(IntoElement)]
pub struct GuestCard {
    on_sign_in: OnClick,
    on_sign_up: OnClick,
}

impl GuestCard {
    pub fn new(
        on_sign_in: impl Fn(&mut Window, &mut App) + 'static,
        on_sign_up: impl Fn(&mut Window, &mut App) + 'static,
    ) -> Self {
        Self {
            on_sign_in: Rc::new(on_sign_in),
            on_sign_up: Rc::new(on_sign_up),
        }
    }
}

impl RenderOnce for GuestCard {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let (on_sign_in, on_sign_up) = (self.on_sign_in, self.on_sign_up);
        titled_card("Playing as a guest", cx).child(
            v_flex()
                .gap_4()
                .px_6()
                .py_5()
                .child(
                    Label::new("Sign in or create an account to play online and keep a rating.")
                        .text_sm()
                        .text_color(cx.theme().muted_foreground),
                )
                .child(StartButton::new(
                    "guest-sign-in",
                    AccountForm::SignIn.title(),
                    move |window, cx| on_sign_in(window, cx),
                ))
                .child(
                    Button::new("guest-sign-up")
                        .w_full()
                        .large()
                        .label(AccountForm::SignUp.title())
                        .on_click(move |_, window, cx| on_sign_up(window, cx)),
                ),
        )
    }
}

/// Who is signed in, across the page: their picture, name and email, Sign out and Delete.
#[derive(IntoElement)]
pub struct SignedInCard {
    username: String,
    email: String,
    /// Their picture's asset path; none shows their initial.
    picture: Option<String>,
    /// A new picture is on its way up.
    uploading: bool,
    /// Why the last picture was not taken.
    picture_problem: Option<String>,
    on_sign_out: OnClick,
    on_delete: OnClick,
    on_change_picture: OnClick,
    on_remove_picture: OnClick,
}

impl SignedInCard {
    pub fn new(
        (username, email, picture): (String, String, Option<String>),
        on_sign_out: impl Fn(&mut Window, &mut App) + 'static,
        on_delete: impl Fn(&mut Window, &mut App) + 'static,
    ) -> Self {
        Self {
            username,
            email,
            picture,
            uploading: false,
            picture_problem: None,
            on_sign_out: Rc::new(on_sign_out),
            on_delete: Rc::new(on_delete),
            on_change_picture: Rc::new(|_, _| {}),
            on_remove_picture: Rc::new(|_, _| {}),
        }
    }

    /// Change picture picks a file to upload; Remove goes back to the initial.
    pub fn picture_actions(
        mut self,
        (uploading, problem): (bool, Option<String>),
        on_change: impl Fn(&mut Window, &mut App) + 'static,
        on_remove: impl Fn(&mut Window, &mut App) + 'static,
    ) -> Self {
        self.uploading = uploading;
        self.picture_problem = problem;
        self.on_change_picture = Rc::new(on_change);
        self.on_remove_picture = Rc::new(on_remove);
        self
    }
}

impl RenderOnce for SignedInCard {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let (on_sign_out, on_delete) = (self.on_sign_out, self.on_delete);
        let theme = cx.theme();
        let who = v_flex()
            .flex_1()
            .min_w_0()
            .gap_1()
            .child(
                div()
                    .text_2xl()
                    .font_weight(FontWeight::BOLD)
                    .child(self.username.clone()),
            )
            .child(
                div()
                    .text_sm()
                    .text_color(theme.muted_foreground)
                    .child(self.email),
            );
        let actions = h_flex()
            .flex_shrink_0()
            .gap_2()
            .child(
                Button::new("delete-account")
                    .ghost()
                    .text_color(theme.danger)
                    .label("Delete account")
                    .on_click(move |_, window, cx| on_delete(window, cx)),
            )
            .child(
                StartButton::new("sign-out", "Sign out", move |window, cx| {
                    on_sign_out(window, cx);
                })
                .icon(IconName::LogOut)
                .danger()
                .small(),
            );
        h_flex()
            .w_full()
            .gap_5()
            .p_5()
            .rounded(px(8.))
            .bg(theme.group_box)
            .child(picture_column(
                (&self.username, self.picture.as_deref(), self.uploading),
                self.picture_problem,
                (self.on_change_picture, self.on_remove_picture),
                cx,
            ))
            .child(who)
            .child(actions)
    }
}

/// The player's picture with Change picture under it, and Remove once there is one.
fn picture_column(
    (name, picture, uploading): (&str, Option<&str>, bool),
    problem: Option<String>,
    (on_change, on_remove): (OnClick, OnClick),
    cx: &App,
) -> Div {
    let has_picture = picture.is_some();
    let danger = cx.theme().danger;
    v_flex()
        .flex_shrink_0()
        .items_center()
        .gap_1()
        .child(avatar(name, picture, px(72.), cx))
        .child(
            Button::new("change-picture")
                .ghost()
                .xsmall()
                .label("Change picture")
                .loading(uploading)
                .on_click(move |_, window, cx| on_change(window, cx)),
        )
        .when(has_picture && !uploading, |this| {
            this.child(
                Button::new("remove-picture")
                    .ghost()
                    .xsmall()
                    .text_color(cx.theme().muted_foreground)
                    .label("Remove")
                    .on_click(move |_, window, cx| on_remove(window, cx)),
            )
        })
        .when_some(problem, |this, problem| {
            this.child(
                div()
                    .max_w(px(140.))
                    .text_xs()
                    .text_center()
                    .text_color(danger)
                    .child(problem),
            )
        })
}

/// Deleting the account once the password is typed again: what goes, what stays, the red button.
#[derive(IntoElement)]
pub struct CloseAccountCard {
    password: FormField,
    busy: bool,
    on_confirm: OnClick,
    on_cancel: OnClick,
}

impl CloseAccountCard {
    pub fn new(
        password: FormField,
        busy: bool,
        on_confirm: impl Fn(&mut Window, &mut App) + 'static,
        on_cancel: impl Fn(&mut Window, &mut App) + 'static,
    ) -> Self {
        Self {
            password,
            busy,
            on_confirm: Rc::new(on_confirm),
            on_cancel: Rc::new(on_cancel),
        }
    }
}

impl RenderOnce for CloseAccountCard {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let (on_confirm, on_cancel) = (self.on_confirm, self.on_cancel);
        card(cx)
            .child(Label::new("Delete your account?").text_xl())
            .child(
                div()
                    .text_sm()
                    .text_color(cx.theme().muted_foreground)
                    .child(
                        "Your friends, chats, ratings and puzzle scores are deleted, and you \
                         cannot sign in again. Your games stay for your opponents, under a \
                         removed name.",
                    ),
            )
            .child(self.password)
            .child(
                Button::new("confirm-delete-account")
                    .danger()
                    .w_full()
                    .label("Delete my account")
                    .loading(self.busy)
                    .on_click(move |_, window, cx| on_confirm(window, cx)),
            )
            .child(
                Button::new("cancel-delete-account")
                    .ghost()
                    .w_full()
                    .label("Cancel")
                    .on_click(move |_, window, cx| on_cancel(window, cx)),
            )
    }
}

/// A moment of waiting, said in a sentence.
#[derive(IntoElement)]
pub struct WaitingCard {
    message: &'static str,
}

impl WaitingCard {
    pub fn new(message: &'static str) -> Self {
        Self { message }
    }
}

impl RenderOnce for WaitingCard {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        card(cx).child(
            Label::new(self.message)
                .text_sm()
                .text_color(cx.theme().muted_foreground),
        )
    }
}

/// A card with the panels' title band over it: the Play picture and `title`.
fn titled_card(title: &'static str, cx: &App) -> Div {
    v_flex()
        .w(px(420.))
        .rounded(px(8.))
        .overflow_hidden()
        .bg(cx.theme().group_box)
        .child(PanelHeader::new(PLAY_PICTURE, title))
}

/// A thin line with "or" in its middle, between signing in and playing as a guest.
fn or_rule(cx: &App) -> Div {
    let theme = cx.theme();
    let line = || div().flex_1().h_px().bg(theme.border);
    h_flex()
        .gap_3()
        .text_xs()
        .text_color(theme.muted_foreground)
        .child(line())
        .child("or")
        .child(line())
}

/// The account's cards, as the app's panels: their colour and corners.
fn card(cx: &App) -> Div {
    v_flex()
        .w_96()
        .gap_4()
        .p_6()
        .rounded(px(8.))
        .bg(cx.theme().group_box)
}
