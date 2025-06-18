#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum App {
    #[default]
    Terminal,
    TextEditor,
}

impl std::fmt::Display for App {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            App::Terminal => "Terminal",
            App::TextEditor => "Text editor",
        }
        .fmt(f)
    }
}

make_state!(state, App);
