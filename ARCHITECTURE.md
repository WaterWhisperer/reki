# Architecture

Reki is a read-only, tig-like Git log browser for the terminal. The v1 design favors graph fidelity, responsive navigation, and a dense single-pane log view.

## Target Shape

The binary should stay thin: parse `reki [path]`, create the terminal, start the Git loader, and run the event loop. Most behavior belongs in an unstable internal library so state transitions and Git/query logic can be tested without a terminal.

Planned subsystems:

- `cli`: parse the optional repository path and report startup errors before entering the TUI.
- `state`: hold `AppState`, `Action`, `Effect`, selection, viewport, and active view mode.
- `git`: isolate `gix` repository access and convert backend data into internal commit rows/details.
- `graph`: convert commit parent relationships into renderable lane cells.
- `worker`: load Git data on a background thread and send typed messages to the UI loop.
- `ui`: render the log view and Enter-opened inspect view with Ratatui.

## Data Flow

The UI loop translates terminal input into actions. The reducer updates state and may emit effects such as requesting more commits. Worker messages append commit rows, mark loading complete, or surface errors. Rendering reads state only; it should not perform Git IO.

## V1 Boundaries

V1 shows HEAD history with topo/date ordering, branch/tag/HEAD decorations, and a metadata plus diffstat inspect view. It does not stage files, checkout refs, mutate repositories, run arbitrary revision queries, or render full patches.
