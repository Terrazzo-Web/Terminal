use std::sync::OnceLock;

use base64::Engine as _;
use base64::prelude::BASE64_STANDARD;
use base64::prelude::BASE64_STANDARD_NO_PAD;
use base64::prelude::BASE64_URL_SAFE;
use base64::prelude::BASE64_URL_SAFE_NO_PAD;
use regex::Regex;

use super::AddConversionFn;

pub fn add_base64(input: &str, add: &mut impl AddConversionFn) -> bool {
    static BASE64_REGEX: OnceLock<Regex> = OnceLock::new();
    let base64_regex = BASE64_REGEX
        .get_or_init(|| Regex::new(r"^[-_+/A-Za-z0-9 \r\n\t]+(?:=*)[ \r\n\t]*$").unwrap());
    if !base64_regex.is_match(input) {
        return false;
    }
    let input: String = input.split('\n').map(str::trim).collect();
    let Some(input) = parse_base64(&input) else {
        return false;
    };

    let input = &input;
    return [
        super::pkcs7::add_pkcs7(input, add),
        super::asn1::add_asn1(input, add),
    ]
    .into_iter()
    .any(|t| t);
}

pub(super) fn parse_base64(data: &str) -> Option<Vec<u8>> {
    if !data.contains(['+', '/']) {
        if data.ends_with('=') {
            if let Ok(base64) = BASE64_URL_SAFE.decode(data) {
                return Some(base64);
            }
        } else if let Ok(base64) = BASE64_URL_SAFE_NO_PAD.decode(data) {
            return Some(base64);
        }
    }
    if !data.contains(['-', '_']) {
        if data.ends_with('=') {
            if let Ok(base64) = BASE64_STANDARD.decode(data) {
                return Some(base64);
            }
        } else if let Ok(base64) = BASE64_STANDARD_NO_PAD.decode(data) {
            return Some(base64);
        }
    }
    return None;
}
