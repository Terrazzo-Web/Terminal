use std::sync::OnceLock;

use regex::Regex;

use super::AddConversionFn;
use crate::converter::api::Language;

pub fn add_asn1(input: &str, add: &mut impl AddConversionFn) -> bool {
    static BASE64_REGEX: OnceLock<Regex> = OnceLock::new();
    let base64_regex = BASE64_REGEX.get_or_init(|| Regex::new(r"^[A-Za-z0-9+/\s]+$").unwrap());
    if !base64_regex.is_match(input) {
        dbg!("Not a base64!!!");
        return false;
    }
    let input: String = input.split('\n').map(str::trim).collect();
    let Some(input) = super::jwt::parse_base64(&input) else {
        dbg!("Fails to parse as base64!!!");
        return false;
    };

    let Ok(asn1) = simple_asn1::from_der(&input) else {
        dbg!("Fails to parse as asn1");
        return false;
    };

    add(Language::new("asn1"), format!("{asn1:?}"));
    return true;
}
