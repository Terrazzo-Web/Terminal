use base64::Engine as _;
use base64::prelude::BASE64_STANDARD;
use base64::prelude::BASE64_STANDARD_NO_PAD;
use base64::prelude::BASE64_URL_SAFE;
use base64::prelude::BASE64_URL_SAFE_NO_PAD;

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
