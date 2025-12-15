mod app;
mod ui;
mod tmux;
mod models;

use anyhow::Result;
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEventKind},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::prelude::*;
use std::{
    env,
    os::unix::process::CommandExt,
    process::Command,
    time::Duration,
};

use app::{App, AppState, FocusArea};

fn main() -> Result<()> {
    enable_raw_mode()?;
    let mut stdout = std::io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut app = App::new();
    let res = run_loop(&mut terminal, &mut app);

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen, DisableMouseCapture)?;
    terminal.show_cursor()?;

    if let Err(err) = res {
        println!("Error: {:?}", err);
    }

    if let Some(target) = app.target_attach {
        let in_tmux = env::var("TMUX").is_ok();
        if in_tmux {
            Command::new("tmux").args(["switch-client", "-t", &target]).spawn()?.wait()?;
        } else {
            let _ = Command::new("tmux").args(["attach", "-t", &target]).exec();
        }
    }
    Ok(())
}

fn run_loop<B: Backend>(terminal: &mut Terminal<B>, app: &mut App) -> Result<()> {
    loop {
        terminal.draw(|f| ui::draw(f, app))?;

        if event::poll(Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                if key.kind == KeyEventKind::Press {
                    match app.state {
                        // --- NORMAL MODE ---
                        AppState::Normal => match key.code {
                            KeyCode::Char('q') => app.should_quit = true,
                            KeyCode::Char('r') => app.refresh_all(),
                            // Navigation
                            KeyCode::Down | KeyCode::Char('j') => app.nav_down(),
                            KeyCode::Up | KeyCode::Char('k') => app.nav_up(),
                            KeyCode::Right | KeyCode::Tab => app.cycle_focus(),
                            KeyCode::Left | KeyCode::BackTab => app.cycle_focus_back(),
                            
                            // Context Actions: New (n)
                            KeyCode::Char('n') => match app.focus {
                                FocusArea::Sessions => {
                                    app.state = AppState::InputNewSession;
                                    app.input_buffer.clear();
                                },
                                FocusArea::Windows => {
                                    if app.get_selected_session().is_some() {
                                        app.state = AppState::InputNewWindow;
                                        app.input_buffer.clear();
                                    }
                                },
                                FocusArea::Panes => {
                                    let win_id = app.get_selected_window().map(|w| w.id.clone());
                                    if let Some(id) = win_id {
                                        tmux::create_pane(&id);
                                        app.refresh_all();
                                    }
                                }
                            },

                            // Context Actions: Rename (R - Shift+r)
                            KeyCode::Char('R') => match app.focus {
                                FocusArea::Sessions => {
                                    let current_name = app.get_selected_session().map(|s| s.name.clone());
                                    if let Some(name) = current_name {
                                        app.state = AppState::InputRenameSession;
                                        app.input_buffer = name;
                                    }
                                },
                                FocusArea::Windows => {
                                    let current_name = app.get_selected_window().map(|w| w.name.clone());
                                    if let Some(name) = current_name {
                                        app.state = AppState::InputRenameWindow;
                                        app.input_buffer = name;
                                    }
                                },
                                _ => {}
                            },

                            // Context Actions: Delete (d)
                            KeyCode::Char('d') => match app.focus {
                                FocusArea::Sessions => {
                                    if app.get_selected_session().is_some() {
                                        app.state = AppState::ConfirmDeleteSession;
                                    }
                                },
                                FocusArea::Windows => {
                                    if app.get_selected_window().is_some() {
                                        app.state = AppState::ConfirmDeleteWindow;
                                    }
                                },
                                FocusArea::Panes => {
                                    if app.get_selected_pane().is_some() {
                                        app.state = AppState::ConfirmDeletePane;
                                    }
                                }
                            },

                            // Attach (Enter)
                            KeyCode::Enter => {
                                match app.focus {
                                    FocusArea::Sessions => {
                                        // Attach to Session
                                        let target = app.get_selected_session().map(|s| s.name.clone());
                                        if let Some(t) = target {
                                            app.target_attach = Some(t);
                                            app.should_quit = true;
                                        }
                                    },
                                    FocusArea::Windows => {
                                        // Attach to specific Window (Format: "session:window_id")
                                        let sess_name = app.get_selected_session().map(|s| s.name.clone());
                                        let win_id = app.get_selected_window().map(|w| w.id.clone());
                                        
                                        if let (Some(s), Some(w)) = (sess_name, win_id) {
                                            app.target_attach = Some(format!("{}:{}", s, w));
                                            app.should_quit = true;
                                        }
                                    },
                                    _ => {}
                                }
                            }
                            _ => {}
                        },

                        // --- INPUT MODES ---
                        AppState::InputNewSession | AppState::InputRenameSession | 
                        AppState::InputNewWindow | AppState::InputRenameWindow => {
                            match key.code {
                                KeyCode::Enter => {
                                    handle_input_submission(app);
                                    app.state = AppState::Normal;
                                    app.refresh_all();
                                },
                                KeyCode::Esc => app.state = AppState::Normal,
                                KeyCode::Char(c) => app.input_buffer.push(c),
                                KeyCode::Backspace => { app.input_buffer.pop(); },
                                _ => {}
                            }
                        },

                        // --- CONFIRMATION MODES ---
                        AppState::ConfirmDeleteSession | AppState::ConfirmDeleteWindow | AppState::ConfirmDeletePane => {
                            match key.code {
                                KeyCode::Char('y') | KeyCode::Enter => {
                                    handle_confirmation(app);
                                    app.state = AppState::Normal;
                                    app.refresh_all();
                                },
                                KeyCode::Char('n') | KeyCode::Esc => app.state = AppState::Normal,
                                _ => {}
                            }
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

fn handle_input_submission(app: &mut App) {
    if app.input_buffer.trim().is_empty() { return; }
    let val = app.input_buffer.trim().to_string();

    match app.state {
        AppState::InputNewSession => tmux::create_session(&val),
        AppState::InputRenameSession => {
            let old_name = app.get_selected_session().map(|s| s.name.clone());
            if let Some(old) = old_name {
                tmux::rename_session(&old, &val);
            }
        },
        AppState::InputNewWindow => {
            let sess_id = app.get_selected_session().map(|s| s.id.clone());
            if let Some(id) = sess_id {
                tmux::create_window(&id, &val);
            }
        },
        AppState::InputRenameWindow => {
            let win_id = app.get_selected_window().map(|w| w.id.clone());
            if let Some(id) = win_id {
                tmux::rename_window(&id, &val);
            }
        },
        _ => {}
    }
}

fn handle_confirmation(app: &mut App) {
    match app.state {
        AppState::ConfirmDeleteSession => {
            let sess_name = app.get_selected_session().map(|s| s.name.clone());
            if let Some(name) = sess_name {
                tmux::kill_session(&name);
            }
        },
        AppState::ConfirmDeleteWindow => {
            let win_id = app.get_selected_window().map(|w| w.id.clone());
            if let Some(id) = win_id {
                tmux::kill_window(&id);
            }
        },
        AppState::ConfirmDeletePane => {
            let pane_id = app.get_selected_pane().map(|p| p.id.clone());
            if let Some(id) = pane_id {
                tmux::kill_pane(&id);
            }
        },
        _ => {}
    }
}