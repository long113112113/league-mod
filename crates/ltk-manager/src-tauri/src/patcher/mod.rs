pub mod api;

use std::sync::atomic::AtomicBool;
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;

#[derive(Clone)]
pub struct PatcherState(pub Arc<Mutex<PatcherStateInner>>);

impl PatcherState {
    pub fn new() -> Self {
        Self(Arc::new(Mutex::new(PatcherStateInner::new())))
    }
}

impl Default for PatcherState {
    fn default() -> Self {
        Self::new()
    }
}

pub struct PatcherStateInner {
    /// Flag to signal the patcher thread to stop.
    pub stop_flag: Arc<AtomicBool>,
    /// Handle to the patcher thread.
    pub thread_handle: Option<JoinHandle<()>>,
    /// The config path used when starting.
    pub config_path: Option<String>,
    /// The child process of the running mod-tools.
    pub child_process: Option<tokio::process::Child>,
    /// Token to cancel the operation.
    pub cancel_token: Option<tokio_util::sync::CancellationToken>,
}

impl PatcherStateInner {
    pub fn new() -> Self {
        Self {
            stop_flag: Arc::new(AtomicBool::new(false)),
            thread_handle: None,
            config_path: None,
            child_process: None,
            cancel_token: None,
        }
    }

    pub fn is_running(&self) -> bool {
        self.thread_handle
            .as_ref()
            .map(|h| !h.is_finished())
            .unwrap_or(false)
    }
}

impl Drop for PatcherStateInner {
    fn drop(&mut self) {
        if let Some(mut child) = self.child_process.take() {
            let _ = child.start_kill();
        }
    }
}

impl Default for PatcherStateInner {
    fn default() -> Self {
        Self::new()
    }
}
