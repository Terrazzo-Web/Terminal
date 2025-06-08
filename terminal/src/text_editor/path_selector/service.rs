#![cfg(feature = "server")]

use std::fs::Metadata;

use super::PathSelector;

impl PathSelector {
    pub fn accept(self, metadata: &Metadata) -> bool {
        match self {
            Self::BasePath => metadata.is_dir(),
            Self::FilePath => true,
        }
    }
}
