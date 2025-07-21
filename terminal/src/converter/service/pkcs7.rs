use base64::Engine as _;
use base64::prelude::BASE64_STANDARD;
use cms::cert::x509::der::Decode as _;
use cms::cert::x509::der::Encode as _;
use cms::cert::x509::der::Tagged as _;
use oid_registry::OID_PKCS7_ID_SIGNED_DATA;
use openssl::x509::X509;

use super::AddConversionFn;
use super::asn1::print_bytes;
use super::asn1::print_oid;
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
        encapsulated_content_type: print_oid(encap_content_info.econtent_type),
        encapsulated_content: encap_content_info.econtent.map(Any::from),
    };
    let mut x509s = vec![];
    let certificates = {
        let mut list = vec![];
        for certificate in certificates
            .map(|certificates| certificates.0.into_vec())
            .unwrap_or_default()
        {
            let certificate = match certificate {
                cms::cert::CertificateChoices::Certificate(certificate) => {
                    let x509 = add_certificate(certificate, |name, x509| x509s.push((name, x509)));
                    CertificateChoices::Certificate(
                        x509.unwrap_or_else(|error| {
                            format!("Failed to parse certificate: {error}")
                        }),
                    )
                }
                cms::cert::CertificateChoices::Other(other) => {
                    CertificateChoices::Other(OtherCertificateFormat {
                        format: print_oid(other.other_cert_format),
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
            signature_algorithm: signer_info.signature_algorithm.into(),
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
        content_type: print_oid(content_info.content_type),
        content: signed_data,
    };
    let content_info =
        serde_yaml_ng::to_string(&content_info).unwrap_or_else(|error| error.to_string());

    add(Language::new("PKCS #7"), content_info);
    for (name, x509) in x509s {
        add(name, x509);
    }
    return Some(());
}

fn make_revoked_certificates(
    revoked_certificates: Option<Vec<cms::cert::x509::crl::RevokedCert>>,
) -> Vec<RevokedCert> {
    revoked_certificates
        .unwrap_or_default()
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
    mut add: impl AddConversionFn,
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
    #[serde(default)]
    parameters: Option<Any>,
}

impl From<cms::cert::x509::spki::AlgorithmIdentifier<cms::cert::x509::der::Any>>
    for AlgorithmIdentifier
{
    fn from(value: cms::cert::x509::spki::AlgorithmIdentifier<cms::cert::x509::der::Any>) -> Self {
        Self {
            oid: print_oid(value.oid),
            parameters: value.parameters.map(Any::from),
        }
    }
}

#[derive(serde::Serialize)]
struct EncapsulatedContentInfo {
    encapsulated_content_type: String,
    #[serde(default)]
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
    #[serde(default)]
    next_update: Option<String>,
    #[serde(default)]
    revoked_certificates: Vec<RevokedCert>,
    #[serde(default)]
    crl_extensions: Vec<Extension>,
}

#[derive(serde::Serialize)]
struct RevokedCert {
    serial_number: String,
    revocation_date: String,
    #[serde(default)]
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
            extn_id: print_oid(extension.extn_id),
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
    #[serde(default)]
    signed_attributes: Vec<Attribute>,
    signature_algorithm: AlgorithmIdentifier,
    signature: String,
    #[serde(default)]
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
        .map(|attributes| attributes.into_vec())
        .unwrap_or_default()
        .into_iter()
        .map(|attribute| Attribute {
            oid: print_oid(attribute.oid),
            values: attribute
                .values
                .into_vec()
                .into_iter()
                .map(Any::from)
                .collect(),
        })
        .collect()
}

#[derive(serde::Serialize)]
enum Any {
    ASN1(Vec<super::asn1::ASN1Block>),
    Any { tag: String, value: String },
}

impl From<cms::cert::x509::der::Any> for Any {
    fn from(any: cms::cert::x509::der::Any) -> Self {
        match any
            .to_der()
            .ok()
            .and_then(|der| simple_asn1::from_der(&der).ok())
        {
            Some(asn1) => Self::ASN1(asn1.into_iter().map(From::from).collect()),
            None => Self::Any {
                tag: format!("{:?}", any.tag()),
                value: match any.to_der() {
                    Ok(der) => BASE64_STANDARD.encode(der),
                    Err(error) => error.to_string(),
                },
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::tests::GetConversionForTest as _;

    const PKCS7: &str = r#"
        MIIGbwYJKoZIhvcNAQcCoIIGYDCCBlwCAQMxDTALBglghkgBZQMEAgEwTAYJKoZI
        hvcNAQcBoD8EPVF3ZXJ0eToxNzUxNTY1NDU1OjE3NTkzNDE0NTU6kRUUrisPBOum
        QZFjzBv3nuSKJdSET7MZunHPuNog5dKgggSNMIICMDCCAdegAwIBAgIUc+sFRlXS
        pv9Z5Qmgd26dtPCM2bcwCgYIKoZIzj0EAwIwNzERMA8GA1UECgwIVGVycmF6em8x
        IjAgBgNVBAMMGVRlcnJhenpvIFRlcm1pbmFsIFJvb3QgQ0EwHhcNMjUwNjA2MTAw
        MTI3WhcNNDUwNjAxMTAwMTI3WjA/MREwDwYDVQQKDAhUZXJyYXp6bzEqMCgGA1UE
        AwwhVGVycmF6em8gVGVybWluYWwgSW50ZXJtZWRpYXRlIENBMFkwEwYHKoZIzj0C
        AQYIKoZIzj0DAQcDQgAEB+CNedOu2PPOvj/TTergYiXEp2IaU+yP1ko2e71Kszau
        jf6XPP1cIKZcI0pBIFnJqdUvpiNL5Rcr+v7HEbx586OBuDCBtTAdBgNVHQ4EFgQU
        N73AjS6bmzRO6ZLfVSH77PQtxh0wDwYDVR0TAQH/BAUwAwEB/zAOBgNVHQ8BAf8E
        BAMCAQYwcwYDVR0jBGwwaoAUEC5YRL04bEDiZ9oic1PZc7bR9P6hO6Q5MDcxETAP
        BgNVBAoMCFRlcnJhenpvMSIwIAYDVQQDDBlUZXJyYXp6byBUZXJtaW5hbCBSb290
        IENBghUA1I34FSXUp/xeMTxqdKlcazUd9pYwCgYIKoZIzj0EAwIDRwAwRAIgO0sa
        I1IhBA5JWuwHJN8xg6YonELaRWks8mOLp31zVcwCIB6davlIEQD5SUJdkdyeRCkE
        cTH3ieyfSB9y3RU8x5WrMIICVTCCAfygAwIBAgIVANjjN6M2z0b7lmQbFLiO3A3+
        PvczMAoGCCqGSM49BAMCMD8xETAPBgNVBAoMCFRlcnJhenpvMSowKAYDVQQDDCFU
        ZXJyYXp6byBUZXJtaW5hbCBJbnRlcm1lZGlhdGUgQ0EwHhcNMjUwNjA2MTAwMTI3
        WhcNNDUwNjAxMTAwMTI3WjAnMREwDwYDVQQKDAhUZXJyYXp6bzESMBAGA1UEAwwJ
        bG9jYWxob3N0MFkwEwYHKoZIzj0CAQYIKoZIzj0DAQcDQgAEUQ0mUiIAnNNSIwoI
        JTfq5ET29vGeeMOiKRbiV2D5Ldyt+JYZA6udaaReeZs+keGC+9+lokZ2ox05rjdA
        ZTv/iaOB7DCB6TAdBgNVHQ4EFgQU+MTUsf9CoMwIp7W6+tNfJMnTSHkwDAYDVR0T
        AQH/BAIwADAOBgNVHQ8BAf8EBAMCB4AwIAYDVR0lAQH/BBYwFAYIKwYBBQUHAwEG
        CCsGAQUFBwMCMHIGA1UdIwRrMGmAFDe9wI0um5s0TumS31Uh++z0LcYdoTukOTA3
        MREwDwYDVQQKDAhUZXJyYXp6bzEiMCAGA1UEAwwZVGVycmF6em8gVGVybWluYWwg
        Um9vdCBDQYIUc+sFRlXSpv9Z5Qmgd26dtPCM2bcwFAYDVR0RBA0wC4IJbG9jYWxo
        b3N0MAoGCCqGSM49BAMCA0cAMEQCIFgfEiLtslZ8Rpn/so4gSn1CDZBdduxW7qRJ
        8XZqXIzbAiAUlY+8jm/MsvjbA8fhdl0FCFx11GwBgSId1J/nA+dkojGCAWcwggFj
        AgEDgBT4xNSx/0KgzAintbr6018kydNIeTALBglghkgBZQMEAgGggeQwGAYJKoZI
        hvcNAQkDMQsGCSqGSIb3DQEHATAcBgkqhkiG9w0BCQUxDxcNMjUwNzAzMTc1NzM1
        WjAvBgkqhkiG9w0BCQQxIgQgIAY6VZ2h3mTj4t69iuasleesv+95wvw0+Qk/WFa3
        vnkweQYJKoZIhvcNAQkPMWwwajALBglghkgBZQMEASowCwYJYIZIAWUDBAEWMAsG
        CWCGSAFlAwQBAjAKBggqhkiG9w0DBzAOBggqhkiG9w0DAgICAIAwDQYIKoZIhvcN
        AwICAUAwBwYFKw4DAgcwDQYIKoZIhvcNAwICASgwCgYIKoZIzj0EAwIESDBGAiEA
        lFl2IlVDuD3evzK7JO9vxdHkRMwzdtuW1CUhukselxsCIQDGIb6r0UBp0zs6KGwS
        /1c1cm2xB+9u2C+2Pi49mXrg1g==
        "#;

    #[tokio::test]
    async fn pkcs7() {
        let conversion = PKCS7.get_conversion("PKCS #7").await;
        dbg!(&conversion);
        assert!(conversion.contains("Terrazzo Terminal Intermediate CA"));
        assert!(conversion.contains("localhost"));
        assert!(conversion.contains("2025-07-03 17:57:35.0"));
    }
}
