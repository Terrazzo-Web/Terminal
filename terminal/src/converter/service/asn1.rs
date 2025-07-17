use std::sync::OnceLock;

use regex::Regex;

use super::AddConversionFn;
use crate::converter::api::Language;

pub fn add_asn1(input: &str, add: &mut impl AddConversionFn) -> bool {
    static BASE64_REGEX: OnceLock<Regex> = OnceLock::new();
    let base64_regex =
        BASE64_REGEX.get_or_init(|| Regex::new(r"^[-_+/A-Za-z0-9 \r\n\t]+(?:=*)$").unwrap());
    if !base64_regex.is_match(input) {
        return false;
    }
    let input: String = input.split('\n').map(str::trim).collect();
    let Some(input) = super::jwt::parse_base64(&input) else {
        return false;
    };

    let Ok(asn1) = simple_asn1::from_der(&input) else {
        return false;
    };

    add(Language::new("ASN.1"), format!("{asn1:#?}"));
    return true;
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
        RD66UosBh50="#;

    #[tokio::test]
    async fn asn1() {
        let conversion = ASN1.get_conversion("ASN.1").await;
        assert_eq!(
            r#"
header:
  alg: HS256
  typ: JWT
message:
  exp: 1752686240 = 2025-07-16T17:17:20Z (in 5m 55s)
  nbf: 1752685880 = 2025-07-16T17:11:20Z (5s ago)
signature: voEB1O4AnPdCWHARf_1jTNA5CpayxWGyXfMf6p_wfbw"#
                .trim(),
            conversion.trim()
        );
        assert_eq!("Not found", ASN1.get_conversion("json").await);
    }
}
