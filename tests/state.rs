use reki::model::{CommitDetails, CommitId, CommitRow, CommitSignature, DiffStat, Patch};
use reki::state::{Action, AppState, Effect, LoadStatus, ViewMode};

fn row(id: &str) -> CommitRow {
    row_with_summary(id, format!("commit {id}"))
}

fn row_with_summary(id: &str, summary: impl Into<String>) -> CommitRow {
    CommitRow {
        id: CommitId::new(id),
        parent_ids: Vec::new(),
        graph: String::new(),
        summary: summary.into(),
        author: "A. User".to_string(),
        time: 0,
        refs: Vec::new(),
    }
}

fn enter_search(state: &mut AppState, query: &str) {
    state.apply(Action::BeginSearch);
    for ch in query.chars() {
        state.apply(Action::PushSearchChar(ch));
    }
    state.apply(Action::FinishSearch);
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

    state.apply(Action::CommitDetailsFailed {
        id: CommitId::new("b"),
        message: "missing object".to_string(),
    });
    assert_eq!(state.details_error.as_deref(), Some("missing object"));

    let signature = CommitSignature {
        name: "A. User".to_string(),
        email: "a@example.com".to_string(),
        time: 0,
        offset_seconds: 0,
    };
    let details = CommitDetails {
        row: row("b"),
        author: signature.clone(),
        committer: signature,
        message: "commit b\n\nbody".to_string(),
        diffstat: DiffStat {
            files_changed: 1,
            insertions: 2,
            deletions: 0,
            files: Vec::new(),
        },
        patch: Patch { lines: Vec::new() },
    };
    state.apply(Action::CommitDetailsLoaded(Box::new(details.clone())));
    assert_eq!(state.details, Some(details));
    assert_eq!(state.details_error, None);

    state.apply(Action::CloseInspect);
    assert_eq!(state.view, ViewMode::Log);
    assert_eq!(state.details, None);
    assert_eq!(state.details_error, None);
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

#[test]
fn inspect_cursor_movement_clamps_and_scroll_follows() {
    let mut state = AppState::default();
    state.apply(Action::CommitBatchLoaded {
        rows: vec![row("a")],
        all_loaded: true,
    });

    state.apply(Action::OpenInspect);
    state.apply(Action::SetInspectMetrics {
        line_count: 5,
        visible_height: 3,
    });

    state.apply(Action::MoveInspectDown(99));
    assert_eq!(state.inspect_cursor_y, 4);
    assert_eq!(state.inspect_scroll_y, 2);
    assert_eq!(state.inspect_max_scroll_y, 2);

    state.apply(Action::MoveInspectUp(2));
    assert_eq!(state.inspect_cursor_y, 2);
    assert_eq!(state.inspect_scroll_y, 2);

    state.apply(Action::MoveInspectUp(99));
    assert_eq!(state.inspect_cursor_y, 0);
    assert_eq!(state.inspect_scroll_y, 0);
}

#[test]
fn inspect_page_movement_uses_current_line_model() {
    let mut state = AppState::default();
    state.apply(Action::CommitBatchLoaded {
        rows: vec![row("a")],
        all_loaded: true,
    });

    state.apply(Action::OpenInspect);
    state.apply(Action::SetInspectMetrics {
        line_count: 10,
        visible_height: 4,
    });

    state.apply(Action::MoveInspectDown(4));
    assert_eq!(state.inspect_cursor_y, 4);
    assert_eq!(state.inspect_scroll_y, 1);

    state.apply(Action::MoveInspectDown(4));
    assert_eq!(state.inspect_cursor_y, 8);
    assert_eq!(state.inspect_scroll_y, 5);

    state.apply(Action::MoveInspectUp(4));
    assert_eq!(state.inspect_cursor_y, 4);
    assert_eq!(state.inspect_scroll_y, 4);
}

#[test]
fn inspect_position_resets_with_view_changes() {
    let mut state = AppState::default();
    state.apply(Action::CommitBatchLoaded {
        rows: vec![row("a")],
        all_loaded: true,
    });

    state.apply(Action::OpenInspect);
    state.apply(Action::SetInspectMetrics {
        line_count: 10,
        visible_height: 4,
    });
    state.apply(Action::MoveInspectDown(8));

    state.apply(Action::CloseInspect);
    assert_eq!(state.inspect_cursor_y, 0);
    assert_eq!(state.inspect_scroll_y, 0);
    assert_eq!(state.inspect_max_scroll_y, 0);
    assert_eq!(state.inspect_line_count, 0);
    assert_eq!(state.inspect_visible_height, 0);

    state.apply(Action::OpenInspect);
    assert_eq!(state.inspect_cursor_y, 0);
    assert_eq!(state.inspect_scroll_y, 0);
    assert_eq!(state.inspect_max_scroll_y, 0);
    assert_eq!(state.inspect_line_count, 0);
    assert_eq!(state.inspect_visible_height, 0);
}

#[test]
fn search_selects_next_match_and_cycles_forward() {
    let mut state = AppState::default();
    state.apply(Action::CommitBatchLoaded {
        rows: vec![
            row_with_summary("a", "initial import"),
            row_with_summary("b", "fix parser"),
            row_with_summary("c", "write docs"),
            row_with_summary("d", "fix terminal redraw"),
        ],
        all_loaded: true,
    });

    enter_search(&mut state, "fix");
    assert_eq!(state.selected, 1);

    state.apply(Action::FindNext);
    assert_eq!(state.selected, 3);

    state.apply(Action::FindNext);
    assert_eq!(state.selected, 1);
}

#[test]
fn search_finish_keeps_current_match_before_next_match() {
    let mut state = AppState::default();
    state.apply(Action::CommitBatchLoaded {
        rows: vec![
            row_with_summary("a", "initial import"),
            row_with_summary("b", "feat search highlight"),
            row_with_summary("c", "write docs"),
            row_with_summary("d", "feat inspect colors"),
        ],
        all_loaded: true,
    });
    state.apply(Action::MoveDown(1));

    enter_search(&mut state, "feat");
    assert_eq!(state.selected, 1);

    state.apply(Action::FindNext);
    assert_eq!(state.selected, 3);
}

#[test]
fn search_selects_previous_match_and_keeps_selection_when_missing() {
    let mut state = AppState::default();
    state.apply(Action::CommitBatchLoaded {
        rows: vec![
            row_with_summary("a", "initial import"),
            row_with_summary("b", "fix parser"),
            row_with_summary("c", "write docs"),
            row_with_summary("d", "fix terminal redraw"),
        ],
        all_loaded: true,
    });

    enter_search(&mut state, "fix");
    state.apply(Action::FindPrevious);
    assert_eq!(state.selected, 3);

    enter_search(&mut state, "missing");
    assert_eq!(state.selected, 3);
}
