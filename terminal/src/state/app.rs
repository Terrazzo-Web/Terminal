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

    #[cfg(feature = "converter")]
    #[cfg_attr(not(any(feature = "terminal", feature = "text-editor")), default)]
    #[cfg_attr(not(feature = "diagnostics"), serde(rename = "C"))]
    Converter,
}

impl std::fmt::Display for App {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            #[cfg(feature = "terminal")]
            App::Terminal => "Terminal",
            #[cfg(feature = "text-editor")]
            App::TextEditor => "Text editor",
            #[cfg(feature = "converter")]
            App::Converter => "Converter",
        }
        .fmt(f)
    }
}

make_state!(state, App);
