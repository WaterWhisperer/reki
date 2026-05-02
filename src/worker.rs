use crate::model::CommitRow;
use crate::state::Action;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum WorkerMessage {
    CommitBatchLoaded {
        rows: Vec<CommitRow>,
        all_loaded: bool,
    },
    LoadFailed(String),
}

impl From<WorkerMessage> for Action {
    fn from(message: WorkerMessage) -> Self {
        match message {
            WorkerMessage::CommitBatchLoaded { rows, all_loaded } => {
                Action::CommitBatchLoaded { rows, all_loaded }
            }
            WorkerMessage::LoadFailed(message) => Action::LoadFailed(message),
        }
    }
}
