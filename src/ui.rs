use crate::app::{
    ActionAvailability, App, BannerTone, ConfirmIntent, FocusArea, InputIntent, ModalState,
};
use crate::tmux::TmuxConnectionState;
use ratatui::{prelude::*, widgets::*};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LayoutMode {
    Compact,
    Split,
    Wide,
}

pub fn draw(frame: &mut Frame, app: &mut App) {
    let size = frame.size();
    let layout_mode = layout_mode_for(size.width);
    let outer = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(2),
            Constraint::Min(0),
            Constraint::Length(2),
            Constraint::Length(1),
        ])
        .split(size);

    frame.render_widget(Block::default().style(Style::default()), size);
    render_header(frame, outer[0], app, layout_mode);
    render_body(frame, outer[1], app, layout_mode);
    render_banner(frame, outer[2], app);
    render_shortcuts(frame, outer[3], app);

    if app.filter.active {
        render_filter_overlay(frame, app);
    }

    match &app.modal {
        ModalState::Input(modal) => render_input_modal(frame, app, modal),
        ModalState::Confirm(modal) => render_confirm_modal(frame, app, modal),
        ModalState::None => {}
    }

    if app.help.visible {
        render_help_overlay(frame);
    }
}

pub fn layout_mode_for(width: u16) -> LayoutMode {
    if width >= 140 {
        LayoutMode::Wide
    } else if width >= 100 {
        LayoutMode::Split
    } else {
        LayoutMode::Compact
    }
}

fn render_header(frame: &mut Frame, area: Rect, app: &App, layout_mode: LayoutMode) {
    let focus_label = format!("Focus {}", app.focus.title());
    let line1 = Line::from(vec![
        Span::styled(
            " TMUXUI ",
            Style::default()
                .fg(Color::Black)
                .bg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw(" "),
        badge(
            app.connection_label(),
            connection_style(app.connection),
            Style::default().fg(Color::White).bg(Color::DarkGray),
        ),
        Span::raw(" "),
        badge(
            &focus_label,
            Style::default().fg(Color::Black).bg(Color::White),
            Style::default().fg(Color::White).bg(Color::DarkGray),
        ),
        Span::raw(" "),
        badge(
            match layout_mode {
                LayoutMode::Wide => "Wide",
                LayoutMode::Split => "Split",
                LayoutMode::Compact => "Compact",
            },
            Style::default().fg(Color::Black).bg(Color::Gray),
            Style::default().fg(Color::White).bg(Color::DarkGray),
        ),
    ]);

    let counts = format!(
        "{} sessions | {} windows | {} panes",
        app.sessions.len(),
        app.windows.len(),
        app.panes.len()
    );
    let secondary = if let Some(filter) = app.filter_summary() {
        format!("{counts} | Filter {filter}")
    } else {
        counts
    };

    frame.render_widget(
        Paragraph::new(vec![
            line1,
            Line::styled(secondary, Style::default().fg(Color::Gray)),
        ]),
        area,
    );
}

fn render_body(frame: &mut Frame, area: Rect, app: &mut App, layout_mode: LayoutMode) {
    match layout_mode {
        LayoutMode::Wide => render_wide_body(frame, area, app),
        LayoutMode::Split => render_split_body(frame, area, app),
        LayoutMode::Compact => render_compact_body(frame, area, app),
    }
}

fn render_wide_body(frame: &mut Frame, area: Rect, app: &mut App) {
    let columns = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(24),
            Constraint::Percentage(24),
            Constraint::Percentage(28),
            Constraint::Percentage(24),
        ])
        .split(area);

    render_sessions_panel(frame, columns[0], app);
    render_windows_panel(frame, columns[1], app);
    render_panes_panel(frame, columns[2], app);
    render_details_panel(frame, columns[3], app);
}

fn render_split_body(frame: &mut Frame, area: Rect, app: &mut App) {
    let sections = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(0), Constraint::Length(10)])
        .split(area);

    let top = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(42), Constraint::Percentage(58)])
        .split(sections[0]);
    let right = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(top[1]);

    render_sessions_panel(frame, top[0], app);
    render_windows_panel(frame, right[0], app);
    render_panes_panel(frame, right[1], app);
    render_details_panel(frame, sections[1], app);
}

fn render_compact_body(frame: &mut Frame, area: Rect, app: &mut App) {
    let sections = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(2),
            Constraint::Min(0),
            Constraint::Length(10),
        ])
        .split(area);

    render_compact_context(frame, sections[0], app);
    match app.focus {
        FocusArea::Sessions => render_sessions_panel(frame, sections[1], app),
        FocusArea::Windows => render_windows_panel(frame, sections[1], app),
        FocusArea::Panes => render_panes_panel(frame, sections[1], app),
    }
    render_details_panel(frame, sections[2], app);
}

fn render_compact_context(frame: &mut Frame, area: Rect, app: &App) {
    let breadcrumb = Line::styled(
        "Sessions > Windows > Panes",
        Style::default().add_modifier(Modifier::BOLD),
    );

    let summary = match app.focus {
        FocusArea::Sessions => "Showing sessions. Choose one to reveal windows.".to_string(),
        FocusArea::Windows => format!(
            "Showing windows for {}.",
            app.selected_session_name()
                .unwrap_or("the selected session")
        ),
        FocusArea::Panes => format!(
            "Showing panes for {}.",
            app.selected_window_name().unwrap_or("the selected window")
        ),
    };

    frame.render_widget(
        Paragraph::new(vec![
            breadcrumb,
            Line::styled(
                format!("Focus: {} | {summary}", app.focus.title()),
                Style::default().fg(Color::Gray),
            ),
        ]),
        area,
    );
}

fn render_sessions_panel(frame: &mut Frame, area: Rect, app: &mut App) {
    let visible = app.visible_session_indices();
    let items = if visible.is_empty() {
        vec![empty_item(session_empty_state(app))]
    } else {
        visible
            .iter()
            .map(|index| {
                let session = &app.sessions[*index];
                ListItem::new(Line::from(vec![
                    Span::styled(
                        session.name.clone(),
                        Style::default().add_modifier(Modifier::BOLD),
                    ),
                    Span::styled(
                        format!("  {} windows", session.window_count),
                        Style::default().fg(Color::Gray),
                    ),
                    Span::styled(
                        format!("  {}", compact_created(&session.created)),
                        Style::default().fg(Color::Gray),
                    ),
                ]))
            })
            .collect()
    };

    render_list_panel(
        frame,
        area,
        app.focus == FocusArea::Sessions,
        format!("Sessions [{}]", visible.len()),
        items,
        &mut app.session_list_state,
    );
}

fn render_windows_panel(frame: &mut Frame, area: Rect, app: &mut App) {
    let visible = app.visible_window_indices();
    let show_layout = area.width >= 28;
    let items = if visible.is_empty() {
        vec![empty_item(window_empty_state(app))]
    } else {
        visible
            .iter()
            .map(|index| {
                let window = &app.windows[*index];
                let mut spans = vec![
                    Span::styled(
                        format!("{} {}", if window.active { "*" } else { " " }, window.name),
                        Style::default().add_modifier(Modifier::BOLD),
                    ),
                    Span::styled(format!("  {}", window.id), Style::default().fg(Color::Gray)),
                ];
                if show_layout {
                    spans.push(Span::styled(
                        format!("  {}", window.layout),
                        Style::default().fg(Color::Gray),
                    ));
                }
                ListItem::new(Line::from(spans))
            })
            .collect()
    };

    render_list_panel(
        frame,
        area,
        app.focus == FocusArea::Windows,
        format!("Windows [{}]", visible.len()),
        items,
        &mut app.window_list_state,
    );
}

fn render_panes_panel(frame: &mut Frame, area: Rect, app: &mut App) {
    let visible = app.visible_pane_indices();
    let path_width = area.width.saturating_sub(10) as usize;
    let items = if visible.is_empty() {
        vec![empty_item(pane_empty_state(app))]
    } else {
        visible
            .iter()
            .map(|index| {
                let pane = &app.panes[*index];
                ListItem::new(vec![
                    Line::from(vec![
                        Span::styled(
                            format!(
                                "{} {}",
                                if pane.active { "*" } else { " " },
                                pane.current_command
                            ),
                            Style::default().add_modifier(Modifier::BOLD),
                        ),
                        Span::styled(
                            format!("  {}  {}x{}", pane.id, pane.width, pane.height),
                            Style::default().fg(Color::Gray),
                        ),
                    ]),
                    Line::from(vec![
                        Span::styled("Path ", Style::default().fg(Color::Gray)),
                        Span::raw(truncate_middle(&pane.current_path, path_width)),
                    ]),
                ])
            })
            .collect()
    };

    render_list_panel(
        frame,
        area,
        app.focus == FocusArea::Panes,
        format!("Panes [{}]", visible.len()),
        items,
        &mut app.pane_list_state,
    );
}

fn render_list_panel(
    frame: &mut Frame,
    area: Rect,
    is_focused: bool,
    title: String,
    items: Vec<ListItem<'static>>,
    state: &mut ListState,
) {
    let title = if is_focused {
        format!("{title} ACTIVE")
    } else {
        title
    };

    frame.render_stateful_widget(
        List::new(items)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(format!(" {title} "))
                    .border_style(panel_border_style(is_focused)),
            )
            .highlight_style(Style::default().add_modifier(Modifier::REVERSED | Modifier::BOLD))
            .highlight_symbol("> "),
        area,
        state,
    );
}

fn render_details_panel(frame: &mut Frame, area: Rect, app: &App) {
    let lines = selection_lines(app, app.action_availability(), area.width);
    frame.render_widget(
        Paragraph::new(lines)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(" Selection ")
                    .border_style(panel_border_style(false)),
            )
            .wrap(Wrap { trim: true }),
        area,
    );
}

fn selection_lines(app: &App, actions: ActionAvailability, width: u16) -> Vec<Line<'static>> {
    let mut lines = Vec::new();
    match app.focus {
        FocusArea::Sessions => {
            if let Some(session) = app.get_selected_session() {
                lines.push(detail_line("Name", session.name.clone()));
                lines.push(detail_line("Id", session.id.clone()));
                lines.push(detail_line("Started", compact_created(&session.created)));
                lines.push(detail_line("Windows", session.window_count.to_string()));
            } else {
                lines.push(Line::styled(
                    "No session is selected.",
                    Style::default().fg(Color::Gray),
                ));
                lines.push(Line::styled(
                    "Use j/k to choose a session or press n to create one.",
                    Style::default().fg(Color::Gray),
                ));
            }
        }
        FocusArea::Windows => {
            if let Some(window) = app.get_selected_window() {
                lines.push(detail_line("Name", window.name.clone()));
                lines.push(detail_line("Id", window.id.clone()));
                lines.push(detail_line("Layout", window.layout.clone()));
                lines.push(detail_line(
                    "Active",
                    if window.active { "yes" } else { "no" }.to_string(),
                ));
            } else {
                lines.push(Line::styled(
                    "No window is selected.",
                    Style::default().fg(Color::Gray),
                ));
                lines.push(Line::styled(
                    "Choose a session first, then move into its windows.",
                    Style::default().fg(Color::Gray),
                ));
            }
        }
        FocusArea::Panes => {
            if let Some(pane) = app.get_selected_pane() {
                lines.push(detail_line("Command", pane.current_command.clone()));
                lines.push(detail_line("Id", pane.id.clone()));
                lines.push(detail_line(
                    "Size",
                    format!("{}x{}", pane.width, pane.height),
                ));
                lines.push(detail_line(
                    "Path",
                    truncate_middle(&pane.current_path, width.saturating_sub(14) as usize),
                ));
            } else {
                lines.push(Line::styled(
                    "No pane is selected.",
                    Style::default().fg(Color::Gray),
                ));
                lines.push(Line::styled(
                    "Choose a window first, then inspect its panes.",
                    Style::default().fg(Color::Gray),
                ));
            }
        }
    }

    lines.push(Line::raw(""));
    lines.push(Line::styled(
        "Actions",
        Style::default().add_modifier(Modifier::BOLD),
    ));
    lines.extend(action_lines(actions));
    lines
}

fn action_lines(actions: ActionAvailability) -> Vec<Line<'static>> {
    [
        actions.attach,
        actions.create,
        actions.rename,
        actions.delete,
    ]
    .into_iter()
    .map(|action| {
        let status_style = if action.enabled {
            Style::default().fg(Color::Gray)
        } else {
            Style::default().fg(Color::Yellow)
        };
        Line::from(vec![
            Span::styled(
                format!("{:>5}", action.key),
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw(" "),
            Span::styled(
                format!("{:<11}", action.label),
                if action.enabled {
                    Style::default().add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(Color::Gray)
                },
            ),
            Span::raw(" "),
            Span::styled(action.reason, status_style),
        ])
    })
    .collect()
}

fn render_banner(frame: &mut Frame, area: Rect, app: &App) {
    let tone = banner_style(&app.banner.tone);
    frame.render_widget(
        Paragraph::new(vec![
            Line::from(vec![
                Span::styled(
                    format!("[{}] ", banner_label(&app.banner.tone)),
                    tone.add_modifier(Modifier::BOLD),
                ),
                Span::styled(app.banner.title.as_str(), tone.add_modifier(Modifier::BOLD)),
            ]),
            Line::styled(app.banner.body.as_str(), tone),
        ])
        .wrap(Wrap { trim: true }),
        area,
    );
}

fn render_shortcuts(frame: &mut Frame, area: Rect, app: &App) {
    frame.render_widget(
        Paragraph::new(shortcuts(app)).style(Style::default().fg(Color::Gray)),
        area,
    );
}

fn render_filter_overlay(frame: &mut Frame, app: &App) {
    let area = centered_rect(62, 18, frame.size());
    frame.render_widget(Clear, area);

    let block = Block::default()
        .title(format!(" Filter {} ", app.filter.target.title()))
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan));
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let query = if app.filter.query.is_empty() {
        "Type to narrow the current list."
    } else {
        app.filter.query.as_str()
    };
    frame.render_widget(
        Paragraph::new(vec![
            Line::styled(
                "Type to filter. Enter keeps it. Esc clears it.",
                Style::default().fg(Color::Gray),
            ),
            Line::raw(""),
            Line::raw(query),
        ]),
        inner,
    );

    let cursor_x = inner.x + app.filter.query.chars().count() as u16;
    let cursor_y = inner.y + 2;
    frame.set_cursor(
        cursor_x.min(inner.right().saturating_sub(1)),
        cursor_y.min(inner.bottom().saturating_sub(1)),
    );
}

fn render_input_modal(frame: &mut Frame, app: &App, modal: &crate::app::InputModalState) {
    let area = centered_rect(68, 42, frame.size());
    frame.render_widget(Clear, area);

    let (title, prompt, field_label, submit_copy) = input_modal_copy(app, modal);
    let block = Block::default()
        .title(format!(" {title} "))
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Yellow));
    let inner = block.inner(area);
    let sections = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(2),
            Constraint::Length(3),
            Constraint::Length(2),
            Constraint::Min(2),
        ])
        .split(inner);

    frame.render_widget(block, area);
    frame.render_widget(
        Paragraph::new(prompt).style(Style::default().fg(Color::Gray)),
        sections[0],
    );
    frame.render_widget(
        Paragraph::new(modal.value.as_str())
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(format!(" {field_label} "))
                    .border_style(Style::default().fg(Color::White)),
            )
            .wrap(Wrap { trim: false }),
        sections[1],
    );

    let feedback = modal
        .error
        .as_deref()
        .map(|error| {
            Line::styled(
                error,
                Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
            )
        })
        .unwrap_or_else(|| Line::styled(submit_copy, Style::default().fg(Color::Gray)));
    frame.render_widget(Paragraph::new(feedback), sections[2]);

    let cursor = visible_cursor(&modal.value, sections[1].width.saturating_sub(2) as usize);
    frame.set_cursor(sections[1].x + 1 + cursor as u16, sections[1].y + 1);
}

fn render_confirm_modal(frame: &mut Frame, app: &App, modal: &crate::app::ConfirmModalState) {
    let area = centered_rect(66, 38, frame.size());
    frame.render_widget(Clear, area);

    let (title, target, impact, confirm_copy) = confirm_modal_copy(app, modal.intent);
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
            Line::styled(impact, Style::default().fg(Color::Gray)),
            Line::raw(""),
            modal
                .error
                .as_deref()
                .map(|error| {
                    Line::styled(
                        error,
                        Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
                    )
                })
                .unwrap_or_else(|| Line::styled(confirm_copy, Style::default().fg(Color::Yellow))),
        ])
        .alignment(Alignment::Center)
        .wrap(Wrap { trim: true }),
        inner,
    );
}

fn render_help_overlay(frame: &mut Frame) {
    let area = centered_rect(82, 78, frame.size());
    frame.render_widget(Clear, area);

    let block = Block::default()
        .title(" Help ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan));
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let help = vec![
        Line::styled("Move", Style::default().add_modifier(Modifier::BOLD)),
        Line::raw("j/k or arrows move within a list."),
        Line::raw("Tab or h/l moves focus between sessions, windows, and panes."),
        Line::raw("g/G jumps to the first or last visible row."),
        Line::raw(""),
        Line::styled("Work", Style::default().add_modifier(Modifier::BOLD)),
        Line::raw("Enter attaches to the selected session, window, or pane."),
        Line::raw("n creates a session or window, or splits the selected pane."),
        Line::raw("R renames the selected session or window."),
        Line::raw("d deletes the selected item after confirmation."),
        Line::raw("/ opens the quick filter for the current list."),
        Line::raw("Type to filter, Enter keeps it, and Esc clears it."),
        Line::raw("r refreshes data from tmux."),
        Line::raw(""),
        Line::styled("Dialogs", Style::default().add_modifier(Modifier::BOLD)),
        Line::raw("Enter confirms. Esc cancels. Ctrl+U clears text while typing."),
        Line::raw(""),
        Line::styled("Leave", Style::default().add_modifier(Modifier::BOLD)),
        Line::raw("q or Esc closes tmuxui. Ctrl+C exits immediately."),
        Line::raw("Press Esc, q, or ? to close this help panel."),
    ];

    frame.render_widget(Paragraph::new(help).wrap(Wrap { trim: true }), inner);
}

fn panel_border_style(is_focused: bool) -> Style {
    if is_focused {
        Style::default()
            .fg(Color::Cyan)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::DarkGray)
    }
}

fn badge<'a>(text: &'a str, style: Style, fallback: Style) -> Span<'a> {
    let style = if matches!(style.fg, Some(Color::Reset)) {
        fallback
    } else {
        style
    };
    Span::styled(format!(" {text} "), style.add_modifier(Modifier::BOLD))
}

fn detail_line(label: &str, value: String) -> Line<'static> {
    Line::from(vec![
        Span::styled(
            format!("{label:<7}"),
            Style::default()
                .fg(Color::Gray)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw(" "),
        Span::raw(value),
    ])
}

fn connection_style(connection: TmuxConnectionState) -> Style {
    match connection {
        TmuxConnectionState::Connected => Style::default().fg(Color::Black).bg(Color::Green),
        TmuxConnectionState::NoServer => Style::default().fg(Color::Black).bg(Color::Yellow),
        TmuxConnectionState::Missing => Style::default().fg(Color::White).bg(Color::Red),
        TmuxConnectionState::CommandFailed => Style::default().fg(Color::White).bg(Color::Red),
    }
}

fn banner_style(tone: &BannerTone) -> Style {
    match tone {
        BannerTone::Info => Style::default().fg(Color::Cyan),
        BannerTone::Success => Style::default().fg(Color::Green),
        BannerTone::Warning => Style::default().fg(Color::Yellow),
        BannerTone::Error => Style::default().fg(Color::Red),
    }
}

fn banner_label(tone: &BannerTone) -> &'static str {
    match tone {
        BannerTone::Info => "Info",
        BannerTone::Success => "Done",
        BannerTone::Warning => "Heads up",
        BannerTone::Error => "Error",
    }
}

fn compact_created(created: &str) -> String {
    let parts: Vec<&str> = created.split_whitespace().collect();
    if parts.len() >= 4 {
        let time = parts[3].chars().take(5).collect::<String>();
        format!("{} {} {}", parts[1], parts[2], time)
    } else {
        created.to_string()
    }
}

fn truncate_middle(text: &str, width: usize) -> String {
    let chars: Vec<char> = text.chars().collect();
    if width == 0 || chars.len() <= width {
        return text.to_string();
    }

    if width <= 3 {
        return ".".repeat(width);
    }

    let left = (width - 3) / 2;
    let right = width - 3 - left;
    let start: String = chars.iter().take(left).collect();
    let end: String = chars.iter().rev().take(right).rev().collect();
    format!("{start}...{end}")
}

fn visible_cursor(value: &str, width: usize) -> usize {
    let len = value.chars().count();
    if len < width {
        len
    } else {
        width
    }
}

fn session_empty_state(app: &App) -> String {
    match app.connection {
        TmuxConnectionState::NoServer => {
            "No tmux server is running. Press n to create your first session.".to_string()
        }
        TmuxConnectionState::Missing => {
            "tmux is not installed. Install tmux, then restart tmuxui.".to_string()
        }
        _ if app.filter.target == FocusArea::Sessions && app.filter.has_query() => format!(
            "No sessions match \"{}\". Press / to edit the filter or Esc in the filter to clear it.",
            app.filter.query.trim()
        ),
        _ => "No sessions yet. Press n to create one.".to_string(),
    }
}

fn window_empty_state(app: &App) -> String {
    if app.filter.target == FocusArea::Windows && app.filter.has_query() {
        return format!(
            "No windows match \"{}\". Press / to edit the filter or Esc in the filter to clear it.",
            app.filter.query.trim()
        );
    }
    if app.get_selected_session().is_some() {
        "This session has no windows. Press n to create one.".to_string()
    } else {
        "Select a session to inspect its windows.".to_string()
    }
}

fn pane_empty_state(app: &App) -> String {
    if app.filter.target == FocusArea::Panes && app.filter.has_query() {
        return format!(
            "No panes match \"{}\". Press / to edit the filter or Esc in the filter to clear it.",
            app.filter.query.trim()
        );
    }
    if app.get_selected_window().is_some() {
        "This window has no panes to show.".to_string()
    } else {
        "Select a window to inspect its panes.".to_string()
    }
}

fn input_modal_copy(
    app: &App,
    modal: &crate::app::InputModalState,
) -> (&'static str, String, &'static str, String) {
    match modal.intent {
        InputIntent::NewSession => (
            "New Session",
            "Create a tmux session from here without leaving the browser.".to_string(),
            "Session name",
            format!("Press Enter to create session `{}`.", modal.value.trim()),
        ),
        InputIntent::RenameSession => (
            "Rename Session",
            format!(
                "Rename {} and keep it selected after the refresh.",
                app.selected_session_name()
                    .unwrap_or("the selected session")
            ),
            "Session name",
            format!(
                "Press Enter to rename the session to `{}`.",
                modal.value.trim()
            ),
        ),
        InputIntent::NewWindow => (
            "New Window",
            format!(
                "Create a new window inside {}.",
                app.selected_session_name()
                    .unwrap_or("the selected session")
            ),
            "Window name",
            format!("Press Enter to create window `{}`.", modal.value.trim()),
        ),
        InputIntent::RenameWindow => (
            "Rename Window",
            format!(
                "Rename {} and keep it selected after the refresh.",
                app.selected_window_name().unwrap_or("the selected window")
            ),
            "Window name",
            format!(
                "Press Enter to rename the window to `{}`.",
                modal.value.trim()
            ),
        ),
    }
}

fn confirm_modal_copy(app: &App, intent: ConfirmIntent) -> (&'static str, String, String, String) {
    match intent {
        ConfirmIntent::Session => {
            let target = app.selected_session_name().unwrap_or("No session selected");
            let window_count = app
                .get_selected_session()
                .map(|session| session.window_count)
                .unwrap_or(0);
            (
                "Delete Session",
                target.to_string(),
                format!("This closes all {window_count} window(s) and every pane in the session."),
                "Press Enter to delete the session, or Esc to keep it.".to_string(),
            )
        }
        ConfirmIntent::Window => (
            "Delete Window",
            app.selected_window_name()
                .unwrap_or("No window selected")
                .to_string(),
            "This permanently removes the selected window.".to_string(),
            "Press Enter to delete the window, or Esc to keep it.".to_string(),
        ),
        ConfirmIntent::Pane => (
            "Delete Pane",
            app.selected_pane_id()
                .unwrap_or("No pane selected")
                .to_string(),
            "This permanently removes the selected pane.".to_string(),
            "Press Enter to delete the pane, or Esc to keep it.".to_string(),
        ),
    }
}

fn shortcuts(app: &App) -> String {
    if app.help.visible {
        "Esc close help".to_string()
    } else if app.filter.active {
        "Type to filter  Enter keep  Esc clear  Ctrl+U reset".to_string()
    } else {
        match app.modal {
            ModalState::Input(_) => "Type a name  Enter save  Esc cancel  Ctrl+U clear".to_string(),
            ModalState::Confirm(_) => "Enter confirm  Esc cancel".to_string(),
            ModalState::None => match app.focus {
                FocusArea::Sessions => {
                    "Tab focus  j/k move  n new  R rename  d delete  Enter attach  / filter  ? help"
                        .to_string()
                }
                FocusArea::Windows => {
                    "Tab focus  j/k move  n new  R rename  d delete  Enter attach  / filter  ? help"
                        .to_string()
                }
                FocusArea::Panes => {
                    "Tab focus  j/k move  n split  d delete  Enter attach  / filter  ? help"
                        .to_string()
                }
            },
        }
    }
}

fn empty_item(message: String) -> ListItem<'static> {
    ListItem::new(Line::styled(message, Style::default().fg(Color::Gray)))
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::{App, ConfirmModalState, FilterState, HelpOverlayState, InputModalState};
    use crate::models::{Pane, Session, Window};
    use ratatui::backend::TestBackend;

    fn sample_app() -> App {
        let mut app = App {
            sessions: vec![
                Session {
                    id: "%0".to_string(),
                    name: "development".to_string(),
                    window_count: 2,
                    created: "Sun Apr 19 12:00:00 2026".to_string(),
                },
                Session {
                    id: "%1".to_string(),
                    name: "operations".to_string(),
                    window_count: 1,
                    created: "Sun Apr 19 13:00:00 2026".to_string(),
                },
            ],
            windows: vec![
                Window {
                    id: "@1".to_string(),
                    name: "editor".to_string(),
                    active: true,
                    layout: "main-vertical".to_string(),
                },
                Window {
                    id: "@2".to_string(),
                    name: "logs".to_string(),
                    active: false,
                    layout: "tiled".to_string(),
                },
            ],
            panes: vec![Pane {
                id: "%11".to_string(),
                width: 120,
                height: 30,
                current_path: "/tmp/very/long/path/for/the/project/src".to_string(),
                current_command: "cargo watch".to_string(),
                active: true,
            }],
            ..App::default()
        };
        app.session_list_state.select(Some(0));
        app.window_list_state.select(Some(0));
        app.pane_list_state.select(Some(0));
        app.connection = TmuxConnectionState::Connected;
        app
    }

    fn render_to_string(mut app: App, width: u16, height: u16) -> String {
        let backend = TestBackend::new(width, height);
        let mut terminal = Terminal::new(backend).expect("terminal should build");
        terminal
            .draw(|frame| draw(frame, &mut app))
            .expect("draw should succeed");
        let backend = terminal.backend();
        let buffer = backend.buffer().clone();

        (0..height)
            .map(|y| {
                (0..width)
                    .map(|x| buffer.get(x, y).symbol().to_string())
                    .collect::<String>()
            })
            .collect::<Vec<_>>()
            .join("\n")
    }

    #[test]
    fn layout_mode_breakpoints_match_the_plan() {
        assert_eq!(layout_mode_for(80), LayoutMode::Compact);
        assert_eq!(layout_mode_for(100), LayoutMode::Split);
        assert_eq!(layout_mode_for(140), LayoutMode::Wide);
    }

    #[test]
    fn renders_compact_layout_at_eighty_columns() {
        let output = render_to_string(sample_app(), 80, 24);

        assert!(output.contains("Compact"));
        assert!(output.contains("Sessions > Windows > Panes"));
        assert!(output.contains("Selection"));
        assert!(output.contains("Enter attach"));
    }

    #[test]
    fn renders_split_layout_at_one_hundred_columns() {
        let output = render_to_string(sample_app(), 100, 30);

        assert!(output.contains("Split"));
        assert!(output.contains("Sessions [2]"));
        assert!(output.contains("Windows [2]"));
        assert!(output.contains("Panes [1]"));
    }

    #[test]
    fn renders_wide_layout_at_one_hundred_forty_columns() {
        let output = render_to_string(sample_app(), 140, 40);

        assert!(output.contains("Wide"));
        assert!(output.contains("Selection"));
        assert!(output.contains("cargo watch"));
    }

    #[test]
    fn renders_onboarding_state_for_no_server() {
        let mut app = sample_app();
        app.sessions.clear();
        app.windows.clear();
        app.panes.clear();
        app.connection = TmuxConnectionState::NoServer;
        app.banner.title = "No tmux server".to_string();
        app.banner.body = "Press n to create your first tmux session.".to_string();

        let output = render_to_string(app, 100, 30);

        assert!(output.contains("No tmux server"));
        assert!(output.contains("create your first"));
    }

    #[test]
    fn renders_filter_and_long_paths_without_four_line_panes() {
        let mut app = sample_app();
        app.filter = FilterState {
            active: false,
            target: FocusArea::Panes,
            query: "cargo".to_string(),
        };

        let output = render_to_string(app, 140, 40);

        assert!(output.contains("Filter Panes: cargo"));
        assert!(output.contains("cargo watch"));
        assert!(output.contains("Path"));
    }

    #[test]
    fn renders_input_modal_with_inline_error() {
        let mut app = sample_app();
        app.modal = ModalState::Input(InputModalState {
            intent: InputIntent::NewSession,
            value: String::new(),
            error: Some("Enter a name to continue.".to_string()),
        });

        let output = render_to_string(app, 100, 30);

        assert!(output.contains("New Session"));
        assert!(output.contains("Enter a name to continue."));
    }

    #[test]
    fn renders_delete_confirmation() {
        let mut app = sample_app();
        app.modal = ModalState::Confirm(ConfirmModalState {
            intent: ConfirmIntent::Session,
            error: None,
        });

        let output = render_to_string(app, 100, 30);

        assert!(output.contains("Delete Session"));
        assert!(output.contains("This closes all 2 window"));
    }

    #[test]
    fn renders_error_and_success_banners() {
        let mut error_app = sample_app();
        error_app.banner.tone = BannerTone::Error;
        error_app.banner.title = "Command failed".to_string();
        error_app.banner.body = "tmux returned an error.".to_string();
        let error_output = render_to_string(error_app, 100, 30);
        assert!(error_output.contains("[Error]"));
        assert!(error_output.contains("tmux returned an error."));

        let mut success_app = sample_app();
        success_app.banner.tone = BannerTone::Success;
        success_app.banner.title = "Window created".to_string();
        success_app.banner.body = "editor is ready.".to_string();
        let success_output = render_to_string(success_app, 100, 30);
        assert!(success_output.contains("[Done]"));
        assert!(success_output.contains("editor is ready."));
    }

    #[test]
    fn renders_help_overlay() {
        let mut app = sample_app();
        app.help = HelpOverlayState { visible: true };

        let output = render_to_string(app, 100, 30);

        assert!(output.contains("Help"));
        assert!(output.contains("Type to filter"));
    }
}
