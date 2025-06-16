//! UI logger for warnings and errors.

use std::sync::{Arc, Mutex};

/// Logger that pushes warnings and errors to both log and UI.
#[derive(Clone)]
pub struct UiLogger {
    pub buffer: Arc<Mutex<Vec<String>>>,
}

impl UiLogger {
    /// Create a new UI logger.
    pub fn new() -> Self {
        Self {
            buffer: Arc::new(Mutex::new(Vec::new())),
        }
    }

    /// Log a warning (to log and UI).
    pub fn warn(&self, msg: impl Into<String>) {
        let msg = msg.into();
        log::warn!("{}", msg);
        let mut buf = self.buffer.lock().unwrap();
        buf.push(format!("Warning: {}", msg));
        if buf.len() > 10 {
            buf.remove(0);
        }
    }

    /// Log an error (to log and UI).
    pub fn error(&self, msg: impl Into<String>) {
        let msg = msg.into();
        log::error!("{}", msg);
        let mut buf = self.buffer.lock().unwrap();
        buf.push(format!("Error: {}", msg));
        if buf.len() > 10 {
            buf.remove(0);
        }
    }

    /// Get all messages for display.
    pub fn messages(&self) -> Vec<String> {
        self.buffer.lock().unwrap().clone()
    }
}