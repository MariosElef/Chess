use std::rc::Rc;

use chess_domain::{Category, TimeControl};
use gpui_kit::{
    component::{
        ActiveTheme as _, Disableable as _, Selectable as _, Sizable as _,
        button::{Button, ButtonVariants},
        h_flex,
        label::Label,
        switch::Switch,
        v_flex,
    },
    prelude::FluentBuilder as _,
    *,
};

use super::speed_label::speed_label;
use crate::services::play::presets_in;

type OnChoose = Rc<dyn Fn(Option<TimeControl>, &mut Window, &mut App)>;
type OnStart = Rc<dyn Fn(&mut Window, &mut App)>;
type OnRated = Rc<dyn Fn(bool, &mut Window, &mut App)>;

/// What changes when the game is against someone online: no game without a clock, a choice
/// of rated or not (guests play unrated), and waiting for an opponent.
pub struct OnlineOptions {
    pub rated: bool,
    /// Signed in with an account; a guest's game is never rated.
    pub can_rate: bool,
    /// Waiting for an opponent: the button says so and Cancel stops it.
    pub seeking: bool,
    /// Why the last request went wrong, or that the server is out of reach.
    pub problem: Option<String>,
    pub on_rated: OnRated,
    pub on_cancel: OnStart,
}

/// What changes when the game is against a bot: who it is, and why a game would not start.
pub struct BotOptions {
    pub name: &'static str,
    pub rating: u32,
    pub problem: Option<String>,
}

/// The new-game menu: a time control by speed, or none, then Start; online, then Find an
/// opponent; against a bot, Play.
#[derive(IntoElement)]
pub struct GameSetup {
    /// `None` is a game without a clock.
    choice: Option<TimeControl>,
    on_choose: OnChoose,
    on_start: OnStart,
    online: Option<OnlineOptions>,
    bot: Option<BotOptions>,
}

impl GameSetup {
    pub fn new(
        choice: Option<TimeControl>,
        on_choose: impl Fn(Option<TimeControl>, &mut Window, &mut App) + 'static,
        on_start: impl Fn(&mut Window, &mut App) + 'static,
    ) -> Self {
        Self {
            choice,
            on_choose: Rc::new(on_choose),
            on_start: Rc::new(on_start),
            online: None,
            bot: None,
        }
    }

    pub fn bot(mut self, options: BotOptions) -> Self {
        self.bot = Some(options);
        self
    }

    pub fn online(mut self, options: OnlineOptions) -> Self {
        self.online = Some(options);
        self
    }
}

impl RenderOnce for GameSetup {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        v_flex()
            .w_96()
            .gap_5()
            .p_6()
            .rounded(cx.theme().radius)
            .border_1()
            .border_color(cx.theme().border)
            .child(mode_heading(self.subtitle(), cx))
            .children(Category::ALL.map(|category| self.category_section(category)))
            .when(self.online.is_none(), |this| {
                this.child(self.no_clock_button())
            })
            .when_some(self.online.as_ref(), |this, online| {
                this.child(rated_switch(online, cx)).when_some(
                    online.problem.clone(),
                    |this, problem| {
                        this.child(div().text_sm().text_color(cx.theme().danger).child(problem))
                    },
                )
            })
            .when_some(
                self.bot.as_ref().and_then(|bot| bot.problem.clone()),
                |this, problem| {
                    this.child(div().text_sm().text_color(cx.theme().danger).child(problem))
                },
            )
            .child(self.start_button())
            .when_some(
                self.online.as_ref().filter(|online| online.seeking),
                |this, online| this.child(cancel_button(Rc::clone(&online.on_cancel))),
            )
    }
}

impl GameSetup {
    fn category_section(&self, category: Category) -> Div {
        v_flex().gap_2().child(speed_label(category)).child(
            h_flex().gap_2().children(
                presets_in(category)
                    .into_iter()
                    .map(|control| self.preset_button(control)),
            ),
        )
    }

    fn preset_button(&self, control: TimeControl) -> Button {
        self.choice_button(
            SharedString::from(format!("time-{}", control.label())),
            control.label(),
            Some(control),
        )
        .flex_1()
    }

    fn no_clock_button(&self) -> Button {
        self.choice_button("time-none".into(), "No clock".to_owned(), None)
            .w_full()
    }

    /// Outlined until picked, filled once it is the choice.
    fn choice_button(&self, id: SharedString, label: String, value: Option<TimeControl>) -> Button {
        let on_choose = Rc::clone(&self.on_choose);
        Button::new(id)
            .outline()
            .label(label)
            .selected(self.choice == value)
            .on_click(move |_, window, cx| on_choose(value, window, cx))
    }

    fn start_button(&self) -> Button {
        let on_start = Rc::clone(&self.on_start);
        let (label, seeking) = match &self.online {
            Some(online) if online.seeking => ("Looking for an opponent…", true),
            Some(_) => ("Find an opponent", false),
            None if self.bot.is_some() => ("Play", false),
            None => ("Start game", false),
        };
        Button::new("start-game")
            .primary()
            .large()
            .w_full()
            .label(label)
            .loading(seeking)
            .on_click(move |_, window, cx| on_start(window, cx))
    }
}

impl GameSetup {
    fn subtitle(&self) -> String {
        match (&self.online, &self.bot) {
            (Some(_), _) => "Online · against the next player who wants the same game".to_owned(),
            (None, Some(bot)) => format!("Bot · against {} ({})", bot.name, bot.rating),
            (None, None) => "Local · two players on this computer".to_owned(),
        }
    }
}

fn mode_heading(subtitle: String, cx: &App) -> Div {
    v_flex()
        .gap_1()
        .child(Label::new("New game").text_xl())
        .child(
            div()
                .text_sm()
                .text_color(cx.theme().muted_foreground)
                .child(subtitle),
        )
}

/// Rated or not; a guest's switch stays off, with the reason beside it.
fn rated_switch(online: &OnlineOptions, cx: &App) -> Div {
    let on_rated = Rc::clone(&online.on_rated);
    h_flex()
        .justify_between()
        .child(
            v_flex()
                .child(Label::new("Rated").text_sm())
                .when(!online.can_rate, |this| {
                    this.child(
                        div()
                            .text_xs()
                            .text_color(cx.theme().muted_foreground)
                            .child("Sign up to play rated games"),
                    )
                }),
        )
        .child(
            Switch::new("rated-switch")
                .checked(online.rated && online.can_rate)
                .disabled(!online.can_rate)
                .on_click(move |on, window, cx| on_rated(*on, window, cx)),
        )
}

fn cancel_button(on_cancel: OnStart) -> Button {
    Button::new("cancel-seek")
        .ghost()
        .w_full()
        .label("Cancel")
        .on_click(move |_, window, cx| on_cancel(window, cx))
}
