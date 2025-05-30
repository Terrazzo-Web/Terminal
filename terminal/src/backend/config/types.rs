use std::fmt::Debug;
use std::marker::PhantomData;
use std::time::Duration;

use serde::Deserialize;
use serde::Serialize;
use trz_gateway_common::retry_strategy::RetryStrategy;

pub trait ConfigTypes: Clone {
    type String: Serialize + for<'t> Deserialize<'t> + Debug + Default;
    type MaybeString: Serialize + for<'t> Deserialize<'t> + Debug + Default;
    type Port: Serialize + for<'t> Deserialize<'t> + Debug + Default;
    type Duration: Serialize + for<'t> Deserialize<'t> + Debug + Default;
    type RetryStrategy: Serialize + for<'t> Deserialize<'t> + Debug + Default;
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct ConfigFileTypes<T = RuntimeTypes>(PhantomData<T>);

impl<T: ConfigTypes> ConfigTypes for ConfigFileTypes<T> {
    type String = Option<T::String>;
    type MaybeString = T::MaybeString;
    type Port = Option<T::Port>;
    type Duration = Option<String>;
    type RetryStrategy = Option<RetryStrategy>;
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct RuntimeTypes(PhantomData<()>);

impl ConfigTypes for RuntimeTypes {
    type String = String;
    type MaybeString = Option<String>;
    type Port = u16;
    type Duration = Duration;
    type RetryStrategy = RetryStrategy;
}

#[must_use]
#[derive(Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct Password {
    #[serde(with = "password_serde")]
    pub hash: Vec<u8>,

    pub iterations: u32,

    #[serde(with = "password_serde")]
    pub salt: Vec<u8>,
}

mod password_serde {
    use base64::Engine as _;
    use base64::engine::general_purpose;
    use serde::Deserialize;
    use serde::Deserializer;
    use serde::Serializer;

    pub fn serialize<S>(bytes: &Vec<u8>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let encoded = general_purpose::STANDARD_NO_PAD.encode(bytes);
        serializer.serialize_str(&encoded)
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Vec<u8>, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        general_purpose::STANDARD_NO_PAD
            .decode(&s)
            .map_err(serde::de::Error::custom)
    }
}

impl std::fmt::Debug for Password {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        use base64::Engine as _;
        use base64::engine::general_purpose;
        f.debug_struct("Password")
            .field("hash", &general_purpose::STANDARD_NO_PAD.encode(&self.hash))
            .field("iterations", &self.iterations)
            .field("salt", &general_purpose::STANDARD_NO_PAD.encode(&self.salt))
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::Password;

    #[test]
    fn serialize_deserialize() {
        let password = Password {
            hash: vec![1, 2, 3, 4],
            iterations: 42,
            salt: vec![11, 12, 13, 14],
        };
        let password = toml::ser::to_string(&password).unwrap();
        assert_eq!(
            "hash = \"AQIDBA\"\niterations = 42\nsalt = \"CwwNDg\"\n",
            password
        );

        let password: Password = toml::de::from_str(&password).unwrap();
        assert_eq!(password.hash, vec![1, 2, 3, 4]);
        assert_eq!(password.iterations, 42);
        assert_eq!(password.salt, vec![11, 12, 13, 14]);
    }
}
