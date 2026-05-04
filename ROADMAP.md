# Roadmap

## V1: Tig-like Fast Log

- Rebuild as `src/lib.rs` plus a thin `src/main.rs`.
- Switch Git reads from `git2` to `gix` behind an internal repository layer.
- Add reducer-driven state for selection, scrolling, loading, and view transitions.
- Move commit loading to a worker thread with typed messages.
- Render a dense single-pane log with graph, hash, date, author, refs, and summary.
- Add Enter-opened commit inspect view with metadata and diffstat.
- Search loaded commits with `/`, `n`, and `N`.

## Verification Goals

- Unit-test graph behavior with small DAG fixtures.
- Unit-test reducer behavior without terminal or Git IO.
- Integration-test repository loading with temporary Git repositories.
- Snapshot stable row/inspect rendering where textual output should not drift.
- Keep CI on `cargo fmt -- --check`, `cargo clippy -- -D warnings`, and `cargo test`.

## Later

- Search refinements such as match highlighting and path-aware/global search.
- In-app help view opened with `h`.
- Full patch view with large-file and binary-file safeguards.
- Optional revision/pathspec arguments.
- Broader platform validation beyond Unix-first development.
- Structured diagnostics with `tracing` if worker/UI interactions become hard to debug.
- Release automation around tags and `git-cliff` once versioned releases begin.
