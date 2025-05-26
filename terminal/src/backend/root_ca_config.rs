use std::convert::Infallible;
use std::sync::Arc;

use nameth::NamedEnumValues as _;
use nameth::nameth;
use openssl::asn1::Asn1Time;
use openssl::x509::X509;
use openssl::x509::store::X509Store;
use trz_gateway_common::certificate_info::CertificateInfo;
use trz_gateway_common::certificate_info::X509CertificateInfo;
use trz_gateway_common::security_configuration::certificate::CertificateConfig;
use trz_gateway_common::security_configuration::certificate::as_trusted_store::AsTrustedStoreError;
use trz_gateway_common::security_configuration::certificate::as_trusted_store::as_trusted_store;
use trz_gateway_common::security_configuration::certificate::cache::CachedCertificate;
use trz_gateway_common::security_configuration::certificate::pem::PemCertificate;
use trz_gateway_common::security_configuration::trusted_store::TrustedStoreConfig;
use trz_gateway_common::x509::name::CertitficateName;
use trz_gateway_common::x509::validity::Validity;
use trz_gateway_server::server::root_ca_configuration;
use trz_gateway_server::server::root_ca_configuration::RootCaConfigError;

use super::config::DynConfig;

#[derive(Clone)]
pub struct PrivateRootCa(CachedCertificate);

impl PrivateRootCa {
    pub fn load(config: &DynConfig) -> Result<Self, PrivateRootCaError> {
        let server = &config.server;
        let root_ca = server
            .with(|server| {
                root_ca_configuration::load_root_ca(
                    CertitficateName {
                        organization: Some("Terrazzo"),
                        common_name: Some("Terrazzo Terminal Root CA"),
                        ..CertitficateName::default()
                    },
                    CertificateInfo {
                        certificate: format!("{}.cert", server.private_root_ca),
                        private_key: format!("{}.key", server.private_root_ca),
                    },
                    Validity {
                        from: 0,
                        to: 365 * 20,
                    }
                    .try_map(Asn1Time::days_from_now)
                    .expect("Asn1Time::days_from_now")
                    .as_deref()
                    .try_into()
                    .expect("Asn1Time to SystemTime"),
                )
            })
            .map_err(PrivateRootCaError::Load)?
            .cache()
            .map_err(PrivateRootCaError::Cache)?;
        Ok(Self(root_ca))
    }
}

#[nameth]
#[derive(thiserror::Error, Debug)]
pub enum PrivateRootCaError {
    #[error("[{n}] {0}", n = self.name())]
    Load(RootCaConfigError),

    #[error("[{n}] {0}", n = self.name())]
    Cache(<PemCertificate as CertificateConfig>::Error),
}

impl CertificateConfig for PrivateRootCa {
    type Error = Infallible;

    fn intermediates(&self) -> Result<Arc<Vec<X509>>, Self::Error> {
        self.0.intermediates()
    }

    fn certificate(&self) -> Result<Arc<X509CertificateInfo>, Self::Error> {
        self.0.certificate()
    }
}

impl TrustedStoreConfig for PrivateRootCa {
    type Error = AsTrustedStoreError<Infallible>;

    fn root_certificates(&self) -> Result<Arc<X509Store>, Self::Error> {
        as_trusted_store(self)
    }
}

mod debug {
    use std::fmt::Debug;

    use super::PrivateRootCa;

    impl Debug for PrivateRootCa {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            f.debug_tuple("PrivateRootCa").finish()
        }
    }
}
