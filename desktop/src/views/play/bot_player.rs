use std::time::{Duration, Instant};

use chess_domain::{
    Move, Position,
    engine::{Command, SearchInfo, UciMessage},
};
use gpui_kit::*;

use crate::{
    clients::engine::Engine,
    enums::engine::EngineEvent,
    models::engine::EngineSettings,
    services::{
        engine::SearchQueue,
        play::{BotStyle, bot_options, bot_style, pick_bot_move},
    },
};

/// A bot needs little memory: its searches are short.
const BOT_HASH_MB: usize = 16;

/// What the bot says: the move it chose for a position, or that its engine stopped.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BotEvent {
    /// `fen` is the position it was asked about, so a stale answer can be told apart.
    Moved {
        fen: String,
        mv: Move,
    },
    Failed,
}

/// A position the bot was asked about, and how long it should seem to think.
struct Asked {
    position: Position,
    fen: String,
    since: Instant,
    think: Duration,
}

/// A computer opponent: its own Stockfish, set to play at its strength. Asked for a move, it
/// searches, chooses as a player of its rating would, waits out a human-feeling pause, and
/// says what it plays.
pub struct BotPlayer {
    engine: Engine,
    queue: SearchQueue,
    style: BotStyle,
    asked: Option<Asked>,
    /// The lines of the search under way, by MultiPV.
    lines: Vec<SearchInfo>,
    _events: Task<()>,
    _reply: Option<Task<()>>,
}

impl EventEmitter<BotEvent> for BotPlayer {}

impl BotPlayer {
    /// A bot of `rating`, playing on a Stockfish just started.
    pub fn new(
        (engine, events): (Engine, gpui_kit::base::async_util::Receiver<EngineEvent>),
        rating: u32,
        cx: &mut Context<Self>,
    ) -> Self {
        let style = bot_style(rating);
        let settings = EngineSettings {
            threads: 1,
            hash_mb: BOT_HASH_MB,
            depth: style.depth,
        };
        let (queue, first) = SearchQueue::new(settings);
        let mut queue = queue.with_options(bot_options(&style));
        if let Some(millis) = style.search_ms {
            queue = queue.searching_for(millis);
        }
        let mut bot = Self {
            engine,
            queue,
            style,
            asked: None,
            lines: Vec::new(),
            _events: listen(events, cx),
            _reply: None,
        };
        bot.send(&first, cx);
        bot
    }

    /// Asks for a move in `position`, to come after about `think`.
    pub fn ask(&mut self, position: Position, think: Duration, cx: &mut Context<Self>) {
        let fen = position.to_string();
        self.asked = Some(Asked {
            position,
            fen: fen.clone(),
            since: Instant::now(),
            think,
        });
        self.lines.clear();
        self._reply = None;
        let commands = self.queue.request(fen);
        self.send(&commands, cx);
    }

    fn on_event(&mut self, event: EngineEvent, cx: &mut Context<Self>) {
        match event {
            EngineEvent::Message(message) => self.on_message(message, cx),
            EngineEvent::Stopped => cx.emit(BotEvent::Failed),
        }
    }

    fn on_message(&mut self, message: UciMessage, cx: &mut Context<Self>) {
        let commands = match message {
            UciMessage::UciOk => self.queue.on_uciok(),
            UciMessage::ReadyOk => self.queue.on_readyok(),
            UciMessage::Info(info) => {
                if self.queue.accepts_info() {
                    self.lines.retain(|line| line.multipv != info.multipv);
                    self.lines.push(info);
                }
                Vec::new()
            }
            UciMessage::BestMove(_) => {
                let commands = self.queue.on_bestmove();
                self.reply(cx);
                commands
            }
            UciMessage::Other => Vec::new(),
        };
        self.send(&commands, cx);
    }

    /// Chooses among what the search found, then answers once the pause is over.
    fn reply(&mut self, cx: &mut Context<Self>) {
        let Some(asked) = self.asked.take() else {
            return;
        };
        let mut lines = std::mem::take(&mut self.lines);
        lines.sort_by_key(|line| line.multipv);
        let best: Vec<Move> = lines
            .iter()
            .filter_map(|line| line.pv.first().copied())
            .collect();
        let legal = asked.position.legal_moves();
        let rolls = (rand::random::<f64>(), rand::random::<f64>());
        let Some(mv) = pick_bot_move(&self.style, (&best, &legal), rolls) else {
            return;
        };
        let wait = asked.think.saturating_sub(asked.since.elapsed());
        let fen = asked.fen;
        self._reply = Some(cx.spawn(async move |bot, cx| {
            cx.background_executor().timer(wait).await;
            let _ = bot.update(cx, |_, cx| cx.emit(BotEvent::Moved { fen, mv }));
        }));
    }

    fn send(&mut self, commands: &[Command], cx: &mut Context<Self>) {
        if commands
            .iter()
            .any(|command| self.engine.send(command).is_err())
        {
            cx.emit(BotEvent::Failed);
        }
    }
}

/// Reads the bot's engine output until it ends or the bot is gone.
fn listen(
    events: gpui_kit::base::async_util::Receiver<EngineEvent>,
    cx: &mut Context<BotPlayer>,
) -> Task<()> {
    cx.spawn(async move |bot, cx| {
        while let Ok(event) = events.recv().await {
            if bot.update(cx, |bot, cx| bot.on_event(event, cx)).is_err() {
                break;
            }
        }
    })
}
