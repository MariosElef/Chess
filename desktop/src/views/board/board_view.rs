use std::rc::Rc;

use chess_domain::{Board, Move, PieceKind, Position, Side, Square};
use gpui_kit::*;

use crate::{
    components::board::{BoardGrid, PromotionPrompt},
    enums::{board::ClickMode, sound::Sound},
    models::board::{MoveMade, ShownPosition},
    services::board::{BoardInteraction, PremoveQueue, click_mode},
    views::sound::play_sound,
};

/// Reads what the board shows from the page that owns it; `None` once that page is gone.
type ShownSource = Rc<dyn Fn(&App) -> Option<ShownPosition>>;

/// The board every page shares: picking up and dropping pieces, premoves, the promotion
/// picker, marks and arrows, and which way up it is. The page says what to show and plays
/// the moves made on it, each its own way.
pub struct BoardView {
    interaction: BoardInteraction,
    /// Moves each side has queued for when its turn comes.
    premoves: PremoveQueue,
    /// The open promotion picker is for a premove, not a move.
    pending_premove: bool,
    shown: ShownSource,
}

impl EventEmitter<MoveMade> for BoardView {}

impl BoardView {
    pub fn new(shown: impl Fn(&App) -> Option<ShownPosition> + 'static) -> Self {
        Self {
            interaction: BoardInteraction::default(),
            premoves: PremoveQueue::default(),
            pending_premove: false,
            shown: Rc::new(shown),
        }
    }

    pub fn flipped(&self) -> bool {
        self.interaction.flipped()
    }

    pub fn flip(&mut self, cx: &mut Context<Self>) {
        self.interaction.flip();
        cx.notify();
    }

    /// The side to move's next premove, if it is legal in `position`; an illegal one drops
    /// that side's whole queue, as chess.com does.
    pub fn next_premove(&mut self, position: &Position) -> Option<Move> {
        self.premoves.next_playable(position)
    }

    pub fn drop_premoves(&mut self, side: Side) {
        self.premoves.clear(side);
    }

    pub fn drop_all_premoves(&mut self) {
        self.premoves.clear_all();
    }

    /// Puts down a picked-up piece, closes the promotion picker and wipes the drawings,
    /// keeping the premoves.
    pub fn put_down(&mut self, cx: &mut Context<Self>) {
        self.interaction.clear();
        cx.notify();
    }

    /// Everything the player was doing on the board goes, premoves too.
    pub fn reset(&mut self, cx: &mut Context<Self>) {
        self.interaction.clear();
        self.premoves.clear_all();
        self.pending_premove = false;
        cx.notify();
    }

    #[cfg(test)]
    pub fn premoves(&self) -> &PremoveQueue {
        &self.premoves
    }

    #[cfg(test)]
    pub fn selected(&self) -> Option<Square> {
        self.interaction.selected()
    }

    fn click_square(&mut self, square: Square, cx: &mut Context<Self>) {
        let Some(shown) = (self.shown)(cx) else {
            return;
        };
        match self.click_mode(&shown, square) {
            ClickMode::Move => {
                let mv = self
                    .interaction
                    .click(&shown.position, square, shown.can_move);
                self.move_if_any(mv, cx);
            }
            ClickMode::Premove => {
                let mv = self.interaction.premove_click(
                    &self.preview(&shown),
                    waiting_side(&shown),
                    square,
                );
                self.queue_if_any(mv, &shown, cx);
            }
        }
        cx.notify();
    }

    fn drop_piece(&mut self, from: Square, to: Square, cx: &mut Context<Self>) {
        let Some(shown) = (self.shown)(cx) else {
            return;
        };
        let mode = self.click_mode(&shown, from);
        let mv = match mode {
            ClickMode::Move => self
                .interaction
                .drop(&shown.position, from, to, shown.can_move),
            ClickMode::Premove => {
                self.interaction
                    .premove_drop(&self.preview(&shown), waiting_side(&shown), from, to)
            }
        };
        self.sound_if_refused(&shown, mv, from, to, cx);
        match mode {
            ClickMode::Move => self.move_if_any(mv, cx),
            ClickMode::Premove => self.queue_if_any(mv, &shown, cx),
        }
        cx.notify();
    }

    /// A piece dropped on another square that took no move and opened no promotion picker
    /// went somewhere it cannot go.
    fn sound_if_refused(
        &self,
        shown: &ShownPosition,
        mv: Option<Move>,
        from: Square,
        to: Square,
        cx: &App,
    ) {
        if shown.can_move && from != to && mv.is_none() && !self.interaction.is_promoting() {
            play_sound(Sound::Illegal, cx);
        }
    }

    fn promote(&mut self, kind: PieceKind, cx: &mut Context<Self>) {
        let Some(shown) = (self.shown)(cx) else {
            return;
        };
        let mv = self.interaction.promote(kind);
        if self.pending_premove {
            self.queue_if_any(mv, &shown, cx);
        } else {
            self.move_if_any(mv, cx);
        }
        cx.notify();
    }

    fn cancel_promotion(&mut self, cx: &mut Context<Self>) {
        self.interaction.cancel_promotion();
        cx.notify();
    }

    /// A right-click with premoves queued cancels them all, as on chess.com, and draws nothing.
    fn right_down(&mut self, square: Square, cx: &mut Context<Self>) {
        if self.premoves.is_empty() {
            self.interaction.right_down(square);
        } else {
            self.premoves.clear_all();
            cx.notify();
        }
    }

    fn right_up(&mut self, square: Square, cx: &mut Context<Self>) {
        if self.interaction.right_up(square) {
            cx.notify();
        }
    }

    /// Hands a finished move to the page.
    fn move_if_any(&mut self, mv: Option<Move>, cx: &mut Context<Self>) {
        self.pending_premove = false;
        if let Some(mv) = mv {
            cx.emit(MoveMade(mv));
        }
    }

    /// Queues the premove, or remembers that the promotion picker now open is for one.
    fn queue_if_any(&mut self, mv: Option<Move>, shown: &ShownPosition, cx: &App) {
        self.pending_premove = self.interaction.is_promoting();
        if let Some(mv) = mv {
            self.premoves.queue(waiting_side(shown), mv);
            play_sound(Sound::Premove, cx);
        }
    }

    /// A click on the side to move's piece is a move, on the waiting side's a premove; on a
    /// board that takes no moves, nothing is premoved.
    fn click_mode(&self, shown: &ShownPosition, square: Square) -> ClickMode {
        if !shown.can_move {
            return ClickMode::Move;
        }
        click_mode(
            &self.preview(shown),
            waiting_side(shown),
            self.interaction.selected(),
            square,
        )
    }

    /// The pieces as the queued premoves leave them; a board that takes no moves as it is.
    fn preview(&self, shown: &ShownPosition) -> Board {
        if shown.can_move {
            self.premoves
                .preview(shown.position.board(), shown.position.side_to_move())
        } else {
            *shown.position.board()
        }
    }

    /// The dots: where the selected piece may move, or may premove on the preview.
    fn targets(&self, shown: &ShownPosition, board: &Board) -> Vec<Square> {
        match self.interaction.selected() {
            Some(selected) if self.click_mode(shown, selected) == ClickMode::Premove => {
                self.interaction.premove_targets(board)
            }
            Some(_) | None => self.interaction.legal_targets(&shown.position),
        }
    }

    /// Either side's pieces can be dragged, the side to move's to move and the other's to
    /// premove, unless the board takes no moves.
    fn draggable(&self, shown: &ShownPosition) -> Vec<Side> {
        if self.interaction.accepts_moves(shown.can_move) {
            vec![Side::White, Side::Black]
        } else {
            Vec::new()
        }
    }

    /// The pawn's side: the mover's, or the waiting side's for a premove.
    fn promoting_side(&self, shown: &ShownPosition) -> Side {
        let mover = shown.position.side_to_move();
        if self.pending_premove {
            mover.opponent()
        } else {
            mover
        }
    }

    fn promotion_prompt(
        &self,
        shown: &ShownPosition,
        view: WeakEntity<Self>,
    ) -> Option<PromotionPrompt> {
        self.interaction.is_promoting().then(|| PromotionPrompt {
            side: self.promoting_side(shown),
            on_pick: Rc::new(pick_promotion(view.clone())),
            on_cancel: Rc::new(cancel_promotion(view)),
        })
    }
}

impl Render for BoardView {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let Some(shown) = (self.shown)(cx) else {
            return div().into_any_element();
        };
        let view = cx.entity().downgrade();
        let board = self.preview(&shown);
        BoardGrid::for_interaction(
            &self.interaction,
            &shown.position,
            &board,
            self.targets(&shown, &board),
            shown.last_move,
            self.draggable(&shown),
        )
        .premoves(self.premoves.pairs())
        .engine_arrow(shown.engine_arrow)
        .promotion(self.promotion_prompt(&shown, view.clone()))
        .on_right_button(right_down(view.clone()), right_up(view.clone()))
        .on_square_click(click_square(view.clone()))
        .on_piece_drop(drop_piece(view))
        .into_any_element()
    }
}

/// The side not to move: the one that may queue premoves.
fn waiting_side(shown: &ShownPosition) -> Side {
    shown.position.side_to_move().opponent()
}

// The view outlives what it renders; a failed update means the window is closing.

fn click_square(view: WeakEntity<BoardView>) -> impl Fn(Square, &mut Window, &mut App) + 'static {
    move |square, _, cx| {
        let _ = view.update(cx, |view, cx| view.click_square(square, cx));
    }
}

fn drop_piece(
    view: WeakEntity<BoardView>,
) -> impl Fn(Square, Square, &mut Window, &mut App) + 'static {
    move |from, to, _, cx| {
        let _ = view.update(cx, |view, cx| view.drop_piece(from, to, cx));
    }
}

fn right_down(view: WeakEntity<BoardView>) -> impl Fn(Square, &mut Window, &mut App) + 'static {
    move |square, _, cx| {
        let _ = view.update(cx, |view, cx| view.right_down(square, cx));
    }
}

fn right_up(view: WeakEntity<BoardView>) -> impl Fn(Square, &mut Window, &mut App) + 'static {
    move |square, _, cx| {
        let _ = view.update(cx, |view, cx| view.right_up(square, cx));
    }
}

fn pick_promotion(
    view: WeakEntity<BoardView>,
) -> impl Fn(PieceKind, &mut Window, &mut App) + 'static {
    move |kind, _, cx| {
        let _ = view.update(cx, |view, cx| view.promote(kind, cx));
    }
}

fn cancel_promotion(view: WeakEntity<BoardView>) -> impl Fn(&mut Window, &mut App) + 'static {
    move |_, cx| {
        let _ = view.update(cx, |view, cx| view.cancel_promotion(cx));
    }
}
