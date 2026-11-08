use std::{
    io::{self, BufRead as _, BufReader, Write as _},
    path::Path,
    process::{Child, ChildStdin, ChildStdout, Command as Process, Stdio},
    thread,
};

use chess_domain::engine::{Command, UciMessage};
use gpui_kit::base::async_util::{Receiver, Sender, unbounded};

/// What arrives from the engine process.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EngineEvent {
    Message(UciMessage),
    /// Its output ended: it exited or crashed.
    Stopped,
}

/// A running engine. Dropping it ends the process.
#[derive(Debug)]
pub struct Engine {
    child: Child,
    stdin: ChildStdin,
}

impl Engine {
    /// Starts the engine at `path`; its lines arrive on the returned receiver, read by a
    /// thread of their own so the app's executors never block on the process.
    pub fn spawn(path: &Path) -> io::Result<(Self, Receiver<EngineEvent>)> {
        let mut child = Process::new(path)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()?;
        let stdin = child.stdin.take().ok_or_else(|| missing_pipe("stdin"))?;
        let stdout = child.stdout.take().ok_or_else(|| missing_pipe("stdout"))?;
        let (sender, receiver) = unbounded();
        thread::Builder::new()
            .name("stockfish-reader".to_owned())
            .spawn(move || read_lines(stdout, &sender))?;
        Ok((Self { child, stdin }, receiver))
    }

    /// Writes one command; an engine that died answers with `BrokenPipe`.
    pub fn send(&mut self, command: &Command) -> io::Result<()> {
        writeln!(self.stdin, "{command}")?;
        self.stdin.flush()
    }
}

impl Drop for Engine {
    /// Asks the engine to quit, then makes sure it is gone and reaped.
    fn drop(&mut self) {
        let _ = self.send(&Command::Quit);
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

/// Forwards every line until the engine's output ends or the app stops listening.
fn read_lines(stdout: ChildStdout, sender: &Sender<EngineEvent>) {
    for line in BufReader::new(stdout).lines().map_while(Result::ok) {
        let event = EngineEvent::Message(UciMessage::parse(&line));
        if sender.send_blocking(event).is_err() {
            return;
        }
    }
    let _ = sender.send_blocking(EngineEvent::Stopped);
}

fn missing_pipe(name: &str) -> io::Error {
    io::Error::other(format!("the engine's {name} was not piped"))
}

#[cfg(test)]
mod tests {
    use chess_domain::engine::UciMessage;

    use super::{Engine, EngineEvent};
    use crate::engine::{EngineSettings, SearchQueue, locate_stockfish};

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
