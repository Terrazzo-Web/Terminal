use std::net::SocketAddr;
use std::str::FromStr;
use std::sync::LazyLock;

use futures::FutureExt as _;
use futures::future::BoxFuture;
use futures::future::Shared;
use hickory_client::client::Client;
use hickory_client::client::ClientHandle;
use hickory_client::proto::op::Message;
use hickory_client::proto::rr::DNSClass;
use hickory_client::proto::rr::Name;
use hickory_client::proto::rr::RecordType;
use hickory_client::proto::runtime::TokioRuntimeProvider;
use hickory_client::proto::udp::UdpClientStream;
use tracing::warn;

use crate::converter::api::Language;

pub async fn add_dns(input: &str, add: &mut impl super::AddConversionFn) -> bool {
    add_dns_impl(input, add).await.is_some()
}

async fn add_dns_impl(input: &str, add: &mut impl super::AddConversionFn) -> Option<()> {
    let name = Name::from_str(input).ok()?;
    let responses = futures::future::join_all([
        query_dns(&name, RecordType::A),
        query_dns(&name, RecordType::AAAA),
        query_dns(&name, RecordType::CNAME),
        query_dns(&name, RecordType::TXT),
        query_dns(&name, RecordType::MX),
        query_dns(&name, RecordType::SRV),
    ])
    .await
    .into_iter()
    .filter_map(|response| response)
    .collect::<Vec<_>>();

    let response = serde_yaml_ng::to_string(&responses).ok()?;
    add(Language::new("DNS"), response);
    Some(())
}

async fn query_dns(name: &Name, record_type: RecordType) -> Option<DnsResponse> {
    static CLIENT: LazyLock<Shared<BoxFuture<Option<Client>>>> = LazyLock::new(|| {
        let address = SocketAddr::from(([8, 8, 8, 8], 53));
        let conn = UdpClientStream::builder(address, TokioRuntimeProvider::default()).build();
        async move {
            let (client, bg) = Client::connect(conn)
                .await
                .inspect_err(|error| warn!("Failed to initialize DNS client: {error}"))
                .ok()?;
            tokio::spawn(bg);
            Some(client)
        }
        .boxed()
        .shared()
    });
    let mut client = LazyLock::force(&CLIENT).clone().await?;

    let response = client
        .query(name.to_owned(), DNSClass::IN, record_type)
        .await
        .ok()?
        .into_message();
    let response = DnsResponse {
        record_type,
        response,
    };
    Some(response)
}

#[derive(serde::Serialize)]
struct DnsResponse {
    record_type: RecordType,
    response: Message,
}
