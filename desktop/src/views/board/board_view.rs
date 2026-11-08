use std::{cell::Cell, rc::Rc};

use chess_domain::{Board, Move, PieceKind, Position, Side, Square};
use gpui_kit::*;

use super::drag_layer::DragLayer;
use crate::{
    components::board::{
        BoardBounds, BoardGrid, BoardStyle, PromotionPrompt, SLIDE_TIME, piece_image,
    },
    enums::{board::ClickMode, sound::Sound},
    models::board::{MoveMade, ShownPosition},
    services::board::{BoardInteraction, PremoveQueue, click_mode, end_badges, move_slides},
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
    /// The position last drawn and the move that led to it, to see what changed since.
    seen: Option<(Position, Option<Move>)>,
    /// Pieces gliding to their new squares, and which slide this is.
    sliding: Vec<(Square, Square)>,
    slide_run: usize,
    /// A move made on this board: the player already put the piece there, so it does not slide.
    own_move: Option<Move>,
    _slide_end: Option<Task<()>>,
    /// The square whose piece the player holds down, drawn by the drag layer meanwhile.
    held: Option<Square>,
    /// Where the board is drawn, measured as it is laid out, for the held piece.
    bounds: BoardBounds,
    /// The held piece, following the cursor: only it redraws as the mouse moves.
    drag_layer: Entity<DragLayer>,
    /// A piece dragged away and back to its own square: the click that the press and release
    /// make there is not one.
    skip_click: Option<Square>,
    /// How many times the board was drawn, for tests that it is not redrawn needlessly.
    #[cfg(test)]
    renders: usize,
}

impl EventEmitter<MoveMade> for BoardView {}

impl BoardView {
    pub fn new(
        shown: impl Fn(&App) -> Option<ShownPosition> + 'static,
        cx: &mut Context<Self>,
    ) -> Self {
        let bounds = BoardBounds::new(Cell::new(Bounds::default()));
        let view = cx.weak_entity();
        let drag_layer = cx.new(|_| DragLayer::new(Rc::clone(&bounds), view));
        Self {
            interaction: BoardInteraction::default(),
            premoves: PremoveQueue::default(),
            pending_premove: false,
            shown: Rc::new(shown),
            seen: None,
            sliding: Vec::new(),
            slide_run: 0,
            own_move: None,
            _slide_end: None,
            held: None,
            bounds,
            drag_layer,
            skip_click: None,
            #[cfg(test)]
            renders: 0,
        }
    }

    pub fn flipped(&self) -> bool {
        self.interaction.flipped()
    }

    /// The side playing from the bottom of the board.
    pub fn bottom_side(&self) -> Side {
        if self.flipped() {
            Side::Black
        } else {
            Side::White
        }
    }

    /// Turns the board so `side` plays from the bottom.
    pub fn face(&mut self, side: Side, cx: &mut Context<Self>) {
        if self.interaction.flipped() != (side == Side::Black) {
            self.flip(cx);
        }
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
        self.let_go(cx);
        cx.notify();
    }

    /// Everything the player was doing on the board goes, premoves too.
    pub fn reset(&mut self, cx: &mut Context<Self>) {
        self.let_go(cx);
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
    pub fn renders(&self) -> usize {
        self.renders
    }

    #[cfg(test)]
    pub fn selected(&self) -> Option<Square> {
        self.interaction.selected()
    }

    fn click_square(&mut self, square: Square, cx: &mut Context<Self>) {
        if self.skip_click.take() == Some(square) {
            return;
        }
        let Some(shown) = (self.shown)(cx) else {
            return;
        };
        match self.click_mode(&shown, square) {
            ClickMode::Move => {
                let mv = self
                    .interaction
                    .click(&shown.position, square, moves_allowed(&shown));
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
            ClickMode::Move => {
                self.interaction
                    .drop(&shown.position, from, to, moves_allowed(&shown))
            }
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
        if moves_allowed(shown) && from != to && mv.is_none() && !self.interaction.is_promoting() {
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

    /// A piece that can be moved is pressed: it is held, gliding to the cursor and then
    /// following it, until the button comes up.
    fn grab(&mut self, square: Square, cursor: Point<Pixels>, cx: &mut Context<Self>) {
        let Some(shown) = (self.shown)(cx) else {
            return;
        };
        let Some(piece) = self.preview(&shown).piece_at(square) else {
            return;
        };
        let picture = piece_image(piece, BoardStyle::current(cx).pieces).path;
        let flipped = self.interaction.flipped();
        self.held = Some(square);
        self.drag_layer.update(cx, |layer, cx| {
            layer.hold((square, picture), flipped, cursor, cx);
        });
        cx.notify();
    }

    /// The held piece is let go: dragged onto a square, it is a drop there, as a move or a
    /// premove; otherwise it goes back, and a press without a drag counts as a click.
    pub(super) fn land(
        &mut self,
        from: Square,
        to: Option<Square>,
        dragged: bool,
        cx: &mut Context<Self>,
    ) {
        self.held = None;
        match to.filter(|_| dragged) {
            Some(to) if to == from => {
                self.skip_click = Some(from);
                cx.notify();
            }
            Some(to) => self.drop_piece(from, to, cx),
            None => cx.notify(),
        }
    }

    /// The held piece goes back to its square without a move.
    fn let_go(&mut self, cx: &mut Context<Self>) {
        if self.held.take().is_some() {
            self.drag_layer.update(cx, |layer, cx| layer.let_go(cx));
            cx.notify();
        }
    }

    /// A right-click puts back a held piece; with premoves queued it cancels them all, as on
    /// chess.com; either way it draws nothing.
    fn right_down(&mut self, square: Square, cx: &mut Context<Self>) {
        if self.held.is_some() {
            self.let_go(cx);
            return;
        }
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
            self.own_move = Some(mv);
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
    /// board that takes no moves or no premoves, nothing is premoved.
    fn click_mode(&self, shown: &ShownPosition, square: Square) -> ClickMode {
        if !premoves_allowed(shown) {
            return ClickMode::Move;
        }
        click_mode(
            &self.preview(shown),
            waiting_side(shown),
            self.interaction.selected(),
            square,
        )
    }

    /// The pieces as the queued premoves leave them; a board without premoves as it is.
    fn preview(&self, shown: &ShownPosition) -> Board {
        if shown.can_move && shown.premoves {
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

    /// The side to move's pieces can be dragged to move, and with premoves the other side's
    /// to premove; none on a board that takes no moves.
    fn draggable(&self, shown: &ShownPosition) -> Vec<Side> {
        if !self.interaction.accepts_moves(shown.can_move) {
            return Vec::new();
        }
        let mover = shown.position.side_to_move();
        let movable = [
            (mover, moves_allowed(shown)),
            (mover.opponent(), premoves_allowed(shown)),
        ];
        movable
            .into_iter()
            .filter_map(|(side, allowed)| allowed.then_some(side))
            .collect()
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

impl BoardView {
    /// A step of one move forward or back slides the pieces that moved; a jump, a new position,
    /// or a move the player made here does not.
    fn follow_shown(&mut self, shown: &ShownPosition, cx: &mut Context<Self>) {
        let now = (shown.position, shown.last_move);
        let Some(before) = self.seen.replace(now) else {
            return;
        };
        if before.0.is_same_position(&now.0) && before.1 == now.1 {
            return;
        }
        let own = self.own_move.take();
        let slides = if own.is_some() && own == now.1 {
            Vec::new()
        } else {
            move_slides((&before.0, before.1), (&now.0, now.1))
        };
        self.sliding = slides;
        if self.sliding.is_empty() {
            self._slide_end = None;
            return;
        }
        self.slide_run += 1;
        self._slide_end = Some(end_slide_later(self.slide_run, cx));
    }
}

/// The icons on the board: the reviewed move's class, and the kings' once the game is over.
fn badges_of(shown: &ShownPosition) -> Vec<(Square, &'static str)> {
    let review = shown
        .badge
        .map(|(square, class)| (square, class.icon_path()));
    let kings = shown.outcome.into_iter().flat_map(|outcome| {
        end_badges(shown.position.board(), outcome)
            .into_iter()
            .map(|(square, badge)| (square, badge.icon_path()))
    });
    review.into_iter().chain(kings).collect()
}

/// Puts the pieces back on their squares once `run` has had its time.
fn end_slide_later(run: usize, cx: &mut Context<BoardView>) -> Task<()> {
    cx.spawn(async move |view, cx| {
        cx.background_executor().timer(SLIDE_TIME).await;
        let _ = view.update(cx, |view, cx| {
            if view.slide_run == run {
                view.sliding.clear();
                cx.notify();
            }
        });
    })
}

impl Render for BoardView {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let Some(shown) = (self.shown)(cx) else {
            return div().into_any_element();
        };
        self.follow_shown(&shown, cx);
        #[cfg(test)]
        {
            self.renders += 1;
        }
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
        .badges(badges_of(&shown))
        .last_move_tint(shown.badge.map(|(_, class)| class.color()))
        .best_arrow(shown.best_arrow)
        .slides(self.sliding.clone(), self.slide_run)
        .promotion(self.promotion_prompt(&shown, view.clone()))
        .on_right_button(right_down(view.clone()), right_up(view.clone()))
        .on_square_click(click_square(view.clone()))
        .on_piece_grab(grab(view))
        .held(self.held)
        .bounds(Rc::clone(&self.bounds))
        .overlay(self.drag_layer.clone())
        .into_any_element()
    }
}

/// The side to move may move: the board takes moves, and is theirs or anyone's.
fn moves_allowed(shown: &ShownPosition) -> bool {
    shown.can_move
        && shown
            .player
            .is_none_or(|player| player == shown.position.side_to_move())
}

/// The side waiting may premove: the board takes premoves, and is theirs or anyone's.
fn premoves_allowed(shown: &ShownPosition) -> bool {
    shown.can_move
        && shown.premoves
        && shown
            .player
            .is_none_or(|player| player == waiting_side(shown))
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

fn grab(
    view: WeakEntity<BoardView>,
) -> impl Fn(Square, Point<Pixels>, &mut Window, &mut App) + 'static {
    move |square, cursor, _, cx| {
        let _ = view.update(cx, |view, cx| view.grab(square, cursor, cx));
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
