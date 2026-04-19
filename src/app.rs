use crate::models::{Pane, Session, Window};
use crate::tmux;
use ratatui::widgets::ListState;

#[derive(PartialEq, Debug)]
pub enum AppState {
    Normal,
    InputNewSession,
    InputRenameSession,
    ConfirmDeleteSession,
    InputNewWindow,
    InputRenameWindow,
    ConfirmDeleteWindow,
    ConfirmDeletePane,
}

#[derive(PartialEq, Clone, Copy, Debug)]
pub enum FocusArea {
    Sessions,
    Windows,
    Panes,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum StatusLevel {
    Info,
    Success,
    Error,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StatusMessage {
    pub level: StatusLevel,
    pub text: String,
}

impl StatusMessage {
    fn info(text: impl Into<String>) -> Self {
        Self {
            level: StatusLevel::Info,
            text: text.into(),
        }
    }

    fn success(text: impl Into<String>) -> Self {
        Self {
            level: StatusLevel::Success,
            text: text.into(),
        }
    }

    fn error(text: impl Into<String>) -> Self {
        Self {
            level: StatusLevel::Error,
            text: text.into(),
        }
    }

    pub fn is_error(&self) -> bool {
        self.level == StatusLevel::Error
    }
}

pub struct App {
    pub sessions: Vec<Session>,
    pub windows: Vec<Window>,
    pub panes: Vec<Pane>,
    pub session_list_state: ListState,
    pub window_list_state: ListState,
    pub pane_list_state: ListState,
    pub focus: FocusArea,
    pub state: AppState,
    pub input_buffer: String,
    pub should_quit: bool,
    pub target_attach: Option<String>,
    pub status: StatusMessage,
}

impl Default for App {
    fn default() -> Self {
        Self {
            sessions: Vec::new(),
            windows: Vec::new(),
            panes: Vec::new(),
            session_list_state: ListState::default(),
            window_list_state: ListState::default(),
            pane_list_state: ListState::default(),
            focus: FocusArea::Sessions,
            state: AppState::Normal,
            input_buffer: String::new(),
            should_quit: false,
            target_attach: None,
            status: StatusMessage::info("Loading tmux data..."),
        }
    }
}

impl App {
    pub fn new() -> Self {
        let mut app = Self::default();
        app.refresh_all();
        if app.sessions.is_empty() && !app.status.is_error() {
            app.set_info("No tmux sessions found. Press n to create one.");
        }
        app
    }

    pub fn refresh_all(&mut self) {
        let selected_session_id = self
            .get_selected_session()
            .map(|session| session.id.clone());
        let selected_window_id = self.get_selected_window().map(|window| window.id.clone());
        let selected_pane_id = self.get_selected_pane().map(|pane| pane.id.clone());

        match tmux::get_sessions() {
            Ok(sessions) => {
                self.sessions = sessions;
                select_matching_or_first(
                    &mut self.session_list_state,
                    &self.sessions,
                    selected_session_id.as_deref(),
                    |session| session.id.as_str(),
                );
            }
            Err(err) => {
                self.clear_sessions();
                self.clear_windows();
                self.clear_panes();
                self.set_error(format!(
                    "Could not load tmux sessions: {}",
                    truncate_status(&err.to_string())
                ));
                return;
            }
        }

        self.refresh_windows_and_panes(selected_window_id.as_deref(), selected_pane_id.as_deref());

        if self.sessions.is_empty() && self.status.level == StatusLevel::Info {
            self.set_info("No tmux sessions found. Press n to create one.");
        }
    }

    pub fn get_selected_session(&self) -> Option<&Session> {
        self.session_list_state
            .selected()
            .and_then(|index| self.sessions.get(index))
    }

    pub fn get_selected_window(&self) -> Option<&Window> {
        self.window_list_state
            .selected()
            .and_then(|index| self.windows.get(index))
    }

    pub fn get_selected_pane(&self) -> Option<&Pane> {
        self.pane_list_state
            .selected()
            .and_then(|index| self.panes.get(index))
    }

    pub fn nav_down(&mut self) {
        match self.focus {
            FocusArea::Sessions => {
                next_item(&mut self.session_list_state, self.sessions.len());
                self.refresh_windows_and_panes(None, None);
            }
            FocusArea::Windows => {
                next_item(&mut self.window_list_state, self.windows.len());
                self.refresh_panes_only();
            }
            FocusArea::Panes => next_item(&mut self.pane_list_state, self.panes.len()),
        }
    }

    pub fn nav_up(&mut self) {
        match self.focus {
            FocusArea::Sessions => {
                prev_item(&mut self.session_list_state, self.sessions.len());
                self.refresh_windows_and_panes(None, None);
            }
            FocusArea::Windows => {
                prev_item(&mut self.window_list_state, self.windows.len());
                self.refresh_panes_only();
            }
            FocusArea::Panes => prev_item(&mut self.pane_list_state, self.panes.len()),
        }
    }

    pub fn nav_first(&mut self) {
        match self.focus {
            FocusArea::Sessions => {
                select_first(&mut self.session_list_state, self.sessions.len());
                self.refresh_windows_and_panes(None, None);
            }
            FocusArea::Windows => {
                select_first(&mut self.window_list_state, self.windows.len());
                self.refresh_panes_only();
            }
            FocusArea::Panes => select_first(&mut self.pane_list_state, self.panes.len()),
        }
    }

    pub fn nav_last(&mut self) {
        match self.focus {
            FocusArea::Sessions => {
                select_last(&mut self.session_list_state, self.sessions.len());
                self.refresh_windows_and_panes(None, None);
            }
            FocusArea::Windows => {
                select_last(&mut self.window_list_state, self.windows.len());
                self.refresh_panes_only();
            }
            FocusArea::Panes => select_last(&mut self.pane_list_state, self.panes.len()),
        }
    }

    pub fn cycle_focus(&mut self) {
        self.focus = match self.focus {
            FocusArea::Sessions => FocusArea::Windows,
            FocusArea::Windows => FocusArea::Panes,
            FocusArea::Panes => FocusArea::Sessions,
        };
    }

    pub fn cycle_focus_back(&mut self) {
        self.focus = match self.focus {
            FocusArea::Sessions => FocusArea::Panes,
            FocusArea::Windows => FocusArea::Sessions,
            FocusArea::Panes => FocusArea::Windows,
        };
    }

    pub fn set_info(&mut self, text: impl Into<String>) {
        self.status = StatusMessage::info(truncate_status(&text.into()));
    }

    pub fn set_success(&mut self, text: impl Into<String>) {
        self.status = StatusMessage::success(truncate_status(&text.into()));
    }

    pub fn set_error(&mut self, text: impl Into<String>) {
        self.status = StatusMessage::error(truncate_status(&text.into()));
    }

    pub fn selected_session_name(&self) -> Option<&str> {
        self.get_selected_session()
            .map(|session| session.name.as_str())
    }

    pub fn selected_window_name(&self) -> Option<&str> {
        self.get_selected_window()
            .map(|window| window.name.as_str())
    }

    pub fn selected_pane_id(&self) -> Option<&str> {
        self.get_selected_pane().map(|pane| pane.id.as_str())
    }

    fn refresh_windows_and_panes(
        &mut self,
        selected_window_id: Option<&str>,
        selected_pane_id: Option<&str>,
    ) {
        let Some(session_id) = self
            .get_selected_session()
            .map(|session| session.id.clone())
        else {
            self.clear_windows();
            self.clear_panes();
            return;
        };

        match tmux::get_windows(&session_id) {
            Ok(windows) => {
                self.windows = windows;
                select_matching_or_first(
                    &mut self.window_list_state,
                    &self.windows,
                    selected_window_id,
                    |window| window.id.as_str(),
                );
            }
            Err(err) => {
                self.clear_windows();
                self.clear_panes();
                self.set_error(format!(
                    "Could not load windows: {}",
                    truncate_status(&err.to_string())
                ));
                return;
            }
        }

        self.refresh_panes(selected_pane_id);
    }

    fn refresh_panes_only(&mut self) {
        let selected_pane_id = self.get_selected_pane().map(|pane| pane.id.clone());
        self.refresh_panes(selected_pane_id.as_deref());
    }

    fn refresh_panes(&mut self, selected_pane_id: Option<&str>) {
        let Some(window_id) = self.get_selected_window().map(|window| window.id.clone()) else {
            self.clear_panes();
            return;
        };

        match tmux::get_panes(&window_id) {
            Ok(panes) => {
                self.panes = panes;
                select_matching_or_first(
                    &mut self.pane_list_state,
                    &self.panes,
                    selected_pane_id,
                    |pane| pane.id.as_str(),
                );
            }
            Err(err) => {
                self.clear_panes();
                self.set_error(format!(
                    "Could not load panes: {}",
                    truncate_status(&err.to_string())
                ));
            }
        }
    }

    fn clear_sessions(&mut self) {
        self.sessions.clear();
        self.session_list_state.select(None);
    }

    fn clear_windows(&mut self) {
        self.windows.clear();
        self.window_list_state.select(None);
    }

    fn clear_panes(&mut self) {
        self.panes.clear();
        self.pane_list_state.select(None);
    }
}

fn next_item(state: &mut ListState, len: usize) {
    if len == 0 {
        return;
    }

    let next = match state.selected() {
        Some(index) if index >= len - 1 => 0,
        Some(index) => index + 1,
        None => 0,
    };
    state.select(Some(next));
}

fn prev_item(state: &mut ListState, len: usize) {
    if len == 0 {
        return;
    }

    let next = match state.selected() {
        Some(0) | None => len - 1,
        Some(index) => index - 1,
    };
    state.select(Some(next));
}

fn select_first(state: &mut ListState, len: usize) {
    if len == 0 {
        state.select(None);
    } else {
        state.select(Some(0));
    }
}

fn select_last(state: &mut ListState, len: usize) {
    if len == 0 {
        state.select(None);
    } else {
        state.select(Some(len - 1));
    }
}

fn select_matching_or_first<T, F>(
    state: &mut ListState,
    items: &[T],
    selected_id: Option<&str>,
    get_id: F,
) where
    F: Fn(&T) -> &str,
{
    if items.is_empty() {
        state.select(None);
        return;
    }

    if let Some(selected_id) = selected_id {
        if let Some(index) = items.iter().position(|item| get_id(item) == selected_id) {
            state.select(Some(index));
            return;
        }
    }

    state.select(Some(0));
}

fn truncate_status(text: &str) -> String {
    const LIMIT: usize = 120;
    let chars: Vec<char> = text.chars().collect();
    if chars.len() <= LIMIT {
        return text.to_string();
    }

    let truncated: String = chars.into_iter().take(LIMIT - 3).collect();
    format!("{truncated}...")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn selection_wraps_in_both_directions() {
        let mut state = ListState::default();

        next_item(&mut state, 3);
        assert_eq!(state.selected(), Some(0));

        prev_item(&mut state, 3);
        assert_eq!(state.selected(), Some(2));
    }

    #[test]
    fn matching_selection_prefers_existing_item() {
        let mut state = ListState::default();
        state.select(Some(1));

        let sessions = vec![
            Session {
                id: "%0".to_string(),
                name: "first".to_string(),
                window_count: 1,
                created: "today".to_string(),
            },
            Session {
                id: "%1".to_string(),
                name: "second".to_string(),
                window_count: 2,
                created: "today".to_string(),
            },
        ];

        select_matching_or_first(&mut state, &sessions, Some("%1"), |session| {
            session.id.as_str()
        });

        assert_eq!(state.selected(), Some(1));
    }

    #[test]
    fn status_messages_are_truncated_for_the_footer() {
        let long = "x".repeat(150);

        let truncated = truncate_status(&long);

        assert!(truncated.len() <= 120);
        assert!(truncated.ends_with("..."));
    }
}
