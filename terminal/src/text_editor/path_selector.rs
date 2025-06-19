use nameth::nameth;

mod service;
pub mod ui;

#[nameth]
#[derive(Debug, serde::Serialize, serde::Deserialize, PartialEq, Eq, Clone, Copy)]
pub enum PathSelector {
    #[cfg_attr(not(debug_assertions), serde(rename = "B"))]
    BasePath,
    #[cfg_attr(not(debug_assertions), serde(rename = "F"))]
    FilePath,
}
