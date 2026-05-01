# Roadmap

## V1: Tig-like Fast Log

- Rebuild as `src/lib.rs` plus a thin `src/main.rs`.
- Switch Git reads from `git2` to `gix` behind an internal repository layer.
- Add reducer-driven state for selection, scrolling, loading, and view transitions.
- Move commit loading to a worker thread with typed messages.
- Render a dense single-pane log with graph, hash, date, author, refs, and summary.
- Add Enter-opened commit inspect view with metadata and diffstat.

## Verification Goals

- Unit-test graph behavior with small DAG fixtures.
- Unit-test reducer behavior without terminal or Git IO.
- Integration-test repository loading with temporary Git repositories.
- Snapshot stable row/inspect rendering where textual output should not drift.
- Keep CI on `cargo fmt -- --check`, `cargo clippy -- -D warnings`, and `cargo test`.

## Later

- Search with `/`, `n`, and `N`.
- Full patch view with large-file and binary-file safeguards.
- Optional revision/pathspec arguments.
- Broader platform validation beyond Unix-first development.
