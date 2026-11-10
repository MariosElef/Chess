//! The intro the app opens with, over its first screen, quick enough never to pass for loading:
//! the app's green pawn pops up in the middle, the 32 pieces of a chess set spiral in from off
//! the screen with trails behind them and are swallowed by it, the pawn swelling and wobbling
//! with each one, then it crouches, leaps, and slams down with a flash and two shockwaves, and
//! rushes past the viewer, fading into the app.
#![expect(
    clippy::float_arithmetic,
    reason = "the intro's motion is worked out frame by frame"
)]

use std::{f32::consts::TAU, time::Duration};

use chess_domain::{Board, Piece, Square};
use gpui_kit::{component::ActiveTheme as _, *};

use super::app_logo::APP_LOGO;
use crate::{
    components::board::{BoardStyle, piece_image},
    enums::settings::PieceSet,
};

/// How long the intro lasts, from the pawn popping up to the app.
pub const INTRO_TIME: Duration = Duration::from_millis(950);

// The timeline, as shares of the intro.
/// The pawn popping up.
const POP_IN: f32 = 0.08;
/// The pieces: the first leaves at the first time, the last at the second.
const PIECE_WAVE: (f32, f32) = (0.02, 0.24);
/// How long one piece takes to spiral in.
const FLIGHT: f32 = 0.16;
/// The pawn crouching, leaping up and falling back, each from its start to its end.
const CROUCH: (f32, f32) = (0.42, 0.50);
const LEAP: (f32, f32) = (0.50, 0.60);
const FALL: (f32, f32) = (0.60, 0.68);
/// How long the pawn takes to spring back into shape after it lands.
const SETTLE: f32 = 0.10;
/// The flash as it lands, from its start to its end, brightest in the middle.
const FLASH: (f32, f32) = (0.68, 0.80);
/// When each shockwave leaves the landing, and how long one takes to spread and fade.
const SHOCKWAVES: [(f32, u32); 2] = [(0.68, LOGO_GREEN), (0.73, 0x00ff_ffff)];
const SHOCKWAVE: f32 = 0.28;
/// The rush past the viewer and the fade, to the end.
const RUSH_FROM: f32 = 0.78;

/// The stage everything is placed on, as shares of it, so growing it in the rush grows all.
const STAGE_SIZE: f32 = 360.;
const RUSH_SCALE: f32 = 8.;
/// The pawn's height standing in the middle, as a share of the stage, and its width to height.
const PAWN_HEIGHT: f32 = 0.5;
const PAWN_ASPECT: f32 = 290. / 400.;
/// The pawn's size before the pieces feed it, as a share of its full size.
const HUNGRY: f32 = 0.55;
/// How far it wobbles as the pieces hit it, and how many times.
const WOBBLE: (f32, f32) = (0.06, 16.);
/// How far it leaps, as a share of the stage, and how much it squashes and stretches.
const LEAP_HEIGHT: f32 = 0.6;
const SQUASH_DEPTH: f32 = 0.28;
const STRETCH: f32 = 0.2;
/// How far off a piece starts, in stage widths from the middle: just off the screen.
const LAUNCH: f32 = 1.7;
/// How sharply a piece speeds up on its way in: higher pulls harder at the end.
const PULL: f32 = 1.5;
/// How far round a piece swirls on its way in, in turns.
const SWIRL_TURNS: f32 = 0.6;
/// A piece's size as it starts, as a share of the stage, and the share of that it shrinks away.
const PIECE_SIZE: f32 = 0.2;
const SHRINK: f32 = 0.7;
/// Each trail behind a piece: how far behind in its flight, and how see-through.
const TRAILS: [(f32, f32); 2] = [(0.12, 0.4), (0.24, 0.18)];
/// The flash at its brightest.
const FLASH_PEAK: f32 = 0.75;
/// The logo's lighter green, for the first shockwave.
const LOGO_GREEN: u32 = 0x0081_b64c;
/// A shockwave's size as it leaves the pawn and as it fades, as shares of the stage.
const SHOCKWAVE_SIZE: (f32, f32) = (0.3, 3.2);
/// Turning by the golden angle spreads the pieces' launch directions all round, unevenly.
const GOLDEN_ANGLE: f32 = 2.399_963;

// Quick enough never to look like loading, every piece swallowed before the pawn crouches, the
// flash and first shockwave with the landing, and the rush starting before the end.
const _: () = assert!(INTRO_TIME.as_millis() < 1000);
const _: () = assert!(PIECE_WAVE.1 + FLIGHT <= CROUCH.0);
const _: () = assert!(FLASH.0 == FALL.1 && SHOCKWAVES[0].0 == FALL.1);
const _: () = assert!(RUSH_FROM < 1.);

/// The intro over the whole window; the shell shows it for `INTRO_TIME`, then drops it.
#[derive(IntoElement)]
pub struct Intro;

impl RenderOnce for Intro {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let set = BoardStyle::current(cx).pieces;
        div()
            .absolute()
            .inset_0()
            // Nothing under it can be clicked while it plays.
            .occlude()
            .bg(cx.theme().background)
            .child(stage(set))
            .child(flash())
            .with_animation("intro", Animation::new(INTRO_TIME), |this, progress| {
                this.opacity(1. - rushing(progress).powi(2))
            })
    }
}

/// The middle of the window, where the pieces swirl into the pawn; it grows in the rush.
fn stage(set: PieceSet) -> impl IntoElement {
    let pieces = starting_pieces()
        .into_iter()
        .flat_map(move |(order, piece)| spiralling_piece(order, piece, set));
    div()
        .absolute()
        .children(pieces)
        .children(SHOCKWAVES.map(|(start, colour)| shockwave(start, colour)))
        .child(green_pawn())
        .with_animation(
            "intro-stage",
            Animation::new(INTRO_TIME),
            |this, progress| {
                let size = STAGE_SIZE * (1. + (RUSH_SCALE - 1.) * rushing(progress).powi(3));
                this.size(px(size))
                    .left(relative(0.5))
                    .top(relative(0.5))
                    .ml(px(-size / 2.))
                    .mt(px(-size / 2.))
            },
        )
}

/// A piece spiralling in from off the screen, faster and smaller as it nears the pawn, with its
/// trails behind it; gone once swallowed.
fn spiralling_piece(order: u16, piece: Piece, set: PieceSet) -> Vec<AnyElement> {
    let path = piece_image(piece, set).path;
    let launch_angle = GOLDEN_ANGLE * f32::from(order);
    let start = along(PIECE_WAVE, f32::from(order) / 31.);
    [(0., 1.)]
        .into_iter()
        .chain(TRAILS)
        .zip(0u64..)
        .map(|((lag, opacity), copy)| {
            img(path)
                .absolute()
                .with_animation(
                    ElementId::NamedInteger("intro-piece".into(), u64::from(order) * 3 + copy),
                    Animation::new(INTRO_TIME),
                    move |this, progress| {
                        let flight = stage_of(progress, start, FLIGHT);
                        let pulled = (flight - lag).max(0.).powf(PULL);
                        let angle = launch_angle + SWIRL_TURNS * TAU * pulled;
                        let reach = LAUNCH * (1. - pulled);
                        let size = PIECE_SIZE * (1. - SHRINK * pulled);
                        let flying = flight > 0. && flight < 1.;
                        centred(
                            this,
                            (0.5 + angle.cos() * reach, 0.5 + angle.sin() * reach),
                            (size, size),
                        )
                        .opacity(if flying { opacity } else { 0. })
                    },
                )
                .into_any_element()
        })
        .collect()
}

/// The green pawn: it pops up, swells and wobbles as it swallows the pieces, crouches, leaps,
/// and slams back down, squashing flat and springing back.
fn green_pawn() -> impl IntoElement {
    img(APP_LOGO).absolute().with_animation(
        "intro-pawn",
        Animation::new(INTRO_TIME),
        |this, progress| {
            let size =
                PAWN_HEIGHT * back_out(stage_of(progress, 0., POP_IN)).max(0.) * fed(progress);
            let (wide, tall, lift) = pawn_shape(progress);
            let (width, height) = (size * PAWN_ASPECT * wide, size * tall);
            // Its foot stays put as it squashes and stretches; the leap lifts it.
            let foot = 0.5 + size / 2. - lift;
            this.left(relative(0.5 - width / 2.))
                .top(relative(foot - height))
                .w(relative(width))
                .h(relative(height))
        },
    )
}

/// How big the pieces have made the pawn, wobbling as each one hits.
fn fed(progress: f32) -> f32 {
    let eaten = stage_of(progress, PIECE_WAVE.0 + FLIGHT, PIECE_WAVE.1 - PIECE_WAVE.0);
    let wobble = if eaten > 0. && eaten < 1. {
        WOBBLE.0 * (eaten * WOBBLE.1 * TAU).sin()
    } else {
        0.
    };
    HUNGRY + (1. - HUNGRY) * eaten + wobble
}

/// The pawn's width and height as shares of its own, and how high it is off its spot.
fn pawn_shape(progress: f32) -> (f32, f32, f32) {
    let crouch = stage_of(progress, CROUCH.0, CROUCH.1 - CROUCH.0);
    let leap = stage_of(progress, LEAP.0, LEAP.1 - LEAP.0);
    let fall = stage_of(progress, FALL.0, FALL.1 - FALL.0);
    let settle = stage_of(progress, FALL.1, SETTLE);
    if settle > 0. {
        let squash = SQUASH_DEPTH * (1. - settle).powi(2);
        (1. + squash, 1. - squash, 0.)
    } else if fall > 0. {
        (
            1. - STRETCH,
            1. + STRETCH,
            LEAP_HEIGHT * (1. - fall.powi(2)),
        )
    } else if leap > 0. {
        // From the crouch into the stretch as it springs up, slowing at the top.
        let spring = ease_out_cubic(leap);
        let wide = 1. + SQUASH_DEPTH - (SQUASH_DEPTH + STRETCH) * spring;
        let tall = 1. - SQUASH_DEPTH + (SQUASH_DEPTH + STRETCH) * spring;
        (wide, tall, LEAP_HEIGHT * spring)
    } else {
        (1. + SQUASH_DEPTH * crouch, 1. - SQUASH_DEPTH * crouch, 0.)
    }
}

/// A ring blasting out from where the pawn lands, fading as it spreads.
fn shockwave(start: f32, colour: u32) -> impl IntoElement {
    div()
        .absolute()
        .rounded_full()
        .border_8()
        .border_color(rgb(colour))
        .with_animation(
            ElementId::NamedInteger("intro-shockwave".into(), u64::from(colour)),
            Animation::new(INTRO_TIME),
            move |this, progress| {
                let wave = stage_of(progress, start, SHOCKWAVE);
                let size = along(SHOCKWAVE_SIZE, ease_out_cubic(wave));
                let spreading = wave > 0. && wave < 1.;
                centred(this, (0.5, 0.5 + PAWN_HEIGHT / 2.), (size, size)).opacity(if spreading {
                    1. - wave
                } else {
                    0.
                })
            },
        )
}

/// The white flash as the pawn lands.
fn flash() -> impl IntoElement {
    div().absolute().inset_0().bg(white()).with_animation(
        "intro-flash",
        Animation::new(INTRO_TIME),
        |this, progress| {
            let through = stage_of(progress, FLASH.0, FLASH.1 - FLASH.0);
            let lit = through > 0. && through < 1.;
            this.opacity(if lit {
                FLASH_PEAK * (1. - (2. * through - 1.).abs())
            } else {
                0.
            })
        },
    )
}

/// The 32 pieces of a chess set, as the starting position has them, each with its place in the
/// order they fly in.
fn starting_pieces() -> Vec<(u16, Piece)> {
    let board = Board::starting();
    (0..64u8)
        .filter_map(|index| board.piece_at(Square::new(index % 8, index / 8)?))
        .zip(0u16..)
        .map(|(piece, order)| (order, piece))
        .collect()
}

/// `element` `size` across, as shares of the stage, centred on `at`.
fn centred<E: Styled>(element: E, at: (f32, f32), (width, height): (f32, f32)) -> E {
    element
        .left(relative(at.0 - width / 2.))
        .top(relative(at.1 - height / 2.))
        .w(relative(width))
        .h(relative(height))
}

/// How far through a stage that starts at `start` and lasts `length`, from 0 to 1.
fn stage_of(progress: f32, start: f32, length: f32) -> f32 {
    ((progress - start) / length).clamp(0., 1.)
}

/// The point `share` of the way through a span.
fn along((from, to): (f32, f32), share: f32) -> f32 {
    from + (to - from) * share
}

/// How far through the rush and fade, from 0 to 1.
fn rushing(progress: f32) -> f32 {
    stage_of(progress, RUSH_FROM, 1. - RUSH_FROM)
}

fn ease_out_cubic(t: f32) -> f32 {
    1. - (1. - t).powi(3)
}

/// Past 1 and back: the overshoot of a pop.
fn back_out(t: f32) -> f32 {
    const OVERSHOOT: f32 = 1.7;
    let t = t - 1.;
    1. + t * t * ((OVERSHOOT + 1.) * t + OVERSHOOT)
}

#[cfg(test)]
mod tests {
    use super::{CROUCH, FALL, LEAP, back_out, fed, pawn_shape, rushing, starting_pieces};

    #[test]
    fn a_whole_set_spirals_in() {
        let pieces = starting_pieces();
        assert_eq!(pieces.len(), 32);
        assert_eq!(pieces.last().map(|(order, _)| *order), Some(31));
    }

    #[test]
    fn the_pawn_grows_as_it_eats_and_ends_full_size() {
        assert!(fed(0.) < 1.);
        assert!((fed(CROUCH.0) - 1.).abs() < 1e-6);
    }

    #[test]
    fn the_pawn_crouches_leaps_and_lands_back_in_shape() {
        let (wide, tall, lift) = pawn_shape(CROUCH.1 - 0.01);
        assert!(
            wide > 1. && tall < 1. && lift.abs() < f32::EPSILON,
            "crouching"
        );
        let (wide, tall, lift) = pawn_shape(LEAP.1);
        assert!(wide < 1. && tall > 1. && lift > 0.5, "stretched at the top");
        let (wide, tall, lift) = pawn_shape(FALL.1 + 0.5);
        assert!((wide - 1.).abs() < 1e-6 && (tall - 1.).abs() < 1e-6 && lift.abs() < 1e-6);
    }

    #[test]
    fn pops_overshoot_and_the_rush_comes_last() {
        assert!(back_out(0.7) > 1.);
        assert!(rushing(0.7).abs() < f32::EPSILON);
        assert!((rushing(1.) - 1.).abs() < f32::EPSILON);
    }
}
