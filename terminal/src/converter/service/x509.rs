use base64::Engine as _;
use base64::prelude::BASE64_STANDARD;
use openssl::nid::Nid;
use openssl::x509::X509;
use openssl::x509::X509Ref;
use trz_gateway_common::security_configuration::common::parse_pem_certificates;
use x509_parser::prelude::FromDer as _;
use x509_parser::prelude::X509Certificate;

use super::AddConversionFn;
use crate::converter::api::Language;

pub fn add_x509_pem(input: &str, add: &mut impl AddConversionFn) -> bool {
    if !input.contains("-----BEGIN CERTIFICATE-----") {
        return false;
    }
    let input = input
        .split('\n')
        .map(|line| line.trim())
        .collect::<Vec<_>>()
        .join("\n");
    let certificates = parse_pem_certificates(&input)
        .filter_map(|x509| x509.ok())
        .collect::<Vec<_>>();
    let mut result = false;
    for x509 in certificates {
        let Ok(Ok(mut text)) = x509.to_text().map(String::from_utf8) else {
            continue;
        };
        add_extensions(&x509, &mut text);
        add(Language::new(get_certificate_name(&x509)), text);
        result = true;
    }
    result
}

pub fn add_x509_base64(input: &[u8], add: &mut impl AddConversionFn) -> bool {
    let Ok(x509) = X509::from_der(input) else {
        return false;
    };
    let Ok(Ok(mut text)) = x509.to_text().map(String::from_utf8) else {
        return false;
    };
    add_extensions(&x509, &mut text);
    add(Language::new(get_certificate_name(&x509)), text);
    return true;
}

fn get_certificate_name(x509: &X509Ref) -> String {
    get_certificate_common_name(x509).unwrap_or_else(|| format!("{:?}", x509.subject_name()))
}

fn get_certificate_common_name(x509: &X509Ref) -> Option<String> {
    Some(
        x509.subject_name()
            .entries_by_nid(Nid::COMMONNAME)
            .next()?
            .data()
            .as_utf8()
            .ok()?
            .to_string(),
    )
}

fn add_extensions(x509: &X509Ref, text: &mut String) -> Option<()> {
    let der = x509.to_der().ok()?;
    let (_, certificate) = X509Certificate::from_der(&der).ok()?;
    let mut extensions = vec![];
    for extension in certificate.extensions() {
        let is_ascii = extension.value.iter().all(|c| c.is_ascii_graphic());
        extensions.push(Extension {
            oid: extension.oid.to_id_string(),
            critical: extension.critical,
            value: is_ascii
                .then(|| str::from_utf8(extension.value).ok().map(str::to_owned))
                .flatten()
                .unwrap_or_else(|| {
                    BASE64_STANDARD
                        .encode(extension.value)
                        .as_bytes()
                        .chunks(64)
                        .map(|chunk| std::str::from_utf8(chunk).unwrap())
                        .collect::<Vec<_>>()
                        .join("\n")
                }),
        });
    }
    if !extensions.is_empty() {
        text.extend(serde_yaml_ng::to_string(&Extensions { extensions }));
    }
    Some(())
}

#[derive(serde::Serialize)]
struct Extensions {
    extensions: Vec<Extension>,
}

#[derive(serde::Serialize)]
struct Extension {
    oid: String,
    critical: bool,
    value: String,
}

#[cfg(test)]
mod tests {
    use super::super::tests::GetConversionForTest as _;

    #[tokio::test]
    async fn single_x509_pem() {
        const CERTIFICATE: &str = r#"
-----BEGIN CERTIFICATE-----
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
-----END CERTIFICATE-----
"#;
        let conversion = CERTIFICATE
            .get_conversion("Terrazzo Terminal Root CA")
            .await
            .to_ascii_lowercase();
        assert!(conversion.contains(&"Issuer".to_ascii_lowercase()));
        assert!(conversion.contains(&"Subject".to_ascii_lowercase()));
        assert!(conversion.contains(&"Not Before".to_ascii_lowercase()));
        assert!(conversion.contains(&"Not After".to_ascii_lowercase()));

        let conversion = CERTIFICATE
            .replace("\n", "\n\t")
            .as_str()
            .get_conversion("Terrazzo Terminal Root CA")
            .await
            .to_ascii_lowercase();
        assert!(conversion.contains(&"Issuer".to_ascii_lowercase()));

        assert_eq!(
            vec!["Terrazzo Terminal Root CA"],
            CERTIFICATE.get_languages().await
        );
    }

    #[tokio::test]
    async fn single_x509_der() {
        const CERTIFICATE: &str = r#"
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
        let conversion = CERTIFICATE
            .get_conversion("Terrazzo Terminal Root CA")
            .await;
        assert!(conversion.contains(&"Terrazzo Terminal Root CA"));
        assert_eq!(
            vec!["ASN.1", "Terrazzo Terminal Root CA"],
            CERTIFICATE.get_languages().await
        );
    }

    #[tokio::test]
    async fn x509_chain() {
        const CERTIFICATE: &str = r#"
-----BEGIN CERTIFICATE-----
MIIIvTCCCGSgAwIBAgIUKA0KjYMYx9iWca5bQRJP/uya/T4wCgYIKoZIzj0EAwIw
NzERMA8GA1UECgwIVGVycmF6em8xIjAgBgNVBAMMGVRlcnJhenpvIFRlcm1pbmFs
IFJvb3QgQ0EwHhcNMjUwNzAzMTc1NzM1WhcNMjUxMDAxMTc1NzM1WjARMQ8wDQYD
VQQDDAZRd2VydHkwWTATBgcqhkjOPQIBBggqhkjOPQMBBwNCAATdtKVt+VVwOTuu
hswmVT2PVSqZrB3NVs/tVa09QGlbW5htyGupxfkoERuXWwiXoPZ9ILE/Lg/ae5q4
UhTtHPyho4IHcjCCB24wHQYDVR0OBBYEFHkQhATtZpUOaS6HqIxoEzWsKTPVMAwG
A1UdEwEB/wQCMAAwDgYDVR0PAQH/BAQDAgeAMCAGA1UdJQEB/wQWMBQGCCsGAQUF
BwMBBggrBgEFBQcDAjBzBgNVHSMEbDBqgBQQLlhEvThsQOJn2iJzU9lzttH0/qE7
pDkwNzERMA8GA1UECgwIVGVycmF6em8xIjAgBgNVBAMMGVRlcnJhenpvIFRlcm1p
bmFsIFJvb3QgQ0GCFQDUjfgVJdSn/F4xPGp0qVxrNR32ljARBgNVHREECjAIggZR
d2VydHkwggaDBgorBgEEAYI3CmMBBIIGczCCBm8GCSqGSIb3DQEHAqCCBmAwggZc
AgEDMQ0wCwYJYIZIAWUDBAIBMEwGCSqGSIb3DQEHAaA/BD1Rd2VydHk6MTc1MTU2
NTQ1NToxNzU5MzQxNDU1OpEVFK4rDwTrpkGRY8wb957kiiXUhE+zGbpxz7jaIOXS
oIIEjTCCAjAwggHXoAMCAQICFHPrBUZV0qb/WeUJoHdunbTwjNm3MAoGCCqGSM49
BAMCMDcxETAPBgNVBAoMCFRlcnJhenpvMSIwIAYDVQQDDBlUZXJyYXp6byBUZXJt
aW5hbCBSb290IENBMB4XDTI1MDYwNjEwMDEyN1oXDTQ1MDYwMTEwMDEyN1owPzER
MA8GA1UECgwIVGVycmF6em8xKjAoBgNVBAMMIVRlcnJhenpvIFRlcm1pbmFsIElu
dGVybWVkaWF0ZSBDQTBZMBMGByqGSM49AgEGCCqGSM49AwEHA0IABAfgjXnTrtjz
zr4/003q4GIlxKdiGlPsj9ZKNnu9SrM2ro3+lzz9XCCmXCNKQSBZyanVL6YjS+UX
K/r+xxG8efOjgbgwgbUwHQYDVR0OBBYEFDe9wI0um5s0TumS31Uh++z0LcYdMA8G
A1UdEwEB/wQFMAMBAf8wDgYDVR0PAQH/BAQDAgEGMHMGA1UdIwRsMGqAFBAuWES9
OGxA4mfaInNT2XO20fT+oTukOTA3MREwDwYDVQQKDAhUZXJyYXp6bzEiMCAGA1UE
AwwZVGVycmF6em8gVGVybWluYWwgUm9vdCBDQYIVANSN+BUl1Kf8XjE8anSpXGs1
HfaWMAoGCCqGSM49BAMCA0cAMEQCIDtLGiNSIQQOSVrsByTfMYOmKJxC2kVpLPJj
i6d9c1XMAiAenWr5SBEA+UlCXZHcnkQpBHEx94nsn0gfct0VPMeVqzCCAlUwggH8
oAMCAQICFQDY4zejNs9G+5ZkGxS4jtwN/j73MzAKBggqhkjOPQQDAjA/MREwDwYD
VQQKDAhUZXJyYXp6bzEqMCgGA1UEAwwhVGVycmF6em8gVGVybWluYWwgSW50ZXJt
ZWRpYXRlIENBMB4XDTI1MDYwNjEwMDEyN1oXDTQ1MDYwMTEwMDEyN1owJzERMA8G
A1UECgwIVGVycmF6em8xEjAQBgNVBAMMCWxvY2FsaG9zdDBZMBMGByqGSM49AgEG
CCqGSM49AwEHA0IABFENJlIiAJzTUiMKCCU36uRE9vbxnnjDoikW4ldg+S3crfiW
GQOrnWmkXnmbPpHhgvvfpaJGdqMdOa43QGU7/4mjgewwgekwHQYDVR0OBBYEFPjE
1LH/QqDMCKe1uvrTXyTJ00h5MAwGA1UdEwEB/wQCMAAwDgYDVR0PAQH/BAQDAgeA
MCAGA1UdJQEB/wQWMBQGCCsGAQUFBwMBBggrBgEFBQcDAjByBgNVHSMEazBpgBQ3
vcCNLpubNE7pkt9VIfvs9C3GHaE7pDkwNzERMA8GA1UECgwIVGVycmF6em8xIjAg
BgNVBAMMGVRlcnJhenpvIFRlcm1pbmFsIFJvb3QgQ0GCFHPrBUZV0qb/WeUJoHdu
nbTwjNm3MBQGA1UdEQQNMAuCCWxvY2FsaG9zdDAKBggqhkjOPQQDAgNHADBEAiBY
HxIi7bJWfEaZ/7KOIEp9Qg2QXXbsVu6kSfF2alyM2wIgFJWPvI5vzLL42wPH4XZd
BQhcddRsAYEiHdSf5wPnZKIxggFnMIIBYwIBA4AU+MTUsf9CoMwIp7W6+tNfJMnT
SHkwCwYJYIZIAWUDBAIBoIHkMBgGCSqGSIb3DQEJAzELBgkqhkiG9w0BBwEwHAYJ
KoZIhvcNAQkFMQ8XDTI1MDcwMzE3NTczNVowLwYJKoZIhvcNAQkEMSIEICAGOlWd
od5k4+LevYrmrJXnrL/vecL8NPkJP1hWt755MHkGCSqGSIb3DQEJDzFsMGowCwYJ
YIZIAWUDBAEqMAsGCWCGSAFlAwQBFjALBglghkgBZQMEAQIwCgYIKoZIhvcNAwcw
DgYIKoZIhvcNAwICAgCAMA0GCCqGSIb3DQMCAgFAMAcGBSsOAwIHMA0GCCqGSIb3
DQMCAgEoMAoGCCqGSM49BAMCBEgwRgIhAJRZdiJVQ7g93r8yuyTvb8XR5ETMM3bb
ltQlIbpLHpcbAiEAxiG+q9FAadM7OihsEv9XNXJtsQfvbtgvtj4uPZl64NYwCgYI
KoZIzj0EAwIDRwAwRAIgK2gPIIZ9vlvcri6dW+96WM5u5wKa1CADK02ZxpyThFkC
IH0+EtbcGRS0S+pPg4PlTbttE8pP61nGTg/U3Lf17WZ+
-----END CERTIFICATE-----

-----BEGIN CERTIFICATE-----
MIIEVzCCAj+gAwIBAgIRALBXPpFzlydw27SHyzpFKzgwDQYJKoZIhvcNAQELBQAw
TzELMAkGA1UEBhMCVVMxKTAnBgNVBAoTIEludGVybmV0IFNlY3VyaXR5IFJlc2Vh
cmNoIEdyb3VwMRUwEwYDVQQDEwxJU1JHIFJvb3QgWDEwHhcNMjQwMzEzMDAwMDAw
WhcNMjcwMzEyMjM1OTU5WjAyMQswCQYDVQQGEwJVUzEWMBQGA1UEChMNTGV0J3Mg
RW5jcnlwdDELMAkGA1UEAxMCRTYwdjAQBgcqhkjOPQIBBgUrgQQAIgNiAATZ8Z5G
h/ghcWCoJuuj+rnq2h25EqfUJtlRFLFhfHWWvyILOR/VvtEKRqotPEoJhC6+QJVV
6RlAN2Z17TJOdwRJ+HB7wxjnzvdxEP6sdNgA1O1tHHMWMxCcOrLqbGL0vbijgfgw
gfUwDgYDVR0PAQH/BAQDAgGGMB0GA1UdJQQWMBQGCCsGAQUFBwMCBggrBgEFBQcD
ATASBgNVHRMBAf8ECDAGAQH/AgEAMB0GA1UdDgQWBBSTJ0aYA6lRaI6Y1sRCSNsj
v1iU0jAfBgNVHSMEGDAWgBR5tFnme7bl5AFzgAiIyBpY9umbbjAyBggrBgEFBQcB
AQQmMCQwIgYIKwYBBQUHMAKGFmh0dHA6Ly94MS5pLmxlbmNyLm9yZy8wEwYDVR0g
BAwwCjAIBgZngQwBAgEwJwYDVR0fBCAwHjAcoBqgGIYWaHR0cDovL3gxLmMubGVu
Y3Iub3JnLzANBgkqhkiG9w0BAQsFAAOCAgEAfYt7SiA1sgWGCIpunk46r4AExIRc
MxkKgUhNlrrv1B21hOaXN/5miE+LOTbrcmU/M9yvC6MVY730GNFoL8IhJ8j8vrOL
pMY22OP6baS1k9YMrtDTlwJHoGby04ThTUeBDksS9RiuHvicZqBedQdIF65pZuhp
eDcGBcLiYasQr/EO5gxxtLyTmgsHSOVSBcFOn9lgv7LECPq9i7mfH3mpxgrRKSxH
pOoZ0KXMcB+hHuvlklHntvcI0mMMQ0mhYj6qtMFStkF1RpCG3IPdIwpVCQqu8GV7
s8ubknRzs+3C/Bm19RFOoiPpDkwvyNfvmQ14XkyqqKK5oZ8zhD32kFRQkxa8uZSu
h4aTImFxknu39waBxIRXE4jKxlAmQc4QjFZoq1KmQqQg0J/1JF8RlFvJas1VcjLv
YlvUB2t6npO6oQjB3l+PNf0DpQH7iUx3Wz5AjQCi6L25FjyE06q6BZ/QlmtYdl/8
ZYao4SRqPEs/6cAiF+Qf5zg2UkaWtDphl1LKMuTNLotvsX99HP69V2faNyegodQ0
LyTApr/vT01YPE46vNsDLgK+4cL6TrzC/a4WcmF5SRJ938zrv/duJHLXQIku5v0+
EwOy59Hdm0PT/Er/84dDV0CSjdR/2XuZM3kpysSKLgD1cKiDA+IRguODCxfO9cyY
Ig46v9mFmBvyH04=
-----END CERTIFICATE-----
"#;
        assert_eq!(vec!["E6", "Qwerty"], CERTIFICATE.get_languages().await);
        let conversion = CERTIFICATE
            .get_conversion("Qwerty")
            .await
            .to_ascii_lowercase();
        assert!(conversion.contains(&"Issuer".to_ascii_lowercase()));
        let conversion = CERTIFICATE.get_conversion("E6").await.to_ascii_lowercase();
        assert!(conversion.contains(&"Issuer".to_ascii_lowercase()));
    }
}
