use std::ops::Deref;
use std::sync::Arc;

use nameth::nameth;
use tracing::Instrument as _;
use tracing::info;
use tracing::info_span;
use tracing::warn;
use trz_gateway_client::client::AuthCode;
use trz_gateway_client::client_config::ClientConfig;
use trz_gateway_client::client_service::ClientService;
use trz_gateway_client::load_client_certificate::load_client_certificate;
use trz_gateway_client::retry_strategy::RetryStrategy;
use trz_gateway_client::tunnel_config::TunnelConfig;
use trz_gateway_common::certificate_info::CertificateInfo;
use trz_gateway_common::id::ClientName;
use trz_gateway_common::security_configuration::certificate::pem::PemCertificate;
use trz_gateway_common::security_configuration::trusted_store::cache::CachedTrustedStoreConfig;
use trz_gateway_common::security_configuration::trusted_store::load::LoadTrustedStore;
use trz_gateway_server::server::Server;

use super::config::mesh::MeshConfig;
use crate::backend::client_service::ClientServiceImpl;
use crate::backend::protos::terrazzo::gateway::client::client_service_server::ClientServiceServer;

#[nameth]
pub struct AgentTunnelConfig {
    client_config: AgentClientConfig,
    client_certificate: Arc<PemCertificate>,
    retry_strategy: RetryStrategy,
    server: Arc<Server>,
}

#[nameth]
pub struct AgentClientConfig {
    client_name: ClientName,
    gateway_url: String,
    gateway_pki: CachedTrustedStoreConfig,
}

const CLIENT_CERTIFICATE_FILE_SUFFIX: CertificateInfo<&str> = CertificateInfo {
    certificate: "cert",
    private_key: "key",
};

impl AgentTunnelConfig {
    pub async fn new(auth_code: AuthCode, mesh: &MeshConfig, server: &Arc<Server>) -> Option<Self> {
        async move {
            let client_name = mesh.client_name.as_str().into();
            let gateway_url = mesh.gateway_url.clone();

            let gateway_pki = mesh
                .gateway_pki
                .as_deref()
                .map(LoadTrustedStore::File)
                .unwrap_or(LoadTrustedStore::Native);

            let client_config = AgentClientConfig {
                gateway_url,
                gateway_pki: gateway_pki
                    .load()
                    .inspect_err(|error| warn!("Failed to load Gateway PKI: {error}"))
                    .ok()?,
                client_name,
            };

            let client_certificate = Arc::new(
                load_client_certificate(
                    &client_config,
                    auth_code,
                    CLIENT_CERTIFICATE_FILE_SUFFIX
                        .map(|suffix| format!("{}.{suffix}", mesh.client_certificate)),
                )
                .await
                .inspect_err(|error| warn!("Failed to load Client Certificate: {error}"))
                .ok()?,
            );

            Some(Self {
                client_config,
                client_certificate,
                retry_strategy: RetryStrategy::default(),
                server: server.clone(),
            })
        }
        .instrument(info_span!("Agent tunnel config"))
        .await
    }
}

impl ClientConfig for AgentTunnelConfig {
    type GatewayPki = CachedTrustedStoreConfig;
    fn gateway_pki(&self) -> Self::GatewayPki {
        self.client_config.gateway_pki()
    }

    fn base_url(&self) -> impl std::fmt::Display {
        self.client_config.base_url()
    }

    fn client_name(&self) -> ClientName {
        self.client_config.client_name()
    }
}

impl TunnelConfig for AgentTunnelConfig {
    type ClientCertificate = Arc<PemCertificate>;
    fn client_certificate(&self) -> Self::ClientCertificate {
        self.client_certificate.clone()
    }

    fn client_service(&self) -> impl ClientService {
        let client_name = self.client_name();
        let gateway_server = self.server.clone();
        move |mut server: tonic::transport::Server| {
            info!("Configuring Client gRPC service");
            server.add_service(ClientServiceServer::new(ClientServiceImpl::new(
                client_name.clone(),
                gateway_server.clone(),
            )))
        }
    }

    fn retry_strategy(&self) -> RetryStrategy {
        self.retry_strategy.clone()
    }
}

impl Deref for AgentTunnelConfig {
    type Target = AgentClientConfig;

    fn deref(&self) -> &Self::Target {
        &self.client_config
    }
}

impl ClientConfig for AgentClientConfig {
    type GatewayPki = CachedTrustedStoreConfig;
    fn gateway_pki(&self) -> Self::GatewayPki {
        self.gateway_pki.clone()
    }

    fn base_url(&self) -> impl std::fmt::Display {
        &self.gateway_url
    }

    fn client_name(&self) -> ClientName {
        self.client_name.clone()
    }
}

mod debug {
    use std::fmt::Debug;

    use nameth::NamedType as _;

    use super::AgentClientConfig;
    use super::AgentTunnelConfig;

    impl Debug for AgentClientConfig {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            f.debug_struct(AgentClientConfig::type_name())
                .field("gateway_url", &self.gateway_url)
                .field("client_name", &self.client_name)
                .finish()
        }
    }

    impl Debug for AgentTunnelConfig {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            f.debug_struct(AgentTunnelConfig::type_name())
                .field("gateway_url", &self.gateway_url)
                .field("client_name", &self.client_name)
                .field("client_certificate", &self.client_certificate)
                .field("retry_strategy", &self.retry_strategy)
                .finish()
        }
    }
}
