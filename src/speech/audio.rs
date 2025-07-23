pub mod format;
mod recorder;

use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};

pub use recorder::AudioRecorder;

#[derive(Clone)]
pub struct StopTrigger {
    has_triggered: Arc<AtomicBool>,
}

impl Default for StopTrigger {
    fn default() -> Self {
        Self::new()
    }
}

impl StopTrigger {
    #[must_use]
    pub fn new() -> Self {
        let has_triggered = Arc::new(AtomicBool::new(false));

        Self { has_triggered }
    }

    pub fn stop(self) {
        self.has_triggered.store(true, Ordering::Relaxed);
    }

    fn has_stopped(&self) -> bool {
        self.has_triggered.load(Ordering::Relaxed)
    }
}
