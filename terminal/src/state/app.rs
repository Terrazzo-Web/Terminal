#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum App {
    #[cfg(feature = "terminal")]
    #[default]
    #[cfg_attr(not(feature = "diagnostics"), serde(rename = "T"))]
    Terminal,

    #[cfg_attr(not(feature = "terminal"), default)]
    #[cfg_attr(not(feature = "diagnostics"), serde(rename = "E"))]
    TextEditor,

    #[cfg_attr(not(feature = "diagnostics"), serde(rename = "E"))]
    Converter,
}

impl std::fmt::Display for App {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            #[cfg(feature = "terminal")]
            App::Terminal => "Terminal",
            App::TextEditor => "Text editor",
            App::Converter => "Converter",
        }
        .fmt(f)
    }
}

make_state!(state, App);
