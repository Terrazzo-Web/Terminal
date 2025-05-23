use std::sync::Arc;

use nameth::NamedEnumValues as _;
use nameth::nameth;
use pbkdf2::hmac::Hmac;
use pbkdf2::hmac::digest::InvalidLength;
use sha2::Sha256;

use super::ServerConfig;
use super::io::ConfigFileError;
use super::server::DynamicServerConfig;
use crate::backend::config_file::types::Password;

impl DynamicServerConfig {
    pub fn set_password(&self) -> Result<(), SetPasswordError> {
        let password = rpassword::prompt_password("Password: ")?;
        self.try_set(|server| {
            let password = server.hash_password(&password)?;
            Ok::<_, SetPasswordError>(Arc::new(ServerConfig {
                password: Some(password),
                ..server.as_ref().clone()
            }))
        });
        debug_assert!(matches!(self.get().verify_password(&password), Ok(())));
        Ok(())
    }
}

impl ServerConfig {
    fn hash_password(&self, password: &str) -> Result<Password, SetPasswordError> {
        let mut hash = [0u8; 20];
        let salt = uuid::Uuid::new_v4();
        let iterations = 60_000;
        let () = pbkdf2::pbkdf2::<Hmac<Sha256>>(
            password.as_bytes(),
            salt.as_bytes(),
            iterations,
            &mut hash,
        )?;
        Ok(Password {
            hash: hash.to_vec(),
            iterations,
            salt: salt.as_bytes().to_vec(),
        })
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
        let config_file = ServerConfig::default();
        config_file.hash_password("pa$$word").unwrap();
        assert!(matches!(config_file.verify_password("pa$$word"), Ok(())));
        assert!(matches!(
            config_file.verify_password("pa$$word2"),
            Err(VerifyPasswordError::InvalidPassword)
        ));
    }
}
