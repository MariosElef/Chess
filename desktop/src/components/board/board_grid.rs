use std::rc::Rc;

use chess_domain::{Board, Move, Position, Side, Square};
use gpui_kit::{
    component::{ActiveTheme as _, h_flex, v_flex},
    prelude::FluentBuilder as _,
    *,
};

use super::{
    arrows::paint_arrows,
    board_colors::BoardColors,
    board_square::{BoardSquare, OnPieceGrab, OnSquareClick, RightButton},
    board_style::BoardStyle,
    piece_images::piece_image,
    promotion_picker::{PromotionPicker, PromotionPrompt},
    sliding_pieces::{SlidingPiece, sliding_layer},
};
use crate::{
    enums::{board::SquareHighlight, settings::PieceSet},
    services::board::{BoardInteraction, Cell},
};

/// Where the board is drawn on the window, measured each time it is laid out.
pub type BoardBounds = Rc<std::cell::Cell<Bounds<Pixels>>>;

/// The 8×8 board, square at whatever height its parent gives it.
#[derive(IntoElement)]
pub struct BoardGrid {
    board: Board,
    flipped: bool,
    selected: Option<Square>,
    targets: Vec<Square>,
    last_move: Option<Move>,
    check: Option<Square>,
    /// Whose pieces can be dragged; empty while the board is only being looked at.
    draggable: Vec<Side>,
    /// Queued premoves, each a from and a to square, drawn red.
    premoves: Vec<(Square, Square)>,
    promotion: Option<PromotionPrompt>,
    marks: Vec<Square>,
    arrows: Vec<(Square, Square)>,
    engine_arrow: Option<(Square, Square)>,
    /// Icons in the corners of squares: a reviewed move's class, the kings' at the end.
    badges: Vec<(Square, &'static str)>,
    /// A reviewed move's better choice, drawn in green.
    best_arrow: Option<(Square, Square)>,
    /// Pieces gliding to their squares, and which slide this is.
    slides: Vec<(Square, Square)>,
    slide_run: usize,
    /// The last move's squares take this colour (`0xRRGGBB`) once it is reviewed.
    last_move_tint: Option<u32>,
    on_square_click: Option<OnSquareClick>,
    on_piece_grab: Option<OnPieceGrab>,
    right_button: Option<RightButton>,
    /// The square whose piece the player holds: it is drawn by the overlay, not there.
    held: Option<Square>,
    /// Filled with where the board is drawn.
    bounds: Option<BoardBounds>,
    /// Drawn over everything: the held piece.
    overlay: Option<AnyElement>,
}

impl BoardGrid {
    pub fn new(board: Board) -> Self {
        Self {
            board,
            flipped: false,
            selected: None,
            targets: Vec::new(),
            last_move: None,
            check: None,
            draggable: Vec::new(),
            premoves: Vec::new(),
            promotion: None,
            marks: Vec::new(),
            arrows: Vec::new(),
            engine_arrow: None,
            badges: Vec::new(),
            best_arrow: None,
            slides: Vec::new(),
            slide_run: 0,
            last_move_tint: None,
            on_square_click: None,
            on_piece_grab: None,
            right_button: None,
            held: None,
            bounds: None,
            overlay: None,
        }
    }

    /// The square whose piece the player holds, left empty while they do.
    pub fn held(mut self, held: Option<Square>) -> Self {
        self.held = held;
        self
    }

    /// Notes where the board is drawn here, each time it is laid out.
    pub fn bounds(mut self, bounds: BoardBounds) -> Self {
        self.bounds = Some(bounds);
        self
    }

    /// Something drawn over the whole board.
    pub fn overlay(mut self, overlay: impl IntoElement) -> Self {
        self.overlay = Some(overlay.into_any_element());
        self
    }

    /// Squares the player marked red.
    pub fn marks(mut self, marks: Vec<Square>) -> Self {
        self.marks = marks;
        self
    }

    /// Arrows the player drew, from one square to another.
    pub fn arrows(mut self, arrows: Vec<(Square, Square)>) -> Self {
        self.arrows = arrows;
        self
    }

    /// The engine's best move, drawn in its own colour under the player's arrows.
    pub fn engine_arrow(mut self, arrow: Option<(Square, Square)>) -> Self {
        self.engine_arrow = arrow;
        self
    }

    /// Slides pieces from their old squares to their new ones; `run` tells one slide from the next.
    pub fn slides(mut self, slides: Vec<(Square, Square)>, run: usize) -> Self {
        self.slides = slides;
        self.slide_run = run;
        self
    }

    /// What should have been played, as a green arrow.
    pub fn best_arrow(mut self, arrow: Option<(Square, Square)>) -> Self {
        self.best_arrow = arrow;
        self
    }

    /// Colours the last move's squares (`0xRRGGBB`), as a reviewed move's class does.
    pub fn last_move_tint(mut self, tint: Option<u32>) -> Self {
        self.last_move_tint = tint;
        self
    }

    /// Icons, by asset path, each in the corner of its square.
    pub fn badges(mut self, badges: Vec<(Square, &'static str)>) -> Self {
        self.badges = badges;
        self
    }

    /// The right button went down, then up, over a square.
    pub fn on_right_button(
        mut self,
        on_down: impl Fn(Square, &mut Window, &mut App) + 'static,
        on_up: impl Fn(Square, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.right_button = Some(RightButton {
            on_down: Rc::new(on_down),
            on_up: Rc::new(on_up),
        });
        self
    }

    /// Black at the bottom instead of White.
    pub fn flipped(mut self, flipped: bool) -> Self {
        self.flipped = flipped;
        self
    }

    /// The square whose piece is picked up, drawn highlighted.
    pub fn selected(mut self, selected: Option<Square>) -> Self {
        self.selected = selected;
        self
    }

    /// The squares that piece can move to, each marked with a dot (or a ring around a capture).
    pub fn targets(mut self, targets: Vec<Square>) -> Self {
        self.targets = targets;
        self
    }

    pub fn last_move(mut self, last_move: Option<Move>) -> Self {
        self.last_move = last_move;
        self
    }

    /// The king's square while it is in check, drawn red.
    pub fn check(mut self, check: Option<Square>) -> Self {
        self.check = check;
        self
    }

    /// Whose pieces can be dragged: the side to move for its move, the other to queue a premove.
    pub fn draggable_sides(mut self, sides: Vec<Side>) -> Self {
        self.draggable = sides;
        self
    }

    pub fn premoves(mut self, premoves: Vec<(Square, Square)>) -> Self {
        self.premoves = premoves;
        self
    }

    pub fn promotion(mut self, prompt: Option<PromotionPrompt>) -> Self {
        self.promotion = prompt;
        self
    }

    pub fn on_square_click(
        mut self,
        handler: impl Fn(Square, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_square_click = Some(Rc::new(handler));
        self
    }

    /// A piece that can be moved was pressed, with where the cursor is on the window.
    pub fn on_piece_grab(
        mut self,
        handler: impl Fn(Square, Point<Pixels>, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_piece_grab = Some(Rc::new(handler));
        self
    }
}

impl BoardGrid {
    /// A board drawing `board` with everything `interaction` shows.
    pub fn for_interaction(
        interaction: &BoardInteraction,
        position: &Position,
        board: &Board,
        targets: Vec<Square>,
        last_move: Option<Move>,
        draggable: Vec<Side>,
    ) -> Self {
        Self::new(*board)
            .flipped(interaction.flipped())
            .selected(interaction.selected())
            .targets(targets)
            .last_move(last_move)
            .check(check_square(position))
            .draggable_sides(draggable)
            .marks(interaction.marks().to_vec())
            .arrows(interaction.arrows().to_vec())
    }
}

impl RenderOnce for BoardGrid {
    fn render(mut self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let colors = BoardColors::current(cx);
        let pieces = BoardStyle::current(cx).pieces;
        let overlay = self.overlay.take();
        v_flex()
            .relative()
            .h_full()
            .max_w_full()
            .aspect_square()
            .rounded(cx.theme().radius)
            .overflow_hidden()
            .children(
                self.display_ranks()
                    .map(|rank| self.rank_row(rank, colors, pieces)),
            )
            .child(sliding_layer(self.sliding_pieces(pieces), self.slide_run))
            .child(self.arrow_layer(colors))
            .children(overlay)
            .when_some(self.promotion.clone(), |this, prompt| {
                this.child(PromotionPicker::new(prompt, colors))
            })
    }
}

impl BoardGrid {
    /// Top to bottom: rank 8 first for White's view, rank 1 first when flipped.
    fn display_ranks(&self) -> [u8; 8] {
        if self.flipped {
            [0, 1, 2, 3, 4, 5, 6, 7]
        } else {
            [7, 6, 5, 4, 3, 2, 1, 0]
        }
    }

    /// Left to right: file a first for White's view, file h first when flipped.
    fn display_files(&self) -> [u8; 8] {
        if self.flipped {
            [7, 6, 5, 4, 3, 2, 1, 0]
        } else {
            [0, 1, 2, 3, 4, 5, 6, 7]
        }
    }

    /// The arrows over the squares, catching no clicks; its layout notes where the board is.
    fn arrow_layer(&self, colors: BoardColors) -> Canvas<()> {
        let measured = self.bounds.clone();
        let engine: Vec<(Cell, Cell)> = self.cells(self.engine_arrow.iter());
        let best: Vec<(Cell, Cell)> = self.cells(self.best_arrow.iter());
        let player: Vec<(Cell, Cell)> = self.cells(self.arrows.iter());
        canvas(
            move |bounds, _, _| {
                if let Some(measured) = &measured {
                    measured.set(bounds);
                }
            },
            move |bounds, (), window, _| {
                paint_arrows(bounds, &engine, colors.engine_arrow, window);
                paint_arrows(bounds, &best, colors.best_arrow, window);
                paint_arrows(bounds, &player, colors.arrow, window);
            },
        )
        .absolute()
        .inset_0()
    }

    /// Each sliding piece with its picture, read from where it stands now.
    fn sliding_pieces(&self, set: PieceSet) -> Vec<SlidingPiece> {
        self.slides
            .iter()
            .filter_map(|&(from, to)| {
                let piece = self.board.piece_at(to)?;
                Some(SlidingPiece {
                    picture: piece_image(piece, set).path,
                    from: self.cell(from),
                    to: self.cell(to),
                })
            })
            .collect()
    }

    fn cells<'a>(&self, arrows: impl Iterator<Item = &'a (Square, Square)>) -> Vec<(Cell, Cell)> {
        arrows
            .map(|&(from, to)| (self.cell(from), self.cell(to)))
            .collect()
    }

    /// Where a square is drawn: its column from the left and row from the top.
    fn cell(&self, square: Square) -> Cell {
        (
            display_index(&self.display_files(), square.file()),
            display_index(&self.display_ranks(), square.rank()),
        )
    }

    fn rank_row(&self, rank: u8, colors: BoardColors, pieces: PieceSet) -> Div {
        h_flex().flex_1().items_stretch().children(
            self.display_files()
                .map(|file| self.square((file, rank), colors, pieces)),
        )
    }

    fn square(&self, (file, rank): (u8, u8), colors: BoardColors, pieces: PieceSet) -> BoardSquare {
        let square = Square::new(file, rank).expect("0..8 stays on the board");
        let piece = self.board.piece_at(square);
        let elsewhere =
            self.held == Some(square) || self.slides.iter().any(|&(_, to)| to == square);
        BoardSquare::new(square, (piece, pieces), colors)
            .highlight(self.highlight(square))
            .target(self.targets.contains(&square))
            .draggable(piece.is_some_and(|piece| self.draggable.contains(&piece.side)))
            .labels(self.file_label(square), self.rank_label(square))
            .hide_piece(elsewhere)
            .badge(
                self.badges
                    .iter()
                    .find(|(badged, _)| *badged == square)
                    .map(|(_, path)| *path),
            )
            .on_click(self.on_square_click.clone())
            .on_grab(self.on_piece_grab.clone())
            .right_button(self.right_button.clone())
    }

    fn highlight(&self, square: Square) -> SquareHighlight {
        if self.check == Some(square) {
            SquareHighlight::Check
        } else if self.selected == Some(square) {
            SquareHighlight::Selected
        } else if self.is_premove_square(square) {
            SquareHighlight::Premove
        } else if self.marks.contains(&square) {
            SquareHighlight::Marked
        } else if self
            .last_move
            .is_some_and(|mv| mv.from == square || mv.to == square)
        {
            self.last_move_tint
                .map_or(SquareHighlight::LastMove, SquareHighlight::Reviewed)
        } else {
            SquareHighlight::None
        }
    }

    fn is_premove_square(&self, square: Square) -> bool {
        self.premoves
            .iter()
            .any(|&(from, to)| from == square || to == square)
    }

    /// Letters on whichever rank is drawn at the bottom.
    fn file_label(&self, square: Square) -> Option<char> {
        let bottom_rank = self.display_ranks()[7];
        (square.rank() == bottom_rank).then(|| char::from(b'a' + square.file()))
    }

    /// Numbers on whichever file is drawn on the left.
    fn rank_label(&self, square: Square) -> Option<char> {
        let left_file = self.display_files()[0];
        (square.file() == left_file).then(|| char::from(b'1' + square.rank()))
    }
}

/// The position of `value` in a display order of the eight files or ranks.
fn display_index(order: &[u8; 8], value: u8) -> u8 {
    order
        .iter()
        .zip(0..)
        .find_map(|(&shown, index)| (shown == value).then_some(index))
        .expect("every file and rank is drawn once")
}

/// The king's square while the side to move is in check.
fn check_square(position: &Position) -> Option<Square> {
    position
        .is_check()
        .then(|| position.board().king_of(position.side_to_move()))
        .flatten()
}
