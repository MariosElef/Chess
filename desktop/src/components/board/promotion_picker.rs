use std::rc::Rc;

use chess_domain::{Piece, PieceKind, Side};
use gpui_kit::{
    assets::IconName,
    component::{ActiveTheme as _, Icon, v_flex},
    prelude::FluentBuilder as _,
    *,
};

use super::{board_style::BoardStyle, piece_images::piece_image};
use crate::services::board::Cell;

/// The choices from the promotion square inwards, as chess.com orders them.
const CHOICES: [PieceKind; 4] = [
    PieceKind::Queen,
    PieceKind::Knight,
    PieceKind::Rook,
    PieceKind::Bishop,
];
/// One square, and the column's four and a half squares, as shares of the board.
const SQUARE: f32 = 0.125;
const COLUMN: f32 = 0.5625;
/// Within the column, each piece takes a square and Cancel half of one.
const PIECE_SHARE: f32 = 2. / 9.;
const CANCEL_SHARE: f32 = 1. / 9.;
/// Where each file starts, as a share of the board's width.
const FILE_STARTS: [f32; 8] = [0., 0.125, 0.25, 0.375, 0.5, 0.625, 0.75, 0.875];

pub type OnPick = Rc<dyn Fn(PieceKind, &mut Window, &mut App)>;
pub type OnCancel = Rc<dyn Fn(&mut Window, &mut App)>;

/// Asks what a pawn on the last rank becomes, on the square it promotes on; clicking anywhere
/// else, or the cross, cancels the move.
#[derive(Clone)]
pub struct PromotionPrompt {
    pub side: Side,
    /// The promotion square as drawn: the column hangs down from it at the top of the board, or
    /// rises from it at the bottom.
    pub cell: Cell,
    pub on_pick: OnPick,
    pub on_cancel: OnCancel,
}

#[derive(IntoElement)]
pub struct PromotionPicker {
    prompt: PromotionPrompt,
}

impl PromotionPicker {
    pub fn new(prompt: PromotionPrompt) -> Self {
        Self { prompt }
    }
}

impl RenderOnce for PromotionPicker {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let on_cancel = Rc::clone(&self.prompt.on_cancel);
        // A clear layer over the board: a click beside the column cancels, as on chess.com.
        div()
            .id("promotion-backdrop")
            .absolute()
            .inset_0()
            .on_click(move |_, window, cx| on_cancel(window, cx))
            .child(column(&self.prompt, cx))
    }
}

/// The pieces in a white column on the promotion square's file, the queen on that square.
fn column(prompt: &PromotionPrompt, cx: &App) -> Stateful<Div> {
    let (file, row) = prompt.cell;
    let from_top = row == 0;
    let on_cancel = Rc::clone(&prompt.on_cancel);
    let cancel = div()
        .id("promote-cancel")
        .w_full()
        .h(relative(CANCEL_SHARE))
        .flex()
        .items_center()
        .justify_center()
        .bg(rgb(0x00_ee_ee_ee))
        .text_color(rgb(0x00_77_77_77))
        .cursor_pointer()
        .hover(|this| this.bg(rgb(0x00_dd_dd_dd)))
        .on_click(move |_, window, cx| {
            cx.stop_propagation();
            on_cancel(window, cx);
        })
        .child(Icon::new(IconName::Close).size_4());
    v_flex()
        .id("promotion-choices")
        .absolute()
        .left(relative(FILE_STARTS[usize::from(file.min(7))]))
        .w(relative(SQUARE))
        .h(relative(COLUMN))
        .when(from_top, |this| this.top_0())
        // At the bottom of the board the column rises, the queen still on the promotion square.
        .when(!from_top, |this| this.bottom_0().flex_col_reverse())
        .overflow_hidden()
        .rounded(px(3.))
        .bg(white())
        .shadow_lg()
        // A click inside the column but between choices changes nothing.
        .on_click(|_, _, cx| cx.stop_propagation())
        .children(CHOICES.map(|kind| choice(kind, prompt, cx)))
        .child(cancel)
}

/// One piece to promote to; its click stops at the piece so the backdrop does not cancel.
fn choice(kind: PieceKind, prompt: &PromotionPrompt, cx: &App) -> Stateful<Div> {
    let on_pick = Rc::clone(&prompt.on_pick);
    div()
        .id(("promote", kind_index(kind)))
        .debug_selector(move || format!("promote-{}", kind_index(kind)))
        .w_full()
        .h(relative(PIECE_SHARE))
        .p(relative(0.06))
        .cursor_pointer()
        .hover(|this| this.bg(cx.theme().accent))
        .on_click(move |_, window, cx| {
            cx.stop_propagation();
            on_pick(kind, window, cx);
        })
        .child(piece_choice(Piece::new(prompt.side, kind), cx))
}

fn piece_choice(piece: Piece, cx: &App) -> Img {
    img(piece_image(piece, BoardStyle::current(cx).pieces).path).size_full()
}

fn kind_index(kind: PieceKind) -> usize {
    match kind {
        PieceKind::Queen => 0,
        PieceKind::Rook => 1,
        PieceKind::Bishop => 2,
        PieceKind::Knight => 3,
        PieceKind::King => 4,
        PieceKind::Pawn => 5,
    }
}
