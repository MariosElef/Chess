use std::rc::Rc;

use chess_domain::{Category, TimeControl};
use gpui_kit::{
    component::{
        ActiveTheme as _,
        button::{Button, ButtonVariants},
        h_flex, v_flex,
    },
    prelude::FluentBuilder as _,
    *,
};

use super::speed_label::speed_label;
use crate::components::panel::{ChoiceTile, RatedSwitch, StartButton};
use crate::{
    i18n::tr,
    services::play::{control_label, presets_in},
};

type OnChoose = Rc<dyn Fn(Option<TimeControl>, &mut Window, &mut App)>;
type OnStart = Rc<dyn Fn(&mut Window, &mut App)>;
type OnRated = Rc<dyn Fn(bool, &mut Window, &mut App)>;

/// What changes online: a clock is required, rated or not, and waiting for an opponent.
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

/// The Play panel's choices: rated or not online, a time control by speed, and Start Game.
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
    /// The panel's content: rated or not online, the speeds, no clock unless online, then Start.
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
                this.child({
                    let on_rated = Rc::clone(&online.on_rated);
                    RatedSwitch::new(
                        ("rated-switch", tr!("play.setup.guest_cannot_rate")),
                        (online.rated, online.can_rate),
                        move |on, window, cx| on_rated(on, window, cx),
                    )
                })
            })
            .children(Category::ALL.map(|category| self.category_section(category)))
            .when(self.online.is_none(), |this| {
                this.child(self.no_clock_button())
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
    fn category_section(&self, category: Category) -> Div {
        v_flex()
            .gap_1p5()
            .child(speed_label(category))
            .child(
                h_flex()
                    .gap_2()
                    .children(presets_in(category).into_iter().map(|control| {
                        div()
                            .flex_1()
                            .child(self.choice_tile(control_label(control), Some(control)))
                    })),
            )
    }

    fn no_clock_button(&self) -> ChoiceTile {
        self.choice_tile(tr!("play.time.no_clock"), None)
    }

    /// A time to pick; shorter against a bot, whose cards need the room.
    fn choice_tile(&self, label: String, value: Option<TimeControl>) -> ChoiceTile {
        let on_choose = Rc::clone(&self.on_choose);
        let height = if self.bot.is_some() { px(34.) } else { px(42.) };
        let id = match value {
            Some(control) => format!("time-{}", control.label()),
            None => "time-none".to_owned(),
        };
        ChoiceTile::new(id, label, self.choice == value, move |window, cx| {
            on_choose(value, window, cx);
        })
        .height(height)
    }

    /// The green Start Game; while it looks for an opponent it says so.
    fn start_button(&self) -> StartButton {
        let on_start = Rc::clone(&self.on_start);
        let seeking = self.online.as_ref().is_some_and(|online| online.seeking);
        let label = if seeking {
            tr!("play.setup.looking")
        } else {
            tr!("play.setup.start")
        };
        StartButton::new("start-game", label, move |window, cx| on_start(window, cx)).busy(seeking)
    }
}

fn cancel_button(on_cancel: OnStart) -> Button {
    Button::new("cancel-seek")
        .ghost()
        .w_full()
        .label(tr!("play.setup.cancel"))
        .on_click(move |_, window, cx| on_cancel(window, cx))
}
