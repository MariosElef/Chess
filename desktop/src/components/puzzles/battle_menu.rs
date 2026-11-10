use std::rc::Rc;

use gpui_kit::{
    assets::IconName,
    component::{
        ActiveTheme as _, Sizable as _,
        button::{Button, ButtonVariants},
        h_flex, v_flex,
    },
    prelude::FluentBuilder as _,
    *,
};

use super::puzzle_pictures::BATTLE_PICTURE;
use crate::{
    components::panel::{ChoiceTile, PanelTitle, RatedSwitch, StartButton, StatCard},
    i18n::tr,
    models::{
        puzzles::{BattleChallenge, BattleSummary},
        social::Friend,
    },
    services::puzzles::score_text,
};

type OnClick = Rc<dyn Fn(&mut Window, &mut App)>;
type OnPick = Rc<dyn Fn(Option<String>, &mut Window, &mut App)>;
type OnRated = Rc<dyn Fn(bool, &mut Window, &mut App)>;
type OnInvitation = Rc<dyn Fn(String, &mut Window, &mut App)>;

/// What the Puzzle Battle panel shows before a battle.
pub struct BattleMenuState {
    pub summary: BattleSummary,
    /// The player's friends who are online, to challenge.
    pub friends: Vec<Friend>,
    /// The friend picked; `None` is a random opponent.
    pub picked: Option<String>,
    pub received: Vec<BattleChallenge>,
    pub sent: Vec<BattleChallenge>,
    pub seeking: bool,
    pub problem: Option<String>,
    /// A guest's battles are never rated, and the switch says so.
    pub guest: bool,
    pub rated: bool,
}

/// What its buttons do.
pub struct BattleMenuActions {
    pub on_back: OnClick,
    pub on_pick: OnPick,
    pub on_play: OnClick,
    pub on_cancel: OnClick,
    pub on_accept: OnInvitation,
    pub on_decline: OnInvitation,
    pub on_rated: OnRated,
}

/// Puzzle Battle's panel: this month's wins, the battle rating, an opponent, invitations, and Play.
#[derive(IntoElement)]
pub struct BattleMenu {
    state: BattleMenuState,
    actions: BattleMenuActions,
}

impl BattleMenu {
    pub fn new(state: BattleMenuState, actions: BattleMenuActions) -> Self {
        Self { state, actions }
    }
}

impl RenderOnce for BattleMenu {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = cx.theme();
        let Self { state, actions } = self;
        let on_back = Rc::clone(&actions.on_back);
        let rating = state.summary.rating.map(|rating| rating.rating);
        let waiting_on = state.sent.first().map(|sent| sent.to.username.clone());
        let busy = state.seeking || waiting_on.is_some();
        let play_label = match (&waiting_on, state.seeking) {
            (Some(friend), _) => tr!("puzzles.battle.waiting_for", friend = friend),
            (None, true) => tr!("puzzles.battle.looking"),
            (None, false) => tr!("puzzles.battle.play"),
        };
        let on_play = Rc::clone(&actions.on_play);
        let on_cancel = Rc::clone(&actions.on_cancel);
        v_flex()
            .size_full()
            .gap_3()
            .child(
                PanelTitle::new(BATTLE_PICTURE, tr!("puzzles.battle.title")).on_back(
                    move |window, cx| {
                        on_back(window, cx);
                    },
                ),
            )
            .child(
                h_flex()
                    .gap_3()
                    .child(StatCard::new(
                        (IconName::Swords, theme.green),
                        state.summary.wins_this_month.to_string(),
                        tr!("puzzles.battle.wins_this_month"),
                    ))
                    .child(StatCard::new(
                        (IconName::Trophy, theme.blue),
                        score_text(rating),
                        tr!("puzzles.battle.rating"),
                    )),
            )
            .child({
                let on_rated = Rc::clone(&actions.on_rated);
                RatedSwitch::new(
                    ("battle-rated", tr!("puzzles.battle.sign_up_rated")),
                    (state.rated, !state.guest),
                    move |on, window, cx| on_rated(on, window, cx),
                )
            })
            .children(
                state
                    .received
                    .iter()
                    .map(|invitation| invitation_row(invitation, &actions, cx)),
            )
            .child(opponent_tile(
                None,
                &tr!("puzzles.battle.vs_random"),
                &state,
                &actions,
            ))
            .child(
                div()
                    .text_sm()
                    .font_weight(FontWeight::SEMIBOLD)
                    .text_color(theme.muted_foreground)
                    .child(tr!(
                        "puzzles.battle.friends_online",
                        count = state.friends.len()
                    )),
            )
            .when(state.friends.is_empty(), |this| {
                this.child(
                    div()
                        .text_sm()
                        .text_color(theme.muted_foreground)
                        .child(tr!("puzzles.battle.none_online")),
                )
            })
            .child(
                // The friends scroll, so Play stays in reach however many are online.
                div()
                    .id("battle-friends")
                    .flex_1()
                    .min_h_0()
                    .overflow_y_scroll()
                    .child(
                        v_flex()
                            .gap_2()
                            .children(state.friends.iter().map(|friend| {
                                opponent_tile(
                                    Some(friend.id.clone()),
                                    &friend.username,
                                    &state,
                                    &actions,
                                )
                            })),
                    ),
            )
            .when_some(state.problem.clone(), |this, problem| {
                this.child(div().text_sm().text_color(theme.danger).child(problem))
            })
            .child(
                StartButton::new("battle-play", play_label, move |window, cx| {
                    on_play(window, cx);
                })
                .busy(busy),
            )
            .when(busy, |this| {
                this.child(
                    Button::new("battle-cancel")
                        .ghost()
                        .w_full()
                        .label(tr!("puzzles.battle.cancel"))
                        .on_click(move |_, window, cx| on_cancel(window, cx)),
                )
            })
    }
}

/// A random opponent, or a friend: picked, it is who Play asks.
fn opponent_tile(
    friend: Option<String>,
    label: &str,
    state: &BattleMenuState,
    actions: &BattleMenuActions,
) -> ChoiceTile {
    let on_pick = Rc::clone(&actions.on_pick);
    let id = match &friend {
        Some(friend) => format!("battle-friend-{friend}"),
        None => "battle-random".to_owned(),
    };
    let icon = if friend.is_some() {
        IconName::User
    } else {
        IconName::Users
    };
    let picked = state.picked == friend;
    ChoiceTile::new(id, label.to_owned(), picked, move |window, cx| {
        on_pick(friend.clone(), window, cx);
    })
    .icon(icon)
}

/// "Bob invites you", with Accept and Decline.
fn invitation_row(invitation: &BattleChallenge, actions: &BattleMenuActions, cx: &App) -> Div {
    let (accept, decline) = (
        Rc::clone(&actions.on_accept),
        Rc::clone(&actions.on_decline),
    );
    let (accept_id, decline_id) = (invitation.id.clone(), invitation.id.clone());
    h_flex()
        .justify_between()
        .gap_2()
        .p_3()
        .rounded(px(6.))
        .bg(cx.theme().muted)
        .child(
            div()
                .flex_1()
                .min_w_0()
                .overflow_hidden()
                .whitespace_nowrap()
                .text_ellipsis()
                .text_sm()
                .font_weight(FontWeight::SEMIBOLD)
                .child(if invitation.rated {
                    tr!(
                        "puzzles.battle.invites_rated",
                        name = invitation.from.username
                    )
                } else {
                    tr!(
                        "puzzles.battle.invites_casual",
                        name = invitation.from.username
                    )
                }),
        )
        .child(
            h_flex()
                .gap_2()
                .child(
                    StartButton::new(
                        format!("accept-battle-{accept_id}"),
                        tr!("puzzles.battle.accept"),
                        move |window, cx| {
                            accept(accept_id.clone(), window, cx);
                        },
                    )
                    .small(),
                )
                .child(
                    Button::new(SharedString::from(format!("decline-battle-{decline_id}")))
                        .ghost()
                        .small()
                        .label(tr!("puzzles.battle.decline"))
                        .on_click(move |_, window, cx| decline(decline_id.clone(), window, cx)),
                ),
        )
}
