use std::path::Path;

use tracing::warn;

use super::ConfigFile;
use super::types::ConfigFileTypes;

impl ConfigFile<ConfigFileTypes> {
    pub fn load(path: impl AsRef<Path>) -> Option<Self> {
        let content = std::fs::read_to_string(path.as_ref())
            .inspect_err(|error| warn!("Failed to read config file {:?}: {error}", path.as_ref()))
            .ok()?;
        toml::from_str(&content)
            .inspect_err(|error| warn!("Failed to parse config file {:?}: {error}", path.as_ref()))
            .ok()
    }
}
