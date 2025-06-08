use nameth::nameth;

mod service;
pub mod ui;

#[nameth]
#[derive(Debug, serde::Serialize, serde::Deserialize, PartialEq, Eq, Clone, Copy)]
pub enum PathSelector {
    BasePath,
    FilePath,
}
