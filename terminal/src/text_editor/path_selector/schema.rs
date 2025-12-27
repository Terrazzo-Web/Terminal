use nameth::nameth;

#[nameth]
#[derive(Debug, serde::Serialize, serde::Deserialize, PartialEq, Eq, Clone, Copy)]
pub enum PathSelector {
    #[cfg_attr(not(feature = "diagnostics"), serde(rename = "B"))]
    BasePath,
    #[cfg_attr(not(feature = "diagnostics"), serde(rename = "F"))]
    FilePath,
}
