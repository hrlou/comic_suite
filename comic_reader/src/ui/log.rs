//! UI logger for warnings and errors.

use crate::prelude::*;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum UiLogLevel {
    Warning,
    Error,
    Info,
}
/// Logger that pushes warnings and errors to both log and UI.
#[derive(Clone)]
pub struct UiLogger {
    pub message: Option<(String, UiLogLevel)>,
    message_time: Option<Instant>,
}

impl UiLogger {
    /// Create a new UI logger.
    pub fn new() -> Self {
        Self {
            message: None,
            message_time: None,
        }
    }

    /// Internal helper to set message and timestamp.
    fn set_message(&mut self, msg: String, level: UiLogLevel) {
        self.message = Some((msg, level));
        self.message_time = Some(Instant::now());
    }

    /// Log a warning (to log and UI).
    pub fn warn(&mut self, msg: impl Into<String>) {
        let msg = msg.into();
        log::warn!("{}", msg);
        self.set_message(msg, UiLogLevel::Warning);
    }

    /// Log an error (to log and UI).
    pub fn error(&mut self, msg: impl Into<String>) {
        let msg = msg.into();
        log::error!("{}", msg);
        self.set_message(msg, UiLogLevel::Error);
    }

    /// Log info (to log and UI).
    pub fn info(&mut self, msg: impl Into<String>) {
        let msg = msg.into();
        log::info!("{}", msg);
        self.set_message(msg, UiLogLevel::Info);
    }

    /// Call this regularly (e.g., every UI frame) to clear old messages.
    pub fn clear_expired(&mut self) {
        if let Some(t) = self.message_time {
            if t.elapsed() >= Duration::from_secs(LOG_TIMEOUT as u64) {
                self.message = None;
                self.message_time = None;
            }
        }
    }
}
