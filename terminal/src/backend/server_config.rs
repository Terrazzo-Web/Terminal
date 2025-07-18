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
use trz_gateway_common::dynamic_config::DynamicConfig;
use trz_gateway_common::dynamic_config::has_diff::DiffArc;
use trz_gateway_common::dynamic_config::mode::RO;
use trz_gateway_common::security_configuration::SecurityConfig;
use trz_gateway_common::security_configuration::certificate::cache::CachedCertificate;
use trz_gateway_common::security_configuration::certificate::dynamic::DynamicCertificate;
use trz_gateway_common::security_configuration::either::EitherConfig;
use trz_gateway_common::security_configuration::trusted_store::native::NativeTrustedStoreConfig;
use trz_gateway_server::server::Server;
use trz_gateway_server::server::acme::active_challenges::ActiveChallenges;
use trz_gateway_server::server::acme::certificate_config::AcmeCertificateConfig;
use trz_gateway_server::server::gateway_config::GatewayConfig;
use trz_gateway_server::server::gateway_config::app_config::AppConfig;

use super::auth::AuthConfig;
use super::auth::layer::AuthLayer;
use super::config::DynConfig;
use super::root_ca_config::PrivateRootCa;
use crate::api;

#[nameth]
pub struct TerminalBackendServer {
    pub config: DiffArc<DynConfig>,

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
    ///
    /// Either way the certificate used by the Terrazzo Gateway to [sign](GatewayConfig::client_certificate_issuer)
    /// the extension uses the same PKI as the [TLS server](GatewayConfig::tls) certificate.
    pub tls_config: std::sync::Arc<DynamicConfig<TlsConfig, RO>>,

    /// Configuration for authentication
    pub auth_config: DiffArc<DynamicConfig<DiffArc<AuthConfig>, RO>>,

    pub active_challenges: ActiveChallenges,
}

type TlsConfig = EitherConfig<
    SecurityConfig<PrivateRootCa, CachedCertificate>,
    SecurityConfig<NativeTrustedStoreConfig, AcmeCertificateConfig>,
>;

impl GatewayConfig for TerminalBackendServer {
    type RootCaConfig = PrivateRootCa;
    fn root_ca(&self) -> Self::RootCaConfig {
        self.root_ca.clone()
    }

    type TlsConfig = Arc<DynamicCertificate<TlsConfig, RO>>;
    fn tls(&self) -> Self::TlsConfig {
        Arc::new(DynamicCertificate::from(self.tls_config.clone()))
    }

    type ClientCertificateIssuerConfig = Arc<DynamicConfig<TlsConfig, RO>>;
    fn client_certificate_issuer(&self) -> Self::ClientCertificateIssuerConfig {
        self.tls_config.clone()
    }

    fn enable_tracing(&self) -> bool {
        true
    }

    fn host(&self) -> String {
        self.config.server.with(|server| server.host.to_owned())
    }

    fn port(&self) -> u16 {
        self.config.server.with(|server| server.port)
    }

    fn app_config(&self) -> impl AppConfig {
        let config = self.config.clone();
        let auth_config = self.auth_config.clone();
        let active_challenges = self.active_challenges.clone();
        move |server: Arc<Server>, router: Router| {
            let router = router
                .route("/", get(|| static_assets::get("index.html")))
                .route(
                    "/static/{*file}",
                    get(|Path(path): Path<String>| static_assets::get(&path)),
                )
                .nest_service(
                    "/api",
                    api::server::api_routes(&config, &auth_config, &server),
                )
                .merge(active_challenges.route())
                .merge(
                    Router::new()
                        .route(
                            "/api/fn/{path}",
                            get(server_fn::axum::handle_server_fn)
                                .post(server_fn::axum::handle_server_fn)
                                .patch(server_fn::axum::handle_server_fn)
                                .put(server_fn::axum::handle_server_fn),
                        )
                        .route_layer(AuthLayer {
                            auth_config: auth_config.clone(),
                        }),
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
    use trz_gateway_server::server::gateway_config::GatewayConfig as _;

    use super::TerminalBackendServer;

    impl Debug for TerminalBackendServer {
        fn fmt(&self, f: &mut Formatter<'_>) -> Result {
            f.debug_struct(TerminalBackendServer::type_name())
                .field("host", &self.host())
                .field("port", &self.port())
                .finish()
        }
    }
}
