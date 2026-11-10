use std::rc::Rc;

use gpui_kit::{
    assets::IconName,
    component::{ActiveTheme as _, h_flex, v_flex},
    *,
};

use crate::{
    components::{
        panel::{ChoiceTile, PanelTitle, StartButton, StatCard},
        play::BULLET_PICTURE,
    },
    enums::puzzles::RushKind,
    i18n::tr,
    models::puzzles::BestRush,
    services::puzzles::score_text,
};

/// Puzzle Rush's picture: bullet's lightning.
pub const RUSH_PICTURE: &str = BULLET_PICTURE;

type OnClick = Rc<dyn Fn(&mut Window, &mut App)>;
type OnPick = Rc<dyn Fn(RushKind, &mut Window, &mut App)>;

/// Puzzle Rush's panel: today's best and the top score of the kind picked, its tiles, and Play.
#[derive(IntoElement)]
pub struct RushMenu {
    picked: RushKind,
    best: BestRush,
    today: BestRush,
    on_back: OnClick,
    on_pick: OnPick,
    on_play: OnClick,
}

impl RushMenu {
    pub fn new(
        picked: RushKind,
        (best, today): (BestRush, BestRush),
        on_back: impl Fn(&mut Window, &mut App) + 'static,
        on_pick: impl Fn(RushKind, &mut Window, &mut App) + 'static,
        on_play: impl Fn(&mut Window, &mut App) + 'static,
    ) -> Self {
        Self {
            picked,
            best,
            today,
            on_back: Rc::new(on_back),
            on_pick: Rc::new(on_pick),
            on_play: Rc::new(on_play),
        }
    }
}

impl RenderOnce for RushMenu {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = cx.theme();
        let (on_back, on_play) = (self.on_back, self.on_play);
        let tiles = RushKind::ALL.map(|kind| {
            let on_pick = Rc::clone(&self.on_pick);
            ChoiceTile::new(
                format!("rush-{}", kind.key()),
                short_name(kind),
                kind == self.picked,
                move |window, cx| on_pick(kind, window, cx),
            )
            .icon(kind_icon(kind))
            .height(px(48.))
        });
        v_flex()
            .size_full()
            .gap_3()
            .child(
                PanelTitle::new(RUSH_PICTURE, tr!("puzzles.rush.title")).on_back(
                    move |window, cx| {
                        on_back(window, cx);
                    },
                ),
            )
            .child(
                h_flex()
                    .gap_3()
                    .child(StatCard::new(
                        (IconName::Calendar, theme.green),
                        score_text(best_of(self.today, self.picked)),
                        tr!("puzzles.rush.best_today"),
                    ))
                    .child(StatCard::new(
                        (IconName::Trophy, theme.blue),
                        score_text(best_of(self.best, self.picked)),
                        tr!("puzzles.rush.top_score"),
                    )),
            )
            .child(div().h_2())
            .children(tiles)
            .child(div().flex_1())
            .child(StartButton::new(
                "rush-play",
                tr!("puzzles.rush.play"),
                move |window, cx| {
                    on_play(window, cx);
                },
            ))
    }
}

/// The kind's name on its tile, as chess.com writes it.
fn short_name(kind: RushKind) -> String {
    match kind {
        RushKind::ThreeMinutes => tr!("puzzles.rush.short.three_minutes"),
        RushKind::FiveMinutes => tr!("puzzles.rush.short.five_minutes"),
        RushKind::Survival => tr!("puzzles.rush.short.survival"),
    }
}

fn kind_icon(kind: RushKind) -> IconName {
    match kind {
        RushKind::ThreeMinutes => IconName::Zap,
        RushKind::FiveMinutes => IconName::Timer,
        RushKind::Survival => IconName::Swords,
    }
}

fn best_of(best: BestRush, kind: RushKind) -> Option<i32> {
    match kind {
        RushKind::ThreeMinutes => best.three_minutes,
        RushKind::FiveMinutes => best.five_minutes,
        RushKind::Survival => best.survival,
    }
}
