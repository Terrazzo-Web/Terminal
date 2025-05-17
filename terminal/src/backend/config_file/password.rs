use std::path::Path;

use nameth::NamedEnumValues as _;
use nameth::nameth;
use pbkdf2::hmac::Hmac;
use pbkdf2::hmac::digest::InvalidLength;
use sha2::Sha256;

use super::ConfigFile;
use super::ServerConfig;
use super::io::ConfigFileError;
use crate::backend::config_file::types::Password;

impl ConfigFile {
    pub fn set_password(
        mut self,
        config_file: Option<impl AsRef<Path>>,
    ) -> Result<(), SetPasswordError> {
        let Some(config_file) = config_file else {
            return Err(SetPasswordError::ConfigFile);
        };
        let password = rpassword::prompt_password("Password: ")?;
        self.server.hash_password(&password)?;
        debug_assert!(matches!(self.server.verify_password(&password), Ok(())));
        let () = self.save(config_file)?;
        Ok(())
    }
}

impl ServerConfig {
    fn hash_password(&mut self, password: &str) -> Result<(), SetPasswordError> {
        self.password = {
            let mut hash = [0u8; 20];
            let salt = uuid::Uuid::new_v4();
            let iterations = 60_000;
            let () = pbkdf2::pbkdf2::<Hmac<Sha256>>(
                password.as_bytes(),
                salt.as_bytes(),
                iterations,
                &mut hash,
            )?;
            Some(Password {
                hash: hash.to_vec(),
                iterations,
                salt: salt.as_bytes().to_vec(),
            })
        };
        Ok(())
    }

    pub fn verify_password(&self, password: &str) -> Result<(), VerifyPasswordError> {
        let Some(password_hash) = &self.password else {
            return Err(VerifyPasswordError::PasswordNotDefined);
        };
        let mut hash = [0u8; 20];
        let () = pbkdf2::pbkdf2::<Hmac<Sha256>>(
            password.as_bytes(),
            &password_hash.salt,
            password_hash.iterations,
            &mut hash,
        )?;
        if hash.as_slice() == password_hash.hash.as_slice() {
            Ok(())
        } else {
            Err(VerifyPasswordError::InvalidPassword)
        }
    }
}

#[nameth]
#[derive(thiserror::Error, Debug)]
pub enum SetPasswordError {
    #[error("[{n}] Missing configuration for the confg file path.", n = self.name())]
    ConfigFile,

    #[error("[{n}] Failed read password: {0}", n = self.name())]
    Prompt(#[from] std::io::Error),

    #[error("[{n}] Failed to save config file with password: {0}", n = self.name())]
    Save(#[from] ConfigFileError),

    #[error("[{n}] {0}", n = self.name())]
    Pbkdf2(#[from] InvalidLength),
}

#[nameth]
#[derive(thiserror::Error, Debug)]
pub enum VerifyPasswordError {
    #[error("[{n}] The password is not configured", n = self.name())]
    PasswordNotDefined,

    #[error("[{n}] The password doesn't match", n = self.name())]
    InvalidPassword,

    #[error("[{n}] {0}", n = self.name())]
    Pbkdf2(#[from] InvalidLength),
}

#[cfg(test)]
mod tests {
    use crate::backend::config_file::ServerConfig;
    use crate::backend::config_file::password::VerifyPasswordError;

    #[test]
    fn test_password() {
        let mut config_file = ServerConfig::default();
        config_file.hash_password("pa$$word").unwrap();
        assert!(matches!(config_file.verify_password("pa$$word"), Ok(())));
        assert!(matches!(
            config_file.verify_password("pa$$word2"),
            Err(VerifyPasswordError::InvalidPassword)
        ));
    }
}
