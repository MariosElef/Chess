use std::rc::Rc;

use crate::{components::panel::fill_scroll, enums::analysis::MoveClass, i18n::tr};

use gpui_kit::{
    component::{ActiveTheme as _, h_flex, v_flex},
    prelude::FluentBuilder as _,
    *,
};

type OnSelect = Rc<dyn Fn(usize, &mut Window, &mut App)>;

/// The moves in numbered rows, White's then Black's; clicking one shows the position after it.
#[derive(IntoElement)]
pub struct MoveList {
    sans: Vec<String>,
    /// The shown position: 0 is the start, `n` is after the n-th move.
    viewing: usize,
    /// Each move's class once the game is reviewed; empty otherwise.
    classes: Vec<MoveClass>,
    /// Every move at full height, for a list inside a scroll of its own.
    unbounded: bool,
    on_select: OnSelect,
}

impl MoveList {
    pub fn new(
        sans: Vec<String>,
        viewing: usize,
        on_select: impl Fn(usize, &mut Window, &mut App) + 'static,
    ) -> Self {
        Self {
            sans,
            viewing,
            classes: Vec::new(),
            unbounded: false,
            on_select: Rc::new(on_select),
        }
    }

    /// Shows every move at full height, inside a scroll that holds more than the list.
    pub fn unbounded(mut self) -> Self {
        self.unbounded = true;
        self
    }

    /// Shows each move's review icon beside it.
    pub fn classes(mut self, classes: Vec<MoveClass>) -> Self {
        self.classes = classes;
        self
    }
}

impl RenderOnce for MoveList {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        // It takes the room its panel has, and scrolls inside it, unless its panel scrolls it.
        let list = v_flex()
            .w_full()
            .text_sm()
            .when(self.sans.is_empty(), |this| this.child(empty_text(cx)))
            .children(self.rows(cx));
        if self.unbounded {
            list.into_any_element()
        } else {
            fill_scroll("move-list", rems(10.), list).into_any_element()
        }
    }
}

impl MoveList {
    fn rows(&self, cx: &App) -> Vec<Div> {
        move_rows(&self.sans)
            .into_iter()
            .enumerate()
            .map(|(place, row)| self.row(row, place % 2 == 1, cx))
            .collect()
    }

    /// A numbered row, every other one on a faint stripe.
    fn row(&self, row: MoveRow<'_>, striped: bool, cx: &App) -> Div {
        h_flex()
            .w_full()
            .px_2()
            .py_0p5()
            .gap_2()
            .rounded(px(4.))
            .when(striped, |this| this.bg(cx.theme().muted.opacity(0.5)))
            .child(number_cell(row.number, cx))
            .child(self.move_cell(row.white, cx))
            .child(match row.black {
                Some(black) => self.move_cell(black, cx).into_any_element(),
                None => div().flex_1().into_any_element(),
            })
    }

    /// One move with its review icon, highlighted while the board shows the position after it.
    fn move_cell(&self, (index, san): (usize, &str), cx: &App) -> Stateful<Div> {
        let on_select = Rc::clone(&self.on_select);
        let class = self.classes.get(index).copied();
        h_flex()
            .id(("move", index))
            .gap_1()
            .flex_1()
            .px_2()
            .py_1()
            .rounded(cx.theme().radius)
            .cursor_pointer()
            .hover(|this| this.bg(cx.theme().accent))
            .font_weight(FontWeight::SEMIBOLD)
            .when(self.viewing == index + 1, |this| {
                this.bg(cx.theme().accent)
                    .border_b_2()
                    .border_color(cx.theme().input)
            })
            .on_click(move |_, window, cx| on_select(index, window, cx))
            .when_some(class, |this, class| {
                this.child(img(class.icon_path()).size_4().flex_shrink_0())
            })
            .child(san.to_owned())
    }
}

fn number_cell(number: usize, cx: &App) -> Div {
    div()
        .w_8()
        .py_1()
        .text_color(cx.theme().muted_foreground)
        .child(format!("{number}."))
}

fn empty_text(cx: &App) -> Div {
    div()
        .py_2()
        .text_color(cx.theme().muted_foreground)
        .child(tr!("history.moves.empty"))
}

/// A move number with White's move and Black's reply, each with its index into the move list.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct MoveRow<'a> {
    number: usize,
    white: (usize, &'a str),
    black: Option<(usize, &'a str)>,
}

fn move_rows(sans: &[String]) -> Vec<MoveRow<'_>> {
    sans.chunks(2)
        .enumerate()
        .map(|(row, pair)| MoveRow {
            number: row + 1,
            white: (row * 2, pair[0].as_str()),
            black: pair.get(1).map(|san| (row * 2 + 1, san.as_str())),
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::{MoveRow, move_rows};

    #[test]
    fn moves_pair_up_into_numbered_rows() {
        let sans = ["e4", "e5", "Nf3"].map(String::from);
        assert_eq!(
            move_rows(&sans),
            [
                MoveRow {
                    number: 1,
                    white: (0, "e4"),
                    black: Some((1, "e5")),
                },
                MoveRow {
                    number: 2,
                    white: (2, "Nf3"),
                    black: None,
                },
            ]
        );
    }

    #[test]
    fn no_moves_means_no_rows() {
        assert!(move_rows(&[]).is_empty());
    }
}
