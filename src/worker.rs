use std::{
    sync::mpsc::{self, Receiver, Sender},
    thread,
};

use anyhow::Result;

use crate::git::{CommitInfo, Repo};
use crate::graph::Graph;
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

pub(crate) fn spawn_loader(repo: Repo) -> Receiver<WorkerMessage> {
    let (sender, receiver) = mpsc::channel();

    thread::spawn(move || {
        if let Err(error) = load_commits(repo, &sender) {
            let _ = sender.send(WorkerMessage::LoadFailed(error.to_string()));
        }
    });

    receiver
}

fn load_commits(mut repo: Repo, sender: &Sender<WorkerMessage>) -> Result<()> {
    let mut graph = Graph::default();

    loop {
        let commits = repo.load_commits()?;
        let all_loaded = commits.is_empty();
        let rows = rows_from_commits_with_graph(commits, &mut graph);

        if sender
            .send(WorkerMessage::CommitBatchLoaded { rows, all_loaded })
            .is_err()
        {
            break;
        }

        if all_loaded {
            break;
        }
    }

    Ok(())
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
