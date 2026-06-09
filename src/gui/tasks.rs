use crate::domain::gui_settings::GuiSettings;
use crate::gui::data::{GuiLinkOpError, ReloadedLinks, RemoveOutcome};
use crate::gui::state::{LinkSnapshot, SettingsDraft};
use std::sync::mpsc::{self, Receiver, TryRecvError};
use std::thread;

pub struct GuiTask {
    rx: Receiver<GuiTaskResult>,
}

pub enum GuiTaskResult {
    Reload(Result<ReloadedLinks, String>),
    LinkOp(Result<String, GuiLinkOpError>),
    Remove(Result<RemoveOutcome, String>),
    SettingsApply {
        draft: SettingsDraft,
        result: Result<SettingsApplyOutcome, String>,
    },
}

pub struct SettingsApplyOutcome {
    pub settings: GuiSettings,
    pub snapshot: Option<LinkSnapshot>,
    pub data_dir_changed: bool,
    pub save_error: Option<String>,
}

pub enum TaskPoll {
    Pending,
    Ready(Box<GuiTaskResult>),
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
            Ok(result) => TaskPoll::Ready(Box::new(result)),
            Err(TryRecvError::Empty) => TaskPoll::Pending,
            Err(TryRecvError::Disconnected) => TaskPoll::Disconnected,
        }
    }
}
