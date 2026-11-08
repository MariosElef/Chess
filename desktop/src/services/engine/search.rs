use chess_domain::engine::Command;

use crate::models::engine::EngineSettings;

/// How many lines the engine reports at once: the best and two alternatives.
const LINES: usize = 3;

/// Where the engine is in its life and its current search.
#[derive(Debug, Clone, PartialEq, Eq)]
enum SearchState {
    /// Before `readyok`; a position asked for meanwhile is kept for when it is ready.
    Starting {
        next: Option<String>,
    },
    Idle,
    Searching {
        fen: String,
    },
    /// `stop` was sent and its `bestmove` has not arrived; `next` is searched once it does.
    Stopping {
        next: Option<String>,
    },
}

/// Keeps exactly one search running; each method returns the commands to send, in order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SearchQueue {
    state: SearchState,
    settings: EngineSettings,
    /// Engine options set after the usual ones, overriding them where they overlap.
    options: Vec<(&'static str, String)>,
    /// Each search runs this long instead of to the settings' depth.
    move_time_ms: Option<u64>,
}

impl SearchQueue {
    /// A queue for a freshly started engine, and the first command to send it.
    pub fn new(settings: EngineSettings) -> (Self, Vec<Command>) {
        let queue = Self {
            state: SearchState::Starting { next: None },
            settings,
            options: Vec::new(),
            move_time_ms: None,
        };
        (queue, vec![Command::Uci])
    }

    /// Sets these engine options too, after the usual ones: a bot's strength, say.
    pub fn with_options(mut self, options: Vec<(&'static str, String)>) -> Self {
        self.options = options;
        self
    }

    /// Searches each position for this long rather than to a depth.
    pub fn searching_for(mut self, millis: u64) -> Self {
        self.move_time_ms = Some(millis);
        self
    }

    /// The FEN being searched now.
    pub fn searching(&self) -> Option<&str> {
        match &self.state {
            SearchState::Searching { fen } => Some(fen),
            SearchState::Starting { .. } | SearchState::Idle | SearchState::Stopping { .. } => None,
        }
    }

    /// Search lines count only while a search is running; during a stop they are stale.
    pub fn accepts_info(&self) -> bool {
        self.searching().is_some()
    }

    /// The engine identified itself: set it up, then ask whether it is ready.
    pub fn on_uciok(&mut self) -> Vec<Command> {
        let mut commands = vec![
            Command::SetOption("MultiPV", LINES.to_string()),
            Command::SetOption("Threads", self.settings.threads.to_string()),
            Command::SetOption("Hash", self.settings.hash_mb.to_string()),
        ];
        commands.extend(
            self.options
                .iter()
                .map(|(name, value)| Command::SetOption(name, value.clone())),
        );
        commands.push(Command::IsReady);
        commands
    }

    /// The engine is ready: start the position asked for while it was starting, if any.
    pub fn on_readyok(&mut self) -> Vec<Command> {
        let SearchState::Starting { next } = &mut self.state else {
            return Vec::new();
        };
        let next = next.take();
        let mut commands = vec![Command::UciNewGame];
        commands.extend(self.start_or_idle(next));
        commands
    }

    /// Analyse `fen` instead of whatever is being analysed now.
    pub fn request(&mut self, fen: String) -> Vec<Command> {
        match &mut self.state {
            SearchState::Starting { next } | SearchState::Stopping { next } => {
                *next = Some(fen);
                Vec::new()
            }
            SearchState::Idle => self.start(fen),
            SearchState::Searching { fen: current } if *current == fen => Vec::new(),
            SearchState::Searching { .. } => {
                self.state = SearchState::Stopping { next: Some(fen) };
                vec![Command::Stop]
            }
        }
    }

    /// Stop analysing, as when the engine is switched off or the page is hidden.
    pub fn stop(&mut self) -> Vec<Command> {
        match &mut self.state {
            SearchState::Starting { next } | SearchState::Stopping { next } => {
                *next = None;
                Vec::new()
            }
            SearchState::Idle => Vec::new(),
            SearchState::Searching { .. } => {
                self.state = SearchState::Stopping { next: None };
                vec![Command::Stop]
            }
        }
    }

    /// A search ended, stopped or finished at its depth: start the position waiting, if any.
    pub fn on_bestmove(&mut self) -> Vec<Command> {
        match &mut self.state {
            SearchState::Stopping { next } => {
                let next = next.take();
                self.start_or_idle(next)
            }
            SearchState::Searching { .. } => {
                self.state = SearchState::Idle;
                Vec::new()
            }
            SearchState::Starting { .. } | SearchState::Idle => Vec::new(),
        }
    }

    fn start_or_idle(&mut self, next: Option<String>) -> Vec<Command> {
        match next {
            Some(fen) => self.start(fen),
            None => {
                self.state = SearchState::Idle;
                Vec::new()
            }
        }
    }

    fn start(&mut self, fen: String) -> Vec<Command> {
        let go = match (self.move_time_ms, self.settings.depth) {
            (Some(millis), _) => Command::GoMoveTime(millis),
            (None, Some(depth)) => Command::GoDepth(depth),
            (None, None) => Command::GoInfinite,
        };
        let commands = vec![Command::Position(fen.clone()), go];
        self.state = SearchState::Searching { fen };
        commands
    }
}

#[cfg(test)]
mod tests {
    use chess_domain::engine::{Command, UciMessage};

    use super::SearchQueue;
    use crate::{
        clients::engine::{Engine, locate_stockfish},
        enums::engine::EngineEvent,
        models::engine::EngineSettings,
    };

    /// No depth limit, so a search runs until stopped.
    const ENDLESS: EngineSettings = EngineSettings {
        threads: 4,
        hash_mb: 256,
        depth: None,
    };

    const START: &str = "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1";
    const AFTER_E4: &str = "rnbqkbnr/pppppppp/8/8/4P3/8/PPPP1PPP/RNBQKBNR b KQkq e3 0 1";

    fn go(fen: &str) -> Vec<Command> {
        vec![Command::Position(fen.to_owned()), Command::GoInfinite]
    }

    fn ready() -> SearchQueue {
        let (mut queue, first) = SearchQueue::new(ENDLESS);
        assert_eq!(first, [Command::Uci]);
        queue.on_uciok();
        queue.on_readyok();
        queue
    }

    #[test]
    fn setup_asks_for_three_lines_then_readiness() {
        let (mut queue, _) = SearchQueue::new(ENDLESS);
        let setup = queue.on_uciok();
        assert_eq!(setup[0], Command::SetOption("MultiPV", "3".to_owned()));
        assert_eq!(setup[1], Command::SetOption("Threads", "4".to_owned()));
        assert_eq!(setup.last(), Some(&Command::IsReady));
    }

    #[test]
    fn a_position_asked_for_before_ready_starts_once_ready() {
        let (mut queue, _) = SearchQueue::new(ENDLESS);
        assert!(queue.request(START.to_owned()).is_empty());
        queue.on_uciok();
        let mut expected = vec![Command::UciNewGame];
        expected.extend(go(START));
        assert_eq!(queue.on_readyok(), expected);
        assert_eq!(queue.searching(), Some(START));
    }

    #[test]
    fn a_new_position_stops_the_search_and_starts_after_its_bestmove() {
        let mut queue = ready();
        assert_eq!(queue.request(START.to_owned()), go(START));
        assert_eq!(queue.request(AFTER_E4.to_owned()), [Command::Stop]);
        assert!(
            !queue.accepts_info(),
            "lines from the stopped search are stale"
        );
        assert_eq!(queue.on_bestmove(), go(AFTER_E4));
        assert!(queue.accepts_info());
    }

    #[test]
    fn several_requests_during_a_stop_keep_only_the_last() {
        let mut queue = ready();
        queue.request(START.to_owned());
        queue.request(AFTER_E4.to_owned());
        queue.request(START.to_owned());
        assert_eq!(queue.on_bestmove(), go(START));
    }

    #[test]
    fn a_depth_limit_searches_to_that_depth_and_then_rests() {
        let (mut queue, _) = SearchQueue::new(EngineSettings {
            depth: Some(24),
            ..ENDLESS
        });
        queue.on_uciok();
        queue.on_readyok();
        assert_eq!(
            queue.request(START.to_owned()),
            [Command::Position(START.to_owned()), Command::GoDepth(24)]
        );
        assert!(queue.on_bestmove().is_empty());
        assert_eq!(queue.searching(), None);
    }

    #[test]
    fn asking_again_for_the_same_position_changes_nothing() {
        let mut queue = ready();
        queue.request(START.to_owned());
        assert!(queue.request(START.to_owned()).is_empty());
    }

    #[test]
    fn stopping_goes_idle_and_a_late_bestmove_is_harmless() {
        let mut queue = ready();
        queue.request(START.to_owned());
        assert_eq!(queue.stop(), [Command::Stop]);
        assert!(queue.on_bestmove().is_empty());
        assert!(queue.on_bestmove().is_empty());
        assert_eq!(queue.searching(), None);
        assert_eq!(queue.request(START.to_owned()), go(START));
    }

    /// Talks to the installed Stockfish, if there is one: handshake, a search to depth 8, and quit.
    #[test]
    fn a_real_engine_answers_and_searches() {
        let Some(path) = locate_stockfish() else {
            return;
        };
        let (mut engine, events) = Engine::spawn(&path).expect("stockfish starts");
        let (mut queue, first) = SearchQueue::new(EngineSettings {
            threads: 1,
            hash_mb: 64,
            depth: Some(8),
        });
        let mut pending = first;
        let mut saw_info = false;
        queue.request("rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1".to_owned());
        loop {
            for command in pending.drain(..) {
                engine.send(&command).expect("the engine reads commands");
            }
            match events.recv_blocking().expect("the engine is still talking") {
                EngineEvent::Message(UciMessage::UciOk) => pending = queue.on_uciok(),
                EngineEvent::Message(UciMessage::ReadyOk) => pending = queue.on_readyok(),
                EngineEvent::Message(UciMessage::Info(_)) => saw_info = true,
                EngineEvent::Message(UciMessage::BestMove(best)) => {
                    assert!(best.is_some());
                    break;
                }
                EngineEvent::Message(UciMessage::Other) => {}
                EngineEvent::Stopped => panic!("the engine stopped early"),
            }
        }
        assert!(saw_info);
        drop(engine);
        assert_eq!(events.recv_blocking(), Ok(EngineEvent::Stopped));
    }
}
