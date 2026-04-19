mod app;
mod models;
mod tmux;
mod ui;

use anyhow::{bail, Context, Result};
use app::{App, AppState, FocusArea};
use crossterm::{
    event::{
        self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEventKind, KeyModifiers,
    },
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, prelude::Backend, Terminal};
use std::{env, io::Stdout, process::Command, time::Duration};

#[cfg(unix)]
use std::os::unix::process::CommandExt;

fn main() -> Result<()> {
    let mut app = App::new();
    let run_result = {
        let mut terminal_session = TerminalSession::enter()?;
        run_loop(terminal_session.terminal(), &mut app)
    };

    run_result?;
    handle_attach(&app)
}

struct TerminalSession {
    terminal: Terminal<CrosstermBackend<Stdout>>,
}

impl TerminalSession {
    fn enter() -> Result<Self> {
        enable_raw_mode()?;
        let mut stdout = std::io::stdout();
        execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
        let backend = CrosstermBackend::new(stdout);
        let terminal = Terminal::new(backend)?;

        Ok(Self { terminal })
    }

    fn terminal(&mut self) -> &mut Terminal<CrosstermBackend<Stdout>> {
        &mut self.terminal
    }
}

impl Drop for TerminalSession {
    fn drop(&mut self) {
        let _ = disable_raw_mode();
        let _ = execute!(
            self.terminal.backend_mut(),
            LeaveAlternateScreen,
            DisableMouseCapture
        );
        let _ = self.terminal.show_cursor();
    }
}

fn run_loop<B: Backend>(terminal: &mut Terminal<B>, app: &mut App) -> Result<()> {
    loop {
        terminal.draw(|frame| ui::draw(frame, app))?;

        if event::poll(Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                if key.kind == KeyEventKind::Press {
                    match app.state {
                        AppState::Normal => handle_normal_mode(app, key.code, key.modifiers),
                        AppState::InputNewSession
                        | AppState::InputRenameSession
                        | AppState::InputNewWindow
                        | AppState::InputRenameWindow => {
                            handle_input_mode(app, key.code, key.modifiers)
                        }
                        AppState::ConfirmDeleteSession
                        | AppState::ConfirmDeleteWindow
                        | AppState::ConfirmDeletePane => handle_confirm_mode(app, key.code),
                    }
                }
            }
        }

        if app.should_quit {
            return Ok(());
        }
    }
}

fn handle_normal_mode(app: &mut App, code: KeyCode, modifiers: KeyModifiers) {
    match code {
        KeyCode::Char('c') if modifiers.contains(KeyModifiers::CONTROL) => app.should_quit = true,
        KeyCode::Char('q') | KeyCode::Esc => app.should_quit = true,
        KeyCode::Char('r') => {
            app.refresh_all();
            if !app.status.is_error() {
                app.set_info("Refreshed tmux data.");
            }
        }
        KeyCode::Down | KeyCode::Char('j') => app.nav_down(),
        KeyCode::Up | KeyCode::Char('k') => app.nav_up(),
        KeyCode::Left | KeyCode::Char('h') | KeyCode::BackTab => app.cycle_focus_back(),
        KeyCode::Right | KeyCode::Char('l') | KeyCode::Tab => app.cycle_focus(),
        KeyCode::Home | KeyCode::Char('g') => app.nav_first(),
        KeyCode::End | KeyCode::Char('G') => app.nav_last(),
        KeyCode::Char('n') => handle_new_action(app),
        KeyCode::Char('R') => handle_rename_action(app),
        KeyCode::Char('d') => handle_delete_action(app),
        KeyCode::Enter => handle_attach_action(app),
        _ => {}
    }
}

fn handle_input_mode(app: &mut App, code: KeyCode, modifiers: KeyModifiers) {
    match code {
        KeyCode::Enter => match handle_input_submission(app) {
            Ok(success_message) => {
                app.state = AppState::Normal;
                app.refresh_all();
                if let Some(message) = success_message {
                    if !app.status.is_error() {
                        app.set_success(message);
                    }
                } else if !app.status.is_error() {
                    app.set_info("Nothing to submit.");
                }
            }
            Err(err) => {
                app.state = AppState::Normal;
                app.set_error(format_user_error("Could not complete that action", err));
            }
        },
        KeyCode::Esc => {
            app.state = AppState::Normal;
            app.input_buffer.clear();
            app.set_info("Input cancelled.");
        }
        KeyCode::Backspace => {
            app.input_buffer.pop();
        }
        KeyCode::Char('u') if modifiers.contains(KeyModifiers::CONTROL) => {
            app.input_buffer.clear();
        }
        KeyCode::Char(character) if modifiers.is_empty() || modifiers == KeyModifiers::SHIFT => {
            app.input_buffer.push(character);
        }
        _ => {}
    }
}

fn handle_confirm_mode(app: &mut App, code: KeyCode) {
    match code {
        KeyCode::Char('y') | KeyCode::Enter => match handle_confirmation(app) {
            Ok(success_message) => {
                app.state = AppState::Normal;
                app.refresh_all();
                if let Some(message) = success_message {
                    if !app.status.is_error() {
                        app.set_success(message);
                    }
                }
            }
            Err(err) => {
                app.state = AppState::Normal;
                app.set_error(format_user_error("Could not complete that action", err));
            }
        },
        KeyCode::Char('n') | KeyCode::Esc => {
            app.state = AppState::Normal;
            app.set_info("Deletion cancelled.");
        }
        _ => {}
    }
}

fn handle_new_action(app: &mut App) {
    match app.focus {
        FocusArea::Sessions => {
            app.state = AppState::InputNewSession;
            app.input_buffer.clear();
        }
        FocusArea::Windows => {
            if app.get_selected_session().is_some() {
                app.state = AppState::InputNewWindow;
                app.input_buffer.clear();
            } else {
                app.set_info("Select a session before creating a window.");
            }
        }
        FocusArea::Panes => {
            let pane_id = app.get_selected_pane().map(|pane| pane.id.clone());
            if let Some(pane_id) = pane_id {
                match tmux::create_pane(&pane_id) {
                    Ok(()) => {
                        app.refresh_all();
                        if !app.status.is_error() {
                            app.set_success("Split the selected pane.");
                        }
                    }
                    Err(err) => {
                        app.set_error(format_user_error("Could not split the selected pane", err));
                    }
                }
            } else {
                app.set_info("Select a pane before splitting.");
            }
        }
    }
}

fn handle_rename_action(app: &mut App) {
    match app.focus {
        FocusArea::Sessions => {
            let current_name = app
                .get_selected_session()
                .map(|session| session.name.clone());
            if let Some(name) = current_name {
                app.state = AppState::InputRenameSession;
                app.input_buffer = name;
            } else {
                app.set_info("Select a session before renaming.");
            }
        }
        FocusArea::Windows => {
            let current_name = app.get_selected_window().map(|window| window.name.clone());
            if let Some(name) = current_name {
                app.state = AppState::InputRenameWindow;
                app.input_buffer = name;
            } else {
                app.set_info("Select a window before renaming.");
            }
        }
        FocusArea::Panes => {
            app.set_info("Pane renaming is handled inside tmux itself.");
        }
    }
}

fn handle_delete_action(app: &mut App) {
    match app.focus {
        FocusArea::Sessions => {
            if app.get_selected_session().is_some() {
                app.state = AppState::ConfirmDeleteSession;
            } else {
                app.set_info("Select a session before deleting.");
            }
        }
        FocusArea::Windows => {
            if app.get_selected_window().is_some() {
                app.state = AppState::ConfirmDeleteWindow;
            } else {
                app.set_info("Select a window before deleting.");
            }
        }
        FocusArea::Panes => {
            if app.get_selected_pane().is_some() {
                app.state = AppState::ConfirmDeletePane;
            } else {
                app.set_info("Select a pane before deleting.");
            }
        }
    }
}

fn handle_attach_action(app: &mut App) {
    match app.focus {
        FocusArea::Sessions => {
            let target = app
                .get_selected_session()
                .map(|session| session.name.clone());
            if let Some(target) = target {
                app.target_attach = Some(target);
                app.should_quit = true;
            } else {
                app.set_info("Select a session before attaching.");
            }
        }
        FocusArea::Windows => {
            let session_name = app
                .get_selected_session()
                .map(|session| session.name.clone());
            let window_id = app.get_selected_window().map(|window| window.id.clone());

            match (session_name, window_id) {
                (Some(session_name), Some(window_id)) => {
                    if let Err(err) = tmux::select_window(&window_id) {
                        app.set_error(format_user_error("Could not select that window", err));
                        return;
                    }

                    app.target_attach = Some(session_name);
                    app.should_quit = true;
                }
                _ => app.set_info("Select a window before attaching."),
            }
        }
        FocusArea::Panes => {
            let session_name = app
                .get_selected_session()
                .map(|session| session.name.clone());
            let window_id = app.get_selected_window().map(|window| window.id.clone());
            let pane_id = app.get_selected_pane().map(|pane| pane.id.clone());

            match (session_name, window_id, pane_id) {
                (Some(session_name), Some(window_id), Some(pane_id)) => {
                    if let Err(err) = tmux::select_window(&window_id) {
                        app.set_error(format_user_error("Could not select that window", err));
                        return;
                    }

                    if let Err(err) = tmux::select_pane(&pane_id) {
                        app.set_error(format_user_error("Could not select that pane", err));
                        return;
                    }

                    app.target_attach = Some(session_name);
                    app.should_quit = true;
                }
                _ => app.set_info("Select a pane before attaching."),
            }
        }
    }
}

fn handle_input_submission(app: &mut App) -> Result<Option<String>> {
    let value = app.input_buffer.trim().to_string();
    app.input_buffer.clear();

    if value.is_empty() {
        return Ok(None);
    }

    match app.state {
        AppState::InputNewSession => {
            tmux::create_session(&value)?;
            Ok(Some(format!("Created session `{value}`.")))
        }
        AppState::InputRenameSession => {
            let old_name = app
                .get_selected_session()
                .map(|session| session.name.clone());
            if let Some(old_name) = old_name {
                if old_name == value {
                    return Ok(Some("Session name was unchanged.".to_string()));
                }
                tmux::rename_session(&old_name, &value)?;
                Ok(Some(format!("Renamed session to `{value}`.")))
            } else {
                Ok(None)
            }
        }
        AppState::InputNewWindow => {
            let session_id = app.get_selected_session().map(|session| session.id.clone());
            if let Some(session_id) = session_id {
                tmux::create_window(&session_id, &value)?;
                Ok(Some(format!("Created window `{value}`.")))
            } else {
                Ok(None)
            }
        }
        AppState::InputRenameWindow => {
            let window_id = app.get_selected_window().map(|window| window.id.clone());
            let current_name = app.get_selected_window().map(|window| window.name.clone());
            match (window_id, current_name) {
                (Some(window_id), Some(current_name)) => {
                    if current_name == value {
                        return Ok(Some("Window name was unchanged.".to_string()));
                    }
                    tmux::rename_window(&window_id, &value)?;
                    Ok(Some(format!("Renamed window to `{value}`.")))
                }
                _ => Ok(None),
            }
        }
        _ => Ok(None),
    }
}

fn handle_confirmation(app: &mut App) -> Result<Option<String>> {
    match app.state {
        AppState::ConfirmDeleteSession => {
            let session_name = app
                .get_selected_session()
                .map(|session| session.name.clone());
            if let Some(session_name) = session_name {
                tmux::kill_session(&session_name)?;
                Ok(Some(format!("Deleted session `{session_name}`.")))
            } else {
                Ok(None)
            }
        }
        AppState::ConfirmDeleteWindow => {
            let window_id = app.get_selected_window().map(|window| window.id.clone());
            let window_name = app.get_selected_window().map(|window| window.name.clone());
            match (window_id, window_name) {
                (Some(window_id), Some(window_name)) => {
                    tmux::kill_window(&window_id)?;
                    Ok(Some(format!("Deleted window `{window_name}`.")))
                }
                _ => Ok(None),
            }
        }
        AppState::ConfirmDeletePane => {
            let pane_id = app.get_selected_pane().map(|pane| pane.id.clone());
            if let Some(pane_id) = pane_id {
                tmux::kill_pane(&pane_id)?;
                Ok(Some(format!("Deleted pane `{pane_id}`.")))
            } else {
                Ok(None)
            }
        }
        _ => Ok(None),
    }
}

fn handle_attach(app: &App) -> Result<()> {
    let Some(target) = app.target_attach.as_deref() else {
        return Ok(());
    };

    if env::var("TMUX").is_ok() {
        let status = Command::new("tmux")
            .args(["switch-client", "-t", target])
            .status()
            .with_context(|| format!("could not switch to tmux session `{target}`"))?;

        if !status.success() {
            bail!("tmux switch-client exited with status {status}");
        }

        return Ok(());
    }

    #[cfg(unix)]
    {
        let err = Command::new("tmux").args(["attach", "-t", target]).exec();
        Err(err).with_context(|| format!("could not attach to tmux session `{target}`"))
    }

    #[cfg(not(unix))]
    {
        let status = Command::new("tmux")
            .args(["attach", "-t", target])
            .status()
            .with_context(|| format!("could not attach to tmux session `{target}`"))?;

        if !status.success() {
            bail!("tmux attach exited with status {status}");
        }

        Ok(())
    }
}

fn format_user_error(action: &str, err: anyhow::Error) -> String {
    format!("{action}: {}", err)
}
