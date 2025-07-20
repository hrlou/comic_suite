//! UI logger for warnings and errors.

use crate::prelude::*;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum UiLogLevel {
    Warning,
    Error,
    Info,
}

impl UiLogLevel {
    pub fn as_str(&self) -> &str {
        match self {
            UiLogLevel::Warning => "Warning",
            UiLogLevel::Error => "Error",
            UiLogLevel::Info => "Info",
        }
    }

    pub fn color(&self) -> Color32 {
        match self {
            UiLogLevel::Warning => Color32::YELLOW,
            UiLogLevel::Error => Color32::RED,
            UiLogLevel::Info => Color32::WHITE,
        }
    }
}

/// Logger that pushes warnings and errors to both log and UI.
#[derive(Clone)]
pub struct UiLogger {
    pub message: Option<(String, UiLogLevel)>,
    message_time: Option<Instant>,
    pub timeout_override: Option<u64>,
}

impl UiLogger {
    /// Create a new UI logger.
    pub fn new() -> Self {
        Self {
            message: None,
            message_time: None,
            timeout_override: None,
        }
    }

    /// Internal helper to set message and timestamp.
    fn set_message(&mut self, msg: String, level: UiLogLevel, timeout: Option<u64>) {
        self.message = Some((msg, level));
        self.message_time = Some(Instant::now());
        self.timeout_override = timeout;
    }

    /// Log a warning (to log and UI).
    pub fn warn(&mut self, msg: impl Into<String>, timeout: Option<u64>) {
        let msg = msg.into();
        log::warn!("{}", msg);
        self.set_message(msg, UiLogLevel::Warning, timeout);
    }

    /// Log an error (to log and UI).
    pub fn error(&mut self, msg: impl Into<String>, timeout: Option<u64>) {
        let msg = msg.into();
        log::error!("{}", msg);
        self.set_message(msg, UiLogLevel::Error, timeout);
    }

    /// Log info (to log and UI).
    pub fn info(&mut self, msg: impl Into<String>, timeout: Option<u64>) {
        let msg = msg.into();
        log::info!("{}", msg);
        self.set_message(msg, UiLogLevel::Info, timeout);
    }

    /// Call this regularly (e.g., every UI frame) to clear old messages.
    pub fn clear_expired(&mut self) {
        if let Some(t) = self.message_time {
            let timeout = self.timeout_override.unwrap_or(LOG_TIMEOUT as u64);
            if t.elapsed() >= Duration::from_secs(timeout) {
                self.message = None;
                self.message_time = None;
            }
        }
    }
}
