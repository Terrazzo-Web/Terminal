use std::path::Path;
use std::path::PathBuf;

use nameth::NamedEnumValues as _;
use nameth::nameth;

use super::ConfigFile;

impl ConfigFile {
    pub fn load(path: impl AsRef<Path>) -> Result<Self, ConfigFileError> {
        let path = path.as_ref();
        let content = if std::fs::exists(path).map_err(|error| ConfigFileError::IO {
            config_file: path.to_owned(),
            error,
        })? {
            std::fs::read_to_string(path).map_err(|error| ConfigFileError::IO {
                config_file: path.to_owned(),
                error,
            })?
        } else {
            String::default()
        };
        if content.is_empty() {
            return Ok(Self::default());
        }
        return toml::from_str(&content).map_err(|error| ConfigFileError::Deserialize {
            config_file: path.to_owned(),
            error: error.into(),
        });
    }
}

impl ConfigFile {
    pub fn save(&self, path: impl AsRef<Path>) -> Result<(), ConfigFileError> {
        let json = toml::to_string_pretty(self).map_err(Box::from)?;
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
        error: Box<toml::de::Error>,
    },

    #[error("[{n}] Failed to serialize config file: {0}", n = self.name())]
    Serialize(#[from] Box<toml::ser::Error>),
}
