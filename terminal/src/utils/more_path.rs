use std::borrow::Cow;
use std::ffi::OsStr;
use std::ffi::OsString;
use std::path::Path;
use std::path::PathBuf;

#[allow(dead_code)]
pub trait MorePath {
    fn to_owned_string(self) -> String;
}

impl MorePath for &Path {
    fn to_owned_string(self) -> String {
        self.as_os_str().to_owned_string()
    }
}

impl MorePath for PathBuf {
    fn to_owned_string(self) -> String {
        self.into_os_string().to_owned_string()
    }
}

impl MorePath for &OsStr {
    fn to_owned_string(self) -> String {
        let cow: Cow<'_, str> = self.to_string_lossy();
        return cow.into_owned();
    }
}

impl MorePath for OsString {
    fn to_owned_string(self) -> String {
        match self.into_string() {
            Ok(string) => string,
            Err(os_string) => os_string.as_os_str().to_owned_string(),
        }
    }
}
