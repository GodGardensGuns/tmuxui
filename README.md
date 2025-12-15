# TMUX Manager (tmuxui)

A terminal-based user interface (TUI) for managing tmux sessions, windows, and panes. This tool provides an intuitive, visual way to interact with your tmux environment without having to remember complex tmux commands.

## Overview

TMUX Manager is a Rust-based TUI application that allows you to:
- Browse and manage tmux sessions, windows, and panes
- Create, rename, and delete sessions, windows, and panes
- Navigate between different tmux environments
- Split panes and manage window layouts
- Attach to sessions and windows directly from the interface

## Screenshots

The interface is organized into three columns:
- **Sessions**: List all active tmux sessions with window count and creation date
- **Windows**: Display windows in the selected session with layout information
- **Panes**: Show detailed pane information including current command and working directory

## Requirements

- **Rust**: Version 1.70.0 or later
- **tmux**: Must be installed and available in your PATH
- **Terminal**: Any terminal that supports basic ANSI escape codes

## Installation

### From Source

1. Clone the repository:
```bash
git clone <repository-url>
cd tmuxui
```

2. Build the project:
```bash
cargo build --release
```

3. The binary will be available at `target/release/tmuxui`

### Using Cargo (when published)

```bash
cargo install tmuxui
```

## Usage

### Running the Application

```bash
# If installed via cargo
tmuxui

# Or run directly from the build directory
./target/release/tmuxui
```

### Controls and Navigation

#### Basic Navigation
- **Arrow Keys / hjkl**: Navigate up and down within lists
- **Tab**: Cycle focus between Sessions → Windows → Panes
- **Shift+Tab**: Cycle focus backwards (Panes → Windows → Sessions)
- **r**: Refresh all data
- **q**: Quit the application

#### Session Management
- **Enter**: Attach to selected session
- **n**: Create new session (prompts for name)
- **R**: Rename selected session
- **d**: Delete selected session (with confirmation)

#### Window Management
- **Enter**: Attach to selected window
- **n**: Create new window in selected session (prompts for name)
- **R**: Rename selected window
- **d**: Delete selected window (with confirmation)

#### Pane Management
- **n**: Split selected pane (creates a new pane)
- **d**: Kill selected pane (with confirmation)

#### Input Mode
When entering names for new sessions/windows or renaming existing ones:
- **Enter**: Confirm and submit
- **Esc**: Cancel input
- **Backspace**: Delete last character
- Type any characters to input the name

#### Confirmation Mode
When deleting items:
- **y** or **Enter**: Confirm deletion
- **n** or **Esc**: Cancel deletion

## Features

### Session Management
- View all active tmux sessions
- See session details (window count, creation date)
- Create new sessions
- Rename existing sessions
- Delete sessions (with safety confirmation)
- Attach directly to sessions

### Window Management
- Browse windows within selected sessions
- Display window layout information
- Create new windows
- Rename windows
- Delete windows
- Attach directly to specific windows

### Pane Management
- Detailed pane information display:
  - Pane ID
  - Current running command
  - Working directory
  - Dimensions (width × height)
  - Active status indicator
- Split panes to create new divisions
- Delete individual panes

### User Interface
- Clean, three-column layout
- Visual indicators for active items
- Context-sensitive help in footer
- Modal dialogs for input and confirmations
- Keyboard-driven interaction
- Responsive highlighting and focus management

## Development

### Project Structure

```
src/
├── main.rs      # Application entry point and main event loop
├── app.rs       # Application state and logic
├── ui.rs        # User interface rendering and widgets
├── tmux.rs      # tmux command interface and data parsing
└── models.rs    # Data structures for sessions, windows, and panes
```

### Dependencies

- **ratatui**: Terminal UI framework for Rust
- **crossterm**: Cross-platform terminal handling
- **anyhow**: Error handling

### Building from Source

1. Ensure you have Rust installed (1.70.0+)
2. Clone the repository
3. Run `cargo build --release`
4. The binary will be in `target/release/`

### Running in Development Mode

```bash
cargo run
```

### Testing

```bash
# Run all tests
cargo test

# Run with output
cargo test -- --nocapture
```

## Contributing

We welcome contributions! Here's how to get started:

### Setting Up Development Environment

1. Fork the repository
2. Clone your fork locally
3. Create a new branch for your feature or bugfix:
```bash
git checkout -b feature-name
```

### Development Workflow

1. Make your changes following the existing code style
2. Test your changes thoroughly:
```bash
cargo test
cargo clippy -- -D warnings
```
3. Ensure your code compiles without warnings
4. Update documentation if needed

### Code Style

- Follow Rust's standard formatting (`cargo fmt`)
- Use meaningful variable and function names
- Add comments for complex logic
- Keep functions focused and small
- Handle errors appropriately using `Result` types

### Submitting Changes

1. Commit your changes with clear, descriptive messages
2. Push to your fork
3. Create a pull request with:
   - Clear description of changes
   - Testing you've performed
   - Any relevant context

### Bug Reports

When reporting bugs, please include:
- Operating system and terminal
- tmux version
- Rust version (if applicable)
- Steps to reproduce the issue
- Expected vs actual behavior
- Any error messages

### Feature Requests

We're open to new feature ideas! Please:
- Check existing issues for duplicates
- Provide clear use cases
- Consider implementation complexity
- Suggest UI/UX approaches if applicable

## Troubleshooting

### Common Issues

**Application doesn't start:**
- Ensure tmux is installed and in your PATH
- Check that Rust is properly installed
- Verify your terminal supports ANSI escape codes

**No sessions显示:**
- Make sure you have active tmux sessions
- Try running `tmux list-sessions` to verify tmux is working
- Check if tmux server is running

**Keyboard issues:**
- Some terminals may have different key bindings
- Try alternative keys (hjkl instead of arrow keys)
- Check your terminal's keyboard settings

**Performance issues:**
- Large numbers of sessions/windows may slow down initial loading
- Consider using `r` to refresh if data appears stale

### Debug Mode

To run with additional debugging information:

```bash
RUST_LOG=debug cargo run
```

## License

This project is licensed under the GNU General Public License v3.0 - see the [LICENSE.md](LICENSE.md) file for details.

## Changelog

### Version 0.3.0
- Initial tmux TUI implementation
- Basic session, window, and pane management
- Keyboard navigation and modal dialogs
- Support for creating, renaming, and deleting tmux objects

## Acknowledgments

- Built with [ratatui](https://github.com/ratatui-org/ratatui) for terminal UI
- Uses [crossterm](https://github.com/crossterm-rs/crossterm) for terminal handling
- Inspired by the need for better tmux session management tools

## Support

For support, please:
1. Check this README and troubleshooting section
2. Search existing GitHub issues
3. Create a new issue with detailed information
4. Join our community discussions (if available)

---

**Note**: This tool interfaces directly with tmux and modifies your tmux sessions. Always ensure you have important work saved before making changes, especially when deleting sessions or windows.