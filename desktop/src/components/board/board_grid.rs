use std::rc::Rc;

use chess_domain::{Board, Move, Position, Side, Square};
use gpui_kit::{
    component::{ActiveTheme as _, h_flex, v_flex},
    prelude::FluentBuilder as _,
    *,
};

use super::{
    arrows::{paint_arrows, square_width},
    board_colors::BoardColors,
    board_square::{BoardSquare, OnPieceDrop, OnSquareClick, RightButton, SquareSize},
    promotion_picker::{PromotionPicker, PromotionPrompt},
};
use crate::{
    enums::board::SquareHighlight,
    services::board::{BoardInteraction, Cell},
};

/// The square size used before the board's first layout has measured it.
const FIRST_SQUARE_SIZE: Pixels = px(64.);

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
    on_square_click: Option<OnSquareClick>,
    on_piece_drop: Option<OnPieceDrop>,
    right_button: Option<RightButton>,
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
            on_square_click: None,
            on_piece_drop: None,
            right_button: None,
        }
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

    /// A piece dragged from the first square was dropped on the second.
    pub fn on_piece_drop(
        mut self,
        handler: impl Fn(Square, Square, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_piece_drop = Some(Rc::new(handler));
        self
    }
}

impl BoardGrid {
    /// A board drawing `board` (the position's pieces, or the pieces as queued premoves leave
    /// them) with everything `interaction` shows; the check square comes from `position`.
    /// The page adds its promotion prompt and callbacks.
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
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let colors = BoardColors::current(cx);
        let size = SquareSize::new(std::cell::Cell::new(FIRST_SQUARE_SIZE));
        v_flex()
            .relative()
            .h_full()
            .max_w_full()
            .aspect_square()
            .rounded(cx.theme().radius)
            .overflow_hidden()
            .children(
                self.display_ranks()
                    .map(|rank| self.rank_row(rank, colors, &size)),
            )
            .child(self.arrow_layer(colors, size))
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

    /// The arrows, painted over the squares without catching any clicks; laying it out also
    /// measures the squares for the drag preview.
    fn arrow_layer(&self, colors: BoardColors, size: SquareSize) -> Canvas<()> {
        let engine: Vec<(Cell, Cell)> = self.cells(self.engine_arrow.iter());
        let player: Vec<(Cell, Cell)> = self.cells(self.arrows.iter());
        canvas(
            move |bounds, _, _| size.set(square_width(bounds)),
            move |bounds, (), window, _| {
                paint_arrows(bounds, &engine, colors.engine_arrow, window);
                paint_arrows(bounds, &player, colors.arrow, window);
            },
        )
        .absolute()
        .inset_0()
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

    fn rank_row(&self, rank: u8, colors: BoardColors, size: &SquareSize) -> Div {
        h_flex().flex_1().items_stretch().children(
            self.display_files()
                .map(|file| self.square(file, rank, colors, Rc::clone(size))),
        )
    }

    fn square(&self, file: u8, rank: u8, colors: BoardColors, size: SquareSize) -> BoardSquare {
        let square = Square::new(file, rank).expect("0..8 stays on the board");
        let piece = self.board.piece_at(square);
        BoardSquare::new(square, piece, colors, size)
            .highlight(self.highlight(square))
            .target(self.targets.contains(&square))
            .draggable(piece.is_some_and(|piece| self.draggable.contains(&piece.side)))
            .labels(self.file_label(square), self.rank_label(square))
            .on_click(self.on_square_click.clone())
            .on_drop(self.on_piece_drop.clone())
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
            SquareHighlight::LastMove
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
