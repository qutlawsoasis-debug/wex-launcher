use std::fs::{File, OpenOptions};
use std::io::Write;
use std::path::PathBuf;
use std::sync::Mutex;
use tracing::field::{Field, Visit};
use tracing::span::{Attributes, Id, Record};
use tracing::{Event, Level, Metadata, Subscriber};

pub struct WexLogger {
    file: Option<Mutex<File>>,
    attach_stdout: bool,
}

impl WexLogger {
    pub fn init() {
        let log_path = std::env::var("WEX_LOG_FILE")
            .ok()
            .map(PathBuf::from)
            .or_else(|| {
                std::env::var("WEX_DATA_DIR")
                    .ok()
                    .map(|d| PathBuf::from(d).join("logs").join("launcher.log"))
            });

        let file = if let Some(path) = log_path {
            if let Some(parent) = path.parent() {
                let _ = std::fs::create_dir_all(parent);
            }
            OpenOptions::new()
                .create(true)
                .append(true)
                .open(&path)
                .ok()
                .map(Mutex::new)
        } else {
            None
        };

        #[cfg(windows)]
        unsafe {
            extern "system" {
                fn AttachConsole(dwProcessId: u32) -> i32;
            }
            AttachConsole(0xFFFFFFFF);
        }

        let logger = WexLogger {
            file,
            attach_stdout: true,
        };

        let _ = tracing::subscriber::set_global_default(logger);
    }
}

struct MessageVisitor<'a> {
    message: &'a mut String,
}

impl<'a> Visit for MessageVisitor<'a> {
    fn record_debug(&mut self, field: &Field, value: &dyn std::fmt::Debug) {
        if field.name() == "message" {
            use std::fmt::Write;
            let _ = write!(self.message, "{:?}", value);
        } else {
            use std::fmt::Write;
            let _ = write!(self.message, " {}={:?}", field.name(), value);
        }
    }
    fn record_str(&mut self, field: &Field, value: &str) {
        if field.name() == "message" {
            self.message.push_str(value);
        } else {
            use std::fmt::Write;
            let _ = write!(self.message, " {}={}", field.name(), value);
        }
    }
}

impl Subscriber for WexLogger {
    fn enabled(&self, metadata: &Metadata<'_>) -> bool {
        metadata.level() <= &Level::DEBUG
    }

    fn new_span(&self, _span: &Attributes<'_>) -> Id {
        Id::from_u64(1)
    }

    fn record(&self, _span: &Id, _values: &Record<'_>) {}

    fn record_follows_from(&self, _span: &Id, _follows: &Id) {}

    fn event(&self, event: &Event<'_>) {
        let mut msg = String::new();
        let mut visitor = MessageVisitor { message: &mut msg };
        event.record(&mut visitor);

        let target = event.metadata().target();
        let level = event.metadata().level();
        let ts = chrono::Local::now().format("%Y-%m-%d %H:%M:%S%.3f");
        let formatted = format!("[{}] [{}] [{}]: {}\n", ts, level, target, msg);

        if let Some(ref f) = self.file {
            if let Ok(mut guard) = f.lock() {
                let _ = guard.write_all(formatted.as_bytes());
                let _ = guard.flush();
            }
        }
        if self.attach_stdout {
            print!("{}", formatted);
            let _ = std::io::stdout().flush();
        }
    }

    fn enter(&self, _span: &Id) {}

    fn exit(&self, _span: &Id) {}
}
