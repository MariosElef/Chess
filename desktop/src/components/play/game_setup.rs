use std::rc::Rc;

use chess_domain::{Category, TimeControl};
use gpui_kit::{
    component::{
        ActiveTheme as _, Disableable as _,
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

/// chess.com's Start Game green, its lighter hover, and its darker bottom edge.
const START_GREEN: u32 = 0x0081_b64c;
const START_GREEN_HOVER: u32 = 0x0095_c75a;
const START_GREEN_EDGE: u32 = 0x0045_7536;

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

/// What changes when the game is against a bot: why a game would not start.
pub struct BotOptions {
    pub problem: Option<String>,
}

/// The choices in the Play panel: rated or not online, a time control by speed (or none, but
/// not online), and Start Game.
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
    /// The panel's content: rated or not online, the speeds, no clock unless online, then
    /// the button that starts the game.
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let problem = self
            .online
            .as_ref()
            .and_then(|online| online.problem.clone())
            .or_else(|| self.bot.as_ref().and_then(|bot| bot.problem.clone()));
        v_flex()
            .w_full()
            .gap_2()
            .when_some(self.online.as_ref(), |this, online| {
                this.child(rated_switch(online, cx))
            })
            .children(Category::ALL.map(|category| self.category_section(category, cx)))
            .when(self.online.is_none(), |this| {
                this.child(self.no_clock_button(cx))
            })
            .when_some(problem, |this, problem| {
                this.child(div().text_sm().text_color(cx.theme().danger).child(problem))
            })
            .child(self.start_button())
            .when_some(
                self.online.as_ref().filter(|online| online.seeking),
                |this, online| this.child(cancel_button(Rc::clone(&online.on_cancel))),
            )
    }
}

impl GameSetup {
    fn category_section(&self, category: Category, cx: &App) -> Div {
        v_flex().gap_1p5().child(speed_label(category)).child(
            h_flex().gap_2().children(
                presets_in(category)
                    .into_iter()
                    .map(|control| self.choice_tile(control.label(), Some(control), cx)),
            ),
        )
    }

    fn no_clock_button(&self, cx: &App) -> Stateful<Div> {
        self.choice_tile("No clock".to_owned(), None, cx)
    }

    /// A time to pick, as chess.com draws one: a solid tile with its label, ringed in the
    /// start button's green once picked. Shorter against a bot, whose cards need the room.
    fn choice_tile(&self, label: String, value: Option<TimeControl>, cx: &App) -> Stateful<Div> {
        let on_choose = Rc::clone(&self.on_choose);
        let picked = self.choice == value;
        let height = if self.bot.is_some() { px(34.) } else { px(42.) };
        let selector = match value {
            Some(control) => format!("time-{}", control.label()),
            None => "time-none".to_owned(),
        };
        div()
            .id(SharedString::from(selector.clone()))
            .debug_selector(move || selector.clone())
            .flex_1()
            .h(height)
            .flex()
            .items_center()
            .justify_center()
            .rounded(px(6.))
            .border_2()
            .text_sm()
            .font_weight(FontWeight::SEMIBOLD)
            .cursor_pointer()
            .on_click(move |_, window, cx| on_choose(value, window, cx))
            .child(label)
            .map(|tile| tile_colours(tile, picked, cx))
    }

    /// chess.com's green: solid, bold white text, a darker edge along the bottom; dimmed
    /// while it looks for an opponent.
    fn start_button(&self) -> Stateful<Div> {
        let on_start = Rc::clone(&self.on_start);
        let seeking = self.online.as_ref().is_some_and(|online| online.seeking);
        let label = if seeking {
            "Looking for an opponent…"
        } else {
            "Start Game"
        };
        div()
            .id("start-game")
            .w_full()
            .h(px(48.))
            .flex()
            .items_center()
            .justify_center()
            .rounded(px(8.))
            .bg(rgb(START_GREEN))
            .border_b_4()
            .border_color(rgb(START_GREEN_EDGE))
            .text_color(white())
            .text_lg()
            .font_weight(FontWeight::EXTRA_BOLD)
            .cursor_pointer()
            .when(seeking, |this| this.opacity(0.7))
            .hover(|this| this.bg(rgb(START_GREEN_HOVER)))
            .child(label)
            .on_click(move |_, window, cx| on_start(window, cx))
    }
}

/// The tile's fill and ring: the theme's accent, the picked one ringed in green.
fn tile_colours(tile: Stateful<Div>, picked: bool, cx: &App) -> Stateful<Div> {
    let theme = cx.theme();
    let tile = tile.bg(theme.accent);
    if picked {
        tile.border_color(rgb(START_GREEN))
    } else {
        let hover = theme.input;
        tile.border_color(transparent_black())
            .hover(move |this| this.border_color(hover))
    }
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
