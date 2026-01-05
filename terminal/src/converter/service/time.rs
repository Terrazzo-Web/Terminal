use std::sync::LazyLock;
use std::time::Duration;
use std::time::SystemTime;

use regex::Captures;
use regex::Regex;

use super::AddConversionFn;
use crate::converter::api::Language;

pub fn add_time(input: &str, add: &mut impl AddConversionFn) -> bool {
    static NUMBER: LazyLock<Regex> = LazyLock::new(|| Regex::new("\\d+").unwrap());
    let mut has_time = false;
    let timestamp = NUMBER.replace_all(input, |captures: &Captures<'_>| {
        if let Some(time) = process_time(&captures[0]) {
            has_time = true;
            return time;
        } else {
            return captures[0].to_owned();
        }
    });
    if has_time {
        add(Language::new("Timestamp"), timestamp.into_owned());
    }
    return true;
}

fn process_time(input: &str) -> Option<String> {
    let time = input.parse().ok()?;
    let (time, unit) = match time {
        0..=9_999_999_999 => (Duration::from_secs(time), "seconds"),
        10_000_000_000..=9_999_999_999_999 => (Duration::from_millis(time), "millis"),
        10_000_000_000_000..=9_999_999_999_999_999 => (Duration::from_micros(time), "micros"),
        _ => (Duration::from_nanos(time), "nanos"),
    };
    if !(Duration::from_hours(24 * 365 * 10) < time && time < Duration::from_hours(24 * 365 * 1000))
    {
        return None;
    }
    let time = humantime::format_rfc3339(SystemTime::UNIX_EPOCH.checked_add(time)?).to_string();
    Some(format!("{time} (as {input} {unit})"))
}

#[cfg(test)]
mod tests {

    use super::super::tests::GetConversionForTest as _;

    #[tokio::test]
    async fn time() {
        let conversion = r#"
            January 4, 2026 at 12:12 PM UTC is 1767528720
            January 4, 2026 at 12:12 +30s +512 millis PM UTC is 1767528750512
            "#
        .get_conversion("Timestamp")
        .await;
        assert_eq!(
            r#"
            January 4, 2026 at 12:12 PM UTC is 2026-01-04T12:12:00Z (as 1767528720 seconds)
            January 4, 2026 at 12:12 +30s +512 millis PM UTC is 2026-01-04T12:12:30.512000000Z (as 1767528750512 millis)
            "#.trim(),
            conversion.trim()
        );
    }

    #[tokio::test]
    async fn not_time() {
        let conversion = r#"
            1767528720 is a time
            176752872 is not a time
            176752872176752872176752872 is also not a time
            "#
        .get_conversion("Timestamp")
        .await;
        assert_eq!(
            r#"
            2026-01-04T12:12:00Z (as 1767528720 seconds) is a time
            176752872 is not a time
            176752872176752872176752872 is also not a time
            "#
            .trim(),
            conversion.trim()
        );
    }
}
