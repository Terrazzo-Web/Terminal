use std::time::Duration;
use std::time::SystemTime;

use base64::Engine;
use base64::prelude::BASE64_STANDARD;
use base64::prelude::BASE64_STANDARD_NO_PAD;
use base64::prelude::BASE64_URL_SAFE;
use base64::prelude::BASE64_URL_SAFE_NO_PAD;

use super::AddConversionFn;
use crate::converter::api::Language;

pub fn add_jwt(input: &str, add: &mut impl AddConversionFn) {
    let Some(jwt) = get_jwt_impl(input) else {
        return;
    };
    add(Language::new("jwt"), jwt)
}

fn get_jwt_impl(input: &str) -> Option<String> {
    let mut split = input.splitn(3, '.');
    let header = parse_base64_json(split.next()?)?;
    let mut message = parse_base64_json(split.next()?)?;
    let signature = split.next()?;

    for time_claim in ["iat", "nbf", "exp"] {
        try_convert_time_claim(time_claim, &mut message);
    }

    #[derive(serde::Serialize)]
    struct Jwt<'t> {
        header: serde_json::Value,
        message: serde_json::Value,
        signature: &'t str,
    }
    Some(
        serde_yaml_ng::to_string(&Jwt {
            header,
            message,
            signature,
        })
        .ok()?,
    )
}

fn try_convert_time_claim(time_claim: &str, message: &mut serde_json::Value) -> Option<()> {
    let time_claim = message.get_mut(time_claim)?;
    let serde_json::Value::Number(time) = time_claim else {
        return None;
    };
    let unix_time = time.as_u64()?;
    let time = SystemTime::UNIX_EPOCH.checked_add(Duration::from_secs(unix_time))?;
    let now = if cfg!(test) {
        SystemTime::UNIX_EPOCH + Duration::from_secs(1752685885)
    } else {
        SystemTime::now()
    };
    let delta = if time >= now {
        format!(
            "in {}",
            humantime::format_duration(time.duration_since(now).ok()?)
        )
    } else {
        format!(
            "{} ago",
            humantime::format_duration(now.duration_since(time).ok()?)
        )
    };
    let time = humantime::format_rfc3339(time).to_string();
    let time = format!("{unix_time} = {time} ({delta})");
    *time_claim = serde_json::Value::String(time);
    Some(())
}

fn parse_base64_json(data: &str) -> Option<serde_json::Value> {
    let data = parse_base64(data)?;
    let data = String::from_utf8_lossy(&data);
    serde_json::from_str::<serde_json::Value>(&data).ok()
}

fn parse_base64(data: &str) -> Option<Vec<u8>> {
    for engine in [
        BASE64_STANDARD,
        BASE64_STANDARD_NO_PAD,
        BASE64_URL_SAFE,
        BASE64_URL_SAFE_NO_PAD,
    ] {
        if let Ok(base64) = engine.decode(data) {
            return Some(base64);
        }
    }
    return None;
}
