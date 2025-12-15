#[derive(Clone, Debug)]
pub struct Session {
    pub id: String,
    pub name: String,
    pub count: String,
    pub created: String,
}

#[derive(Clone, Debug)]
pub struct Window {
    pub id: String,
    pub name: String,
    pub active: bool,
    pub layout: String,
}

#[derive(Clone, Debug)]
pub struct Pane {
    pub id: String,
    pub width: String,
    pub height: String,
    pub current_path: String,
    pub current_command: String,
    pub active: bool,
}