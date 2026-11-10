use chess_domain::{Game, Position};
use gpui_kit::{
    assets::IconName,
    component::{
        Disableable as _, Sizable as _,
        button::{Button, ButtonVariants},
        v_flex,
    },
    *,
};

use crate::i18n::tr;

/// The copy button beside the board. Pressed, it turns into FEN, a board's grid, where it
/// stands, with PGN, a list of moves, just under it; either copies and turns it back, as does a
/// click anywhere else.
#[derive(IntoElement)]
pub struct CopyButtons {
    fen: SharedString,
    /// The whole game, once it has a move.
    pgn: Option<SharedString>,
}

/// What the board shows and the game it belongs to.
pub fn copy_buttons(shown: &Position, game: &Game) -> CopyButtons {
    CopyButtons {
        fen: shown.to_string().into(),
        pgn: (!game.sans().is_empty()).then(|| game.to_pgn().into()),
    }
}

impl RenderOnce for CopyButtons {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let open = window.use_keyed_state("copy-open", cx, |_, _| false);
        if !*open.read(cx) {
            return div()
                .debug_selector(|| "copy".to_owned())
                .child(
                    Button::new("copy")
                        .ghost()
                        .xsmall()
                        .icon(IconName::Copy)
                        .tooltip(tr!("board.copy"))
                        .on_click(move |_, _, cx| set_open(&open, true, cx)),
                )
                .into_any_element();
        }
        let outside = open.clone();
        v_flex()
            .items_center()
            .gap_1()
            .on_mouse_down_out(move |_, _, cx| set_open(&outside, false, cx))
            .child(copy_choice(
                ("copy-fen", IconName::Grid3x3, tr!("board.copy_fen")),
                Some(self.fen),
                open.clone(),
            ))
            .child(copy_choice(
                ("copy-pgn", IconName::ListOrdered, tr!("board.copy_pgn")),
                self.pgn,
                open,
            ))
            .into_any_element()
    }
}

/// One choice: it copies its text, then the buttons turn back into the copy button; nothing to
/// copy, and it cannot be pressed.
fn copy_choice(
    (id, icon, tooltip): (&'static str, IconName, String),
    text: Option<SharedString>,
    open: Entity<bool>,
) -> Div {
    div().debug_selector(move || id.to_owned()).child(
        Button::new(id)
            .ghost()
            .xsmall()
            .icon(icon)
            .tooltip(tooltip)
            .disabled(text.is_none())
            .on_click(move |_, _, cx| {
                if let Some(text) = &text {
                    cx.write_to_clipboard(ClipboardItem::new_string(text.to_string()));
                }
                set_open(&open, false, cx);
            }),
    )
}

/// Opens or closes the choices.
fn set_open(open: &Entity<bool>, to: bool, cx: &mut App) {
    open.update(cx, |open, cx| {
        *open = to;
        cx.notify();
    });
}

#[cfg(test)]
mod tests {
    use chess_domain::Game;
    use gpui_kit::{Context, IntoElement, Render, TestAppContext, Window, div, prelude::*};

    use super::copy_buttons;
    use crate::testing::click;

    /// The copy buttons for a game of one move, 1. e4.
    struct Harness {
        game: Game,
    }

    impl Render for Harness {
        fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
            // As beside the board: in a slot that fits it, not stretched across the window.
            div().flex().items_start().child(copy_buttons(
                self.game.positions().last().expect("a position"),
                &self.game,
            ))
        }
    }

    #[gpui_kit::test]
    fn copy_turns_into_fen_where_it_stood_with_pgn_under_it(cx: &mut TestAppContext) {
        cx.update(gpui_kit::init);
        let mut game = Game::new();
        game.play("e2e4".parse().expect("a move")).expect("legal");
        let pgn = game.to_pgn();
        let (_, cx) = cx.add_window_view(|_, _| Harness { game });
        cx.run_until_parked();
        let copy = cx.debug_bounds("copy").expect("the copy button");
        assert!(cx.debug_bounds("copy-fen").is_none(), "closed at first");

        click(cx, "copy");
        cx.run_until_parked();
        let fen = cx.debug_bounds("copy-fen").expect("FEN shows");
        let under = cx.debug_bounds("copy-pgn").expect("PGN shows");
        assert_eq!(fen.top(), copy.top(), "FEN takes the copy button's place");
        assert!(under.top() >= fen.bottom(), "PGN sits under FEN");
        assert!(
            cx.debug_bounds("copy").is_none(),
            "the copy button is gone meanwhile"
        );

        click(cx, "copy-pgn");
        cx.run_until_parked();
        let copied = cx.read_from_clipboard().and_then(|item| item.text());
        assert_eq!(copied.as_deref(), Some(pgn.as_str()));
        assert!(cx.debug_bounds("copy").is_some(), "and it turns back");
    }
}
