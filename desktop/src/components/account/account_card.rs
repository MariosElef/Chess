use std::rc::Rc;

use gpui_kit::{
    component::{
        ActiveTheme as _,
        button::{Button, ButtonVariants},
        label::Label,
        v_flex,
    },
    prelude::FluentBuilder as _,
    *,
};

use super::form_field::FormField;
use crate::enums::account::AccountForm;

type OnClick = Rc<dyn Fn(&mut Window, &mut App)>;

/// The card that holds the sign-in or create-account form: its fields, the form's own
/// complaint, the button, and the link to the other form.
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
        card(cx)
            .child(Label::new(self.form.title()).text_xl())
            .children(self.fields)
            .when_some(self.problem, |this, problem| {
                this.child(div().text_sm().text_color(cx.theme().danger).child(problem))
            })
            .child(
                Button::new("account-submit")
                    .primary()
                    .w_full()
                    .label(self.form.title())
                    .loading(self.busy)
                    .on_click(move |_, window, cx| on_submit(window, cx)),
            )
            .child(
                Button::new("account-switch")
                    .ghost()
                    .w_full()
                    .label(self.form.switch_label())
                    .on_click(move |_, window, cx| on_switch(window, cx)),
            )
            .child(
                Button::new("continue-as-guest")
                    .outline()
                    .w_full()
                    .label("Continue as guest")
                    .on_click(move |_, window, cx| on_guest(window, cx)),
            )
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
        card(cx)
            .child(Label::new("Playing as a guest").text_xl())
            .child(
                Label::new("Sign in or create an account to play online and keep a rating.")
                    .text_sm()
                    .text_color(cx.theme().muted_foreground),
            )
            .child(
                Button::new("guest-sign-in")
                    .primary()
                    .w_full()
                    .label(AccountForm::SignIn.title())
                    .on_click(move |_, window, cx| on_sign_in(window, cx)),
            )
            .child(
                Button::new("guest-sign-up")
                    .outline()
                    .w_full()
                    .label(AccountForm::SignUp.title())
                    .on_click(move |_, window, cx| on_sign_up(window, cx)),
            )
    }
}

/// Who is signed in, and the way out.
#[derive(IntoElement)]
pub struct SignedInCard {
    username: String,
    email: String,
    on_sign_out: OnClick,
    on_delete: OnClick,
}

impl SignedInCard {
    pub fn new(
        username: String,
        email: String,
        on_sign_out: impl Fn(&mut Window, &mut App) + 'static,
        on_delete: impl Fn(&mut Window, &mut App) + 'static,
    ) -> Self {
        Self {
            username,
            email,
            on_sign_out: Rc::new(on_sign_out),
            on_delete: Rc::new(on_delete),
        }
    }
}

impl RenderOnce for SignedInCard {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let (on_sign_out, on_delete) = (self.on_sign_out, self.on_delete);
        card(cx)
            .child(
                Label::new("Signed in")
                    .text_sm()
                    .text_color(cx.theme().muted_foreground),
            )
            .child(Label::new(self.username).text_xl())
            .child(
                Label::new(self.email)
                    .text_sm()
                    .text_color(cx.theme().muted_foreground),
            )
            .child(
                Button::new("sign-out")
                    .outline()
                    .w_full()
                    .label("Sign out")
                    .on_click(move |_, window, cx| on_sign_out(window, cx)),
            )
            .child(
                Button::new("delete-account")
                    .ghost()
                    .w_full()
                    .text_color(cx.theme().danger)
                    .label("Delete account")
                    .on_click(move |_, window, cx| on_delete(window, cx)),
            )
    }
}

/// Deleting the account, once the password is typed again: what goes, what stays for the
/// other players, and the red button.
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

/// The same bordered panel the new-game menu uses.
fn card(cx: &App) -> Div {
    v_flex()
        .w_96()
        .gap_4()
        .p_6()
        .rounded(cx.theme().radius)
        .border_1()
        .border_color(cx.theme().border)
}
