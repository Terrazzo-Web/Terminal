use std::borrow::Cow;
use std::net::SocketAddr;
use std::str::FromStr;
use std::sync::LazyLock;
use std::time::Duration;

use base64::Engine as _;
use futures::FutureExt as _;
use futures::future::BoxFuture;
use futures::future::Shared;
use hickory_client::client::Client;
use hickory_client::client::ClientHandle;
use hickory_client::proto::op::Edns;
use hickory_client::proto::op::EdnsFlags;
use hickory_client::proto::op::Header;
use hickory_client::proto::op::Message;
use hickory_client::proto::op::MessageType;
use hickory_client::proto::op::OpCode;
use hickory_client::proto::op::Query;
use hickory_client::proto::op::ResponseCode;
use hickory_client::proto::rr::DNSClass;
use hickory_client::proto::rr::Name;
use hickory_client::proto::rr::RData;
use hickory_client::proto::rr::Record;
use hickory_client::proto::rr::RecordType;
use hickory_client::proto::rr::rdata::opt::EdnsCode;
use hickory_client::proto::rr::rdata::opt::EdnsOption;
use hickory_client::proto::runtime::TokioRuntimeProvider;
use hickory_client::proto::udp::UdpClientStream;
use regex::Regex;
use tracing::debug;
use tracing::warn;

use crate::converter::api::Language;

pub async fn add_dns(input: &str, add: &mut impl super::AddConversionFn) -> bool {
    add_dns_impl(input, add).await.is_some()
}

async fn add_dns_impl(input: &str, add: &mut impl super::AddConversionFn) -> Option<()> {
    static DNS_REGEX: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r"^[a-z0-9_-]+(\.[a-z0-9_-]+)+\.?$").unwrap());
    if !DNS_REGEX.is_match(input) {
        debug!("Not a valid DNS name: {input}");
        return None;
    }

    let name = Name::from_str(input).ok()?;
    debug!("Running DNS query for {name}");
    let responses = futures::future::join_all([
        query_dns(&name, RecordType::A),
        query_dns(&name, RecordType::AAAA),
        query_dns(&name, RecordType::CNAME),
        query_dns(&name, RecordType::TXT),
        query_dns(&name, RecordType::MX),
        query_dns(&name, RecordType::SRV),
    ])
    .await;
    let responses = responses
        .iter()
        .filter_map(|response| response.as_ref())
        .map(|(record_type, response)| DnsResponse {
            record_type: *record_type,
            response: response.into(),
        })
        .collect::<Vec<_>>();

    let response = serde_yaml_ng::to_string(&responses).ok()?;
    add(Language::new("DNS"), response);
    Some(())
}

#[derive(serde::Serialize)]
struct DnsResponse<'t> {
    record_type: RecordType,
    response: Message2<'t>,
}

async fn query_dns(name: &Name, record_type: RecordType) -> Option<(RecordType, Message)> {
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
    Some((record_type, response))
}

#[derive(serde::Serialize)]
struct Message2<'t> {
    header: Header2,
    #[serde(skip_serializing_if = "is_empty")]
    queries: Vec<Query>,
    #[serde(skip_serializing_if = "is_empty")]
    answers: Vec<Record2<'t>>,
    #[serde(skip_serializing_if = "is_empty")]
    name_servers: Vec<Record2<'t>>,
    #[serde(skip_serializing_if = "is_empty")]
    additionals: Vec<Record2<'t>>,
    #[serde(skip_serializing_if = "is_empty")]
    signature: Vec<Record2<'t>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    edns: Option<Edns2<'t>>,
}

impl<'t> From<&'t Message> for Message2<'t> {
    fn from(value: &'t Message) -> Self {
        Self {
            header: value.header().into(),
            queries: value.queries().into(),
            answers: value.answers().iter().map(Into::into).collect(),
            name_servers: value.name_servers().iter().map(Into::into).collect(),
            additionals: value.additionals().iter().map(Into::into).collect(),
            signature: value.signature().iter().map(Into::into).collect(),
            edns: value.extensions().as_ref().map(Into::into),
        }
    }
}

#[derive(serde::Serialize)]
struct Header2 {
    #[serde(skip_serializing_if = "is_default")]
    id: u16,
    message_type: MessageType,
    op_code: OpCode,
    #[serde(skip_serializing_if = "is_default")]
    authoritative: bool,
    #[serde(skip_serializing_if = "is_default")]
    truncation: bool,
    #[serde(skip_serializing_if = "is_default")]
    recursion_desired: bool,
    #[serde(skip_serializing_if = "is_default")]
    recursion_available: bool,
    #[serde(skip_serializing_if = "is_default")]
    authentic_data: bool,
    #[serde(skip_serializing_if = "is_default")]
    checking_disabled: bool,
    response_code: ResponseCode,
}

impl<'t> From<&'t Header> for Header2 {
    fn from(value: &'t Header) -> Self {
        Self {
            id: value.id(),
            message_type: value.message_type(),
            op_code: value.op_code(),
            authoritative: value.authoritative(),
            truncation: value.truncated(),
            recursion_desired: value.recursion_desired(),
            recursion_available: value.recursion_available(),
            authentic_data: value.authentic_data(),
            checking_disabled: value.checking_disabled(),
            response_code: value.response_code(),
        }
    }
}

#[derive(serde::Serialize)]
struct Record2<'t> {
    name_labels: &'t Name,
    dns_class: DNSClass,
    ttl: String,
    rdata: RData2<'t>,
}

impl<'t> From<&'t Record> for Record2<'t> {
    fn from(value: &'t Record) -> Self {
        Self {
            name_labels: value.name(),
            dns_class: value.dns_class(),
            ttl: humantime::Duration::from(Duration::from_secs(value.ttl() as u64)).to_string(),
            rdata: value.data().into(),
        }
    }
}

#[derive(serde::Serialize)]
enum RData2<'t> {
    #[allow(clippy::upper_case_acronyms)]
    TXT(Vec<Cow<'t, str>>),
    #[serde(untagged)]
    Other(&'t RData),
}

impl<'t> From<&'t RData> for RData2<'t> {
    fn from(value: &'t RData) -> Self {
        match value {
            RData::TXT(txt) => Self::TXT(txt.txt_data().iter().map(to_string_lossy).collect()),
            _ => Self::Other(value),
        }
    }
}

#[derive(serde::Serialize)]
struct Edns2<'t> {
    #[serde(skip_serializing_if = "is_default")]
    rcode_high: u8,
    #[serde(skip_serializing_if = "is_default")]
    version: u8,
    #[serde(skip_serializing_if = "is_default")]
    flags: EdnsFlags2,
    #[serde(skip_serializing_if = "is_default")]
    max_payload: u16,
    #[serde(skip_serializing_if = "is_empty")]
    options: Vec<EdnsOptionEntry<'t>>,
}

impl<'t> From<&'t Edns> for Edns2<'t> {
    fn from(value: &'t Edns) -> Self {
        Self {
            rcode_high: value.rcode_high(),
            version: value.version(),
            flags: value.flags().into(),
            max_payload: value.max_payload(),
            options: value
                .options()
                .as_ref()
                .iter()
                .map(|(code, option)| EdnsOptionEntry {
                    code: *code,
                    value: option.into(),
                })
                .collect(),
        }
    }
}

#[derive(Default, PartialEq, Eq, serde::Serialize)]
struct EdnsFlags2 {
    #[serde(skip_serializing_if = "is_default")]
    dnssec_ok: bool,
    #[serde(skip_serializing_if = "is_default")]
    z: u16,
}

impl<'t> From<&'t EdnsFlags> for EdnsFlags2 {
    fn from(value: &'t EdnsFlags) -> Self {
        Self {
            dnssec_ok: value.dnssec_ok,
            z: value.z,
        }
    }
}

#[derive(serde::Serialize)]
struct EdnsOptionEntry<'t> {
    code: EdnsCode,
    value: EdnsOption2<'t>,
}

#[derive(serde::Serialize)]
enum EdnsOption2<'t> {
    Unknown {
        code: u16,
        value: Cow<'t, str>,
    },
    #[serde(untagged)]
    Other(&'t EdnsOption),
}

impl<'t> From<&'t EdnsOption> for EdnsOption2<'t> {
    fn from(value: &'t EdnsOption) -> Self {
        match value {
            EdnsOption::Unknown(code, items) => Self::Unknown {
                code: *code,
                value: to_string_lossy(items),
            },
            _ => Self::Other(value),
        }
    }
}

fn to_string_lossy(data: &impl AsRef<[u8]>) -> Cow<'_, str> {
    let data = data.as_ref();
    str::from_utf8(data)
        .map(Cow::Borrowed)
        .unwrap_or_else(|_utf8_error| {
            use base64::prelude::BASE64_STANDARD_NO_PAD;
            Cow::Owned(format!(
                "Not UTF-8: {}",
                BASE64_STANDARD_NO_PAD.encode(data)
            ))
        })
}

fn is_default<T: Default + PartialEq>(v: &T) -> bool {
    v == &T::default()
}

fn is_empty<T: AsRef<[E]>, E>(v: &T) -> bool {
    v.as_ref().is_empty()
}
