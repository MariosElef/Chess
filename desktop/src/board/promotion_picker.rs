use std::rc::Rc;

use chess_domain::{Piece, PieceKind, Side};
use gpui_kit::{
    component::{ActiveTheme as _, h_flex},
    *,
};

use super::config::{BoardColors, piece_image};

const CHOICES: [PieceKind; 4] = [
    PieceKind::Queen,
    PieceKind::Rook,
    PieceKind::Bishop,
    PieceKind::Knight,
];

pub type OnPick = Rc<dyn Fn(PieceKind, &mut Window, &mut App)>;
pub type OnCancel = Rc<dyn Fn(&mut Window, &mut App)>;

/// Asks what a pawn on the last rank becomes; clicking beside the choices cancels the move.
#[derive(Clone)]
pub struct PromotionPrompt {
    pub side: Side,
    pub on_pick: OnPick,
    pub on_cancel: OnCancel,
}

#[derive(IntoElement)]
pub struct PromotionPicker {
    prompt: PromotionPrompt,
    colors: BoardColors,
}

impl PromotionPicker {
    pub fn new(prompt: PromotionPrompt, colors: BoardColors) -> Self {
        Self { prompt, colors }
    }
}

impl RenderOnce for PromotionPicker {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let on_cancel = Rc::clone(&self.prompt.on_cancel);
        div()
            .id("promotion-backdrop")
            .absolute()
            .inset_0()
            .flex()
            .items_center()
            .justify_center()
            .bg(self.colors.overlay)
            .on_click(move |_, window, cx| on_cancel(window, cx))
            .child(choices(&self.prompt, cx))
    }
}

fn choices(prompt: &PromotionPrompt, cx: &App) -> Div {
    h_flex()
        .gap_2()
        .p_3()
        .rounded(cx.theme().radius)
        .bg(cx.theme().popover)
        .children(CHOICES.map(|kind| choice(kind, prompt, cx)))
}

/// One piece to promote to; its click stops at the piece so the backdrop does not cancel.
fn choice(kind: PieceKind, prompt: &PromotionPrompt, cx: &App) -> Stateful<Div> {
    let on_pick = Rc::clone(&prompt.on_pick);
    div()
        .id(("promote", kind_index(kind)))
        .size(rems(4.5))
        .rounded(cx.theme().radius)
        .cursor_pointer()
        .hover(|this| this.bg(cx.theme().accent))
        .on_click(move |_, window, cx| {
            cx.stop_propagation();
            on_pick(kind, window, cx);
        })
        .child(img(piece_image(Piece::new(prompt.side, kind)).path).size_full())
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
