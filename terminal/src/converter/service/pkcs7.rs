use base64::Engine as _;
use base64::prelude::BASE64_STANDARD;
use cms::cert::x509::der::Decode as _;
use cms::cert::x509::der::Encode as _;
use cms::cert::x509::der::Tagged as _;
use oid_registry::OID_PKCS7_ID_SIGNED_DATA;
use openssl::x509::X509;

use super::AddConversionFn;
use crate::converter::api::Language;

pub fn add_pkcs7(input: &[u8], add: &mut impl AddConversionFn) -> bool {
    add_pkcs7_impl(input, add).is_some()
}

fn add_pkcs7_impl(input: &[u8], add: &mut impl AddConversionFn) -> Option<()> {
    let content_info = cms::content_info::ContentInfo::from_der(input).ok()?;
    if content_info.content_type.as_bytes() != OID_PKCS7_ID_SIGNED_DATA.as_bytes() {
        return None;
    }
    let cms::signed_data::SignedData {
        version,
        digest_algorithms,
        encap_content_info,
        certificates,
        crls,
        signer_infos,
    } = content_info.content.decode_as().ok()?;

    let version = format!("{version:?}");
    let digest_algorithms = digest_algorithms
        .into_vec()
        .into_iter()
        .map(AlgorithmIdentifier::from)
        .collect();
    let encapsulated_content_info = EncapsulatedContentInfo {
        encapsulated_content_type: encap_content_info.econtent_type.to_string(),
        encapsulated_content: encap_content_info.econtent.map(Any::from),
    };
    let certificates = {
        let mut list = vec![];
        for certificate in certificates
            .map(|certificates| certificates.0.into_vec())
            .unwrap_or_default()
        {
            let certificate =
                match certificate {
                    cms::cert::CertificateChoices::Certificate(certificate) => {
                        let x509 = add_certificate(certificate, add);
                        CertificateChoices::Certificate(x509.unwrap_or_else(|error| {
                            format!("Failed to parse certificate: {error}")
                        }))
                    }
                    cms::cert::CertificateChoices::Other(other) => {
                        CertificateChoices::Other(OtherCertificateFormat {
                            format: other.other_cert_format.to_string(),
                            certificate: other.other_cert.into(),
                        })
                    }
                };
            list.push(certificate);
        }
        list
    };

    let crls = crls.map(|crls| crls.0.into_vec()).unwrap_or_default();
    let crls = crls
        .into_iter()
        .map(|crl| match crl {
            cms::revocation::RevocationInfoChoice::Crl(list) => {
                RevocationInfoChoice::Crl(CertificateList {
                    tbs_cert_list: TbsCertList {
                        version: format!("{:?}", list.tbs_cert_list.version),
                        signature: list.tbs_cert_list.signature.into(),
                        issuer: list.tbs_cert_list.issuer.to_string(),
                        this_update: list.tbs_cert_list.this_update.to_string(),
                        next_update: list.tbs_cert_list.next_update.map(|n| n.to_string()),
                        revoked_certificates: make_revoked_certificates(
                            list.tbs_cert_list.revoked_certificates,
                        ),
                        crl_extensions: list
                            .tbs_cert_list
                            .crl_extensions
                            .unwrap_or_default()
                            .into_iter()
                            .map(Extension::from)
                            .collect(),
                    },
                    signature_algorithm: list.signature_algorithm.into(),
                    signature: list
                        .signature
                        .as_bytes()
                        .map(print_bytes)
                        .unwrap_or_default(),
                })
            }
            cms::revocation::RevocationInfoChoice::Other(other) => {
                RevocationInfoChoice::Other(OtherRevocationInfoFormat {
                    algorithm_identifier: other.other_format.into(),
                    data: other.other.into(),
                })
            }
        })
        .collect();

    let signer_infos = signer_infos.0.into_vec().into_iter();
    let signer_infos = signer_infos
        .map(|signer_info| SignerInfo {
            version: format!("{:?}", signer_info.version),
            signer_identifier: match signer_info.sid {
                cms::signed_data::SignerIdentifier::IssuerAndSerialNumber(
                    issuer_and_serial_number,
                ) => SignerIdentifier::IssuerAndSerialNumber(IssuerAndSerialNumber {
                    issuer: issuer_and_serial_number.issuer.to_string(),
                    serial_number: issuer_and_serial_number.serial_number.to_string(),
                }),
                cms::signed_data::SignerIdentifier::SubjectKeyIdentifier(
                    subject_key_identifier,
                ) => SignerIdentifier::SubjectKeyIdentifier(print_bytes(
                    subject_key_identifier.0.as_bytes(),
                )),
            },
            digest_algorithm: signer_info.digest_alg.into(),
            signed_attributes: make_attributes(signer_info.signed_attrs),
            signature_algorithm: AlgorithmIdentifier {
                oid: signer_info.signature_algorithm.oid.to_string(),
                parameters: signer_info.signature_algorithm.parameters.map(Any::from),
            },
            signature: print_bytes(signer_info.signature.as_bytes()),
            unsigned_attributes: make_attributes(signer_info.unsigned_attrs),
        })
        .collect();

    let signed_data = SignedData {
        version,
        digest_algorithms,
        encapsulated_content_info,
        certificates,
        crls,
        signer_infos,
    };
    let content_info = ContentInfo {
        content_type: content_info.content_type.to_string(),
        content: signed_data,
    };
    let content_info =
        serde_yaml_ng::to_string(&content_info).unwrap_or_else(|error| error.to_string());

    add(Language::new("PKCS #7"), content_info);
    return Some(());
}

fn make_revoked_certificates(
    revoked_certificates: Option<Vec<cms::cert::x509::crl::RevokedCert>>,
) -> Vec<RevokedCert> {
    let revoked_certificates = revoked_certificates
        .map(|revoked_certificates| revoked_certificates)
        .unwrap_or_default();
    revoked_certificates
        .into_iter()
        .map(|revoked_certificate| RevokedCert {
            serial_number: revoked_certificate.serial_number.to_string(),
            revocation_date: revoked_certificate.revocation_date.to_string(),
            crl_entry_extensions: revoked_certificate
                .crl_entry_extensions
                .unwrap_or_default()
                .into_iter()
                .map(Extension::from)
                .collect(),
        })
        .collect()
}

fn add_certificate(
    certificate: cms::cert::x509::certificate::CertificateInner,
    add: &mut impl AddConversionFn,
) -> Result<String, String> {
    let der = certificate.to_der().map_err(|error| error.to_string())?;
    let x509 = X509::from_der(&der).map_err(|error| error.to_string())?;
    let name = format!("{:?}", x509.subject_name());
    let text = x509
        .to_text()
        .map(String::from_utf8)
        .unwrap_or_else(|error| Ok(error.to_string()))
        .unwrap_or_else(|error| error.to_string());
    add(Language::new(name.as_str()), text);
    return Ok(name);
}

#[derive(serde::Serialize)]
struct ContentInfo {
    content_type: String,
    content: SignedData,
}

#[derive(serde::Serialize)]
struct SignedData {
    version: String,
    digest_algorithms: Vec<AlgorithmIdentifier>,
    encapsulated_content_info: EncapsulatedContentInfo,
    certificates: Vec<CertificateChoices>,
    crls: Vec<RevocationInfoChoice>,
    signer_infos: Vec<SignerInfo>,
}

#[derive(serde::Serialize)]
struct AlgorithmIdentifier {
    oid: String,
    parameters: Option<Any>,
}

impl From<cms::cert::x509::spki::AlgorithmIdentifier<cms::cert::x509::der::Any>>
    for AlgorithmIdentifier
{
    fn from(value: cms::cert::x509::spki::AlgorithmIdentifier<cms::cert::x509::der::Any>) -> Self {
        Self {
            oid: value.oid.to_string(),
            parameters: value.parameters.map(Any::from),
        }
    }
}

#[derive(serde::Serialize)]
struct EncapsulatedContentInfo {
    encapsulated_content_type: String,
    encapsulated_content: Option<Any>,
}

#[derive(serde::Serialize)]
enum CertificateChoices {
    Certificate(String),
    Other(OtherCertificateFormat),
}

#[derive(serde::Serialize)]
struct OtherCertificateFormat {
    format: String,
    certificate: Any,
}

#[derive(serde::Serialize)]
enum RevocationInfoChoice {
    Crl(CertificateList),
    Other(OtherRevocationInfoFormat),
}

#[derive(serde::Serialize)]
struct CertificateList {
    tbs_cert_list: TbsCertList,
    signature_algorithm: AlgorithmIdentifier,
    signature: String,
}

#[derive(serde::Serialize)]
struct TbsCertList {
    version: String,
    signature: AlgorithmIdentifier,
    issuer: String,
    this_update: String,
    next_update: Option<String>,
    revoked_certificates: Vec<RevokedCert>,
    crl_extensions: Vec<Extension>,
}

#[derive(serde::Serialize)]
struct RevokedCert {
    serial_number: String,
    revocation_date: String,
    crl_entry_extensions: Vec<Extension>,
}

#[derive(serde::Serialize)]
struct Extension {
    extn_id: String,
    critical: bool,
    extn_value: String,
}

impl From<cms::cert::x509::ext::Extension> for Extension {
    fn from(extension: cms::cert::x509::ext::Extension) -> Self {
        Self {
            extn_id: extension.extn_id.to_string(),
            critical: extension.critical,
            extn_value: BASE64_STANDARD.encode(extension.extn_value),
        }
    }
}

#[derive(serde::Serialize)]
struct OtherRevocationInfoFormat {
    algorithm_identifier: AlgorithmIdentifier,
    data: Any,
}

#[derive(serde::Serialize)]
struct SignerInfo {
    version: String,
    signer_identifier: SignerIdentifier,
    digest_algorithm: AlgorithmIdentifier,
    signed_attributes: Vec<Attribute>,
    signature_algorithm: AlgorithmIdentifier,
    signature: String,
    unsigned_attributes: Vec<Attribute>,
}

#[derive(serde::Serialize)]
enum SignerIdentifier {
    IssuerAndSerialNumber(IssuerAndSerialNumber),
    SubjectKeyIdentifier(String),
}

#[derive(serde::Serialize)]
struct IssuerAndSerialNumber {
    issuer: String,
    serial_number: String,
}

#[derive(serde::Serialize)]
struct Attribute {
    oid: String,
    values: Vec<Any>,
}

fn make_attributes(
    attributes: Option<cms::cert::x509::der::asn1::SetOfVec<cms::cert::x509::attr::Attribute>>,
) -> Vec<Attribute> {
    attributes
        .map(|unsigned_attrs| unsigned_attrs.into_vec())
        .unwrap_or_default()
        .into_iter()
        .map(|unsigned_attr| Attribute {
            oid: unsigned_attr.oid.to_string(),
            values: unsigned_attr
                .values
                .into_vec()
                .into_iter()
                .map(Any::from)
                .collect(),
        })
        .collect()
}

#[derive(serde::Serialize)]
struct Any {
    tag: String,
    value: String,
}

impl From<cms::cert::x509::der::Any> for Any {
    fn from(any: cms::cert::x509::der::Any) -> Self {
        Self {
            tag: format!("{:?}", any.tag()),
            value: BASE64_STANDARD.encode(any.value()),
        }
    }
}

fn print_bytes(bytes: &[u8]) -> String {
    let mut result = String::default();
    let mut iter = bytes.into_iter().peekable();
    while let Some(byte) = iter.next() {
        result += &match iter.peek() {
            Some(_) => format!("{:02X}:", byte),
            None => format!("{:02X}", byte),
        };
    }
    result
}
