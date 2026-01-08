//! Logging configuration
//!
//! Provides structured logging using the `tracing` ecosystem.
//! Supports different log levels and output formats for development vs production.

use tracing_subscriber::{fmt, EnvFilter, layer::SubscriberExt, util::SubscriberInitExt};

/// Log level for the application
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LogLevel {
    Trace,
    Debug,
    Info,
    Warn,
    Error,
}

impl LogLevel {
    fn as_str(&self) -> &'static str {
        match self {
            LogLevel::Trace => "trace",
            LogLevel::Debug => "debug",
            LogLevel::Info => "info",
            LogLevel::Warn => "warn",
            LogLevel::Error => "error",
        }
    }
}

impl Default for LogLevel {
    fn default() -> Self {
        if cfg!(debug_assertions) {
            LogLevel::Debug
        } else {
            LogLevel::Info
        }
    }
}

/// Initialize the logging system
///
/// This should be called once at application startup.
/// Uses environment variable `RUST_LOG` to configure log levels,
/// falling back to the provided default level.
///
/// # Examples
///
/// ```
/// use noter_lib::logging::{init_logging, LogLevel};
///
/// // In development
/// init_logging(LogLevel::Debug);
///
/// // In production
/// init_logging(LogLevel::Info);
/// ```
pub fn init_logging(default_level: LogLevel) {
    // Build filter from env or use default
    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| {
            // Default filter: set our crate to specified level, dependencies to warn
            EnvFilter::new(format!(
                "noter_lib={},tauri={},hyper=warn,reqwest=warn",
                default_level.as_str(),
                if matches!(default_level, LogLevel::Trace | LogLevel::Debug) { "debug" } else { "info" }
            ))
        });

    // Configure format layer
    let fmt_layer = fmt::layer()
        .with_target(true)
        .with_level(true)
        .with_thread_ids(cfg!(debug_assertions))
        .with_file(cfg!(debug_assertions))
        .with_line_number(cfg!(debug_assertions));

    // Initialize subscriber
    let subscriber = tracing_subscriber::registry()
        .with(filter)
        .with(fmt_layer);

    // Try to set as global default (ignore if already set)
    let _ = subscriber.try_init();
}

/// Initialize logging for tests
#[cfg(test)]
pub fn init_test_logging() {
    let _ = tracing_subscriber::fmt()
        .with_env_filter("debug")
        .with_test_writer()
        .try_init();
}

/// Convenience macro for timing operations
/// 
/// Usage:
/// ```ignore
/// let result = timed!("fetch_data", {
///     fetch_something().await
/// });
/// ```
#[macro_export]
macro_rules! timed {
    ($name:expr, $block:expr) => {{
        let start = std::time::Instant::now();
        let result = $block;
        tracing::debug!(
            target: "timing",
            operation = $name,
            duration_ms = start.elapsed().as_millis() as u64,
            "Operation completed"
        );
        result
    }};
}

/// Log macros with consistent formatting
/// These re-export tracing macros with added context

/// Log an audio-related event
#[macro_export]
macro_rules! log_audio {
    ($level:ident, $($arg:tt)*) => {
        tracing::$level!(target: "audio", $($arg)*)
    };
}

/// Log a transcription-related event
#[macro_export]
macro_rules! log_transcription {
    ($level:ident, $($arg:tt)*) => {
        tracing::$level!(target: "transcription", $($arg)*)
    };
}

/// Log a meeting-related event
#[macro_export]
macro_rules! log_meeting {
    ($level:ident, $($arg:tt)*) => {
        tracing::$level!(target: "meeting", $($arg)*)
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_log_level_default() {
        let level = LogLevel::default();
        // In debug build, should be Debug
        assert!(matches!(level, LogLevel::Debug | LogLevel::Info));
    }
    
    #[test]
    fn test_log_level_as_str() {
        assert_eq!(LogLevel::Info.as_str(), "info");
        assert_eq!(LogLevel::Error.as_str(), "error");
    }
}
