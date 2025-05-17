use std::path::Path;
use std::path::PathBuf;

use nameth::NamedEnumValues as _;
use nameth::nameth;

use super::ConfigFile;
use super::types::ConfigFileTypes;
use super::types::RuntimeTypes;

impl ConfigFile<ConfigFileTypes> {
    pub fn load(path: impl AsRef<Path>) -> Result<Self, ConfigFileError> {
        let content =
            std::fs::read_to_string(path.as_ref()).map_err(|error| ConfigFileError::IO {
                config_file: path.as_ref().to_owned(),
                error,
            })?;
        if content.is_empty() {
            return Ok(Self::default());
        }
        return toml::from_str(&content).map_err(|error| ConfigFileError::Deserialize {
            config_file: path.as_ref().to_owned(),
            error,
        });
    }
}

impl ConfigFile<RuntimeTypes> {
    pub fn save(&self, path: impl AsRef<Path>) -> Result<(), ConfigFileError> {
        let json = toml::to_string_pretty(self)?;
        std::fs::write(path.as_ref(), &json).map_err(|error| ConfigFileError::IO {
            config_file: path.as_ref().to_owned(),
            error,
        })?;
        Ok(())
    }
}

#[nameth]
#[derive(thiserror::Error, Debug)]
pub enum ConfigFileError {
    #[error("[{n}] Failed to read config file {config_file:?}: {error}", n = self.name())]
    IO {
        config_file: PathBuf,
        error: std::io::Error,
    },

    #[error("[{n}] Failed to parse config file {config_file:?}: {error}", n = self.name())]
    Deserialize {
        config_file: PathBuf,
        error: toml::de::Error,
    },

    #[error("[{n}] Failed to serialize config file: {0}", n = self.name())]
    Serialize(#[from] toml::ser::Error),
}
