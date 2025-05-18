use std::fmt::Debug;

use serde::Deserialize;
use serde::Serialize;

use self::types::ConfigTypes;
use self::types::RuntimeTypes;

pub(in crate::backend) mod io;
pub(in crate::backend) mod kill;
mod merge;
pub(in crate::backend) mod password;
pub(in crate::backend) mod pidfile;
pub mod types;

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct ConfigFile<T: ConfigTypes = RuntimeTypes> {
    pub server: ServerConfig<T>,
    pub mesh: Option<MeshConfig<T>>,
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct ServerConfig<T: ConfigTypes = RuntimeTypes> {
    /// The TCP host to listen to.
    pub host: T::String,

    /// The TCP port to listen to.
    pub port: T::Port,

    /// The file to store the pid of the daemon while it is running,
    pub pidfile: T::String,

    /// The file to the store private Root CA.
    pub private_root_ca: T::String,

    /// The password to login to the UI.
    pub password: T::Password,
    pub token_cookie_lifetime: T::Duration,
    pub token_cookie_refresh: T::Duration,
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct MeshConfig<T: ConfigTypes = RuntimeTypes> {
    /// The Client name.
    pub client_name: T::String,

    /// The Gateway endpoint.
    pub gateway_url: T::String,

    /// The Gateway CA.
    ///
    /// This is the Root CA of the Gateway server certificate.
    pub gateway_pki: T::MaybeString,

    /// The file to store the client certificate.
    pub client_certificate: T::String,
}
