use std::{
    io::{self, BufRead as _, BufReader, Write as _},
    path::Path,
    process::{Child, ChildStdin, ChildStdout, Command as Process, Stdio},
    thread,
};

use chess_domain::engine::{Command, UciMessage};
use gpui_kit::base::async_util::{Receiver, Sender, unbounded};

use crate::enums::engine::EngineEvent;

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
