use std::rc::Rc;

use gpui_kit::{
    component::{ActiveTheme as _, h_flex, label::Label, v_flex},
    prelude::FluentBuilder as _,
    *,
};

type OnSelect = Rc<dyn Fn(usize, &mut Window, &mut App)>;

/// The game's moves in numbered rows, White's then Black's; clicking one shows the position after it.
#[derive(IntoElement)]
pub struct MoveList {
    sans: Vec<String>,
    /// The shown position: 0 is the start, `n` is after the n-th move.
    viewing: usize,
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
            on_select: Rc::new(on_select),
        }
    }
}

impl RenderOnce for MoveList {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        v_flex()
            .id("move-list")
            .w_full()
            .max_h(rems(16.))
            .overflow_y_scroll()
            .p_2()
            .rounded(cx.theme().radius)
            .bg(cx.theme().muted)
            .text_sm()
            .when(self.sans.is_empty(), |this| this.child(empty_text()))
            .children(self.rows(cx))
    }
}

impl MoveList {
    fn rows(&self, cx: &App) -> Vec<Div> {
        move_rows(&self.sans)
            .into_iter()
            .map(|row| self.row(row, cx))
            .collect()
    }

    fn row(&self, row: MoveRow<'_>, cx: &App) -> Div {
        h_flex()
            .gap_2()
            .child(number_cell(row.number, cx))
            .child(self.move_cell(row.white, cx))
            .when_some(row.black, |this, black| {
                this.child(self.move_cell(black, cx))
            })
    }

    /// One move; highlighted while the board shows the position after it.
    fn move_cell(&self, (index, san): (usize, &str), cx: &App) -> Stateful<Div> {
        let on_select = Rc::clone(&self.on_select);
        div()
            .id(("move", index))
            .flex_1()
            .px_2()
            .py_1()
            .rounded(cx.theme().radius)
            .cursor_pointer()
            .hover(|this| this.bg(cx.theme().accent))
            .when(self.viewing == index + 1, |this| {
                this.bg(cx.theme().primary)
                    .text_color(cx.theme().primary_foreground)
            })
            .on_click(move |_, window, cx| on_select(index, window, cx))
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

fn empty_text() -> Label {
    Label::new("No moves yet")
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
