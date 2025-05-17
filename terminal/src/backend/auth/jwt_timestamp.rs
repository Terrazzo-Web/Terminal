use std::ops::Deref;
use std::ops::DerefMut;
use std::time::Duration;
use std::time::SystemTime;

use super::Claims;

/// Timestamp for JWT tokens, serialized as seconds since epoch.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Timestamp(SystemTime);

impl Deref for Timestamp {
    type Target = SystemTime;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for Timestamp {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl From<SystemTime> for Timestamp {
    fn from(value: SystemTime) -> Self {
        Self(value)
    }
}

impl From<Timestamp> for SystemTime {
    fn from(value: Timestamp) -> Self {
        value.0
    }
}

impl Claims<Duration> {
    pub fn into_timestamps(self) -> Claims<Timestamp> {
        let now = SystemTime::now();
        Claims {
            exp: (now + self.exp).into(),
            nbf: (now - self.nbf).into(),
        }
    }
}

impl serde::Serialize for Timestamp {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let duration: u64 = self
            .0
            .duration_since(std::time::UNIX_EPOCH)
            .map_err(serde::ser::Error::custom)?
            .as_secs();
        return duration.serialize(serializer);
    }
}

impl<'t> serde::Deserialize<'t> for Timestamp {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'t>,
    {
        let duration = Duration::from_secs(u64::deserialize(deserializer)?);
        Ok((std::time::UNIX_EPOCH + duration).into())
    }
}
