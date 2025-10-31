use crate::managers::logs::LogManager;
use chrono::Utc;
use log::{Level, Log, Metadata, Record, SetLoggerError};
use std::fs::{self, File, OpenOptions};
use std::io::{BufWriter, Write};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

/// Combined logger that forwards to both env_logger and LogManager
pub struct CombinedLogger {
    env_logger: env_logger::Logger,
    log_manager: Arc<LogManager>,
    file_sinks: Vec<Mutex<BufWriter<File>>>,
}

impl CombinedLogger {
    pub fn new(log_manager: Arc<LogManager>, log_files: Vec<PathBuf>) -> Self {
        // Build env_logger with default configuration
        let env_logger = env_logger::Builder::from_default_env().build();

        let mut sinks: Vec<Mutex<BufWriter<File>>> = Vec::new();
        for path in log_files {
            if let Some(parent) = path.parent() {
                let _ = fs::create_dir_all(parent);
            }
            match OpenOptions::new().create(true).append(true).open(&path) {
                Ok(file) => sinks.push(Mutex::new(BufWriter::new(file))),
                Err(e) => eprintln!("Failed to open log file {:?}: {}", path, e),
            }
        }

        Self {
            env_logger,
            log_manager,
            file_sinks: sinks,
        }
    }

    /// Initialize the combined logger as the global logger
    pub fn init(log_manager: Arc<LogManager>) -> Result<(), SetLoggerError> {
        let logger = CombinedLogger::new(log_manager, Vec::new());
        let max_level = logger.env_logger.filter();

        log::set_boxed_logger(Box::new(logger))?;
        log::set_max_level(max_level);

        Ok(())
    }

    /// Initialize the combined logger with one or more file sinks
    pub fn init_with_files(
        log_manager: Arc<LogManager>,
        log_files: Vec<PathBuf>,
    ) -> Result<(), SetLoggerError> {
        let logger = CombinedLogger::new(log_manager, log_files);
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

        // Best-effort persistent log to files
        for sink in &self.file_sinks {
            if let Ok(mut guard) = sink.lock() {
                let _ = writeln!(
                    &mut *guard,
                    "{} {} {} - {}",
                    Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true),
                    level,
                    record.target(),
                    record.args()
                );
                let _ = guard.flush();
            }
        }
    }

    fn flush(&self) {
        // Delegate flush to env_logger
        self.env_logger.flush();
    }
}
