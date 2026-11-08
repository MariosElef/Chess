use std::rc::Rc;

use gpui_kit::{
    assets::IconName,
    component::{
        ActiveTheme as _, Icon, Sizable as _,
        button::{Button, ButtonVariants},
        h_flex,
        label::Label,
        slider::{Slider, SliderState},
        v_flex,
    },
    prelude::FluentBuilder as _,
    *,
};

use crate::{enums::play::ColourChoice, models::play::Bot};

type OnPick = Rc<dyn Fn(Option<usize>, &mut Window, &mut App)>;
type OnColour = Rc<dyn Fn(ColourChoice, &mut Window, &mut App)>;

/// Who to play: the named bots as cards, a custom bot of any strength, and the colour to play.
#[derive(IntoElement)]
pub struct BotPicker {
    bots: Vec<Bot>,
    /// The bot picked by its place in `bots`; `None` is the custom bot.
    picked: Option<usize>,
    custom_rating: u32,
    strength: Entity<SliderState>,
    colour: ColourChoice,
    on_pick: OnPick,
    on_colour: OnColour,
}

impl BotPicker {
    pub fn new(
        (bots, picked): (Vec<Bot>, Option<usize>),
        (custom_rating, strength): (u32, Entity<SliderState>),
        colour: ColourChoice,
        on_pick: impl Fn(Option<usize>, &mut Window, &mut App) + 'static,
        on_colour: impl Fn(ColourChoice, &mut Window, &mut App) + 'static,
    ) -> Self {
        Self {
            bots,
            picked,
            custom_rating,
            strength,
            colour,
            on_pick: Rc::new(on_pick),
            on_colour: Rc::new(on_colour),
        }
    }
}

impl RenderOnce for BotPicker {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let cards: Vec<Stateful<Div>> = self
            .bots
            .iter()
            .enumerate()
            .map(|(index, bot)| self.card(Some(index), bot, cx))
            .collect();
        let custom = Bot {
            name: "Custom",
            rating: self.custom_rating,
            picture: None,
        };
        v_flex()
            .w(px(460.))
            .gap_5()
            .p_6()
            .rounded(cx.theme().radius)
            .border_1()
            .border_color(cx.theme().border)
            .child(Label::new("Choose a bot").text_xl())
            .child(
                h_flex()
                    .flex_wrap()
                    .gap_3()
                    .children(cards)
                    .child(self.card(None, &custom, cx)),
            )
            .when(self.picked.is_none(), |this| {
                this.child(
                    v_flex()
                        .gap_2()
                        .child(Label::new(format!("Strength ~{}", self.custom_rating)).text_sm())
                        .child(Slider::new(&self.strength)),
                )
            })
            .child(self.colour_row())
    }
}

impl BotPicker {
    /// A bot's picture, name and rating; outlined in the theme's colour once picked.
    fn card(&self, place: Option<usize>, bot: &Bot, cx: &App) -> Stateful<Div> {
        let on_pick = Rc::clone(&self.on_pick);
        let picked = self.picked == place;
        let theme = cx.theme();
        v_flex()
            .id(SharedString::from(format!("bot-{}", bot.name)))
            .w_24()
            .items_center()
            .gap_1()
            .p_2()
            .rounded(theme.radius)
            .border_2()
            .border_color(if picked { theme.primary } else { theme.border })
            .cursor_pointer()
            .hover(|this| this.bg(theme.accent))
            .on_click(move |_, window, cx| on_pick(place, window, cx))
            .child(portrait(bot.picture, cx))
            .child(div().text_sm().child(bot.name))
            .child(
                div()
                    .text_xs()
                    .text_color(theme.muted_foreground)
                    .child(bot.rating.to_string()),
            )
    }

    fn colour_row(&self) -> Div {
        h_flex()
            .justify_between()
            .child(Label::new("Play as").text_sm())
            .child(
                h_flex()
                    .gap_2()
                    .children(ColourChoice::ALL.map(|colour| self.colour_button(colour))),
            )
    }

    fn colour_button(&self, colour: ColourChoice) -> Button {
        let on_colour = Rc::clone(&self.on_colour);
        let button = Button::new(SharedString::from(format!("colour-{}", colour.label())))
            .small()
            .label(colour.label())
            .on_click(move |_, window, cx| on_colour(colour, window, cx));
        if colour == self.colour {
            button.primary()
        } else {
            button.outline()
        }
    }
}

/// The bot's photo in a circle, or a robot-less placeholder for the custom bot.
fn portrait(picture: Option<&'static str>, cx: &App) -> AnyElement {
    match picture {
        Some(path) => img(path).size_16().rounded_full().into_any_element(),
        None => div()
            .size_16()
            .rounded_full()
            .flex()
            .items_center()
            .justify_center()
            .bg(cx.theme().muted)
            .child(Icon::from(IconName::Settings))
            .into_any_element(),
    }
}
