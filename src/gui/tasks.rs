use crate::gui::state::LinkSnapshot;
use std::sync::mpsc::{self, Receiver, TryRecvError};
use std::thread;

pub struct GuiTask {
    rx: Receiver<GuiTaskResult>,
}

pub enum GuiTaskResult {
    Reload(Result<LinkSnapshot, String>),
    Add(Result<String, String>),
    Remove(Result<String, String>),
}

pub enum TaskPoll {
    Pending,
    Ready(GuiTaskResult),
    Disconnected,
}

impl GuiTask {
    pub fn spawn(run: impl FnOnce() -> GuiTaskResult + Send + 'static) -> Self {
        let (tx, rx) = mpsc::channel();
        thread::spawn(move || {
            let _ = tx.send(run());
        });
        Self { rx }
    }

    pub fn poll(&self) -> TaskPoll {
        match self.rx.try_recv() {
            Ok(result) => TaskPoll::Ready(result),
            Err(TryRecvError::Empty) => TaskPoll::Pending,
            Err(TryRecvError::Disconnected) => TaskPoll::Disconnected,
        }
    }
}
