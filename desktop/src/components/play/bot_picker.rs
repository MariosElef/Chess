use std::rc::Rc;

use gpui_kit::{
    assets::IconName,
    component::{
        ActiveTheme as _, Icon, h_flex,
        label::Label,
        slider::{Slider, SliderState},
        v_flex,
    },
    prelude::FluentBuilder as _,
    *,
};

use super::play_pictures::DICE_PICTURE;
use crate::{
    components::panel::StartButton, enums::play::ColourChoice, models::play::Bot,
    services::play::custom_bot,
};

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
        let custom = custom_bot(self.custom_rating);
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

    /// "Play as" over White, the dice for Random, and Black, down the middle.
    fn colour_row(&self) -> Div {
        v_flex()
            .items_center()
            .gap_2()
            .child(Label::new("Play as").text_sm())
            .child(
                h_flex()
                    .items_center()
                    .gap_3()
                    .children(ColourChoice::ALL.map(|colour| self.colour_button(colour))),
            )
    }

    /// White and Black as 3D buttons in their colours, Random as the dice; unpicked ones fade.
    fn colour_button(&self, colour: ColourChoice) -> AnyElement {
        let on_colour = Rc::clone(&self.on_colour);
        let id = format!("colour-{}", colour.label());
        let dimmed = colour != self.colour;
        let pick = move |window: &mut Window, cx: &mut App| on_colour(colour, window, cx);
        match colour {
            ColourChoice::White => StartButton::new(id, colour.label(), pick)
                .small()
                .plain()
                .dimmed(dimmed)
                .into_any_element(),
            ColourChoice::Black => StartButton::new(id, colour.label(), pick)
                .small()
                .dark()
                .dimmed(dimmed)
                .into_any_element(),
            ColourChoice::Random => dice(id, dimmed, pick).into_any_element(),
        }
    }
}

/// Random's dice, pressed to pick it; faded while another colour is picked.
fn dice(id: String, dimmed: bool, pick: impl Fn(&mut Window, &mut App) + 'static) -> Stateful<Div> {
    let selector = id.clone();
    div()
        .id(SharedString::from(id))
        .debug_selector(move || selector.clone())
        .cursor_pointer()
        .when(dimmed, |this| this.opacity(0.45))
        .hover(|this| this.opacity(1.))
        .child(img(DICE_PICTURE).size_8())
        .on_click(move |_, window, cx| pick(window, cx))
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
