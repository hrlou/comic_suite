//! UI logger for warnings and errors.

use std::sync::{Arc, Mutex};

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
}


impl UiLogger {
    /// Create a new UI logger.
    pub fn new() -> Self {
        Self {
            message: None,
        }
    }

    /// Log a warning (to log and UI).
    pub fn warn(&mut self, msg: impl Into<String>) {
        let msg = msg.into();
        log::warn!("{}", msg);
        self.message = Some((msg, UiLogLevel::Warning));
    }

    /// Log an error (to log and UI).
    pub fn error(&mut self, msg: impl Into<String>) {
        let msg = msg.into();
        log::error!("{}", msg);
        self.message = Some((msg, UiLogLevel::Error));
    }

    /// Get all messages for display.
    pub fn info(&mut self, msg: impl Into<String>) {
        let msg = msg.into();
        log::info!("{}", msg);
        self.message = Some((msg, UiLogLevel::Info));
    }
}