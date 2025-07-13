use super::make_state::make_state;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum App {
    #[cfg(feature = "terminal")]
    #[default]
    #[cfg_attr(not(feature = "diagnostics"), serde(rename = "T"))]
    Terminal,

    #[cfg(feature = "text-editor")]
    #[cfg_attr(not(feature = "terminal"), default)]
    #[cfg_attr(not(feature = "diagnostics"), serde(rename = "E"))]
    TextEditor,
}

impl std::fmt::Display for App {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            #[cfg(feature = "terminal")]
            App::Terminal => "Terminal",
            #[cfg(feature = "text-editor")]
            App::TextEditor => "Text editor",
        }
        .fmt(f)
    }
}

make_state!(state, App);
