use base64::Engine as _;
use base64::prelude::BASE64_STANDARD;
use openssl::nid::Nid;
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
    async fn single_x509() {
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
    }

    #[tokio::test]
    async fn x509_chain() {
        const CERTIFICATE: &str = r#"
-----BEGIN CERTIFICATE-----
MIIDizCCAxCgAwIBAgISBiJH8Uh1EqlFCCX0FZww4iSOMAoGCCqGSM49BAMDMDIx
CzAJBgNVBAYTAlVTMRYwFAYDVQQKEw1MZXQncyBFbmNyeXB0MQswCQYDVQQDEwJF
NjAeFw0yNTA2MjYxMDE2MDRaFw0yNTA5MjQxMDE2MDNaMBoxGDAWBgNVBAMTD211
bmljaC5wYXZ5Lm9uZTBZMBMGByqGSM49AgEGCCqGSM49AwEHA0IABKtbB5I+eUGP
WHl1wKGYZq8hu/kIoUWhLXtpdvC/Lm1SJQLHHAPPvVpt0a4zp1iYr6gu58Sgaa0t
/pknyOCUwoCjggIcMIICGDAOBgNVHQ8BAf8EBAMCB4AwHQYDVR0lBBYwFAYIKwYB
BQUHAwEGCCsGAQUFBwMCMAwGA1UdEwEB/wQCMAAwHQYDVR0OBBYEFNXrpRsSLwEm
RWE4m6rPmvMIKkEuMB8GA1UdIwQYMBaAFJMnRpgDqVFojpjWxEJI2yO/WJTSMDIG
CCsGAQUFBwEBBCYwJDAiBggrBgEFBQcwAoYWaHR0cDovL2U2LmkubGVuY3Iub3Jn
LzAaBgNVHREEEzARgg9tdW5pY2gucGF2eS5vbmUwEwYDVR0gBAwwCjAIBgZngQwB
AgEwLQYDVR0fBCYwJDAioCCgHoYcaHR0cDovL2U2LmMubGVuY3Iub3JnLzE1LmNy
bDCCAQMGCisGAQQB1nkCBAIEgfQEgfEA7wB1AMz7D2qFcQll/pWbU87psnwi6YVc
DZeNtql+VMD+TA2wAAABl6vyU/0AAAQDAEYwRAIgPw5fa1/ttZNhXtX1GFN5C1KY
+A1pzc+X9251JJb3wCACIHLtpXqvOV2999aL3Cks6bTyvUTbeBlhqHEC36JtAjcz
AHYAGgT/SdBUHUCv9qDDv/HYxGcvTuzuI0BomGsXQC7ciX0AAAGXq/JcJwAABAMA
RzBFAiAlqFNNtRU1zyONybiJaEKnvikNo+B/V0Wpt+G6BNTZkgIhAO9bbObCVGDW
w6H/+P2L7JPaIwW22rYiY4bui6Pf7Q2xMAoGCCqGSM49BAMDA2kAMGYCMQD2ftY1
AgUe0bybQKh+q9F727g5YRkuGuyiS2JTBPN48KQ/YGcGn720QUsb9t8DGbICMQDK
qPFpuvKo9BaNJh98rMLxxf2UXa81LGRMvYq0NwLlpOgiIFiJj6nMMOJdJcyvIgk=
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
        let conversion = CERTIFICATE
            .get_conversion("munich.pavy.one")
            .await
            .to_ascii_lowercase();
        assert!(conversion.contains(&"Issuer".to_ascii_lowercase()));
        let conversion = CERTIFICATE.get_conversion("E6").await.to_ascii_lowercase();
        assert!(conversion.contains(&"Issuer".to_ascii_lowercase()));
    }
}
