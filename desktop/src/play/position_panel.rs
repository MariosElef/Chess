use chess_domain::Position;
use gpui_kit::{
    component::{
        ActiveTheme as _, clipboard::Clipboard, description_list::DescriptionList, h_flex,
        label::Label, v_flex,
    },
    *,
};

use super::status::side_name;

/// The current position beside the board: its FEN to copy, and what each part of it means.
#[derive(IntoElement)]
pub struct PositionPanel {
    position: Position,
}

impl PositionPanel {
    pub fn new(position: Position) -> Self {
        Self { position }
    }
}

impl RenderOnce for PositionPanel {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let fen = SharedString::from(self.position.to_string());
        v_flex()
            .w_80()
            .flex_shrink_0()
            .gap_3()
            .child(heading(fen.clone()))
            .child(fen_text(fen, cx))
            .child(breakdown(&self.position))
    }
}

fn heading(fen: SharedString) -> Div {
    h_flex()
        .justify_between()
        .child(Label::new("FEN").text_sm())
        .child(Clipboard::new("copy-fen").value(fen).tooltip("Copy FEN"))
}

/// Small and monospaced, so the whole string fits in two lines and the ranks line up.
fn fen_text(fen: SharedString, cx: &App) -> Div {
    div()
        .w_full()
        .overflow_hidden()
        .p_3()
        .rounded(cx.theme().radius)
        .bg(cx.theme().muted)
        .font_family(cx.theme().mono_font_family.clone())
        .text_xs()
        .child(fen)
}

/// Every field after the board layout, one labelled row each.
fn breakdown(position: &Position) -> DescriptionList {
    DescriptionList::horizontal()
        .columns(1)
        .item("Turn", turn_value(position), 1)
        .item("Castling", position.castling().to_string(), 1)
        .item("En passant", en_passant_value(position), 1)
        .item("Half-move clock", halfmove_value(position), 1)
        .item("Move", position.counters().fullmove_number.to_string(), 1)
}

fn turn_value(position: &Position) -> String {
    side_name(position.side_to_move()).to_owned()
}

fn en_passant_value(position: &Position) -> String {
    position
        .en_passant()
        .map_or_else(|| "—".to_owned(), |square| square.to_string())
}

/// Counted toward the fifty-move rule, a draw at 100.
fn halfmove_value(position: &Position) -> String {
    format!("{} / 100", position.counters().halfmove_clock)
}

#[cfg(test)]
mod tests {
    use chess_domain::{Move, Position};

    use super::{en_passant_value, halfmove_value, turn_value};

    fn after_e4_e5() -> Position {
        [("e2", "e4"), ("e7", "e5")]
            .iter()
            .fold(Position::starting(), |position, (from, to)| {
                let mv = Move::new(
                    from.parse().expect("a square"),
                    to.parse().expect("a square"),
                );
                position.play(mv).expect("legal")
            })
    }

    #[test]
    fn the_breakdown_explains_each_field() {
        let position = after_e4_e5();
        assert_eq!(turn_value(&position), "White");
        assert_eq!(en_passant_value(&position), "e6");
        assert_eq!(halfmove_value(&position), "0 / 100");
        assert_eq!(position.castling().to_string(), "KQkq");
    }

    #[test]
    fn no_en_passant_square_shows_a_dash() {
        assert_eq!(en_passant_value(&Position::starting()), "—");
    }
}
