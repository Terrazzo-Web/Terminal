use std::ops::Deref;
use std::sync::Arc;

use nameth::nameth;
use tracing::info;
use tracing::warn;
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

use super::cli::Cli;

#[nameth]
pub struct AgentTunnelConfig {
    client_config: AgentClientConfig,
    client_certificate: Arc<PemCertificate>,
    retry_strategy: RetryStrategy,
}

#[nameth]
pub struct AgentClientConfig {
    gateway_url: String,
    gateway_pki: CachedTrustedStoreConfig,
    client_name: ClientName,
}

const CLIENT_CERTIFICATE_FILE_SUFFIX: CertificateInfo<&str> = CertificateInfo {
    certificate: ".cert",
    private_key: ".key",
};

impl AgentTunnelConfig {
    pub async fn new(cli: &Cli) -> Option<Self> {
        let client_config = AgentClientConfig {
            gateway_url: cli.gateway_url.clone()?,
            gateway_pki: LoadTrustedStore::PEM(cli.gateway_pki.clone()?)
                .load()
                .inspect_err(|error| warn!("Failed to load Gateway PKI: {error}"))
                .ok()?,
            client_name: cli.client_name.as_deref()?.into(),
        };
        let client_certificate = Arc::new(
            load_client_certificate(
                &client_config,
                cli.auth_code.as_str().into(),
                CLIENT_CERTIFICATE_FILE_SUFFIX
                    .map(|suffix| format!("{}.{suffix})", cli.client_certificate)),
            )
            .await
            .inspect_err(|error| warn!("Failed to load Client Certificate: {error}"))
            .ok()?,
        );
        Some(Self {
            client_config,
            client_certificate,
            retry_strategy: RetryStrategy::default(),
        })
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
        |_server| {
            info!("Configuring Client gRPC service");
            todo!()
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
