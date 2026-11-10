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
    /// Starts the engine at `path`; its lines arrive on the returned receiver, read off-thread.
    pub fn spawn(path: &Path) -> io::Result<(Self, Receiver<EngineEvent>)> {
        let mut process = Process::new(path);
        process
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null());
        without_console(&mut process);
        let mut child = process.spawn()?;
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

/// On Windows a window app that starts a console program gets a console window for it, one per
/// engine; the engine talks over pipes, so it is started without one.
#[cfg(windows)]
fn without_console(process: &mut Process) {
    use std::os::windows::process::CommandExt as _;
    const CREATE_NO_WINDOW: u32 = 0x0800_0000;
    process.creation_flags(CREATE_NO_WINDOW);
}

/// Elsewhere a program started from an app has no window of its own.
#[cfg(not(windows))]
fn without_console(_: &mut Process) {}

/// Forwards every line until the engine's output ends or the app stops listening.
fn read_lines(stdout: ChildStdout, sender: &Sender<EngineEvent>) {
    let mut reader = BufReader::new(stdout);
    let mut bytes = Vec::new();
    loop {
        bytes.clear();
        match reader.read_until(b'\n', &mut bytes) {
            Ok(0) => break,
            Ok(_) => {}
            Err(error) if error.kind() == io::ErrorKind::Interrupted => continue,
            Err(_) => break,
        }
        let line = String::from_utf8_lossy(&bytes);
        let event = EngineEvent::Message(UciMessage::parse(line.trim_end_matches(['\n', '\r'])));
        if sender.send_blocking(event).is_err() {
            return;
        }
    }
    let _ = sender.send_blocking(EngineEvent::Stopped);
}

fn missing_pipe(name: &str) -> io::Error {
    io::Error::other(format!("the engine's {name} was not piped"))
}
