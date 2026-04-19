mod app;
mod models;
mod tmux;
mod ui;

use anyhow::{bail, Context, Result};
use app::{App, ConfirmIntent, FocusArea, InputIntent, ModalState};
use crossterm::{
    event::{self, Event, KeyCode, KeyEventKind, KeyModifiers},
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
        execute!(stdout, EnterAlternateScreen)?;
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
        let _ = execute!(self.terminal.backend_mut(), LeaveAlternateScreen);
        let _ = self.terminal.show_cursor();
    }
}

fn run_loop<B: Backend>(terminal: &mut Terminal<B>, app: &mut App) -> Result<()> {
    loop {
        terminal.draw(|frame| ui::draw(frame, app))?;

        if event::poll(Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                if key.kind == KeyEventKind::Press {
                    if app.help.visible {
                        handle_help_mode(app, key.code, key.modifiers);
                    } else if app.filter.active {
                        handle_filter_mode(app, key.code, key.modifiers);
                    } else {
                        match app.modal {
                            ModalState::Input(_) => handle_input_mode(app, key.code, key.modifiers),
                            ModalState::Confirm(_) => {
                                handle_confirm_mode(app, key.code, key.modifiers)
                            }
                            ModalState::None => handle_normal_mode(app, key.code, key.modifiers),
                        }
                    }
                }
            }
        }

        if app.should_quit {
            return Ok(());
        }
    }
}

fn handle_help_mode(app: &mut App, code: KeyCode, modifiers: KeyModifiers) {
    match code {
        KeyCode::Char('c') if modifiers.contains(KeyModifiers::CONTROL) => app.should_quit = true,
        KeyCode::Esc | KeyCode::Char('q') | KeyCode::Char('?') => app.help.visible = false,
        _ => {}
    }
}

fn handle_normal_mode(app: &mut App, code: KeyCode, modifiers: KeyModifiers) {
    match code {
        KeyCode::Char('c') if modifiers.contains(KeyModifiers::CONTROL) => app.should_quit = true,
        KeyCode::Char('q') | KeyCode::Esc => app.should_quit = true,
        KeyCode::Char('?') => app.help.visible = true,
        KeyCode::Char('/') => app.open_filter(),
        KeyCode::Char('r') => {
            app.refresh_all();
            if app.connection == tmux::TmuxConnectionState::Connected {
                app.set_info_banner("Refreshed", "tmux data is up to date.");
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

fn handle_filter_mode(app: &mut App, code: KeyCode, modifiers: KeyModifiers) {
    match code {
        KeyCode::Char('c') if modifiers.contains(KeyModifiers::CONTROL) => app.should_quit = true,
        KeyCode::Char('?') => app.help.visible = true,
        KeyCode::Enter => app.close_filter(),
        KeyCode::Esc => {
            app.clear_filter();
            app.set_info_banner("Filter cleared", "Showing every matching item again.");
        }
        KeyCode::Backspace => app.pop_filter_char(),
        KeyCode::Char('u') if modifiers.contains(KeyModifiers::CONTROL) => app.clear_filter(),
        KeyCode::Char(character) if modifiers.is_empty() || modifiers == KeyModifiers::SHIFT => {
            app.push_filter_char(character);
        }
        _ => {}
    }
}

fn handle_input_mode(app: &mut App, code: KeyCode, modifiers: KeyModifiers) {
    match code {
        KeyCode::Char('c') if modifiers.contains(KeyModifiers::CONTROL) => app.should_quit = true,
        KeyCode::Enter => handle_input_submission(app),
        KeyCode::Esc => {
            app.close_modal();
            app.set_info_banner("Cancelled", "No changes were made.");
        }
        KeyCode::Backspace => {
            if let Some(modal) = app.input_modal_mut() {
                modal.value.pop();
                modal.error = None;
            }
        }
        KeyCode::Char('u') if modifiers.contains(KeyModifiers::CONTROL) => {
            if let Some(modal) = app.input_modal_mut() {
                modal.value.clear();
                modal.error = None;
            }
        }
        KeyCode::Char(character) if modifiers.is_empty() || modifiers == KeyModifiers::SHIFT => {
            if let Some(modal) = app.input_modal_mut() {
                modal.value.push(character);
                modal.error = None;
            }
        }
        _ => {}
    }
}

fn handle_confirm_mode(app: &mut App, code: KeyCode, modifiers: KeyModifiers) {
    match code {
        KeyCode::Char('c') if modifiers.contains(KeyModifiers::CONTROL) => app.should_quit = true,
        KeyCode::Char('y') | KeyCode::Enter => handle_confirmation(app),
        KeyCode::Char('n') | KeyCode::Esc => {
            app.close_modal();
            app.set_info_banner("Cancelled", "No changes were made.");
        }
        _ => {}
    }
}

fn handle_new_action(app: &mut App) {
    match app.focus {
        FocusArea::Sessions => app.open_input_modal(InputIntent::NewSession, ""),
        FocusArea::Windows => {
            if app.get_selected_session().is_some() {
                app.open_input_modal(InputIntent::NewWindow, "");
            } else {
                app.set_info_banner(
                    "Window unavailable",
                    "Select a session before creating a window.",
                );
            }
        }
        FocusArea::Panes => {
            let pane_id = app.get_selected_pane().map(|pane| pane.id.clone());
            if let Some(pane_id) = pane_id {
                match tmux::create_pane(&pane_id) {
                    Ok(()) => {
                        app.refresh_all();
                        app.focus = FocusArea::Panes;
                        app.set_success_banner(
                            "Pane split",
                            "The pane was split. Use Enter to attach or Tab to review the layout.",
                        );
                    }
                    Err(err) => app.set_error_banner(
                        "Could not split pane",
                        format_user_error("Split failed", err),
                    ),
                }
            } else {
                app.set_info_banner("Pane unavailable", "Select a pane before splitting it.");
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
                app.open_input_modal(InputIntent::RenameSession, name);
            } else {
                app.set_info_banner("Rename unavailable", "Select a session before renaming it.");
            }
        }
        FocusArea::Windows => {
            let current_name = app.get_selected_window().map(|window| window.name.clone());
            if let Some(name) = current_name {
                app.open_input_modal(InputIntent::RenameWindow, name);
            } else {
                app.set_info_banner("Rename unavailable", "Select a window before renaming it.");
            }
        }
        FocusArea::Panes => app.set_info_banner(
            "Rename unavailable",
            "Pane names are managed inside tmux itself.",
        ),
    }
}

fn handle_delete_action(app: &mut App) {
    match app.focus {
        FocusArea::Sessions => {
            if app.get_selected_session().is_some() {
                app.open_confirm_modal(ConfirmIntent::Session);
            } else {
                app.set_info_banner("Delete unavailable", "Select a session before deleting it.");
            }
        }
        FocusArea::Windows => {
            if app.get_selected_window().is_some() {
                app.open_confirm_modal(ConfirmIntent::Window);
            } else {
                app.set_info_banner("Delete unavailable", "Select a window before deleting it.");
            }
        }
        FocusArea::Panes => {
            if app.get_selected_pane().is_some() {
                app.open_confirm_modal(ConfirmIntent::Pane);
            } else {
                app.set_info_banner("Delete unavailable", "Select a pane before deleting it.");
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
                app.set_info_banner("Attach unavailable", "Select a session to attach.");
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
                        app.set_error_banner(
                            "Could not select window",
                            format_user_error("Attach failed", err),
                        );
                        return;
                    }

                    app.target_attach = Some(session_name);
                    app.should_quit = true;
                }
                _ => app.set_info_banner("Attach unavailable", "Select a window to attach."),
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
                        app.set_error_banner(
                            "Could not select window",
                            format_user_error("Attach failed", err),
                        );
                        return;
                    }

                    if let Err(err) = tmux::select_pane(&pane_id) {
                        app.set_error_banner(
                            "Could not select pane",
                            format_user_error("Attach failed", err),
                        );
                        return;
                    }

                    app.target_attach = Some(session_name);
                    app.should_quit = true;
                }
                _ => app.set_info_banner("Attach unavailable", "Select a pane to attach."),
            }
        }
    }
}

fn handle_input_submission(app: &mut App) {
    let Some(modal) = app.input_modal().cloned() else {
        return;
    };

    let value = modal.value.trim().to_string();
    if let Some(error) = validate_name(&value) {
        app.set_modal_error(error);
        return;
    }

    match modal.intent {
        InputIntent::NewSession => match tmux::create_session(&value) {
            Ok(()) => {
                app.close_modal();
                app.focus = FocusArea::Sessions;
                app.refresh_all();
                app.select_session_by_name(&value);
                app.set_success_banner(
                    "Session created",
                    format!("`{value}` is ready. Press Enter to attach or Tab to add a window."),
                );
            }
            Err(err) => app.set_modal_error(format_user_error("Could not create session", err)),
        },
        InputIntent::RenameSession => {
            let old_name = app
                .get_selected_session()
                .map(|session| session.name.clone());
            let Some(old_name) = old_name else {
                app.close_modal();
                app.set_warning_banner(
                    "Selection changed",
                    "Select a session and try that rename again.",
                );
                return;
            };

            if old_name == value {
                app.close_modal();
                app.set_info_banner(
                    "Session unchanged",
                    "The selected session already uses that name.",
                );
                return;
            }

            match tmux::rename_session(&old_name, &value) {
                Ok(()) => {
                    app.close_modal();
                    app.focus = FocusArea::Sessions;
                    app.refresh_all();
                    app.select_session_by_name(&value);
                    app.set_success_banner(
                        "Session renamed",
                        format!("The session is now named `{value}`."),
                    );
                }
                Err(err) => app.set_modal_error(format_user_error("Could not rename session", err)),
            }
        }
        InputIntent::NewWindow => {
            let session_id = app.get_selected_session().map(|session| session.id.clone());
            let Some(session_id) = session_id else {
                app.close_modal();
                app.set_warning_banner(
                    "Selection changed",
                    "Select a session and try that window creation again.",
                );
                return;
            };

            match tmux::create_window(&session_id, &value) {
                Ok(()) => {
                    app.close_modal();
                    app.focus = FocusArea::Windows;
                    app.refresh_all();
                    app.select_window_by_name(&value);
                    app.set_success_banner(
                        "Window created",
                        format!("`{value}` is ready. Press Tab to inspect its panes."),
                    );
                }
                Err(err) => app.set_modal_error(format_user_error("Could not create window", err)),
            }
        }
        InputIntent::RenameWindow => {
            let window_id = app.get_selected_window().map(|window| window.id.clone());
            let current_name = app.get_selected_window().map(|window| window.name.clone());
            let (Some(window_id), Some(current_name)) = (window_id, current_name) else {
                app.close_modal();
                app.set_warning_banner(
                    "Selection changed",
                    "Select a window and try that rename again.",
                );
                return;
            };

            if current_name == value {
                app.close_modal();
                app.set_info_banner(
                    "Window unchanged",
                    "The selected window already uses that name.",
                );
                return;
            }

            match tmux::rename_window(&window_id, &value) {
                Ok(()) => {
                    app.close_modal();
                    app.focus = FocusArea::Windows;
                    app.refresh_all();
                    app.select_window_by_name(&value);
                    app.set_success_banner(
                        "Window renamed",
                        format!("The window is now named `{value}`."),
                    );
                }
                Err(err) => app.set_modal_error(format_user_error("Could not rename window", err)),
            }
        }
    }
}

fn handle_confirmation(app: &mut App) {
    let Some(modal) = app.confirm_modal().cloned() else {
        return;
    };

    match modal.intent {
        ConfirmIntent::Session => {
            let session_name = app
                .get_selected_session()
                .map(|session| session.name.clone());
            let Some(session_name) = session_name else {
                app.close_modal();
                app.set_warning_banner(
                    "Selection changed",
                    "Select a session and try that delete again.",
                );
                return;
            };

            match tmux::kill_session(&session_name) {
                Ok(()) => {
                    app.close_modal();
                    app.focus = FocusArea::Sessions;
                    app.refresh_all();
                    let follow_up = if app.sessions.is_empty() {
                        "No sessions remain. Press n to create another."
                    } else {
                        "The next available session is selected."
                    };
                    app.set_success_banner("Session deleted", follow_up);
                }
                Err(err) => app.set_modal_error(format_user_error("Could not delete session", err)),
            }
        }
        ConfirmIntent::Window => {
            let window_id = app.get_selected_window().map(|window| window.id.clone());
            let window_name = app.get_selected_window().map(|window| window.name.clone());
            let (Some(window_id), Some(window_name)) = (window_id, window_name) else {
                app.close_modal();
                app.set_warning_banner(
                    "Selection changed",
                    "Select a window and try that delete again.",
                );
                return;
            };

            match tmux::kill_window(&window_id) {
                Ok(()) => {
                    app.close_modal();
                    app.focus = FocusArea::Windows;
                    app.refresh_all();
                    let follow_up = if app.windows.is_empty() {
                        "That session has no windows left. Press n to create one."
                    } else {
                        "The next available window is selected."
                    };
                    app.set_success_banner(
                        "Window deleted",
                        format!("Deleted `{window_name}`. {follow_up}"),
                    );
                }
                Err(err) => app.set_modal_error(format_user_error("Could not delete window", err)),
            }
        }
        ConfirmIntent::Pane => {
            let pane_id = app.get_selected_pane().map(|pane| pane.id.clone());
            let Some(pane_id) = pane_id else {
                app.close_modal();
                app.set_warning_banner(
                    "Selection changed",
                    "Select a pane and try that delete again.",
                );
                return;
            };

            match tmux::kill_pane(&pane_id) {
                Ok(()) => {
                    app.close_modal();
                    app.focus = FocusArea::Panes;
                    app.refresh_all();
                    let follow_up = if app.panes.is_empty() {
                        "That window has no panes left."
                    } else {
                        "The next available pane is selected."
                    };
                    app.set_success_banner(
                        "Pane deleted",
                        format!("Deleted `{pane_id}`. {follow_up}"),
                    );
                }
                Err(err) => app.set_modal_error(format_user_error("Could not delete pane", err)),
            }
        }
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

fn validate_name(value: &str) -> Option<&'static str> {
    if value.trim().is_empty() {
        Some("Enter a name to continue.")
    } else if value.contains('\n') || value.contains('\r') {
        Some("Names cannot contain line breaks.")
    } else {
        None
    }
}

fn format_user_error(action: &str, err: anyhow::Error) -> String {
    format!("{action}: {err}")
}
