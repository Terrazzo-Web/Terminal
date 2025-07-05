#![cfg(feature = "server")]

use std::ops::Deref;
use std::path::Path;
use std::path::PathBuf;

pub fn concat_base_file_path(
    base_path: impl Deref<Target = str>,
    file_path: impl Deref<Target = str>,
) -> std::path::PathBuf {
    let base_path = base_path.trim();
    let file_path = file_path.trim().trim_start_matches('/');
    let file_path = canonicalize(file_path);
    canonicalize(base_path).join(file_path.strip_prefix("/").unwrap_or(&file_path))
}

pub fn canonicalize(path: impl AsRef<Path>) -> PathBuf {
    let mut canonicalized = PathBuf::new();
    for leg in path.as_ref() {
        if leg == ".." {
            if let Some(parent) = canonicalized.parent() {
                canonicalized = parent.to_owned();
            }
        } else {
            canonicalized.push(leg);
        }
    }
    return canonicalized;
}

#[cfg(test)]
mod tests {
    use std::borrow::Cow;

    use fluent_asserter::prelude::*;

    use crate::utils::more_path::MorePath;

    #[test]
    fn concat_base_file_path() {
        assert_eq!(
            "/",
            super::concat_base_file_path("/", "/").to_owned_string()
        );
        assert_eq!(
            "/a/b",
            super::concat_base_file_path("/a", "/b/").to_owned_string()
        );
    }

    #[test]
    fn canonicalize() {
        assert_that!(
            super::canonicalize("/a/b")
                .iter()
                .map(|leg| leg.to_string_lossy())
                .collect::<Vec<_>>()
        )
        .is_equal_to(["/", "a", "b"].map(Cow::from).into());
        assert_that!(
            super::canonicalize("a/b")
                .iter()
                .map(|leg| leg.to_string_lossy())
                .collect::<Vec<_>>()
        )
        .is_equal_to(["a", "b"].map(Cow::from).into());
        assert_that!(
            super::canonicalize("//a/b//.")
                .iter()
                .map(|leg| leg.to_string_lossy())
                .collect::<Vec<_>>()
        )
        .is_equal_to(["/", "a", "b"].map(Cow::from).into());
        assert_that!(
            super::canonicalize("/a/./b/.")
                .iter()
                .map(|leg| leg.to_string_lossy())
                .collect::<Vec<_>>()
        )
        .is_equal_to(["/", "a", "b"].map(Cow::from).into());
        assert_that!(
            super::canonicalize("/a/./b/../c/.")
                .iter()
                .map(|leg| leg.to_string_lossy())
                .collect::<Vec<_>>()
        )
        .is_equal_to(["/", "a", "c"].map(Cow::from).into());
        assert_that!(
            super::canonicalize("/a/../../b/./c/.")
                .iter()
                .map(|leg| leg.to_string_lossy())
                .collect::<Vec<_>>()
        )
        .is_equal_to(["/", "b", "c"].map(Cow::from).into());
    }
}
