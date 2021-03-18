use log::Log;

pub struct Logger;

impl Log for Logger {
    fn enabled(&self, _metadata: &log::Metadata) -> bool {
        // TODO
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

        eprintln!(
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
