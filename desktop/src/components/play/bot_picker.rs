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
            .w_full()
            .gap_3()
            .child(
                div()
                    .w_full()
                    .grid()
                    .grid_cols(3)
                    .gap_2()
                    .children(cards)
                    .child(self.card(None, &custom, cx)),
            )
            .when(self.picked.is_none(), |this| {
                this.child(
                    h_flex()
                        .gap_3()
                        .child(div().w(px(104.)).flex_shrink_0().child(
                            Label::new(format!("Strength ~{}", self.custom_rating)).text_sm(),
                        ))
                        .child(div().flex_1().child(Slider::new(&self.strength))),
                )
            })
            .child(self.colour_row())
    }
}

impl BotPicker {
    /// A bot's photo, name and rating on a soft tile, taking the theme's colour once picked.
    fn card(&self, place: Option<usize>, bot: &Bot, cx: &App) -> Stateful<Div> {
        let on_pick = Rc::clone(&self.on_pick);
        let picked = self.picked == place;
        let theme = cx.theme();
        let ring = if picked { theme.primary } else { theme.border };
        v_flex()
            .id(SharedString::from(format!("bot-{}", bot.name)))
            .items_center()
            .gap_1()
            .pt_3()
            .pb_2()
            .px_2()
            .rounded(px(12.))
            .border_2()
            .cursor_pointer()
            .when(picked, |this| {
                this.border_color(theme.primary)
                    .bg(theme.primary.opacity(0.12))
            })
            .when(!picked, |this| {
                this.border_color(transparent_black())
                    .bg(theme.muted)
                    .hover(|this| this.bg(theme.accent))
            })
            .on_click(move |_, window, cx| on_pick(place, window, cx))
            .child(
                div()
                    .rounded_full()
                    .border_2()
                    .border_color(ring)
                    .p(px(2.))
                    .child(portrait(bot.picture, cx)),
            )
            .child(
                div()
                    .text_sm()
                    .font_weight(FontWeight::SEMIBOLD)
                    .child(bot.name),
            )
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
        Some(path) => img(path).size_12().rounded_full().into_any_element(),
        None => div()
            .size_12()
            .rounded_full()
            .flex()
            .items_center()
            .justify_center()
            .bg(cx.theme().muted)
            .child(Icon::from(IconName::Settings))
            .into_any_element(),
    }
}
