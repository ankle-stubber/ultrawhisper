use crate::managers::logs::LogManager;
use log::{Level, Log, Metadata, Record, SetLoggerError};
use std::sync::Arc;

/// Combined logger that forwards to both env_logger and LogManager
pub struct CombinedLogger {
    env_logger: env_logger::Logger,
    log_manager: Arc<LogManager>,
}

impl CombinedLogger {
    pub fn new(log_manager: Arc<LogManager>) -> Self {
        // Build env_logger with default configuration
        let env_logger = env_logger::Builder::from_default_env()
            .build();

        Self {
            env_logger,
            log_manager,
        }
    }

    /// Initialize the combined logger as the global logger
    pub fn init(log_manager: Arc<LogManager>) -> Result<(), SetLoggerError> {
        let logger = CombinedLogger::new(log_manager);
        let max_level = logger.env_logger.filter();

        log::set_boxed_logger(Box::new(logger))?;
        log::set_max_level(max_level);

        Ok(())
    }
}

impl Log for CombinedLogger {
    fn enabled(&self, metadata: &Metadata) -> bool {
        // Use env_logger's filtering logic
        self.env_logger.enabled(metadata)
    }

    fn log(&self, record: &Record) {
        // Only process if enabled by env_logger's filter
        if !self.env_logger.enabled(record.metadata()) {
            return;
        }

        // Forward to env_logger for stdout/stderr
        self.env_logger.log(record);

        // Capture to LogManager
        let level = match record.level() {
            Level::Error => "error",
            Level::Warn => "warn",
            Level::Info => "info",
            Level::Debug => "debug",
            Level::Trace => "trace",
        };

        self.log_manager.push(
            level.to_string(),
            record.target().to_string(),
            format!("{}", record.args()),
        );
    }

    fn flush(&self) {
        // Delegate flush to env_logger
        self.env_logger.flush();
    }
}
