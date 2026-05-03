use reki::model::{CommitId, CommitRow};
use reki::state::{Action, AppState, Effect, LoadStatus, ViewMode};

fn row(id: &str) -> CommitRow {
    CommitRow {
        id: CommitId::new(id),
        parent_ids: Vec::new(),
        graph: String::new(),
        summary: format!("commit {id}"),
        author: "A. User".to_string(),
        time: 0,
        refs: Vec::new(),
    }
}

#[test]
fn start_loading_emits_a_load_effect() {
    let mut state = AppState::default();

    let effects = state.apply(Action::StartLoading);

    assert_eq!(state.load_status, LoadStatus::Loading);
    assert_eq!(effects, vec![Effect::LoadNextBatch]);
}

#[test]
fn commit_batches_update_selection_and_inspect_view() {
    let mut state = AppState::default();

    state.apply(Action::CommitBatchLoaded {
        rows: vec![row("a"), row("b")],
        all_loaded: false,
    });
    assert_eq!(state.load_status, LoadStatus::Loading);

    state.apply(Action::MoveDown(10));
    state.apply(Action::OpenInspect);

    assert_eq!(state.selected, 1);
    assert_eq!(state.view, ViewMode::Inspect(CommitId::new("b")));
}

#[test]
fn reducer_owns_quit_and_scroll_state() {
    let mut state = AppState::default();

    state.apply(Action::SetMaxScrollX(5));
    state.apply(Action::ScrollRight(99));
    state.apply(Action::ScrollLeft(2));
    state.apply(Action::Quit);

    assert_eq!(state.scroll_x, 3);
    assert!(state.should_quit);
}
