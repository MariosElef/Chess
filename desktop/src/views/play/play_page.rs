use std::time::{Duration, Instant};

use chess_domain::{Clock, Move, Side, TimeControl};
use gpui_kit::{
    assets::IconName,
    component::{
        ActiveTheme as _, h_flex,
        label::Label,
        slider::{SliderEvent, SliderState},
        v_flex,
    },
    prelude::FluentBuilder as _,
    *,
};

use super::game_controls::game_tile;
use crate::{
    clients::engine::{Engine, locate_stockfish},
    components::{
        board::{BoardColumn, PLAYER_BAR, board_page, copy_fen},
        history::{MoveList, NavigationBar},
        panel::{ChoiceTile, PanelHeader, SidePanel, action_bar},
        play::{
            BotOptions, BotPicker, CapturedRow, ClockFace, GameOverActions, GameOverCard,
            GameSetup, PlayPanel, WaitingBar, with_card,
        },
        shell::PLAY_PICTURE,
        social::avatar,
    },
    enums::{
        play::{ColourChoice, PlayMode},
        sound::Sound,
    },
    models::{
        board::{MoveMade, ShownPosition},
        play::{AnalyzeGame, Bot},
    },
    services::{
        history::GameHistory,
        play::{
            BOTS, CUSTOM_STRENGTH, ClockWarning, DEFAULT_TIME_CONTROL, custom_bot, outcome_reason,
            player_side, result_title, status_text, think_time,
        },
    },
    views::{
        board::{BoardView, flip_button},
        history::{
            HistoryNavigation, Holding, hold_step, select_move, step_history, with_history_keys,
        },
        live::{SessionSource, my_face},
        play::{BotEvent, BotPlayer, OnlineView},
        sound::{play_move_sound, play_sound},
        ticker::{CLOCK_TICK, start_ticker},
    },
};

/// What a premove costs on the clock: one second, as the clock shows it.
const PREMOVE_COST: Duration = Duration::from_secs(1);

/// The page shows the new-game menu until a game starts.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Stage {
    Setup,
    Playing,
}

/// A game against a bot: who it is, the side it plays, and its engine.
struct BotGame {
    bot: Bot,
    /// The bot's side, not the player's.
    side: Side,
    player: Entity<BotPlayer>,
    _moves: Subscription,
}

/// The game being played, the position on the board, and what the player is in the middle of.
pub struct PlayPage {
    stage: Stage,
    /// Against someone here or online; the menu shows the matching options.
    mode: PlayMode,
    /// The online lobby and game, shown in place of the local ones.
    online: Entity<OnlineView>,
    /// Who is signed in, for the player's own bar against a bot.
    session: SessionSource,
    _online_changes: Subscription,
    /// The online game's Game Review goes out as this page's.
    _online_analyze: Subscription,
    /// The time control picked in the menu; `None` is no clock.
    setup_choice: Option<TimeControl>,
    history: GameHistory,
    clock: Option<Clock>,
    /// When the side to move's turn began; `None` before the first move and once the game is over.
    turn_started: Option<Instant>,
    /// Redraws the running clock; dropping it stops the ticking.
    _ticker: Option<Task<()>>,
    /// The board, with what the player is doing on it and the queued premoves.
    board: Entity<BoardView>,
    _board_moves: Subscription,
    /// A key or navigation button held down.
    holding: Holding,
    /// Who has heard the low-time warning this game.
    clock_warning: ClockWarning,
    /// The bot picked in the menu, by its place among the named bots; `None` is the custom one.
    bot_pick: Option<usize>,
    custom_rating: u32,
    bot_strength: Entity<SliderState>,
    _strength_changes: Subscription,
    colour_choice: ColourChoice,
    /// Why a bot game could not start, or why it stopped.
    bot_problem: Option<String>,
    versus: Option<BotGame>,
    /// Keyboard focus, so the arrow keys reach this page.
    focus_handle: FocusHandle,
    /// The end-of-game card was put away, to look at the board.
    card_closed: bool,
}

impl PlayPage {
    pub fn new(online: Entity<OnlineView>, session: SessionSource, cx: &mut Context<Self>) -> Self {
        // An online game taking the page sets a local one aside: its clock would run unseen.
        let _online_changes = cx.observe(&online, |page, online, cx| {
            if online.read(cx).in_game() && page.stage == Stage::Playing {
                page.new_game(cx);
            }
            cx.notify();
        });
        let _online_analyze = cx.subscribe(&online, |_, _, event: &AnalyzeGame, cx| {
            cx.emit(event.clone());
        });
        let page = cx.weak_entity();
        let tabs_page = page.clone();
        online.update(cx, |online, _| {
            online.on_mode(move |mode, _, cx| {
                let _ = tabs_page.update(cx, |page, cx| page.choose_mode(mode, cx));
            });
        });
        let board = cx.new(|board| {
            BoardView::new(
                move |cx| page.upgrade().map(|page| page.read(cx).shown_position()),
                board,
            )
        });
        let _board_moves = cx.subscribe(&board, |page, _, MoveMade(mv), cx| {
            page.play(*mv, cx);
            cx.notify();
        });
        let bot_strength = cx.new(|_| strength_slider());
        let _strength_changes = cx.subscribe(&bot_strength, |page, _, event: &SliderEvent, cx| {
            let (SliderEvent::Change(value) | SliderEvent::Release(value)) = event;
            page.custom_rating = rating_of(value.start());
            cx.notify();
        });
        Self {
            stage: Stage::Setup,
            mode: PlayMode::default(),
            online,
            session,
            _online_changes,
            _online_analyze,
            setup_choice: DEFAULT_TIME_CONTROL,
            history: GameHistory::new(),
            clock: None,
            turn_started: None,
            _ticker: None,
            board,
            _board_moves,
            holding: Holding::default(),
            clock_warning: ClockWarning::default(),
            bot_pick: Some(0),
            custom_rating: CUSTOM_STRENGTH.3,
            bot_strength,
            _strength_changes,
            colour_choice: ColourChoice::default(),
            bot_problem: None,
            versus: None,
            focus_handle: cx.focus_handle(),
            card_closed: false,
        }
    }

    pub fn focus_handle(&self) -> FocusHandle {
        self.focus_handle.clone()
    }

    /// Focuses the page, or the online view while it is shown, so the arrow keys reach it.
    fn focus(&self, window: &mut Window, cx: &mut App) {
        if self.shows_online(cx) {
            self.online.read(cx).focus_handle().focus(window, cx);
        } else {
            self.focus_handle.focus(window, cx);
        }
    }

    /// Connects for online play, so a game the player is still in comes back to the board.
    pub fn connect_online(&mut self, cx: &mut Context<Self>) {
        self.online.update(cx, |online, cx| online.connect(cx));
    }

    fn choose_mode(&mut self, mode: PlayMode, cx: &mut Context<Self>) {
        self.mode = mode;
        if mode == PlayMode::Online {
            self.connect_online(cx);
        }
        cx.notify();
    }

    /// An online game takes the page; otherwise the online lobby when no local game is on.
    pub fn shows_online(&self, cx: &App) -> bool {
        self.online.read(cx).in_game()
            || (self.mode == PlayMode::Online && self.stage == Stage::Setup)
    }

    /// What the board shows: the position looked at, taking moves only on a live game.
    fn shown_position(&self) -> ShownPosition {
        if self.stage == Stage::Setup {
            return ShownPosition::waiting();
        }
        // Worked out once: it looks at every legal move and every earlier position.
        let outcome = self.history.game().outcome();
        let live = self.history.is_live();
        ShownPosition {
            position: *self.history.shown(),
            last_move: self.history.shown_last_move(),
            can_move: live && outcome.is_none(),
            premoves: true,
            player: self.versus.as_ref().map(|game| game.side.opponent()),
            engine_arrow: None,
            badge: None,
            best_arrow: None,
            outcome: outcome.filter(|_| live),
        }
    }

    fn choose_time(&mut self, choice: Option<TimeControl>, cx: &mut Context<Self>) {
        self.setup_choice = choice;
        cx.notify();
    }

    /// Leaves the menu with a fresh game and, for a timed game, a ticking clock.
    fn start_game(&mut self, cx: &mut Context<Self>) {
        if self.mode == PlayMode::Bots {
            // A bot that will not start sends the player back to the bot menu, with why.
            if !self.start_bot(cx) {
                let problem = self.bot_problem.take();
                self.new_game(cx);
                self.bot_problem = problem;
                return;
            }
        } else {
            self.versus = None;
            self.bot_problem = None;
        }
        self.history = GameHistory::new();
        self.card_closed = false;
        self.clock = self.setup_choice.map(Clock::new);
        self.turn_started = None;
        self._ticker = self
            .clock
            .is_some()
            .then(|| start_ticker(CLOCK_TICK, cx, PlayPage::tick));
        self.stage = Stage::Playing;
        self.clock_warning = ClockWarning::default();
        self.end_interaction(cx);
        play_sound(Sound::GameStart, cx);
        self.bot_turn_if_due(cx);
    }

    /// The bot picked in the menu.
    fn picked_bot(&self) -> Bot {
        self.bot_pick
            .and_then(|place| BOTS.get(place).copied())
            .unwrap_or_else(|| custom_bot(self.custom_rating))
    }

    /// Starts the picked bot on its own Stockfish, the board turned to the player; whether it did.
    fn start_bot(&mut self, cx: &mut Context<Self>) -> bool {
        let Some(path) = locate_stockfish() else {
            self.bot_problem =
                Some("Stockfish was not found: install it (brew install stockfish)".to_owned());
            return false;
        };
        let Ok(started) = Engine::spawn(&path) else {
            self.bot_problem = Some("Stockfish would not start".to_owned());
            return false;
        };
        let bot = self.picked_bot();
        let player = cx.new(|cx| BotPlayer::new(started, bot.rating, cx));
        let _moves = cx.subscribe(&player, |page, _, event: &BotEvent, cx| {
            page.on_bot_event(event, cx);
        });
        let side = player_side(self.colour_choice, rand::random::<bool>()).opponent();
        self.board
            .update(cx, |board, cx| board.face(side.opponent(), cx));
        self.bot_problem = None;
        self.versus = Some(BotGame {
            bot,
            side,
            player,
            _moves,
        });
        true
    }

    /// On the bot's turn in a game still going, asks it for a move.
    fn bot_turn_if_due(&mut self, cx: &mut Context<Self>) {
        let Some(game) = &self.versus else {
            return;
        };
        if self.history.game().is_over() || self.side_to_move() != game.side {
            return;
        }
        let think = think_time(self.time_left(game.side), rand::random::<f64>());
        let position = *self.history.live();
        game.player
            .update(cx, |player, cx| player.ask(position, think, cx));
    }

    /// The bot's answer: its move, if it still fits the game, or that its engine stopped.
    fn on_bot_event(&mut self, event: &BotEvent, cx: &mut Context<Self>) {
        let Some(game) = &self.versus else {
            return;
        };
        match event {
            BotEvent::Moved { fen, mv } => {
                let still_due = !self.history.game().is_over()
                    && self.side_to_move() == game.side
                    && self.history.live().to_string() == *fen;
                if still_due {
                    self.play(*mv, cx);
                    cx.notify();
                }
            }
            BotEvent::Failed => {
                self.bot_problem = Some("The bot's engine stopped".to_owned());
                cx.notify();
            }
        }
    }

    /// Plays a legal move, charges the mover's clock and starts the opponent's, then due premoves.
    fn play(&mut self, mv: Move, cx: &mut Context<Self>) {
        let mover = self.side_to_move();
        self.charge_clock(mover, Clock::finish_turn);
        self.record_move(mv, cx);
        self.run_premoves(cx);
        self.bot_turn_if_due(cx);
    }

    /// Adds the move to the game with its sound, and the game's end sound if it was the last.
    fn record_move(&mut self, mv: Move, cx: &App) {
        let before = *self.history.live();
        let mover = before.side_to_move();
        let plain = match &self.versus {
            Some(game) if game.side == mover => Sound::MoveOpponent,
            Some(_) => Sound::MoveSelf,
            None => mover_sound(mover),
        };
        self.history
            .play(mv)
            .expect("moves come from the legal ones");
        let is_over = self.history.game().is_over();
        play_move_sound(&before, self.history.live(), mv, plain, is_over, cx);
        self.turn_started = (!is_over).then(Instant::now);
    }

    /// Plays each side's queued premoves while legal; one that is not drops that side's queue.
    fn run_premoves(&mut self, cx: &mut Context<Self>) {
        loop {
            let side = self.side_to_move();
            if self.history.game().is_over() {
                self.board.update(cx, |board, _| board.drop_all_premoves());
                return;
            }
            if !self.clock_allows_premove(side) {
                self.board.update(cx, |board, _| board.drop_premoves(side));
                return;
            }
            let live = *self.history.live();
            let Some(mv) = self.board.update(cx, |board, _| board.next_premove(&live)) else {
                return;
            };
            self.play_premove(side, mv, cx);
        }
    }

    /// A premove needs more than its cost left on the clock.
    fn clock_allows_premove(&self, side: Side) -> bool {
        self.clock
            .is_none_or(|clock| clock.remaining(side, Duration::ZERO) > PREMOVE_COST)
    }

    /// A premove is played at once for a fixed cost instead of the time the turn took.
    fn play_premove(&mut self, side: Side, mv: Move, cx: &App) {
        if let Some(clock) = self.clock.as_mut() {
            clock.finish_turn(side, PREMOVE_COST);
        }
        self.record_move(mv, cx);
    }

    /// Applies the running turn's time to `side` with `charge`, if a clock is running.
    fn charge_clock(&mut self, side: Side, charge: fn(&mut Clock, Side, Duration)) {
        if let (Some(clock), Some(started)) = (self.clock.as_mut(), self.turn_started) {
            charge(clock, side, started.elapsed());
        }
    }

    /// The game ended mid-turn: the side to move's thinking time is taken, and the clocks freeze.
    fn stop_clock(&mut self) {
        self.charge_clock(self.side_to_move(), Clock::spend);
        self.turn_started = None;
    }

    /// Redraws the clocks and ends the game when time runs out; whether to keep ticking.
    fn tick(&mut self, cx: &mut Context<Self>) -> bool {
        if self.history.game().is_over() {
            return false;
        }
        self.warn_if_low_on_time(cx);
        if self.is_flagged() {
            self.stop_clock();
            self.history
                .flag(self.side_to_move())
                .expect("the game was still going");
            self.end_interaction(cx);
            play_sound(Sound::GameEnd, cx);
            return false;
        }
        cx.notify();
        true
    }

    /// The side whose clock is running hears the warning once it is down to its last seconds.
    fn warn_if_low_on_time(&mut self, cx: &App) {
        let (Some(side), Some(clock)) = (self.running_side(), self.clock) else {
            return;
        };
        let left = self.time_left(side).unwrap_or(Duration::ZERO);
        if self
            .clock_warning
            .is_due(side, left, clock.control().initial)
        {
            play_sound(Sound::LowTime, cx);
        }
    }

    fn is_flagged(&self) -> bool {
        match (self.clock, self.turn_started) {
            (Some(clock), Some(started)) => {
                clock.is_flagged(self.side_to_move(), started.elapsed())
            }
            _ => false,
        }
    }

    /// The side whose clock is running, if any.
    fn running_side(&self) -> Option<Side> {
        self.turn_started.map(|_| self.side_to_move())
    }

    /// Time left for `side` right now, or `None` in a game without a clock.
    fn time_left(&self, side: Side) -> Option<Duration> {
        let elapsed = match (self.running_side(), self.turn_started) {
            (Some(running), Some(started)) if running == side => started.elapsed(),
            _ => Duration::ZERO,
        };
        self.clock.map(|clock| clock.remaining(side, elapsed))
    }

    fn side_to_move(&self) -> Side {
        self.history.live().side_to_move()
    }

    /// The side to move resigns; against a bot, always the player, whoever is to move.
    fn resign(&mut self, cx: &mut Context<Self>) {
        let side = self
            .versus
            .as_ref()
            .map_or(self.side_to_move(), |game| game.side.opponent());
        self.stop_clock();
        self.history
            .resign(side)
            .expect("the button only shows while the game is on");
        self.end_interaction(cx);
        play_sound(Sound::GameEnd, cx);
    }

    fn offer_draw(&mut self, cx: &mut Context<Self>) {
        self.history
            .offer_draw(self.side_to_move())
            .expect("the button only shows while the game is on");
        play_sound(Sound::Notify, cx);
        cx.notify();
    }

    fn accept_draw(&mut self, cx: &mut Context<Self>) {
        self.stop_clock();
        self.history
            .accept_draw(self.side_to_move())
            .expect("the button only shows for an offer made to this side");
        self.end_interaction(cx);
        play_sound(Sound::GameEnd, cx);
    }

    fn decline_draw(&mut self, cx: &mut Context<Self>) {
        self.history
            .decline_draw(self.side_to_move())
            .expect("the button only shows for an offer made to this side");
        cx.notify();
    }

    fn flip(&mut self, cx: &mut Context<Self>) {
        self.board.update(cx, |board, cx| board.flip(cx));
        cx.notify();
    }

    /// Hands the game to the shell for Game Review.
    fn review(&mut self, cx: &mut Context<Self>) {
        cx.emit(AnalyzeGame {
            game: self.history.game().clone(),
            viewing: self.history.viewing(),
            review: true,
            outcome: self.history.game().outcome(),
        });
    }

    /// The end-of-game card, while the game is over and the card has not been put away.
    fn game_over_card(&self, cx: &mut Context<Self>) -> Option<GameOverCard> {
        let outcome = self
            .history
            .game()
            .outcome()
            .filter(|_| !self.card_closed)?;
        // The side kept is the bot's; the player has the other.
        let me = self.versus.as_ref().map(|game| game.side.opponent());
        let opponent = self.versus.as_ref().map(|game| game.bot.name);
        let title = result_title(outcome.winner(), me, opponent);
        let close = cx.entity().downgrade();
        let card = GameOverCard::new(
            (title, outcome_reason(outcome).to_owned()),
            self.game_over_actions(cx),
            move |_, cx| {
                let _ = close.update(cx, |page, cx| {
                    page.card_closed = true;
                    cx.notify();
                });
            },
        );
        Some(card)
    }

    /// Game Review, then a new game or the same one again.
    fn game_over_actions(&self, cx: &mut Context<Self>) -> GameOverActions {
        let new_label = if self.versus.is_some() {
            "New Bot"
        } else {
            "New game"
        };
        let page = cx.entity().downgrade();
        let (review, fresh, again) = (page.clone(), page.clone(), page);
        GameOverActions::new(move |_, cx| {
            let _ = review.update(cx, |page, cx| page.review(cx));
        })
        .choice(("new-game", new_label), IconName::Plus, move |_, cx| {
            let _ = fresh.update(cx, |page, cx| page.new_game(cx));
        })
        .choice(("rematch", "Rematch"), IconName::RotateCcw, move |_, cx| {
            let _ = again.update(cx, |page, cx| page.start_game(cx));
        })
    }

    /// Back to the menu, keeping the last time control picked.
    fn new_game(&mut self, cx: &mut Context<Self>) {
        self.versus = None;
        self.bot_problem = None;
        self._ticker = None;
        self.turn_started = None;
        self.stage = Stage::Setup;
        self.end_interaction(cx);
        // The waiting board faces White, whichever side the last game had the player on.
        self.board
            .update(cx, |board, cx| board.face(Side::White, cx));
    }

    /// Puts down a held piece, closes the promotion picker, wipes drawings and drops premoves.
    fn end_interaction(&mut self, cx: &mut Context<Self>) {
        self.board.update(cx, |board, cx| board.reset(cx));
        cx.notify();
    }

    /// An offer the side to move has received from the other side.
    fn has_offer_to_answer(&self) -> bool {
        self.history.game().draw_offer() == Some(self.side_to_move().opponent())
    }

    /// How the game stands, at the top of the panel.
    fn status(&self, cx: &mut Context<Self>) -> Div {
        v_flex()
            .gap_1()
            .child(Label::new(status_text(self.history.game())).text_lg())
            .when_some(self.bot_problem.clone(), |this, problem| {
                this.child(div().text_sm().text_color(cx.theme().danger).child(problem))
            })
    }

    /// While on: Accept/Decline for an offer, or Draw/Resign; once over, Game Review.
    fn actions(&self, cx: &mut Context<Self>) -> Vec<ChoiceTile> {
        let game = self.history.game();
        let mut buttons = Vec::new();
        if self.versus.is_some() {
            // A bot makes no offers and takes none.
            buttons.push(resign_button(cx));
        } else if self.has_offer_to_answer() {
            buttons.extend([accept_draw_button(cx), decline_draw_button(cx)]);
        } else {
            let already_offered = game.draw_offer() == Some(self.side_to_move());
            buttons.extend([offer_draw_button(already_offered, cx), resign_button(cx)]);
        }
        buttons
    }

    fn navigation(&self, page: WeakEntity<Self>) -> NavigationBar {
        NavigationBar::new(
            self.history.can_go_back(),
            self.history.can_go_forward(),
            step_history(page.clone()),
        )
        .on_hold(hold_step(page))
    }

    /// The panel beside the board: the status, the moves, the arrows and the game's buttons.
    fn side_column(&self, page: WeakEntity<Self>, cx: &mut Context<Self>) -> SidePanel {
        let title = if self.versus.is_some() {
            "Play Bots"
        } else {
            "Play"
        };
        SidePanel::new()
            .header(PanelHeader::new(PLAY_PICTURE, title))
            .child(self.status(cx))
            .child(MoveList::new(
                self.history.sans().to_vec(),
                self.history.viewing(),
                select_move(page.clone()),
            ))
            .child(self.navigation(page))
            .map(|this| {
                if self.history.game().is_over() {
                    this.child(self.game_over_actions(cx))
                } else {
                    this.child(action_bar(self.actions(cx)))
                }
            })
    }

    /// The board between two player bars: the side drawn at the top above it, the other below.
    fn board_column(&self, cx: &mut Context<Self>) -> BoardColumn {
        let bottom = self.board.read(cx).bottom_side();
        BoardColumn::new(with_card(self.board.clone(), self.game_over_card(cx)))
            .top(self.player_bar(bottom.opponent(), cx))
            .bottom(self.player_bar(bottom, cx))
            .tool(flip_button("flip-board", cx, PlayPage::flip))
            .tool(copy_fen(self.history.shown()))
    }

    /// One side's captured pieces and, in a timed game, its clock; against a bot, who plays it.
    fn player_bar(&self, side: Side, cx: &App) -> Div {
        h_flex()
            .w_full()
            .h(PLAYER_BAR)
            .flex_shrink_0()
            .justify_between()
            .gap_4()
            .child(
                h_flex()
                    .gap_3()
                    .when_some(self.player_label(side, cx), |this, label| this.child(label))
                    .child(CapturedRow::for_side(self.history.shown().board(), side)),
            )
            .when_some(self.time_left(side), |this, left| {
                this.child(ClockFace::new(left, self.running_side() == Some(side)))
            })
    }

    /// Against a bot, its picture and name with its rating; the player is "You", with theirs.
    fn player_label(&self, side: Side, cx: &App) -> Option<Div> {
        let game = self.versus.as_ref()?;
        if side != game.side {
            // Round, as the bot's picture beside it; none while signed out.
            let face = my_face(&self.session, cx)
                .map(|face| avatar(&face.name, face.picture.as_deref(), px(24.), cx));
            return Some(h_flex().gap_2().children(face).child(Label::new("You")));
        }
        let bot = game.bot;
        Some(
            h_flex()
                .gap_2()
                .when_some(bot.picture, |this, path| {
                    this.child(img(path).size_6().rounded_full())
                })
                .child(Label::new(format!("{} ({})", bot.name, bot.rating))),
        )
    }

    /// The board waiting between its two bars, and the panel with the tabs and the choices.
    fn setup_view(&self, page: WeakEntity<Self>, cx: &App) -> Div {
        let setup = GameSetup::new(
            self.setup_choice,
            choose_time(page.clone()),
            start_game(page.clone()),
        );
        let against_bot = self.mode == PlayMode::Bots;
        let setup = if against_bot {
            setup.bot(BotOptions {
                problem: self.bot_problem.clone(),
            })
        } else {
            setup
        };
        let picker = against_bot.then(|| self.bot_picker(page.clone()));
        let panel = PlayPanel::new(self.mode, choose_mode(page))
            .children(picker)
            .child(setup);
        board_page(self.waiting_column(cx), panel)
    }

    /// Before the game: the opponent above the board, the player below, each with their clock.
    fn waiting_column(&self, cx: &App) -> BoardColumn {
        let clock = self.setup_choice.map(|control| control.initial);
        let (top, bottom) = if self.mode == PlayMode::Bots {
            let bot = self.picked_bot();
            let top =
                WaitingBar::new(format!("{} ({})", bot.name, bot.rating)).picture(bot.picture);
            (
                top,
                WaitingBar::new("You").person(my_face(&self.session, cx)),
            )
        } else {
            (WaitingBar::new("Black"), WaitingBar::new("White"))
        };
        BoardColumn::new(self.board.clone())
            .top(top.clock(clock))
            .bottom(bottom.clock(clock))
    }

    fn bot_picker(&self, page: WeakEntity<Self>) -> BotPicker {
        let (pick, colour) = (page.clone(), page);
        BotPicker::new(
            (BOTS.to_vec(), self.bot_pick),
            (self.custom_rating, self.bot_strength.clone()),
            self.colour_choice,
            move |place, _, cx| {
                let _ = pick.update(cx, |page, cx| {
                    page.bot_pick = place;
                    cx.notify();
                });
            },
            move |choice, _, cx| {
                let _ = colour.update(cx, |page, cx| {
                    page.colour_choice = choice;
                    cx.notify();
                });
            },
        )
    }

    /// The online lobby, which draws its own tabs, or an online game.
    fn online_view(&self) -> Div {
        div().size_full().child(self.online.clone())
    }

    fn game_view(&self, page: WeakEntity<Self>, cx: &mut Context<Self>) -> Div {
        board_page(self.board_column(cx), self.side_column(page, cx))
    }
}

impl EventEmitter<AnalyzeGame> for PlayPage {}

impl HistoryNavigation for PlayPage {
    fn history_mut(&mut self) -> Option<&mut GameHistory> {
        Some(&mut self.history)
    }

    fn holding(&mut self) -> &mut Holding {
        &mut self.holding
    }

    /// The picked-up piece is put down first.
    fn after_history_move(&mut self, cx: &mut Context<Self>) {
        self.end_interaction(cx);
    }
}

impl Render for PlayPage {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let page = cx.entity().downgrade();
        with_history_keys(v_flex(), cx)
            .track_focus(&self.focus_handle)
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(|page, _, window, cx| page.focus(window, cx)),
            )
            .size_full()
            .child(if self.shows_online(cx) {
                self.online_view()
            } else {
                match self.stage {
                    Stage::Setup => self.setup_view(page, cx),
                    Stage::Playing => self.game_view(page, cx),
                }
            })
    }
}

fn offer_draw_button(already_offered: bool, cx: &mut Context<PlayPage>) -> ChoiceTile {
    game_tile(("offer-draw", "Draw"), false, cx, PlayPage::offer_draw)
        .glyph("½")
        .disabled(already_offered)
}

fn resign_button(cx: &mut Context<PlayPage>) -> ChoiceTile {
    game_tile(("resign", "Resign"), false, cx, PlayPage::resign).icon(IconName::Flag)
}

/// Ringed in green: the offer waits on it.
fn accept_draw_button(cx: &mut Context<PlayPage>) -> ChoiceTile {
    game_tile(
        ("accept-draw", "Accept draw"),
        true,
        cx,
        PlayPage::accept_draw,
    )
    .icon(IconName::Check)
}

fn decline_draw_button(cx: &mut Context<PlayPage>) -> ChoiceTile {
    game_tile(
        ("decline-draw", "Decline"),
        false,
        cx,
        PlayPage::decline_draw,
    )
    .icon(IconName::Close)
}

// The page outlives what it renders; a failed update means the window is closing.

fn choose_mode(page: WeakEntity<PlayPage>) -> impl Fn(PlayMode, &mut Window, &mut App) + 'static {
    move |mode, _, cx| {
        let _ = page.update(cx, |page, cx| page.choose_mode(mode, cx));
    }
}

fn choose_time(
    page: WeakEntity<PlayPage>,
) -> impl Fn(Option<TimeControl>, &mut Window, &mut App) + 'static {
    move |choice, _, cx| {
        let _ = page.update(cx, |page, cx| page.choose_time(choice, cx));
    }
}

fn start_game(page: WeakEntity<PlayPage>) -> impl Fn(&mut Window, &mut App) + 'static {
    move |_, cx| {
        let _ = page.update(cx, |page, cx| page.start_game(cx));
    }
}

/// On the Play board two players share one computer: White's moves and Black's sound apart.
fn mover_sound(mover: Side) -> Sound {
    match mover {
        Side::White => Sound::MoveSelf,
        Side::Black => Sound::MoveOpponent,
    }
}

/// The custom bot's strength slider, at its starting strength.
fn strength_slider() -> SliderState {
    let (lowest, highest, step, start) = CUSTOM_STRENGTH;
    // The maximum first: a minimum above the default maximum of 100 panics.
    SliderState::new()
        .max(slider_point(highest))
        .min(slider_point(lowest))
        .step(slider_point(step))
        .default_value(slider_point(start))
}

fn slider_point(rating: u32) -> f32 {
    f32::from(u16::try_from(rating).unwrap_or(u16::MAX))
}

/// A point on the strength slider as a rating.
fn rating_of(point: f32) -> u32 {
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "the slider's points are whole ratings between 400 and 2800"
    )]
    let rating = point.round() as u32;
    rating
}

#[cfg(test)]
mod tests {
    use chess_domain::{Move, Side, Square, TimeControl};
    use std::time::Duration;

    use gpui_kit::{
        AppContext as _, Entity, Modifiers, MouseButton, TestAppContext, VisualTestContext, point,
        px,
    };

    use std::rc::Rc;

    use super::PlayPage;
    use crate::{
        clients::engine::locate_stockfish,
        enums::play::{ColourChoice, PlayMode},
    };
    use crate::{
        clients::{http::HttpRuntime, server::ServerClient},
        enums::{history::NavStep, sound::Sound},
        testing::{click, drag, lift, release},
        views::history::HistoryNavigation as _,
        views::sound::{played_sounds, record_sounds},
        views::{
            live::{LiveConnection, SessionSource},
            play::OnlineView,
        },
    };

    fn sq(name: &str) -> Square {
        name.parse().expect("a square")
    }

    fn mv(from: &str, to: &str) -> Move {
        Move::new(sq(from), sq(to))
    }

    /// A Play page whose online side reaches no server and has nobody signed in.
    fn offline_page(cx: &mut gpui_kit::Context<PlayPage>) -> PlayPage {
        let client =
            ServerClient::new(HttpRuntime::new().expect("a runtime"), "http://127.0.0.1:9");
        let session: SessionSource = Rc::new(|_| None);
        let connection = cx.new(|_| LiveConnection::new(client, Rc::clone(&session)));
        let online = cx.new(|cx| OnlineView::new(connection, Rc::clone(&session), cx));
        PlayPage::new(online, session, cx)
    }

    /// A Play page with a game started, with a 3 | 2 clock or none.
    fn started(cx: &mut TestAppContext, timed: bool) -> (Entity<PlayPage>, &mut VisualTestContext) {
        cx.update(gpui_kit::init);
        let (page, cx) = cx.add_window_view(|_, cx| offline_page(cx));
        page.update(cx, |page, cx| {
            page.choose_time(timed.then(|| TimeControl::new(3, 2)), cx);
            page.start_game(cx);
        });
        (page, cx)
    }

    fn sans(page: &Entity<PlayPage>, cx: &mut VisualTestContext) -> Vec<String> {
        page.update(cx, |page, _| page.history.sans().to_vec())
    }

    fn premoves(page: &Entity<PlayPage>, cx: &mut VisualTestContext) -> Vec<(Square, Square)> {
        page.update(cx, |page, cx| page.board.read(cx).premoves().pairs())
    }

    #[gpui_kit::test]
    fn the_panel_tabs_switch_between_the_ways_to_play(cx: &mut TestAppContext) {
        cx.update(gpui_kit::init);
        let (page, cx) = cx.add_window_view(|_, cx| offline_page(cx));
        click(cx, "mode-Bots");
        assert_eq!(page.read_with(cx, |page, _| page.mode), PlayMode::Bots);
        click(cx, "mode-Local");
        assert_eq!(page.read_with(cx, |page, _| page.mode), PlayMode::Local);
        click(cx, "mode-Online");
        assert_eq!(page.read_with(cx, |page, _| page.mode), PlayMode::Online);
    }

    #[gpui_kit::test]
    fn a_premove_is_queued_and_plays_after_the_opponent_moves(cx: &mut TestAppContext) {
        let (page, cx) = started(cx, false);
        click(cx, "square-e7");
        let selected = page.update(cx, |page, cx| page.board.read(cx).selected());
        assert_eq!(selected, Some(sq("e7")));
        click(cx, "square-e5");
        assert_eq!(premoves(&page, cx), [(sq("e7"), sq("e5"))]);
        click(cx, "square-e2");
        click(cx, "square-e4");
        assert_eq!(sans(&page, cx), ["e4", "e5"]);
        assert!(premoves(&page, cx).is_empty());
    }

    #[gpui_kit::test]
    fn premoves_chain_through_the_preview(cx: &mut TestAppContext) {
        let (page, cx) = started(cx, false);
        drag(cx, "square-g8", "square-f6");
        drag(cx, "square-f6", "square-g4");
        assert_eq!(premoves(&page, cx).len(), 2);
    }

    #[gpui_kit::test]
    fn a_premove_that_mates_drops_every_other_premove(cx: &mut TestAppContext) {
        let (page, cx) = started(cx, false);
        drag(cx, "square-f2", "square-f3");
        drag(cx, "square-e7", "square-e5");
        drag(cx, "square-d8", "square-h4");
        drag(cx, "square-a7", "square-a6");
        assert_eq!(premoves(&page, cx).len(), 2, "two black premoves queued");
        drag(cx, "square-g2", "square-g4");
        assert_eq!(sans(&page, cx), ["f3", "e5", "g4", "Qh4#"]);
        assert!(
            premoves(&page, cx).is_empty(),
            "a6 is dropped once the game is over"
        );
    }

    #[gpui_kit::test]
    fn looking_back_cancels_the_premoves(cx: &mut TestAppContext) {
        let (page, cx) = started(cx, false);
        drag(cx, "square-e2", "square-e4");
        drag(cx, "square-g1", "square-f3");
        assert!(!premoves(&page, cx).is_empty());
        page.update(cx, |page, cx| page.navigate(NavStep::Back, cx));
        assert!(
            premoves(&page, cx).is_empty(),
            "no red squares on an earlier position"
        );
    }

    #[gpui_kit::test]
    fn a_game_sounds_from_its_start_to_a_premoved_mate(cx: &mut TestAppContext) {
        cx.update(|cx| {
            gpui_kit::init(cx);
            record_sounds(cx);
        });
        let (page, cx) = cx.add_window_view(|_, cx| offline_page(cx));
        page.update(cx, |page, cx| {
            page.choose_time(None, cx);
            page.start_game(cx);
        });
        drag(cx, "square-e2", "square-e5");
        drag(cx, "square-f2", "square-f3");
        drag(cx, "square-e7", "square-e5");
        drag(cx, "square-d8", "square-h4");
        drag(cx, "square-g2", "square-g4");
        let played = cx.update(|_, cx| played_sounds(cx));
        assert_eq!(
            played,
            [
                Sound::GameStart,
                Sound::Illegal,
                Sound::MoveSelf,
                Sound::MoveOpponent,
                Sound::Premove,
                Sound::MoveSelf,
                Sound::Check,
                Sound::GameEnd,
            ]
        );
    }

    #[gpui_kit::test]
    fn real_clicks_premove_in_a_timed_game(cx: &mut TestAppContext) {
        let (page, cx) = started(cx, true);
        for selector in ["square-e7", "square-e5", "square-e2", "square-e4"] {
            click(cx, selector);
        }
        assert_eq!(sans(&page, cx), ["e4", "e5"]);
    }

    #[gpui_kit::test]
    fn a_dragged_piece_leaves_its_square_until_it_is_dropped(cx: &mut TestAppContext) {
        let (_, cx) = started(cx, false);
        let start = lift(cx, "square-e2");
        assert!(
            cx.debug_bounds("piece-e2").is_none(),
            "only the dragged copy shows"
        );
        assert!(
            cx.debug_bounds("piece-d2").is_some(),
            "the other pieces stay"
        );
        release(cx, start);
        assert!(
            cx.debug_bounds("piece-e2").is_some(),
            "dropped back where it was"
        );
    }

    #[gpui_kit::test]
    fn a_pressed_piece_glides_to_the_cursor_and_goes_back_when_let_go(cx: &mut TestAppContext) {
        let (_, cx) = started(cx, false);
        let square = cx.debug_bounds("square-e2").expect("e2");
        let corner = square.origin + point(px(6.), px(6.));
        cx.simulate_mouse_down(corner, MouseButton::Left, Modifiers::none());
        // Frames as the app gets them: each animation frame asks for the next.
        for _ in 0..10 {
            cx.executor().advance_clock(Duration::from_millis(30));
            cx.update(|window, cx| window.simulate_next_frame(cx));
            cx.run_until_parked();
        }
        let piece = cx
            .debug_bounds("pressed-piece")
            .expect("the pawn, lifted")
            .center();
        assert!(
            (piece.x - corner.x).abs() < px(2.) && (piece.y - corner.y).abs() < px(2.),
            "centred on the cursor: {piece:?} for {corner:?}"
        );
        assert!(
            cx.debug_bounds("piece-e2").is_none(),
            "not also left on its square"
        );
        cx.simulate_mouse_up(corner, MouseButton::Left, Modifiers::none());
        cx.run_until_parked();
        let home = cx.debug_bounds("piece-e2").expect("the pawn").center();
        assert!(
            (home.x - square.center().x).abs() < px(2.),
            "back on its square"
        );
        assert!(
            cx.debug_bounds("pressed-piece").is_none(),
            "and no longer lifted"
        );
    }

    /// A bot game against the bot at `pick` (`None`: the custom one), the player as `colour`.
    fn against_bot(
        cx: &mut TestAppContext,
        pick: Option<usize>,
        colour: ColourChoice,
    ) -> (Entity<PlayPage>, &mut VisualTestContext) {
        cx.executor().allow_parking();
        cx.update(gpui_kit::init);
        let (page, cx) = cx.add_window_view(|_, cx| offline_page(cx));
        page.update(cx, |page, cx| {
            page.choose_mode(PlayMode::Bots, cx);
            page.bot_pick = pick;
            page.colour_choice = colour;
            page.choose_time(None, cx);
            page.start_game(cx);
        });
        (page, cx)
    }

    /// Waits, real time and the bot's pause, until the game has `plies` moves.
    fn wait_for_moves(page: &Entity<PlayPage>, cx: &mut VisualTestContext, plies: usize) {
        for _ in 0..500 {
            cx.executor().advance_clock(Duration::from_secs(3));
            cx.run_until_parked();
            if page.read_with(cx, |page, _| page.history.sans().len()) >= plies {
                return;
            }
            std::thread::sleep(Duration::from_millis(20));
        }
        panic!("the bot never moved");
    }

    #[gpui_kit::test]
    fn a_bot_answers_the_players_move_and_opens_when_it_is_white(cx: &mut TestAppContext) {
        if locate_stockfish().is_none() {
            return;
        }
        let (page, cx) = against_bot(cx, Some(4), ColourChoice::White);
        page.update(cx, |page, cx| {
            let versus = page.versus.as_ref().expect("a bot game");
            assert_eq!(versus.side, Side::Black, "the player asked for White");
            assert_eq!(versus.bot.name, "Sporos", "the strongest");
            page.play(mv("e2", "e4"), cx);
        });
        wait_for_moves(&page, cx, 2);
        page.read_with(cx, |page, _| {
            assert!(
                page.shown_position().player == Some(Side::White),
                "only White's pieces"
            );
        });
    }

    #[gpui_kit::test]
    fn a_weak_bot_moves_first_when_the_player_is_black(cx: &mut TestAppContext) {
        if locate_stockfish().is_none() {
            return;
        }
        let (page, cx) = against_bot(cx, Some(0), ColourChoice::Black);
        wait_for_moves(&page, cx, 1);
        page.read_with(cx, |page, cx| {
            assert!(page.board.read(cx).flipped(), "Black plays from the bottom");
        });
    }

    #[gpui_kit::test]
    fn the_kings_show_how_the_game_ended_on_its_final_position_only(cx: &mut TestAppContext) {
        let (page, cx) = started(cx, false);
        page.update(cx, |page, cx| {
            page.play(mv("e2", "e4"), cx);
            page.resign(cx);
            assert_eq!(
                page.shown_position().outcome,
                Some(chess_domain::Outcome::Resignation {
                    winner: Side::White
                }),
                "Black, to move, resigned"
            );
            page.navigate(crate::enums::history::NavStep::Start, cx);
            assert_eq!(page.shown_position().outcome, None, "not before the end");
        });
    }

    #[gpui_kit::test]
    fn a_piece_dropped_off_the_board_goes_back(cx: &mut TestAppContext) {
        let (page, cx) = started(cx, false);
        lift(cx, "square-e2");
        let board = cx.debug_bounds("square-a1").expect("a1").origin;
        let outside = board - point(px(40.), px(-40.));
        cx.simulate_mouse_move(outside, Some(MouseButton::Left), Modifiers::none());
        release(cx, outside);
        assert!(sans(&page, cx).is_empty(), "nothing moved");
        assert!(cx.debug_bounds("piece-e2").is_some(), "back on e2");
    }

    #[gpui_kit::test]
    fn a_piece_dragged_away_and_back_is_not_a_click(cx: &mut TestAppContext) {
        let (page, cx) = started(cx, false);
        let start = lift(cx, "square-e2");
        let e4 = cx.debug_bounds("square-e4").expect("e4").center();
        cx.simulate_mouse_move(e4, Some(MouseButton::Left), Modifiers::none());
        cx.simulate_mouse_move(start, Some(MouseButton::Left), Modifiers::none());
        release(cx, start);
        let selected = page.update(cx, |page, cx| page.board.read(cx).selected());
        assert_eq!(selected, None, "a drag back home selects nothing");
        assert!(sans(&page, cx).is_empty());
    }

    #[gpui_kit::test]
    fn the_square_under_a_dragged_piece_is_outlined(cx: &mut TestAppContext) {
        let (_, cx) = started(cx, false);
        lift(cx, "square-e2");
        let e4 = cx.debug_bounds("square-e4").expect("e4");
        cx.simulate_mouse_move(e4.center(), Some(MouseButton::Left), Modifiers::none());
        cx.run_until_parked();
        let outline = cx.debug_bounds("hover-outline").expect("an outline");
        assert!(
            (outline.center().x - e4.center().x).abs() < px(1.)
                && (outline.center().y - e4.center().y).abs() < px(1.),
            "on e4: {outline:?} for {e4:?}"
        );
    }

    #[gpui_kit::test]
    fn moving_a_held_piece_redraws_the_board_only_when_it_crosses_a_square(
        cx: &mut TestAppContext,
    ) {
        let (page, cx) = started(cx, false);
        let start = lift(cx, "square-e2");
        let board = page.read_with(cx, |page, _| page.board.clone());
        let square = cx.debug_bounds("square-e2").expect("e2").size.height;
        let before = board.read_with(cx, |board, _| board.renders());
        // Twelve moves up through e3 into e4, by shares of a square: two squares crossed.
        let shares = [
            0.15, 0.3, 0.45, 0.6, 0.75, 0.9, 1.05, 1.2, 1.35, 1.5, 1.65, 1.8,
        ];
        for share in shares {
            let at = start - point(px(0.), square * share);
            cx.simulate_mouse_move(at, Some(MouseButton::Left), Modifiers::none());
            cx.run_until_parked();
        }
        let redrawn = board.read_with(cx, |board, _| board.renders()) - before;
        assert!(
            redrawn <= 2,
            "the board redraws as the hovered square changes, not on every move: {redrawn}"
        );
        assert!(
            cx.debug_bounds("pressed-piece").is_some(),
            "the piece is still held"
        );
    }

    #[gpui_kit::test]
    fn a_right_click_puts_a_held_piece_back(cx: &mut TestAppContext) {
        let (page, cx) = started(cx, false);
        lift(cx, "square-e2");
        let target = cx.debug_bounds("square-e4").expect("e4").center();
        cx.simulate_mouse_move(target, Some(MouseButton::Left), Modifiers::none());
        cx.simulate_mouse_down(target, MouseButton::Right, Modifiers::none());
        cx.simulate_mouse_up(target, MouseButton::Right, Modifiers::none());
        cx.run_until_parked();
        assert!(cx.debug_bounds("piece-e2").is_some(), "back on its square");
        release(cx, target);
        assert!(
            sans(&page, cx).is_empty(),
            "letting go after cancelling moves nothing"
        );
        assert!(cx.debug_bounds("piece-e2").is_some());
    }

    #[gpui_kit::test]
    fn real_drags_move_and_premove_in_a_timed_game(cx: &mut TestAppContext) {
        let (page, cx) = started(cx, true);
        drag(cx, "square-g8", "square-f6");
        let first = page.update(cx, |page, cx| {
            page.board.read(cx).premoves().first(Side::Black)
        });
        assert_eq!(
            first,
            Some(mv("g8", "f6")),
            "black premove queued by dragging"
        );
        drag(cx, "square-e2", "square-e4");
        assert_eq!(sans(&page, cx), ["e4", "Nf6"]);
    }
}
