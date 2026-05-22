use crate::error::Result;
use portable_pty::{native_pty_system, CommandBuilder, PtySize};
use std::io::{self, IsTerminal, Read, Write};
#[cfg(unix)]
use std::os::fd::AsRawFd;
use std::process::ExitCode;
use std::sync::mpsc;
use std::thread;

const ESC: u8 = 0x1b;
const BEL: u8 = 0x07;
const OSC_COMMANDS_TO_FILTER: &[u8] = b"012";

#[derive(Debug, Clone, PartialEq, Eq)]
enum FilterState {
    Ground,
    Esc,
    OscStart,
    OscCommand { command: Vec<u8> },
    FilteringOsc,
    FilteringOscEsc,
    PassingOsc,
    PassingOscEsc,
}

#[derive(Debug, Clone)]
pub(crate) struct TerminalTitleFilter {
    state: FilterState,
    pending: Vec<u8>,
}

impl TerminalTitleFilter {
    pub(crate) fn new() -> Self {
        Self {
            state: FilterState::Ground,
            pending: Vec::new(),
        }
    }

    pub(crate) fn push(&mut self, input: &[u8], output: &mut Vec<u8>) {
        for &byte in input {
            match &mut self.state {
                FilterState::Ground => {
                    if byte == ESC {
                        self.pending.clear();
                        self.pending.push(byte);
                        self.state = FilterState::Esc;
                    } else {
                        output.push(byte);
                    }
                }
                FilterState::Esc => {
                    self.pending.push(byte);
                    if byte == b']' {
                        self.state = FilterState::OscStart;
                    } else {
                        output.extend_from_slice(&self.pending);
                        self.pending.clear();
                        self.state = FilterState::Ground;
                    }
                }
                FilterState::OscStart => {
                    self.pending.push(byte);
                    if byte.is_ascii_digit() {
                        self.state = FilterState::OscCommand {
                            command: vec![byte],
                        };
                    } else if byte == BEL {
                        output.extend_from_slice(&self.pending);
                        self.pending.clear();
                        self.state = FilterState::Ground;
                    } else if byte == ESC {
                        output.extend_from_slice(&self.pending);
                        self.pending.clear();
                        self.state = FilterState::PassingOscEsc;
                    } else {
                        output.extend_from_slice(&self.pending);
                        self.pending.clear();
                        self.state = FilterState::PassingOsc;
                    }
                }
                FilterState::OscCommand { command } => {
                    self.pending.push(byte);
                    if byte == b';' {
                        if command.len() == 1 && OSC_COMMANDS_TO_FILTER.contains(&command[0]) {
                            self.pending.clear();
                            self.state = FilterState::FilteringOsc;
                        } else {
                            output.extend_from_slice(&self.pending);
                            self.pending.clear();
                            self.state = FilterState::PassingOsc;
                        }
                    } else if byte.is_ascii_digit() {
                        command.push(byte);
                    } else if byte == BEL {
                        output.extend_from_slice(&self.pending);
                        self.pending.clear();
                        self.state = FilterState::Ground;
                    } else if byte == ESC {
                        output.extend_from_slice(&self.pending);
                        self.pending.clear();
                        self.state = FilterState::PassingOscEsc;
                    } else {
                        output.extend_from_slice(&self.pending);
                        self.pending.clear();
                        self.state = FilterState::PassingOsc;
                    }
                }
                FilterState::FilteringOsc => {
                    if byte == BEL {
                        self.state = FilterState::Ground;
                    } else if byte == ESC {
                        self.state = FilterState::FilteringOscEsc;
                    }
                }
                FilterState::FilteringOscEsc => {
                    if byte == b'\\' {
                        self.state = FilterState::Ground;
                    } else if byte != ESC {
                        self.state = FilterState::FilteringOsc;
                    }
                }
                FilterState::PassingOsc => {
                    output.push(byte);
                    if byte == BEL {
                        self.pending.clear();
                        self.state = FilterState::Ground;
                    } else if byte == ESC {
                        self.state = FilterState::PassingOscEsc;
                    }
                }
                FilterState::PassingOscEsc => {
                    output.push(byte);
                    if byte == b'\\' {
                        self.pending.clear();
                        self.state = FilterState::Ground;
                    } else if byte != ESC {
                        self.state = FilterState::PassingOsc;
                    }
                }
            }
        }
    }

    pub(crate) fn finish(&mut self, output: &mut Vec<u8>) {
        match self.state {
            FilterState::FilteringOsc | FilterState::FilteringOscEsc => {}
            FilterState::Ground => {}
            _ => output.extend_from_slice(&self.pending),
        }
        self.pending.clear();
        self.state = FilterState::Ground;
    }
}

pub(crate) fn filter_terminal_title_bytes(chunks: &[&[u8]]) -> Vec<u8> {
    let mut filter = TerminalTitleFilter::new();
    let mut output = Vec::new();
    for chunk in chunks {
        filter.push(chunk, &mut output);
    }
    filter.finish(&mut output);
    output
}

pub(crate) fn title_protect_command(args: &[String]) -> Result<ExitCode> {
    let command_args = match args.split_first() {
        Some((separator, rest)) if separator == "--" => rest,
        _ => args,
    };
    let Some((program, program_args)) = command_args.split_first() else {
        return Err("usage: config-tools title-protect -- <command> [args...]".to_string());
    };

    run_title_protected(program, program_args)
}

fn run_title_protected(program: &str, args: &[String]) -> Result<ExitCode> {
    let _raw_terminal = RawTerminalMode::enter()?;

    let pty_system = native_pty_system();
    let pair = pty_system
        .openpty(PtySize {
            rows: terminal_size_env("LINES").unwrap_or(24),
            cols: terminal_size_env("COLUMNS").unwrap_or(80),
            pixel_width: 0,
            pixel_height: 0,
        })
        .map_err(|err| format!("cannot open pty: {err}"))?;

    let mut command = CommandBuilder::new(program);
    for arg in args {
        command.arg(arg);
    }

    let mut child = pair
        .slave
        .spawn_command(command)
        .map_err(|err| format!("cannot run {program}: {err}"))?;
    drop(pair.slave);

    let input_thread = if stdin_is_terminal() {
        let mut writer = pair
            .master
            .take_writer()
            .map_err(|err| format!("cannot open pty writer: {err}"))?;
        Some(thread::spawn(move || {
            let _ = io::copy(&mut io::stdin().lock(), &mut writer);
        }))
    } else {
        None
    };

    let mut reader = pair
        .master
        .try_clone_reader()
        .map_err(|err| format!("cannot open pty reader: {err}"))?;
    let (tx, rx) = mpsc::channel();
    let output_thread = thread::spawn(move || {
        let mut filter = TerminalTitleFilter::new();
        let mut stdout = io::stdout().lock();
        let mut buffer = [0_u8; 8192];
        loop {
            let count = match reader.read(&mut buffer) {
                Ok(0) => break,
                Ok(count) => count,
                Err(_) => break,
            };
            let mut filtered = Vec::with_capacity(count);
            filter.push(&buffer[..count], &mut filtered);
            if stdout.write_all(&filtered).is_err() || stdout.flush().is_err() {
                break;
            }
        }
        let mut filtered = Vec::new();
        filter.finish(&mut filtered);
        let _ = stdout.write_all(&filtered);
        let _ = stdout.flush();
        let _ = tx.send(());
    });

    let status = child
        .wait()
        .map_err(|err| format!("cannot wait for {program}: {err}"))?;
    drop(pair.master);
    let _ = rx.recv();
    let _ = output_thread.join();
    if let Some(input_thread) = input_thread {
        drop(input_thread);
    }

    let code = status.exit_code() as u8;
    Ok(ExitCode::from(code))
}

fn terminal_size_env(name: &str) -> Option<u16> {
    std::env::var(name).ok()?.parse().ok()
}

struct RawTerminalMode {
    #[cfg(unix)]
    original: Option<libc::termios>,
}

impl RawTerminalMode {
    fn enter() -> Result<Self> {
        if !stdin_is_terminal() {
            return Ok(Self {
                #[cfg(unix)]
                original: None,
            });
        }

        #[cfg(unix)]
        {
            let fd = io::stdin().as_raw_fd();
            let mut original = unsafe { std::mem::zeroed::<libc::termios>() };
            if unsafe { libc::tcgetattr(fd, &mut original) } != 0 {
                return Err(format!(
                    "cannot read terminal mode: {}",
                    io::Error::last_os_error()
                ));
            }
            let mut raw = original;
            unsafe { libc::cfmakeraw(&mut raw) };
            if unsafe { libc::tcsetattr(fd, libc::TCSANOW, &raw) } != 0 {
                return Err(format!(
                    "cannot enable raw terminal mode: {}",
                    io::Error::last_os_error()
                ));
            }
            Ok(Self {
                original: Some(original),
            })
        }

        #[cfg(not(unix))]
        {
            Ok(Self {})
        }
    }
}

impl Drop for RawTerminalMode {
    fn drop(&mut self) {
        #[cfg(unix)]
        if let Some(original) = self.original {
            let _ = unsafe { libc::tcsetattr(io::stdin().as_raw_fd(), libc::TCSANOW, &original) };
        }
    }
}

fn stdin_is_terminal() -> bool {
    io::stdin().is_terminal()
}
