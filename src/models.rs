#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Session {
    pub id: String,
    pub name: String,
    pub window_count: usize,
    pub created: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Window {
    pub id: String,
    pub name: String,
    pub active: bool,
    pub layout: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Pane {
    pub id: String,
    pub width: u16,
    pub height: u16,
    pub current_path: String,
    pub current_command: String,
    pub active: bool,
}
