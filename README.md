# tmuxui

`tmuxui` is a terminal UI for browsing and managing tmux sessions, windows, and panes without memorizing tmux subcommands.

## What It Does

- Lists sessions, windows, and panes in a three-column TUI
- Creates, renames, and deletes sessions and windows
- Splits and deletes panes
- Attaches directly to the selected session, window, or pane
- Shows clearer status messages when tmux is unavailable or a command fails
- Keeps selections stable across refreshes

## Requirements

- Rust 1.70 or newer
- `tmux` installed and available on `PATH`
- A terminal with ANSI support

## Run It

```bash
cargo run --release
```

Or build the binary:

```bash
cargo build --release
./target/release/tmuxui
```

If no tmux server is running yet, the app opens with an empty state and lets you create the first session from the UI.

## Controls

### Navigation

- `Up` / `Down` / `j` / `k`: move within the focused list
- `Left` / `Right` / `h` / `l`: move focus between Sessions, Windows, and Panes
- `Tab` / `Shift+Tab`: move focus forward or backward
- `g` / `G`: jump to the first or last item in the focused list
- `r`: refresh tmux data
- `q` or `Esc`: quit
- `Ctrl+C`: quit immediately

### Actions

- `Enter`: attach to the selected session, window, or pane
- `n`: create a new session or window, or split the selected pane
- `R`: rename the selected session or window
- `d`: delete the selected session, window, or pane

### Dialogs

- `Enter`: confirm
- `Esc`: cancel
- `Ctrl+U`: clear the input field

## Project Layout

```text
src/
├── app.rs     # application state, selection logic, and status messages
├── main.rs    # terminal lifecycle and keyboard event handling
├── models.rs  # shared data structures
├── tmux.rs    # tmux command execution and parsing
└── ui.rs      # ratatui rendering
```

## Development

Format, lint, and test locally:

```bash
cargo fmt --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-targets --all-features
```

## Notes

- This app talks directly to your tmux server, so delete actions are real.
- Session and pane state are refreshed from tmux after every mutating action.
- Parsing uses a control-character separator instead of `|`, which avoids breaking on common names and paths.

## License

Licensed under GPL v3. See [LICENSE.md](LICENSE.md).
