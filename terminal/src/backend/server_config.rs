use std::iter::once;
use std::sync::Arc;

use nameth::nameth;
use terrazzo::axum::Router;
use terrazzo::axum::extract::Path;
use terrazzo::axum::routing::get;
use terrazzo::http::header::AUTHORIZATION;
use terrazzo::static_assets;
use tower_http::sensitive_headers::SetSensitiveRequestHeadersLayer;
use tower_http::trace::TraceLayer;
use tracing::Level;
use tracing::enabled;
use trz_gateway_common::id::ClientName;
use trz_gateway_common::security_configuration::SecurityConfig;
use trz_gateway_common::security_configuration::certificate::cache::CachedCertificate;
use trz_gateway_server::server::Server;
use trz_gateway_server::server::gateway_config::GatewayConfig;
use trz_gateway_server::server::gateway_config::app_config::AppConfig;

use super::auth::AuthConfig;
use super::config_file::ConfigFile;
use super::root_ca_config::PrivateRootCa;
use crate::api;

#[nameth]
pub struct TerminalBackendServer {
    /// The client name is
    /// - Set as an environment variable [terrazzo_pty::TERRAZZO_CLIENT_NAME] in terminals
    /// - Allocate IDs of terminals
    pub client_name: Option<ClientName>,

    /// The TCP host to listen to.
    pub host: String,

    /// The TCP port to listen to.
    pub port: u16,

    /// The private Root CA is used to issue client certificates.
    /// But security relies on the signed extension.
    pub root_ca: PrivateRootCa,

    /// The TLS config is the external PKI:
    /// - A certificate used to
    ///     1. listen to HTTPS connection
    ///     2. sign the client certificate extension
    /// - A trusted store to validate the client certificate extension
    ///
    /// Terrazzo can run:
    /// - in air-gapped mode wiht a private Root CA, or
    /// - in public mode using the public PKI and Let's Encrypt certificates.
    pub tls_config: SecurityConfig<PrivateRootCa, CachedCertificate>,

    /// Configuration for authentication
    pub auth_config: Arc<AuthConfig>,

    /// Configuration file.
    /// (host, port and client_name are thus redundant, that's OK)
    pub config_file: Arc<ConfigFile>,
}

impl GatewayConfig for TerminalBackendServer {
    type RootCaConfig = PrivateRootCa;
    fn root_ca(&self) -> Self::RootCaConfig {
        self.root_ca.clone()
    }

    type TlsConfig = SecurityConfig<PrivateRootCa, CachedCertificate>;
    fn tls(&self) -> Self::TlsConfig {
        self.tls_config.clone()
    }

    type ClientCertificateIssuerConfig = Self::TlsConfig;
    fn client_certificate_issuer(&self) -> Self::ClientCertificateIssuerConfig {
        self.tls_config.clone()
    }

    fn enable_tracing(&self) -> bool {
        true
    }

    fn host(&self) -> &str {
        &self.host
    }

    fn port(&self) -> u16 {
        self.port
    }

    fn app_config(&self) -> impl AppConfig {
        let client_name = self.client_name.clone();
        let auth_config = self.auth_config.clone();
        let config_file = self.config_file.clone();
        move |server: Arc<Server>, router: Router| {
            let router = router
                .route("/", get(|| static_assets::get("index.html")))
                .route(
                    "/static/{*file}",
                    get(|Path(path): Path<String>| static_assets::get(&path)),
                )
                .nest_service(
                    "/api",
                    api::server::api_routes(&client_name, &auth_config, &config_file, &server),
                );
            let router = router.layer(SetSensitiveRequestHeadersLayer::new(once(AUTHORIZATION)));
            let router = if enabled!(Level::TRACE) {
                router.layer(TraceLayer::new_for_http())
            } else {
                router
            };
            return router;
        }
    }
}

mod debug {
    use std::fmt::Debug;
    use std::fmt::Formatter;
    use std::fmt::Result;

    use nameth::NamedType as _;

    use super::TerminalBackendServer;

    impl Debug for TerminalBackendServer {
        fn fmt(&self, f: &mut Formatter<'_>) -> Result {
            f.debug_struct(TerminalBackendServer::type_name())
                .field("host", &self.host)
                .field("port", &self.port)
                .finish()
        }
    }
}
