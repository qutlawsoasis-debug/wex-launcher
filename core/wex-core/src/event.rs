use std::sync::Arc;
use tokio::sync::mpsc::UnboundedSender;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LogLevel {
    Debug,
    Info,
    Warn,
    Error,
}

#[derive(Debug, Clone)]
pub enum Event {
    StageStarted {
        stage: String,
        step: usize,
        total_steps: usize,
    },
    Progress {
        stage: String,
        current: u64,
        total: u64,
        message: Option<String>,
    },
    FileDownloaded {
        path: String,
        bytes: u64,
    },
    Log {
        level: LogLevel,
        message: String,
    },
    GameLaunched {
        pid: u32,
    },
    GameOutput {
        line: String,
    },
    GameExited {
        exit_code: Option<i32>,
    },
}

pub trait EventHandler: Send + Sync {
    fn on_event(&self, event: &Event);
}

pub struct NoopEventHandler;

impl EventHandler for NoopEventHandler {
    fn on_event(&self, _event: &Event) {}
}

pub struct ConsoleEventHandler;

impl EventHandler for ConsoleEventHandler {
    fn on_event(&self, event: &Event) {
        match event {
            Event::StageStarted { stage, step, total_steps } => {
                println!("[{}/{}] === {} ===", step, total_steps, stage);
            }
            Event::Progress { stage, current, total, message } => {
                let pct = if *total > 0 {
                    (*current as f64 / *total as f64) * 100.0
                } else {
                    0.0
                };
                if let Some(msg) = message {
                    println!("[{}] {:.1}% ({}/{}) - {}", stage, pct, current, total, msg);
                } else {
                    println!("[{}] {:.1}% ({}/{})", stage, pct, current, total);
                }
            }
            Event::FileDownloaded { path, bytes } => {
                tracing::debug!("Downloaded: {} ({} bytes)", path, bytes);
            }
            Event::Log { level, message } => {
                match level {
                    LogLevel::Debug => tracing::debug!("{}", message),
                    LogLevel::Info => println!("[INFO] {}", message),
                    LogLevel::Warn => eprintln!("[WARN] {}", message),
                    LogLevel::Error => eprintln!("[ERROR] {}", message),
                }
            }
            Event::GameLaunched { pid } => {
                println!("[LAUNCH] Game process started with PID: {}", pid);
            }
            Event::GameOutput { line } => {
                println!("[GAME] {}", line);
            }
            Event::GameExited { exit_code } => {
                if let Some(code) = exit_code {
                    println!("[GAME] Process exited with code {}", code);
                } else {
                    println!("[GAME] Process terminated");
                }
            }
        }
    }
}

pub struct ChannelEventHandler {
    sender: UnboundedSender<Event>,
}

impl ChannelEventHandler {
    pub fn new(sender: UnboundedSender<Event>) -> Self {
        Self { sender }
    }
}

impl EventHandler for ChannelEventHandler {
    fn on_event(&self, event: &Event) {
        let _ = self.sender.send(event.clone());
    }
}

pub type SharedEventHandler = Arc<dyn EventHandler>;
