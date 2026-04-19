use crate::models::{Pane, Session, Window};
use crate::tmux::{self, TmuxConnectionState};
use ratatui::widgets::ListState;

#[derive(PartialEq, Clone, Copy, Debug, Eq)]
pub enum FocusArea {
    Sessions,
    Windows,
    Panes,
}

impl FocusArea {
    pub fn title(self) -> &'static str {
        match self {
            Self::Sessions => "Sessions",
            Self::Windows => "Windows",
            Self::Panes => "Panes",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum BannerTone {
    Info,
    Success,
    Warning,
    Error,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BannerState {
    pub tone: BannerTone,
    pub title: String,
    pub body: String,
}

impl BannerState {
    fn info(title: impl Into<String>, body: impl Into<String>) -> Self {
        Self {
            tone: BannerTone::Info,
            title: title.into(),
            body: body.into(),
        }
    }

    fn success(title: impl Into<String>, body: impl Into<String>) -> Self {
        Self {
            tone: BannerTone::Success,
            title: title.into(),
            body: body.into(),
        }
    }

    fn warning(title: impl Into<String>, body: impl Into<String>) -> Self {
        Self {
            tone: BannerTone::Warning,
            title: title.into(),
            body: body.into(),
        }
    }

    fn error(title: impl Into<String>, body: impl Into<String>) -> Self {
        Self {
            tone: BannerTone::Error,
            title: title.into(),
            body: body.into(),
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct HelpOverlayState {
    pub visible: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FilterState {
    pub active: bool,
    pub target: FocusArea,
    pub query: String,
}

impl Default for FilterState {
    fn default() -> Self {
        Self {
            active: false,
            target: FocusArea::Sessions,
            query: String::new(),
        }
    }
}

impl FilterState {
    pub fn has_query(&self) -> bool {
        !self.query.trim().is_empty()
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum InputIntent {
    NewSession,
    RenameSession,
    NewWindow,
    RenameWindow,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InputModalState {
    pub intent: InputIntent,
    pub value: String,
    pub error: Option<String>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ConfirmIntent {
    Session,
    Window,
    Pane,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ConfirmModalState {
    pub intent: ConfirmIntent,
    pub error: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ModalState {
    None,
    Input(InputModalState),
    Confirm(ConfirmModalState),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ActionItem {
    pub key: &'static str,
    pub label: &'static str,
    pub enabled: bool,
    pub reason: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ActionAvailability {
    pub attach: ActionItem,
    pub create: ActionItem,
    pub rename: ActionItem,
    pub delete: ActionItem,
}

pub struct App {
    pub sessions: Vec<Session>,
    pub windows: Vec<Window>,
    pub panes: Vec<Pane>,
    pub session_list_state: ListState,
    pub window_list_state: ListState,
    pub pane_list_state: ListState,
    pub focus: FocusArea,
    pub modal: ModalState,
    pub help: HelpOverlayState,
    pub filter: FilterState,
    pub should_quit: bool,
    pub target_attach: Option<String>,
    pub banner: BannerState,
    pub connection: TmuxConnectionState,
    pub connection_detail: Option<String>,
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
            modal: ModalState::None,
            help: HelpOverlayState::default(),
            filter: FilterState::default(),
            should_quit: false,
            target_attach: None,
            banner: BannerState::info("Loading tmux", "Checking the current tmux server state."),
            connection: TmuxConnectionState::Connected,
            connection_detail: None,
        }
    }
}

impl App {
    pub fn new() -> Self {
        let mut app = Self::default();
        app.refresh_all();
        app
    }

    pub fn refresh_all(&mut self) {
        let selected_session_id = self
            .get_selected_session()
            .map(|session| session.id.clone());
        let selected_window_id = self.get_selected_window().map(|window| window.id.clone());
        let selected_pane_id = self.get_selected_pane().map(|pane| pane.id.clone());

        let snapshot = tmux::get_sessions_snapshot();
        self.sessions = snapshot.sessions;
        self.connection = snapshot.connection;
        self.connection_detail = snapshot.detail;
        self.sync_session_selection(selected_session_id.as_deref());

        if self.connection != TmuxConnectionState::Connected {
            self.clear_windows();
            self.clear_panes();
            self.reset_banner_for_current_state();
            return;
        }

        self.refresh_windows_and_panes(selected_window_id.as_deref(), selected_pane_id.as_deref());

        if self.connection == TmuxConnectionState::Connected {
            self.reset_banner_for_current_state();
        }
    }

    pub fn get_selected_session(&self) -> Option<&Session> {
        self.actual_session_index()
            .and_then(|index| self.sessions.get(index))
    }

    pub fn get_selected_window(&self) -> Option<&Window> {
        self.actual_window_index()
            .and_then(|index| self.windows.get(index))
    }

    pub fn get_selected_pane(&self) -> Option<&Pane> {
        self.actual_pane_index()
            .and_then(|index| self.panes.get(index))
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

    pub fn nav_down(&mut self) {
        match self.focus {
            FocusArea::Sessions => {
                let len = self.visible_session_indices().len();
                next_item(&mut self.session_list_state, len);
                self.refresh_windows_and_panes(None, None);
            }
            FocusArea::Windows => {
                let len = self.visible_window_indices().len();
                next_item(&mut self.window_list_state, len);
                self.refresh_panes_only();
            }
            FocusArea::Panes => {
                let len = self.visible_pane_indices().len();
                next_item(&mut self.pane_list_state, len);
            }
        }
    }

    pub fn nav_up(&mut self) {
        match self.focus {
            FocusArea::Sessions => {
                let len = self.visible_session_indices().len();
                prev_item(&mut self.session_list_state, len);
                self.refresh_windows_and_panes(None, None);
            }
            FocusArea::Windows => {
                let len = self.visible_window_indices().len();
                prev_item(&mut self.window_list_state, len);
                self.refresh_panes_only();
            }
            FocusArea::Panes => {
                let len = self.visible_pane_indices().len();
                prev_item(&mut self.pane_list_state, len);
            }
        }
    }

    pub fn nav_first(&mut self) {
        match self.focus {
            FocusArea::Sessions => {
                let len = self.visible_session_indices().len();
                select_first(&mut self.session_list_state, len);
                self.refresh_windows_and_panes(None, None);
            }
            FocusArea::Windows => {
                let len = self.visible_window_indices().len();
                select_first(&mut self.window_list_state, len);
                self.refresh_panes_only();
            }
            FocusArea::Panes => {
                let len = self.visible_pane_indices().len();
                select_first(&mut self.pane_list_state, len);
            }
        }
    }

    pub fn nav_last(&mut self) {
        match self.focus {
            FocusArea::Sessions => {
                let len = self.visible_session_indices().len();
                select_last(&mut self.session_list_state, len);
                self.refresh_windows_and_panes(None, None);
            }
            FocusArea::Windows => {
                let len = self.visible_window_indices().len();
                select_last(&mut self.window_list_state, len);
                self.refresh_panes_only();
            }
            FocusArea::Panes => {
                let len = self.visible_pane_indices().len();
                select_last(&mut self.pane_list_state, len);
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

    pub fn set_info_banner(&mut self, title: impl Into<String>, body: impl Into<String>) {
        self.banner = BannerState::info(
            truncate_text(&title.into(), 60),
            truncate_text(&body.into(), 180),
        );
    }

    pub fn set_success_banner(&mut self, title: impl Into<String>, body: impl Into<String>) {
        self.banner = BannerState::success(
            truncate_text(&title.into(), 60),
            truncate_text(&body.into(), 180),
        );
    }

    pub fn set_warning_banner(&mut self, title: impl Into<String>, body: impl Into<String>) {
        self.banner = BannerState::warning(
            truncate_text(&title.into(), 60),
            truncate_text(&body.into(), 180),
        );
    }

    pub fn set_error_banner(&mut self, title: impl Into<String>, body: impl Into<String>) {
        self.banner = BannerState::error(
            truncate_text(&title.into(), 60),
            truncate_text(&body.into(), 180),
        );
    }

    pub fn open_filter(&mut self) {
        if self.filter.target != self.focus {
            self.filter.query.clear();
        }
        self.filter.target = self.focus;
        self.filter.active = true;
        self.reconcile_after_filter_change();
    }

    pub fn close_filter(&mut self) {
        self.filter.active = false;
    }

    pub fn clear_filter(&mut self) {
        self.filter.query.clear();
        self.filter.active = false;
        self.reconcile_after_filter_change();
    }

    pub fn clear_filter_for(&mut self, focus: FocusArea) {
        if self.filter.target == focus {
            self.clear_filter();
        }
    }

    pub fn push_filter_char(&mut self, character: char) {
        self.filter.query.push(character);
        self.reconcile_after_filter_change();
    }

    pub fn pop_filter_char(&mut self) {
        self.filter.query.pop();
        self.reconcile_after_filter_change();
    }

    pub fn open_input_modal(&mut self, intent: InputIntent, value: impl Into<String>) {
        self.modal = ModalState::Input(InputModalState {
            intent,
            value: value.into(),
            error: None,
        });
    }

    pub fn open_confirm_modal(&mut self, intent: ConfirmIntent) {
        self.modal = ModalState::Confirm(ConfirmModalState {
            intent,
            error: None,
        });
    }

    pub fn close_modal(&mut self) {
        self.modal = ModalState::None;
    }

    pub fn set_modal_error(&mut self, message: impl Into<String>) {
        let message = truncate_text(&message.into(), 140);
        match &mut self.modal {
            ModalState::Input(modal) => modal.error = Some(message),
            ModalState::Confirm(modal) => modal.error = Some(message),
            ModalState::None => self.set_error_banner("Action failed", message),
        }
    }

    pub fn input_modal_mut(&mut self) -> Option<&mut InputModalState> {
        match &mut self.modal {
            ModalState::Input(modal) => Some(modal),
            _ => None,
        }
    }

    pub fn input_modal(&self) -> Option<&InputModalState> {
        match &self.modal {
            ModalState::Input(modal) => Some(modal),
            _ => None,
        }
    }

    pub fn confirm_modal(&self) -> Option<&ConfirmModalState> {
        match &self.modal {
            ModalState::Confirm(modal) => Some(modal),
            _ => None,
        }
    }

    pub fn filter_summary(&self) -> Option<String> {
        if self.filter.has_query() {
            Some(format!(
                "{}: {}",
                self.filter.target.title(),
                truncate_text(self.filter.query.trim(), 28)
            ))
        } else {
            None
        }
    }

    pub fn connection_label(&self) -> &'static str {
        match self.connection {
            TmuxConnectionState::Connected => "Connected",
            TmuxConnectionState::NoServer => "No tmux server",
            TmuxConnectionState::Missing => "tmux missing",
            TmuxConnectionState::CommandFailed => "Command failed",
        }
    }

    pub fn action_availability(&self) -> ActionAvailability {
        match self.focus {
            FocusArea::Sessions => ActionAvailability {
                attach: action_item(
                    "Enter",
                    "Attach",
                    self.get_selected_session().is_some(),
                    if self.get_selected_session().is_some() {
                        "Open the selected session."
                    } else {
                        "Select a session to attach."
                    },
                ),
                create: action_item("n", "New session", true, "Create a new tmux session."),
                rename: action_item(
                    "R",
                    "Rename",
                    self.get_selected_session().is_some(),
                    if self.get_selected_session().is_some() {
                        "Rename the selected session."
                    } else {
                        "Select a session before renaming."
                    },
                ),
                delete: action_item(
                    "d",
                    "Delete",
                    self.get_selected_session().is_some(),
                    if self.get_selected_session().is_some() {
                        "Delete the selected session."
                    } else {
                        "Select a session before deleting."
                    },
                ),
            },
            FocusArea::Windows => {
                let session_selected = self.get_selected_session().is_some();
                let window_selected = self.get_selected_window().is_some();
                ActionAvailability {
                    attach: action_item(
                        "Enter",
                        "Attach",
                        window_selected,
                        if window_selected {
                            "Attach to the selected window."
                        } else {
                            "Select a window to attach."
                        },
                    ),
                    create: action_item(
                        "n",
                        "New window",
                        session_selected,
                        if session_selected {
                            "Create a window in the selected session."
                        } else {
                            "Select a session before creating a window."
                        },
                    ),
                    rename: action_item(
                        "R",
                        "Rename",
                        window_selected,
                        if window_selected {
                            "Rename the selected window."
                        } else {
                            "Select a window before renaming."
                        },
                    ),
                    delete: action_item(
                        "d",
                        "Delete",
                        window_selected,
                        if window_selected {
                            "Delete the selected window."
                        } else {
                            "Select a window before deleting."
                        },
                    ),
                }
            }
            FocusArea::Panes => {
                let pane_selected = self.get_selected_pane().is_some();
                ActionAvailability {
                    attach: action_item(
                        "Enter",
                        "Attach",
                        pane_selected,
                        if pane_selected {
                            "Attach to the selected pane."
                        } else {
                            "Select a pane to attach."
                        },
                    ),
                    create: action_item(
                        "n",
                        "Split pane",
                        pane_selected,
                        if pane_selected {
                            "Split the selected pane."
                        } else {
                            "Select a pane before splitting."
                        },
                    ),
                    rename: action_item(
                        "R",
                        "Rename",
                        false,
                        "Pane names are managed inside tmux.",
                    ),
                    delete: action_item(
                        "d",
                        "Delete",
                        pane_selected,
                        if pane_selected {
                            "Delete the selected pane."
                        } else {
                            "Select a pane before deleting."
                        },
                    ),
                }
            }
        }
    }

    pub fn select_session_by_name(&mut self, name: &str) -> bool {
        self.clear_filter_for(FocusArea::Sessions);
        let index = self
            .sessions
            .iter()
            .position(|session| session.name == name);
        let selected = self.select_session_by_actual_index(index);
        if selected {
            self.refresh_windows_and_panes(None, None);
        }
        selected
    }

    pub fn select_window_by_name(&mut self, name: &str) -> bool {
        self.clear_filter_for(FocusArea::Windows);
        let index = self.windows.iter().position(|window| window.name == name);
        let selected = self.select_window_by_actual_index(index);
        if selected {
            self.refresh_panes_only();
        }
        selected
    }

    pub(crate) fn visible_session_indices(&self) -> Vec<usize> {
        visible_indices(
            &self.sessions,
            self.active_query(FocusArea::Sessions),
            |session| format!("{} {} {}", session.name, session.id, session.created),
        )
    }

    pub(crate) fn visible_window_indices(&self) -> Vec<usize> {
        visible_indices(
            &self.windows,
            self.active_query(FocusArea::Windows),
            |window| format!("{} {} {}", window.name, window.id, window.layout),
        )
    }

    pub(crate) fn visible_pane_indices(&self) -> Vec<usize> {
        visible_indices(&self.panes, self.active_query(FocusArea::Panes), |pane| {
            format!("{} {} {}", pane.id, pane.current_command, pane.current_path)
        })
    }

    fn actual_session_index(&self) -> Option<usize> {
        selected_visible_index(
            self.session_list_state.selected(),
            &self.visible_session_indices(),
        )
    }

    fn actual_window_index(&self) -> Option<usize> {
        selected_visible_index(
            self.window_list_state.selected(),
            &self.visible_window_indices(),
        )
    }

    fn actual_pane_index(&self) -> Option<usize> {
        selected_visible_index(
            self.pane_list_state.selected(),
            &self.visible_pane_indices(),
        )
    }

    fn active_query(&self, focus: FocusArea) -> Option<&str> {
        if self.filter.target == focus && self.filter.has_query() {
            Some(self.filter.query.trim())
        } else {
            None
        }
    }

    fn select_session_by_actual_index(&mut self, actual_index: Option<usize>) -> bool {
        let visible = self.visible_session_indices();
        if let Some(actual_index) = actual_index {
            if let Some(visible_index) = visible.iter().position(|index| *index == actual_index) {
                self.session_list_state.select(Some(visible_index));
                return true;
            }
        }
        false
    }

    fn select_window_by_actual_index(&mut self, actual_index: Option<usize>) -> bool {
        let visible = self.visible_window_indices();
        if let Some(actual_index) = actual_index {
            if let Some(visible_index) = visible.iter().position(|index| *index == actual_index) {
                self.window_list_state.select(Some(visible_index));
                return true;
            }
        }
        false
    }

    fn reconcile_after_filter_change(&mut self) {
        let selected_session_id = self
            .get_selected_session()
            .map(|session| session.id.clone());
        let selected_window_id = self.get_selected_window().map(|window| window.id.clone());
        let selected_pane_id = self.get_selected_pane().map(|pane| pane.id.clone());

        match self.filter.target {
            FocusArea::Sessions => {
                self.sync_session_selection(selected_session_id.as_deref());
                self.refresh_windows_and_panes(
                    selected_window_id.as_deref(),
                    selected_pane_id.as_deref(),
                );
            }
            FocusArea::Windows => {
                self.sync_window_selection(selected_window_id.as_deref());
                self.refresh_panes(selected_pane_id.as_deref());
            }
            FocusArea::Panes => self.sync_pane_selection(selected_pane_id.as_deref()),
        }
    }

    fn sync_session_selection(&mut self, preferred_id: Option<&str>) {
        let visible = self.visible_session_indices();
        select_matching_visible(
            &mut self.session_list_state,
            &self.sessions,
            &visible,
            preferred_id,
            |session| session.id.as_str(),
        );
    }

    fn sync_window_selection(&mut self, preferred_id: Option<&str>) {
        let visible = self.visible_window_indices();
        select_matching_visible(
            &mut self.window_list_state,
            &self.windows,
            &visible,
            preferred_id,
            |window| window.id.as_str(),
        );
    }

    fn sync_pane_selection(&mut self, preferred_id: Option<&str>) {
        let visible = self.visible_pane_indices();
        select_matching_visible(
            &mut self.pane_list_state,
            &self.panes,
            &visible,
            preferred_id,
            |pane| pane.id.as_str(),
        );
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
                self.sync_window_selection(selected_window_id);
            }
            Err(err) => {
                self.connection = TmuxConnectionState::CommandFailed;
                self.connection_detail = Some(err.to_string());
                self.clear_windows();
                self.clear_panes();
                self.set_error_banner("Could not load windows", err.to_string());
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
                self.sync_pane_selection(selected_pane_id);
            }
            Err(err) => {
                self.connection = TmuxConnectionState::CommandFailed;
                self.connection_detail = Some(err.to_string());
                self.clear_panes();
                self.set_error_banner("Could not load panes", err.to_string());
            }
        }
    }

    fn clear_windows(&mut self) {
        self.windows.clear();
        self.window_list_state.select(None);
    }

    fn clear_panes(&mut self) {
        self.panes.clear();
        self.pane_list_state.select(None);
    }

    fn reset_banner_for_current_state(&mut self) {
        match self.connection {
            TmuxConnectionState::Connected => {
                if self.sessions.is_empty() {
                    self.set_info_banner(
                        "No sessions yet",
                        "Press n to create your first tmux session.",
                    );
                } else if self.filter.has_query() {
                    self.set_info_banner(
                        "Filter active",
                        format!(
                            "Showing matches in {}. Press / to edit the filter or Esc in the filter to clear it.",
                            self.filter.target.title().to_ascii_lowercase()
                        ),
                    );
                } else {
                    self.set_info_banner(
                        "Connected",
                        "Tab moves focus, Enter attaches, and ? shows every shortcut.",
                    );
                }
            }
            TmuxConnectionState::NoServer => {
                self.set_warning_banner(
                    "No tmux server",
                    "Press n to create your first session and start tmux.",
                );
            }
            TmuxConnectionState::Missing => {
                self.set_error_banner("tmux not installed", "Install tmux, then restart tmuxui.");
            }
            TmuxConnectionState::CommandFailed => {
                let detail = self
                    .connection_detail
                    .as_deref()
                    .unwrap_or("tmux returned an unexpected error.")
                    .to_string();
                self.set_error_banner("Command failed", detail);
            }
        }
    }
}

fn action_item(
    key: &'static str,
    label: &'static str,
    enabled: bool,
    reason: impl Into<String>,
) -> ActionItem {
    ActionItem {
        key,
        label,
        enabled,
        reason: reason.into(),
    }
}

fn visible_indices<T, F>(items: &[T], query: Option<&str>, make_text: F) -> Vec<usize>
where
    F: Fn(&T) -> String,
{
    match query {
        Some(query) => {
            let needle = query.to_ascii_lowercase();
            items
                .iter()
                .enumerate()
                .filter(|(_, item)| make_text(item).to_ascii_lowercase().contains(&needle))
                .map(|(index, _)| index)
                .collect()
        }
        None => (0..items.len()).collect(),
    }
}

fn selected_visible_index(selected: Option<usize>, visible_indices: &[usize]) -> Option<usize> {
    selected.and_then(|index| visible_indices.get(index).copied())
}

fn next_item(state: &mut ListState, len: usize) {
    if len == 0 {
        state.select(None);
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
        state.select(None);
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

fn select_matching_visible<T, F>(
    state: &mut ListState,
    items: &[T],
    visible_indices: &[usize],
    preferred_id: Option<&str>,
    get_id: F,
) where
    F: Fn(&T) -> &str,
{
    if visible_indices.is_empty() {
        state.select(None);
        return;
    }

    if let Some(preferred_id) = preferred_id {
        if let Some(visible_index) = visible_indices
            .iter()
            .position(|index| get_id(&items[*index]) == preferred_id)
        {
            state.select(Some(visible_index));
            return;
        }
    }

    state.select(Some(0));
}

fn truncate_text(text: &str, limit: usize) -> String {
    let chars: Vec<char> = text.chars().collect();
    if chars.len() <= limit {
        return text.to_string();
    }

    let truncated: String = chars.into_iter().take(limit.saturating_sub(3)).collect();
    format!("{truncated}...")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_app() -> App {
        let mut app = App {
            sessions: vec![
                Session {
                    id: "%0".to_string(),
                    name: "dev".to_string(),
                    window_count: 2,
                    created: "Sun Apr 19 12:00:00 2026".to_string(),
                },
                Session {
                    id: "%1".to_string(),
                    name: "ops".to_string(),
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
            panes: vec![
                Pane {
                    id: "%10".to_string(),
                    width: 120,
                    height: 30,
                    current_path: "/tmp/project".to_string(),
                    current_command: "nvim".to_string(),
                    active: true,
                },
                Pane {
                    id: "%11".to_string(),
                    width: 120,
                    height: 30,
                    current_path: "/tmp/project".to_string(),
                    current_command: "cargo test".to_string(),
                    active: false,
                },
            ],
            ..App::default()
        };
        app.session_list_state.select(Some(0));
        app.window_list_state.select(Some(0));
        app.pane_list_state.select(Some(0));
        app.connection = TmuxConnectionState::Connected;
        app
    }

    #[test]
    fn selection_wraps_in_both_directions() {
        let mut state = ListState::default();

        next_item(&mut state, 3);
        assert_eq!(state.selected(), Some(0));

        prev_item(&mut state, 3);
        assert_eq!(state.selected(), Some(2));
    }

    #[test]
    fn filtering_reselects_the_first_visible_session() {
        let mut app = sample_app();

        app.filter.target = FocusArea::Sessions;
        app.filter.query = "ops".to_string();
        app.reconcile_after_filter_change();

        assert_eq!(app.visible_session_indices(), vec![1]);
        assert_eq!(app.selected_session_name(), Some("ops"));
    }

    #[test]
    fn action_availability_explains_missing_window_selection() {
        let mut app = sample_app();
        app.focus = FocusArea::Windows;
        app.window_list_state.select(None);

        let actions = app.action_availability();

        assert!(!actions.attach.enabled);
        assert_eq!(actions.attach.reason, "Select a window to attach.");
    }

    #[test]
    fn selecting_session_by_name_clears_session_filter() {
        let mut app = sample_app();
        app.filter.target = FocusArea::Sessions;
        app.filter.query = "dev".to_string();
        app.filter.active = true;

        let selected = app.select_session_by_name("ops");

        assert!(selected);
        assert_eq!(app.selected_session_name(), Some("ops"));
        assert!(app.filter.query.is_empty());
        assert!(!app.filter.active);
    }

    #[test]
    fn truncates_banner_copy_for_safe_rendering() {
        let long = "x".repeat(220);
        let truncated = truncate_text(&long, 120);

        assert!(truncated.len() <= 120);
        assert!(truncated.ends_with("..."));
    }
}
