use std::{
    sync::mpsc::{self, Receiver, Sender},
    thread,
};

use anyhow::Result;

use crate::git::{CommitInfo, Repo};
use crate::graph::Graph;
use crate::model::{CommitId, CommitRow};
use crate::state::Action;

/// Default batch size for incremental commit loading.
const BATCH_SIZE: usize = 200;

pub(crate) struct WorkerHandle {
    pub(crate) commands: Sender<WorkerCommand>,
    pub(crate) messages: Receiver<WorkerMessage>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum WorkerCommand {
    LoadDetails(CommitId),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum WorkerMessage {
    CommitBatchLoaded {
        rows: Vec<CommitRow>,
        all_loaded: bool,
    },
    CommitDetailsLoaded(Box<crate::model::CommitDetails>),
    CommitDetailsFailed {
        id: CommitId,
        message: String,
    },
    LoadFailed(String),
}

pub(crate) fn spawn_loader(repo: Repo) -> WorkerHandle {
    let (message_sender, messages) = mpsc::channel();
    let (commands, command_receiver) = mpsc::channel();

    thread::spawn(move || {
        run(repo, &message_sender, &command_receiver);
    });

    WorkerHandle { commands, messages }
}

fn run(repo: Repo, sender: &Sender<WorkerMessage>, commands: &Receiver<WorkerCommand>) {
    let mut graph = Graph::default();
    let mut cursor = match repo.commit_cursor() {
        Ok(cursor) => cursor,
        Err(error) => {
            let _ = sender.send(WorkerMessage::LoadFailed(error.to_string()));
            return;
        }
    };

    loop {
        match load_next_batch(&mut cursor, &mut graph, sender) {
            Ok(true) => break,
            Ok(false) => drain_commands(&repo, sender, commands),
            Err(error) => {
                let _ = sender.send(WorkerMessage::LoadFailed(error.to_string()));
                return;
            }
        }
    }

    drop(cursor);

    while let Ok(command) = commands.recv() {
        if !handle_command(&repo, sender, command) {
            break;
        }
    }
}

fn load_next_batch(
    cursor: &mut crate::git::CommitCursor<'_>,
    graph: &mut Graph,
    sender: &Sender<WorkerMessage>,
) -> Result<bool> {
    let commits = cursor.next_batch(BATCH_SIZE)?;
    let all_loaded = commits.is_empty();
    let rows = rows_from_commits_with_graph(commits, graph);

    if sender
        .send(WorkerMessage::CommitBatchLoaded { rows, all_loaded })
        .is_err()
    {
        return Ok(true);
    }

    Ok(all_loaded)
}

fn drain_commands(repo: &Repo, sender: &Sender<WorkerMessage>, commands: &Receiver<WorkerCommand>) {
    while let Ok(command) = commands.try_recv() {
        if !handle_command(repo, sender, command) {
            break;
        }
    }
}

fn handle_command(repo: &Repo, sender: &Sender<WorkerMessage>, command: WorkerCommand) -> bool {
    let message = match command {
        WorkerCommand::LoadDetails(id) => match repo.commit_details(&id) {
            Ok(details) => WorkerMessage::CommitDetailsLoaded(Box::new(details)),
            Err(error) => WorkerMessage::CommitDetailsFailed {
                id,
                message: error.to_string(),
            },
        },
    };

    sender.send(message).is_ok()
}

fn rows_from_commits_with_graph(commits: Vec<CommitInfo>, graph: &mut Graph) -> Vec<CommitRow> {
    commits
        .into_iter()
        .map(|commit| {
            let mut row = commit.to_row(String::new());
            row.graph = graph.next_row(&row.id, &row.parent_ids).text;
            row
        })
        .collect()
}

impl From<WorkerMessage> for Action {
    fn from(message: WorkerMessage) -> Self {
        match message {
            WorkerMessage::CommitBatchLoaded { rows, all_loaded } => {
                Action::CommitBatchLoaded { rows, all_loaded }
            }
            WorkerMessage::CommitDetailsLoaded(details) => Action::CommitDetailsLoaded(details),
            WorkerMessage::CommitDetailsFailed { id, message } => {
                Action::CommitDetailsFailed { id, message }
            }
            WorkerMessage::LoadFailed(message) => Action::LoadFailed(message),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::CommitId;

    fn commit(id: &str, parent_ids: Vec<CommitId>) -> CommitInfo {
        CommitInfo {
            id: CommitId::new(id),
            parent_ids,
            summary: format!("commit {id}"),
            author: "A. User".to_string(),
            time: 0,
            refs: Vec::new(),
        }
    }

    #[test]
    fn rows_from_commits_adds_graph_text_without_losing_commit_fields() {
        let mut graph = Graph::default();
        let rows = rows_from_commits_with_graph(
            vec![
                commit(
                    "1111111111111111111111111111111111111111",
                    vec![CommitId::new("2222222222222222222222222222222222222222")],
                ),
                commit("2222222222222222222222222222222222222222", Vec::new()),
            ],
            &mut graph,
        );

        assert_eq!(rows.len(), 2);
        assert_eq!(
            rows[0].summary,
            "commit 1111111111111111111111111111111111111111"
        );
        assert_eq!(rows[0].parent_ids.len(), 1);
        assert!(rows[0].graph.contains('*'));
        assert_eq!(
            rows[1].id,
            CommitId::new("2222222222222222222222222222222222222222")
        );
    }
}
