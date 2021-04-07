use crate::io::*;
use log::Log;

pub fn init<const VERBOSE: bool>(level: log::LevelFilter) -> Result<(), log::SetLoggerError> {
    log::set_logger(&crate::logger::Logger::<VERBOSE>)?;
    log::set_max_level(level);

    Ok(())
}

pub struct Logger<const VERBOSE: bool>;

impl<const VERBOSE: bool> Log for Logger<VERBOSE> {
    fn enabled(&self, _metadata: &log::Metadata) -> bool {
        // TODO?
        true
    }

    fn log(&self, record: &log::Record) {
        if !self.enabled(record.metadata()) {
            return;
        }

        let level = record.level().as_str();
        let color = match record.level() {
            log::Level::Error => "31",
            log::Level::Warn => "33",
            log::Level::Info => "34",
            log::Level::Debug => "35",
            log::Level::Trace => "36",
        };

        let padding = if level.len() < 5 { " " } else { "" };

        if VERBOSE {
            if let (Some(file), Some(line)) = (record.file(), record.line()) {
                println!(
                    "[\x1b[{}m{}{}\x1b[0m] [{}:{}] {}",
                    color,
                    level,
                    padding,
                    file,
                    line,
                    record.args()
                );
                return;
            }
        }

        println!(
            "[\x1b[{}m{}{}\x1b[0m] {}",
            color,
            level,
            padding,
            record.args()
        );
    }

    fn flush(&self) {
        // TODO: add flush logic here when/if we buffer stdout
    }
}
