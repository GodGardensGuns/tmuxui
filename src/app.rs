use ratatui::widgets::ListState;
use crate::models::{Session, Window, Pane};
use crate::tmux;

#[derive(PartialEq, Debug)]
pub enum AppState {
    Normal,
    // Session Actions
    InputNewSession,
    InputRenameSession,
    ConfirmDeleteSession,
    // Window Actions
    InputNewWindow,
    InputRenameWindow,
    ConfirmDeleteWindow,
    // Pane Actions
    ConfirmDeletePane,
}

#[derive(PartialEq, Clone, Copy, Debug)]
pub enum FocusArea {
    Sessions,
    Windows,
    Panes,
}

pub struct App {
    // Data
    pub sessions: Vec<Session>,
    pub windows: Vec<Window>,
    pub panes: Vec<Pane>,

    // UI State
    pub session_list_state: ListState,
    pub window_list_state: ListState,
    pub pane_list_state: ListState,
    
    pub focus: FocusArea,
    pub state: AppState,
    
    // Inputs/Misc
    pub input_buffer: String,
    pub should_quit: bool,
    
    // The name of the session we want to attach to after quitting
    pub target_attach: Option<String>,
}

impl App {
    pub fn new() -> Self {
        let mut app = Self {
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
        };
        app.refresh_all();
        app
    }

    pub fn refresh_all(&mut self) {
        // 1. Sessions
        self.sessions = tmux::get_sessions();
        validate_list_selection(&mut self.session_list_state, self.sessions.len());

        // 2. Windows
        if let Some(idx) = self.session_list_state.selected() {
            if let Some(session) = self.sessions.get(idx) {
                self.windows = tmux::get_windows(&session.id);
            } else {
                self.windows.clear();
            }
        } else {
            self.windows.clear();
        }
        validate_list_selection(&mut self.window_list_state, self.windows.len());

        // 3. Panes
        if let Some(idx) = self.window_list_state.selected() {
            if let Some(window) = self.windows.get(idx) {
                self.panes = tmux::get_panes(&window.id);
            } else {
                self.panes.clear();
            }
        } else {
            self.panes.clear();
        }
        validate_list_selection(&mut self.pane_list_state, self.panes.len());
    }

    pub fn get_selected_session(&self) -> Option<&Session> {
        self.session_list_state.selected().and_then(|i| self.sessions.get(i))
    }

    pub fn get_selected_window(&self) -> Option<&Window> {
        self.window_list_state.selected().and_then(|i| self.windows.get(i))
    }

    pub fn get_selected_pane(&self) -> Option<&Pane> {
        self.pane_list_state.selected().and_then(|i| self.panes.get(i))
    }

    pub fn nav_down(&mut self) {
        match self.focus {
            FocusArea::Sessions => {
                next_item(&mut self.session_list_state, self.sessions.len());
                self.refresh_all(); 
            },
            FocusArea::Windows => {
                next_item(&mut self.window_list_state, self.windows.len());
                self.refresh_panes_only();
            },
            FocusArea::Panes => next_item(&mut self.pane_list_state, self.panes.len()),
        }
    }

    pub fn nav_up(&mut self) {
        match self.focus {
            FocusArea::Sessions => {
                prev_item(&mut self.session_list_state, self.sessions.len());
                self.refresh_all();
            },
            FocusArea::Windows => {
                prev_item(&mut self.window_list_state, self.windows.len());
                self.refresh_panes_only();
            },
            FocusArea::Panes => prev_item(&mut self.pane_list_state, self.panes.len()),
        }
    }

    fn refresh_panes_only(&mut self) {
        if let Some(idx) = self.window_list_state.selected() {
            if let Some(win) = self.windows.get(idx) {
                self.panes = tmux::get_panes(&win.id);
                validate_list_selection(&mut self.pane_list_state, self.panes.len());
            }
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
}

// Helpers
fn next_item(state: &mut ListState, len: usize) {
    if len == 0 { return; }
    let i = match state.selected() {
        Some(i) => if i >= len - 1 { 0 } else { i + 1 },
        None => 0,
    };
    state.select(Some(i));
}

fn prev_item(state: &mut ListState, len: usize) {
    if len == 0 { return; }
    let i = match state.selected() {
        Some(i) => if i == 0 { len - 1 } else { i - 1 },
        None => 0,
    };
    state.select(Some(i));
}

fn validate_list_selection(state: &mut ListState, len: usize) {
    if len == 0 {
        state.select(None);
    } else if let Some(i) = state.selected() {
        if i >= len {
            state.select(Some(len - 1));
        }
    } else {
        state.select(Some(0));
    }
}