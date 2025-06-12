use std::sync::{Arc, Mutex};

#[derive(Clone)]
pub struct UiLogger {
    pub buffer: Arc<Mutex<Vec<String>>>,
}

impl UiLogger {
    pub fn new() -> Self {
        Self {
            buffer: Arc::new(Mutex::new(Vec::new())),
        }
    }

    pub fn warn(&self, msg: impl Into<String>) {
        let msg = msg.into();
        log::warn!("{}", msg);
        let mut buf = self.buffer.lock().unwrap();
        buf.push(format!("Warning: {}", msg));
        if buf.len() > 10 {
            buf.remove(0);
        }
    }

    pub fn error(&self, msg: impl Into<String>) {
        let msg = msg.into();
        log::error!("{}", msg);
        let mut buf = self.buffer.lock().unwrap();
        buf.push(format!("Error: {}", msg));
        if buf.len() > 10 {
            buf.remove(0);
        }
    }

    pub fn messages(&self) -> Vec<String> {
        self.buffer.lock().unwrap().clone()
    }
}