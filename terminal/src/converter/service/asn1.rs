use std::sync::OnceLock;

use cms::cert::x509::spki::ObjectIdentifier;
use oid_registry::OidRegistry;

use super::AddConversionFn;
use crate::converter::api::Language;

pub fn add_asn1(input: &[u8], add: &mut impl AddConversionFn) -> bool {
    let Ok(asn1) = simple_asn1::from_der(input) else {
        return false;
    };
    let asn1 = asn1.into_iter().map(ASN1Block::from).collect::<Vec<_>>();
    let asn1 = serde_yaml_ng::to_string(&asn1).unwrap_or_else(|error| error.to_string());

    add(Language::new("ASN.1"), asn1);
    return true;
}

pub fn print_oid(oid: cms::cert::x509::spki::ObjectIdentifier) -> String {
    static OID_REGISTRY: OnceLock<OidRegistry> = OnceLock::new();
    let oid_registry =
        OID_REGISTRY.get_or_init(|| OidRegistry::default().with_all_crypto().with_x509());
    oid_registry
        .get(&oid_registry::Oid::new(oid.as_bytes().into()))
        .map(|oid_entry| format!("{} ({})", oid_entry.description(), oid))
        .unwrap_or_else(|| oid.to_string())
}

pub fn print_bytes(bytes: &[u8]) -> String {
    if bytes.iter().all(|b| b.is_ascii_graphic())
        && let Ok(string) = str::from_utf8(bytes)
    {
        return string.to_owned();
    }
    let mut result = String::default();
    let mut iter = bytes.iter().peekable();
    while let Some(byte) = iter.next() {
        result += &match iter.peek() {
            Some(_) => format!("{byte:02X}:"),
            None => format!("{byte:02X}"),
        };
    }
    result
}

#[derive(serde::Serialize)]
pub enum ASN1Block {
    Boolean(bool),
    Integer(String),
    BitString(String),
    OctetString(String),
    Null,
    ObjectIdentifier(String),
    UTF8String(String),
    PrintableString(String),
    TeletexString(String),
    IA5String(String),
    UTCTime(String),
    GeneralizedTime(String),
    UniversalString(String),
    BMPString(String),
    Sequence(Vec<ASN1Block>),
    Set(Vec<ASN1Block>),
    Explicit {
        class: String,
        tag: String,
        content: Box<ASN1Block>,
    },
    Unknown {
        class: String,
        constructed: bool,
        tag: String,
        content: String,
    },
}

impl From<simple_asn1::ASN1Block> for ASN1Block {
    fn from(value: simple_asn1::ASN1Block) -> Self {
        match value {
            simple_asn1::ASN1Block::Boolean(_offset, t) => ASN1Block::Boolean(t),
            simple_asn1::ASN1Block::Integer(_offset, big_int) => {
                ASN1Block::Integer(big_int.to_string())
            }
            simple_asn1::ASN1Block::BitString(_offset, _, items) => {
                ASN1Block::BitString(print_bytes(&items))
            }
            simple_asn1::ASN1Block::OctetString(_offset, items) => {
                ASN1Block::OctetString(print_bytes(&items))
            }
            simple_asn1::ASN1Block::Null(_) => ASN1Block::Null,
            simple_asn1::ASN1Block::ObjectIdentifier(_offset, oid) => ASN1Block::ObjectIdentifier(
                oid.as_raw()
                    .map(|oid| {
                        ObjectIdentifier::from_bytes(&oid)
                            .map(print_oid)
                            .unwrap_or_else(|error| error.to_string())
                    })
                    .unwrap_or_else(|error| error.to_string()),
            ),
            simple_asn1::ASN1Block::UTF8String(_offset, s) => ASN1Block::UTF8String(s),
            simple_asn1::ASN1Block::PrintableString(_offset, s) => ASN1Block::PrintableString(s),
            simple_asn1::ASN1Block::TeletexString(_offset, s) => ASN1Block::TeletexString(s),
            simple_asn1::ASN1Block::IA5String(_offset, s) => ASN1Block::IA5String(s),
            simple_asn1::ASN1Block::UTCTime(_offset, date_time) => {
                ASN1Block::UTCTime(date_time.to_string())
            }
            simple_asn1::ASN1Block::GeneralizedTime(_offset, date_time) => {
                ASN1Block::GeneralizedTime(date_time.to_string())
            }
            simple_asn1::ASN1Block::UniversalString(_offset, s) => ASN1Block::UniversalString(s),
            simple_asn1::ASN1Block::BMPString(_offset, s) => ASN1Block::BMPString(s),
            simple_asn1::ASN1Block::Sequence(_offset, asn1_blocks) => {
                ASN1Block::Sequence(asn1_blocks.into_iter().map(ASN1Block::from).collect())
            }
            simple_asn1::ASN1Block::Set(_offset, asn1_blocks) => {
                ASN1Block::Set(asn1_blocks.into_iter().map(ASN1Block::from).collect())
            }
            simple_asn1::ASN1Block::Explicit(class, _offset, tag, mut content) => {
                ASN1Block::Explicit {
                    class: format!("{class:?}"),
                    tag: tag.to_string(),
                    content: Box::new(
                        std::mem::replace(&mut *content, simple_asn1::ASN1Block::Null(0)).into(),
                    ),
                }
            }
            simple_asn1::ASN1Block::Unknown(class, constructed, _offset, tag, items) => {
                ASN1Block::Unknown {
                    class: format!("{class:?}"),
                    constructed,
                    tag: tag.to_string(),
                    content: print_bytes(&items),
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::tests::GetConversionForTest as _;

    const ASN1: &str = r#"
        MIIBtDCCAVmgAwIBAgIVANSN+BUl1Kf8XjE8anSpXGs1HfaWMAoGCCqGSM49BAMC
        MDcxETAPBgNVBAoMCFRlcnJhenpvMSIwIAYDVQQDDBlUZXJyYXp6byBUZXJtaW5h
        bCBSb290IENBMB4XDTI1MDYwNjEwMDEyN1oXDTQ1MDYwMTEwMDEyN1owNzERMA8G
        A1UECgwIVGVycmF6em8xIjAgBgNVBAMMGVRlcnJhenpvIFRlcm1pbmFsIFJvb3Qg
        Q0EwWTATBgcqhkjOPQIBBggqhkjOPQMBBwNCAATGiH+iC1+6+3YxaWLEW8V1RsHQ
        +fToNIBWRRJEV3q9z5YwZWHLZj8RfWCPsc01rKja1lnhfwEGd5qd9UUQk36go0Iw
        QDAdBgNVHQ4EFgQUEC5YRL04bEDiZ9oic1PZc7bR9P4wDwYDVR0TAQH/BAUwAwEB
        /zAOBgNVHQ8BAf8EBAMCAQYwCgYIKoZIzj0EAwIDSQAwRgIhAJuRb4MWDitsOJqy
        VOj7ugn3k0TlZV3rPSRmuL20bjeeAiEAhVOBRet9JDnQbjG/0SG8QVdJplLL66By
        RD66UosBh50=
        "#;

    #[tokio::test]
    async fn asn1() {
        let conversion = ASN1.get_conversion("ASN.1").await;
        assert!(conversion.contains("2025-06-06 10:01:27.0"));
    }
}
