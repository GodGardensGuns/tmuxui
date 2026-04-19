use crate::app::{App, AppState, FocusArea, StatusLevel};
use ratatui::{prelude::*, widgets::*};

pub fn draw(frame: &mut Frame, app: &mut App) {
    let base_style = Style::default().fg(Color::Reset).bg(Color::Reset);
    let highlight_style = Style::default()
        .bg(Color::DarkGray)
        .add_modifier(Modifier::BOLD);
    let border_active = Style::default().fg(Color::Cyan);
    let border_inactive = Style::default().fg(Color::DarkGray);

    frame.render_widget(Block::default().style(base_style), frame.size());

    let layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),
            Constraint::Min(0),
            Constraint::Length(2),
        ])
        .split(frame.size());

    render_header(frame, layout[0], app);

    let columns = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(30),
            Constraint::Percentage(35),
            Constraint::Percentage(35),
        ])
        .split(layout[1]);

    let border_for = |focus: FocusArea| {
        if app.focus == focus {
            border_active
        } else {
            border_inactive
        }
    };

    let sessions = if app.sessions.is_empty() {
        vec![empty_item("No tmux sessions. Press n to create one.")]
    } else {
        app.sessions
            .iter()
            .map(|session| {
                ListItem::new(Line::from(vec![
                    Span::styled(
                        session.name.as_str(),
                        Style::default().add_modifier(Modifier::BOLD),
                    ),
                    Span::raw(format!(" ({} windows)", session.window_count)),
                    Span::styled(
                        format!(" [{}]", session.created),
                        Style::default().fg(Color::DarkGray),
                    ),
                ]))
            })
            .collect()
    };

    frame.render_stateful_widget(
        List::new(sessions)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(" Sessions ")
                    .border_style(border_for(FocusArea::Sessions)),
            )
            .highlight_style(highlight_style),
        columns[0],
        &mut app.session_list_state,
    );

    let windows = if app.windows.is_empty() {
        vec![empty_item(window_empty_state(app))]
    } else {
        app.windows
            .iter()
            .map(|window| {
                let active_indicator = if window.active { "*" } else { " " };
                ListItem::new(Line::from(vec![
                    Span::styled(
                        format!("{active_indicator} {}", window.name),
                        Style::default().add_modifier(Modifier::BOLD),
                    ),
                    Span::styled(
                        format!(" [{}]", window.layout),
                        Style::default().fg(Color::DarkGray),
                    ),
                ]))
            })
            .collect()
    };

    frame.render_stateful_widget(
        List::new(windows)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(" Windows ")
                    .border_style(border_for(FocusArea::Windows)),
            )
            .highlight_style(highlight_style),
        columns[1],
        &mut app.window_list_state,
    );

    let panes = if app.panes.is_empty() {
        vec![empty_item(pane_empty_state(app))]
    } else {
        app.panes
            .iter()
            .map(|pane| {
                let active_indicator = if pane.active { "*" } else { " " };
                ListItem::new(vec![
                    Line::from(vec![
                        Span::styled(
                            format!("{active_indicator} {}", pane.id),
                            Style::default().add_modifier(Modifier::BOLD),
                        ),
                        Span::styled(
                            format!(" {}x{}", pane.width, pane.height),
                            Style::default().fg(Color::DarkGray),
                        ),
                    ]),
                    Line::from(vec![
                        Span::styled("Cmd ", Style::default().fg(Color::Magenta)),
                        Span::raw(pane.current_command.as_str()),
                    ]),
                    Line::from(vec![
                        Span::styled("Dir ", Style::default().fg(Color::DarkGray)),
                        Span::raw(pane.current_path.as_str()),
                    ]),
                    Line::raw(""),
                ])
            })
            .collect()
    };

    frame.render_stateful_widget(
        List::new(panes)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(" Panes ")
                    .border_style(border_for(FocusArea::Panes)),
            )
            .highlight_style(highlight_style),
        columns[2],
        &mut app.pane_list_state,
    );

    let footer = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(1), Constraint::Length(1)])
        .split(layout[2]);

    frame.render_widget(
        Paragraph::new(get_footer_text(app)).style(Style::default().fg(Color::DarkGray)),
        footer[0],
    );
    frame.render_widget(
        Paragraph::new(app.status.text.as_str()).style(status_style(&app.status.level)),
        footer[1],
    );

    match app.state {
        AppState::InputNewSession => render_input(
            frame,
            app,
            "New Session",
            "Enter a name for the new tmux session.",
        ),
        AppState::InputRenameSession => render_input(
            frame,
            app,
            "Rename Session",
            "Update the selected session name.",
        ),
        AppState::InputNewWindow => {
            render_input(frame, app, "New Window", "Enter a name for the new window.")
        }
        AppState::InputRenameWindow => render_input(
            frame,
            app,
            "Rename Window",
            "Update the selected window name.",
        ),
        AppState::ConfirmDeleteSession => render_confirm(
            frame,
            "Delete Session",
            app.selected_session_name().unwrap_or("No session selected"),
            "This closes every window and pane in the session.",
        ),
        AppState::ConfirmDeleteWindow => render_confirm(
            frame,
            "Delete Window",
            app.selected_window_name().unwrap_or("No window selected"),
            "This permanently removes the selected window.",
        ),
        AppState::ConfirmDeletePane => render_confirm(
            frame,
            "Delete Pane",
            app.selected_pane_id().unwrap_or("No pane selected"),
            "This permanently removes the selected pane.",
        ),
        AppState::Normal => {}
    }
}

fn render_header(frame: &mut Frame, area: Rect, app: &App) {
    let title = Line::from(vec![
        Span::styled(
            " TMUXUI ",
            Style::default()
                .bg(Color::Blue)
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw(format!(
            "  sessions: {}  windows: {}  panes: {}",
            app.sessions.len(),
            app.windows.len(),
            app.panes.len()
        )),
    ]);

    frame.render_widget(Paragraph::new(title), area);
}

fn get_footer_text(app: &App) -> String {
    match app.state {
        AppState::Normal => {
            let common = "Nav: arrows/hjkl | Tab/Shift+Tab | g/G | r refresh | q quit";
            match app.focus {
                FocusArea::Sessions => {
                    format!("{common} | Enter attach | n new | R rename | d delete")
                }
                FocusArea::Windows => {
                    format!("{common} | Enter attach | n new | R rename | d delete")
                }
                FocusArea::Panes => format!("{common} | Enter attach | n split | d delete"),
            }
        }
        AppState::InputNewSession
        | AppState::InputRenameSession
        | AppState::InputNewWindow
        | AppState::InputRenameWindow => "Enter save | Esc cancel | Ctrl+U clear".to_string(),
        AppState::ConfirmDeleteSession
        | AppState::ConfirmDeleteWindow
        | AppState::ConfirmDeletePane => "Enter/y confirm | Esc/n cancel".to_string(),
    }
}

fn render_input(frame: &mut Frame, app: &App, title: &str, prompt: &str) {
    let area = centered_rect(60, 20, frame.size());
    frame.render_widget(Clear, area);

    let block = Block::default()
        .title(format!(" {title} "))
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Yellow));
    let inner = block.inner(area);

    frame.render_widget(block, area);
    frame.render_widget(
        Paragraph::new(vec![
            Line::styled(prompt, Style::default().fg(Color::DarkGray)),
            Line::raw(""),
            Line::raw(app.input_buffer.as_str()),
        ]),
        inner,
    );
}

fn render_confirm(frame: &mut Frame, title: &str, target: &str, detail: &str) {
    let area = centered_rect(58, 22, frame.size());
    frame.render_widget(Clear, area);

    let block = Block::default()
        .title(format!(" {title} "))
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Red));
    let inner = block.inner(area);

    frame.render_widget(block, area);
    frame.render_widget(
        Paragraph::new(vec![
            Line::styled(target, Style::default().add_modifier(Modifier::BOLD)),
            Line::raw(""),
            Line::styled(detail, Style::default().fg(Color::DarkGray)),
            Line::raw(""),
            Line::raw("Confirm with Enter or y."),
        ])
        .alignment(Alignment::Center),
        inner,
    );
}

fn empty_item(message: &str) -> ListItem<'static> {
    ListItem::new(Line::styled(
        message.to_string(),
        Style::default().fg(Color::DarkGray),
    ))
}

fn window_empty_state(app: &App) -> &'static str {
    if app.get_selected_session().is_some() {
        "No windows in this session. Press n to create one."
    } else {
        "Select a session to view its windows."
    }
}

fn pane_empty_state(app: &App) -> &'static str {
    if app.get_selected_window().is_some() {
        "No panes found for this window."
    } else {
        "Select a window to view its panes."
    }
}

fn status_style(level: &StatusLevel) -> Style {
    match level {
        StatusLevel::Info => Style::default().fg(Color::DarkGray),
        StatusLevel::Success => Style::default().fg(Color::Green),
        StatusLevel::Error => Style::default().fg(Color::Red),
    }
}

fn centered_rect(percent_x: u16, percent_y: u16, area: Rect) -> Rect {
    let vertical = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(area);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(vertical[1])[1]
}
