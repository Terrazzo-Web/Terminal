use std::time::Duration;
use std::time::SystemTime;

use super::AddConversionFn;
use super::base64::parse_base64;
use crate::converter::api::Language;

pub fn add_jwt(input: &str, add: &mut impl AddConversionFn) -> bool {
    let Some(jwt) = get_jwt_impl(input.trim()) else {
        return false;
    };
    add(Language::new("JWT"), jwt);
    return true;
}

fn get_jwt_impl(input: &str) -> Option<String> {
    let mut split = input.split('.');
    let header = split.next()?;
    let message = split.next()?;
    let signature = split.next()?;
    let None = split.next() else {
        return None;
    };

    let header = parse_base64_json(header)?;
    let mut message = parse_base64_json(message)?;
    let None = split.next() else {
        return None;
    };

    for time_claim in ["iat", "issuedat", "nbf", "notbefore", "exp", "expiration"] {
        try_convert_time_claim(time_claim, &mut message);
    }

    #[derive(serde::Serialize)]
    struct Jwt<'t> {
        header: serde_json::Value,
        message: serde_json::Value,
        signature: &'t str,
    }
    serde_yaml_ng::to_string(&Jwt {
        header,
        message,
        signature,
    })
    .ok()
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

#[cfg(test)]
mod tests {
    use super::super::tests::GetConversionForTest as _;

    const JWT: &str = "eyJ0eXAiOiJKV1QiLCJhbGciOiJIUzI1NiJ9.eyJleHAiOjE3NTI2ODYyNDAsIm5iZiI6MTc1MjY4NTg4MH0.voEB1O4AnPdCWHARf_1jTNA5CpayxWGyXfMf6p_wfbw";

    #[tokio::test]
    async fn jwt() {
        let conversion = JWT.get_conversion("JWT").await;
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
        assert_eq!("Not found", JWT.get_conversion("JSON").await);
    }

    #[tokio::test]
    async fn jwt_trim() {
        let conversion = format!("\r\n\t{JWT}\r\n\t")
            .as_str()
            .get_conversion("JWT")
            .await;
        assert!(conversion.contains("typ: JWT"));
    }

    #[tokio::test]
    async fn jwt_invalid() {
        let conversion = format!("{JWT}.ABC").as_str().get_conversion("JWT").await;
        assert_eq!("Not found", conversion);
    }
}
