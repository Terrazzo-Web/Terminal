use std::borrow::Borrow;
use std::hash::Hash;
use std::ops::Deref;
use std::sync::Arc;

/// Represents a string that is cheap to copy.
#[derive(Debug, Clone)]
pub enum XString {
    Str(&'static str),
    Arc(Arc<str>),
}

#[allow(unused)]
impl XString {
    #[must_use]
    pub fn as_str(&self) -> &str {
        match self {
            XString::Str(str) => str,
            XString::Arc(arc) => arc.as_ref(),
        }
    }
}

impl AsRef<str> for XString {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

impl Deref for XString {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        self.as_ref()
    }
}

impl Borrow<str> for XString {
    fn borrow(&self) -> &str {
        self.as_str()
    }
}

impl std::fmt::Display for XString {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        std::fmt::Display::fmt(self.as_str(), f)
    }
}

impl Default for XString {
    fn default() -> Self {
        Self::Str("")
    }
}

impl PartialEq for XString {
    fn eq(&self, other: &Self) -> bool {
        PartialEq::eq(self.as_str(), other.as_str())
    }
}

impl Eq for XString {}

impl PartialOrd for XString {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(Ord::cmp(&self, &other))
    }
}

impl Ord for XString {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        Ord::cmp(self.as_str(), other.as_str())
    }
}

impl Hash for XString {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        Hash::hash(self.as_str(), state);
    }
}

impl From<String> for XString {
    fn from(string: String) -> Self {
        Self::Arc(string.into())
    }
}

impl From<&'static str> for XString {
    fn from(str: &'static str) -> Self {
        Self::Str(str)
    }
}

impl From<Arc<str>> for XString {
    fn from(arc: Arc<str>) -> Self {
        Self::Arc(arc)
    }
}

impl From<bool> for XString {
    fn from(t: bool) -> Self {
        Self::Str(if t { "true" } else { "false" })
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;
    use std::sync::Arc;

    use super::XString;

    #[test]
    fn from_str() {
        let xs: XString = "value".into();
        assert_eq!(format!(" {xs:?} "), r#" Str("value") "#);
    }

    #[test]
    fn from_arc() {
        let s: Arc<str> = "value".into();
        let xs: XString = s.into();
        assert_eq!(format!(" {xs:?} "), r#" Arc("value") "#);
    }

    #[test]
    fn eq() {
        assert_eq!(XString::Str("A"), XString::Str("A"));
        assert_eq!(XString::Str("A"), XString::Arc("A".into()));
        assert_eq!(XString::Arc("A".into()), XString::Arc("A".into()));
    }

    #[test]
    fn ne() {
        assert_ne!(XString::Str("A"), XString::Str("B"));
        assert_ne!(XString::Str("A"), XString::Arc("B".into()));
    }

    #[test]
    fn ord() {
        let mut strings = vec![
            XString::Arc("b-arc".into()),
            XString::Str("a-str"),
            XString::Str("d-str"),
            XString::Arc("c-arc".into()),
        ];

        strings.sort();
        assert_eq!(
            vec![
                XString::Str("a-str"),
                XString::Arc("b-arc".into()),
                XString::Arc("c-arc".into()),
                XString::Str("d-str"),
            ],
            strings
        );
    }

    #[test]
    fn hash() {
        let strings = [
            XString::Str("a-str"),
            XString::Arc("b-arc".into()),
            XString::Arc("c-arc".into()),
            XString::Str("d-str"),
        ]
        .into_iter()
        .collect::<HashSet<_>>();

        assert!(strings.contains("a-str"));
        assert!(strings.contains("b-arc"));
        assert!(strings.contains("c-arc"));
        assert!(strings.contains("d-str"));

        assert!(strings.contains(&XString::from("a-str")));
        assert!(strings.contains(&XString::from("a-str".to_string())));

        assert!(!strings.contains("x"));
        assert!(!strings.contains(&XString::from("y")));
    }
}
