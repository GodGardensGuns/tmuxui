use ratatui::{prelude::*, widgets::*};
use crate::app::{App, AppState, FocusArea};

pub fn draw(f: &mut Frame, app: &mut App) {
    let base_style = Style::default().fg(Color::Reset).bg(Color::Reset);
    let highlight_style = Style::default().add_modifier(Modifier::REVERSED);
    let border_active = Style::default().fg(Color::Cyan);
    let border_inactive = Style::default().fg(Color::DarkGray);

    // Background
    f.render_widget(Block::default().style(base_style), f.size());

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(1), Constraint::Min(0), Constraint::Length(1)])
        .split(f.size());

    // Header
    f.render_widget(
        Paragraph::new(Line::from(vec![Span::styled(" TMUX MANAGER ", Style::default().bg(Color::Blue).fg(Color::White).add_modifier(Modifier::BOLD))]))
        .style(base_style), 
        chunks[0]
    );

    // Columns
    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(30), Constraint::Percentage(35), Constraint::Percentage(35)])
        .split(chunks[1]);

    let get_border = |focus: FocusArea| if app.focus == focus { border_active } else { border_inactive };

    // 1. Sessions
    let sessions: Vec<ListItem> = app.sessions.iter().map(|s| {
        ListItem::new(Line::from(vec![
            Span::styled(format!("{} {}", "::", s.name), Style::default().add_modifier(Modifier::BOLD)),
            Span::raw(format!(" ({}) ", s.count)),
            Span::styled(format!("[{}]", s.created), Style::default().fg(Color::DarkGray)),
        ]))
    }).collect();
    f.render_stateful_widget(
        List::new(sessions).block(Block::default().borders(Borders::ALL).title(" Sessions ").border_style(get_border(FocusArea::Sessions))).highlight_style(highlight_style),
        cols[0], &mut app.session_list_state
    );

    // 2. Windows
    let windows: Vec<ListItem> = app.windows.iter().map(|w| {
        // Using a safe simple indicator instead of complex unicode for broad compatibility
        let active_indicator = if w.active { "*" } else { " " };
        ListItem::new(Line::from(format!("{} {}: {} [{}]", active_indicator, w.id, w.name, w.layout)))
    }).collect();
    f.render_stateful_widget(
        List::new(windows).block(Block::default().borders(Borders::ALL).title(" Windows ").border_style(get_border(FocusArea::Windows))).highlight_style(highlight_style),
        cols[1], &mut app.window_list_state
    );

    // 3. Panes
    let panes: Vec<ListItem> = app.panes.iter().map(|p| {
        let active_indicator = if p.active { "*" } else { " " };
        let content = vec![
            Line::from(format!("{} ID: {}", active_indicator, p.id)),
            Line::from(format!("   Cmd: {}", p.current_command)).style(Style::default().fg(Color::Magenta)),
            Line::from(format!("   Path: {}", p.current_path)).style(Style::default().fg(Color::DarkGray)),
            Line::from(format!("   Size: {}x{}", p.width, p.height)).style(Style::default().fg(Color::DarkGray)),
            Line::from(""), 
        ];
        ListItem::new(content)
    }).collect();
    f.render_stateful_widget(
        List::new(panes).block(Block::default().borders(Borders::ALL).title(" Panes ").border_style(get_border(FocusArea::Panes))).highlight_style(highlight_style),
        cols[2], &mut app.pane_list_state
    );

    // Footer
    let help_text = get_footer_text(app);
    f.render_widget(Paragraph::new(help_text).style(Style::default().fg(Color::DarkGray)), chunks[2]);

    // Modals
    match app.state {
        AppState::InputNewSession => render_input(f, app, "New Session Name"),
        AppState::InputRenameSession => render_input(f, app, "Rename Session"),
        AppState::InputNewWindow => render_input(f, app, "New Window Name"),
        AppState::InputRenameWindow => render_input(f, app, "Rename Window"),
        AppState::ConfirmDeleteSession => render_confirm(f, "Delete Session?"),
        AppState::ConfirmDeleteWindow => render_confirm(f, "Delete Window?"),
        AppState::ConfirmDeletePane => render_confirm(f, "Delete Pane?"),
        _ => {}
    }
}

fn get_footer_text(app: &App) -> String {
    match app.state {
        AppState::Normal => {
            // Common navigation keys
            let common = "NAV: Arrows/Tab | q: Quit | r: Refresh";
            match app.focus {
                FocusArea::Sessions => format!("{} | Enter: Attach | n: New | d: Del | R: Rename", common),
                FocusArea::Windows => format!("{} | Enter: Attach | n: New Win | d: Del Win | R: Rename", common),
                FocusArea::Panes => format!("{} | Enter: Attach | n: Split Pane | d: Kill Pane", common),
            }
        },
        AppState::InputNewSession | AppState::InputRenameSession | 
        AppState::InputNewWindow | AppState::InputRenameWindow => "Enter: Confirm | Esc: Cancel".to_string(),
        _ => "y: Confirm | n: Cancel".to_string(),
    }
}

fn render_input(f: &mut Frame, app: &App, title: &str) {
    let area = centered_rect(60, 20, f.size());
    f.render_widget(Clear, area);
    let block = Block::default().title(format!(" {} ", title)).borders(Borders::ALL).border_style(Style::default().fg(Color::Yellow));
    f.render_widget(Paragraph::new(app.input_buffer.as_str()).block(block), area);
}

fn render_confirm(f: &mut Frame, title: &str) {
    let area = centered_rect(40, 10, f.size());
    f.render_widget(Clear, area);
    let block = Block::default().title(format!(" {} ", title)).borders(Borders::ALL).border_style(Style::default().fg(Color::Red));
    f.render_widget(Paragraph::new("Are you sure? (y/n)").block(block).alignment(Alignment::Center), area);
}

fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}