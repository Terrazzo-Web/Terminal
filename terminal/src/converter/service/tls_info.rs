use std::sync::Arc;

use tokio::net::TcpStream;
use tokio_rustls::TlsConnector;
use tokio_rustls::client::TlsStream;
use tokio_rustls::rustls::ClientConfig;
use tokio_rustls::rustls::RootCertStore;
use tokio_rustls::rustls::pki_types::ServerName;
use tracing::debug;
use url::Url;

pub async fn add_tls_info(input: &str, add: &mut impl super::AddConversionFn) -> bool {
    add_tls_info_impl(input, add).await.is_ok()
}

async fn add_tls_info_impl(input: &str, add: &mut impl super::AddConversionFn) -> Result<(), ()> {
    let url = Url::parse(input).ignore_err("url")?;
    let host = url.host_str().ignore_err("host")?;
    let port = url.port_or_known_default().ignore_err("port")?;
    super::dns::add_dns(host, add).await;

    let tls: TlsStream<TcpStream> = {
        let tcp = TcpStream::connect((host, port))
            .await
            .ignore_err("TCP connect")?;

        let mut root_store = RootCertStore::empty();
        root_store
            .add_parsable_certificates(rustls_native_certs::load_native_certs().certs.into_iter());

        let client_config = ClientConfig::builder()
            .with_root_certificates(root_store)
            .with_no_client_auth();

        let connector = TlsConnector::from(Arc::new(client_config));
        let server_name = ServerName::try_from(host)
            .ignore_err("server_name")?
            .to_owned();

        connector
            .connect(server_name, tcp)
            .await
            .ignore_err("TLS connect")?
    };

    let (_tcp_stream, session) = tls.get_ref();
    let certificates = session
        .peer_certificates()
        .ignore_err("peer_certificates")?;

    for certificate in certificates {
        super::x509::add_x509_base64(certificate.as_ref(), add);
    }

    Ok(())
}

trait IgnoreErr<T> {
    fn ignore_err(self, error: &'static str) -> Result<T, ()>;
}

impl<T, E> IgnoreErr<T> for Result<T, E> {
    fn ignore_err(self, error: &'static str) -> Result<T, ()> {
        self.map_err(|_| debug!("Failled to parse https TLS info: {error}"))
    }
}

impl<T> IgnoreErr<T> for Option<T> {
    fn ignore_err(self, error: &'static str) -> Result<T, ()> {
        match self {
            Some(v) => Ok(v),
            None => Err(()),
        }
        .ignore_err(error)
    }
}

#[cfg(test)]
mod tests {
    use super::super::tests::GetConversionForTest as _;

    static UNESCAPED: &str = "Unescaped";

    #[tokio::test]
    async fn nothing_to_unescape() {
        let input = r#"A  B"#;
        let conversion = input.get_conversion(UNESCAPED).await;
        assert_eq!("Not found", conversion);
        assert_eq!(vec!["JSON", "YAML"], input.get_languages().await);
    }
}
