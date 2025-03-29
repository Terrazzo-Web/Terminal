use std::iter::once;

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
use trz_gateway_server::server::gateway_config::GatewayConfig;
use trz_gateway_server::server::gateway_config::app_config::AppConfig;

use super::root_ca_config::PrivateRootCa;
use crate::api;

#[nameth]
pub struct TerminalBackendServer {
    pub client_name: Option<ClientName>,
    pub host: String,
    pub port: u16,
    pub root_ca: PrivateRootCa,
    pub tls_config: SecurityConfig<PrivateRootCa, CachedCertificate>,
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
        move |server, router: Router| {
            let router = router
                .route("/", get(|| static_assets::get("index.html")))
                .route(
                    "/static/{*file}",
                    get(|Path(path): Path<String>| static_assets::get(&path)),
                )
                .nest_service("/api", api::server::route(&client_name, &server));
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
