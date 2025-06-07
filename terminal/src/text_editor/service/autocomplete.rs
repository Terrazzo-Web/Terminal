#![cfg(feature = "server")]

use std::cmp::Reverse;
use std::ffi::OsStr;
use std::fmt::Debug;
use std::fs::Metadata;
use std::path::Path;
use std::path::PathBuf;
use std::time::Duration;
use std::time::SystemTime;

use nameth::NamedEnumValues as _;
use nameth::nameth;
use terrazzo::http::StatusCode;
use tracing::debug;
use tracing::debug_span;
use trz_gateway_common::http_error::HttpError;
use trz_gateway_common::http_error::IsHttpError;

use crate::text_editor::PathSelector;

const ROOT: &str = "/";
const MAX_RESULTS: usize = 20;

pub fn autocomplete_path(
    kind: PathSelector,
    prefix: String,
    input: String,
) -> Result<Vec<String>, HttpError<AutoCompleteError>> {
    let prefix = prefix.trim();
    let input = input.trim();
    let input = if prefix.is_empty() && input.is_empty() {
        std::env::home_dir()
            .unwrap_or_default()
            .to_string_lossy()
            .into_owned()
    } else {
        format!("{prefix}/{input}")
    };
    Ok(autocomplete_path_impl(prefix.as_ref(), input, |m| {
        kind.accept(m)
    })?)
}

pub fn autocomplete_path_impl(
    prefix: &Path,
    input: String,
    leaf_filter: impl Fn(&Metadata) -> bool,
) -> Result<Vec<String>, AutoCompleteError> {
    let _span = debug_span!("Autocomplete", input).entered();
    let path = canonicalize(if input.is_empty() { ROOT } else { &input }.as_ref());
    let ends_with_slash = input.ends_with("/");
    let ends_with_slashdot = input.ends_with("/.");
    if !ends_with_slashdot {
        if let Ok(metadata) = path
            .metadata()
            .inspect_err(|error| debug!("Path does not exist, finding best match. Error={error}"))
        {
            if metadata.is_dir() {
                debug!("List directory");
                return list_folders(prefix, &path, leaf_filter);
            } else {
                debug!("List parent directory");
                let parent = path.parent().unwrap_or(ROOT.as_ref());
                return list_folders(prefix, parent, leaf_filter);
            }
        }
    }
    return resolve_path(
        prefix,
        &path,
        ends_with_slash,
        ends_with_slashdot,
        leaf_filter,
    );
}

fn canonicalize(path: &Path) -> PathBuf {
    let mut canonicalized = PathBuf::new();
    for leg in path {
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

fn list_folders(
    prefix: &Path,
    path: &Path,
    leaf_filter: impl Fn(&Metadata) -> bool,
) -> Result<Vec<String>, AutoCompleteError> {
    let mut result = vec![];
    if let Some(parent) = path.parent() {
        result.push(parent.to_owned());
    }
    for child in path.read_dir().map_err(AutoCompleteError::ListDir)? {
        let Ok(child) = child.map_err(|error| debug!("Error when reading {path:?}: {error}"))
        else {
            continue;
        };
        let child_path = child.path();

        // Check that it is not a hidden file.
        {
            let Some(file_name) = child_path
                .file_name()
                .and_then(|file_name| file_name.to_str())
            else {
                continue;
            };
            if file_name.starts_with(".") {
                continue;
            }
        }

        // Check that it is a folder.
        {
            let Ok(metadata) = child_path
                .metadata()
                .map_err(|error| debug!("Error when getting metadata for {child_path:?}: {error}"))
            else {
                continue;
            };
            if !leaf_filter(&metadata) {
                continue;
            }
        }
        result.push(child_path)
    }
    return Ok(sort_result(prefix, result));
}

fn sort_result(prefix: &Path, mut result: Vec<impl AsRef<Path>>) -> Vec<String> {
    if result.len() > MAX_RESULTS {
        result.sort_by_cached_key(|path| {
            let age: Option<Duration> = path
                .as_ref()
                .metadata()
                .ok()
                .and_then(|metadata| metadata.modified().ok())
                .and_then(|modified| modified.duration_since(SystemTime::UNIX_EPOCH).ok());
            // Sort youngest first / oldest last or error.
            Reverse(age.unwrap_or(Duration::ZERO))
        });
        result = result.into_iter().take(MAX_RESULTS).collect();
    }

    let mut result: Vec<String> = result
        .into_iter()
        .filter_map(|path| {
            let path = path.as_ref().strip_prefix(prefix).ok()?;
            Some(path.to_string_lossy().into_owned())
        })
        .collect();
    result.sort();
    return result;
}

fn resolve_path(
    prefix: &Path,
    path: &Path,
    ends_with_slash: bool,
    ends_with_slashdot: bool,
    leaf_filter: impl Fn(&Metadata) -> bool,
) -> Result<Vec<String>, AutoCompleteError> {
    let mut result = vec![];
    if let Some(parent) = path.parent() {
        result.push(parent.to_owned());
    }
    let ancestors = {
        let mut ancestors = vec![];
        for ancestor in path.ancestors() {
            if let Some(ancestor_name) = ancestor.file_name() {
                ancestors.push(ancestor_name.as_ref());
            }
        }
        if ancestors.is_empty() {
            ancestors.push(ROOT.as_ref());
        } else {
            ancestors.reverse();
        }
        if ends_with_slash {
            ancestors.push("".as_ref());
        } else if ends_with_slashdot {
            ancestors.push(".".as_ref());
        }
        ancestors
    };
    populate_paths(
        &mut result,
        PathBuf::from(ROOT),
        None,
        &ancestors,
        &leaf_filter,
    );
    Ok(sort_result(prefix, result))
}

fn populate_paths<'a>(
    result: &mut Vec<PathBuf>,
    accu: PathBuf,
    metadata: Option<Metadata>,
    ancestors: &[&OsStr],
    leaf_filter: &impl Fn(&Metadata) -> bool,
) {
    let [leg, ancestors @ ..] = &ancestors else {
        let metadata = metadata.or_else(|| accu.metadata().ok());
        if metadata
            .map(|metadata| leaf_filter(&metadata))
            .unwrap_or(false)
        {
            debug!("Found matching leaf {accu:?}");
            result.push(accu);
        }
        return;
    };

    debug!(?accu, ?leg, "Populate path. ancestors={ancestors:?}");

    // If "/{accu}/{leg}" exists, return it.
    // Note: only the last leg can be "" (or ".") if ends_with_slash (or ends_with_slashdot).
    if !ancestors.is_empty() || leg.as_encoded_bytes() != b"." && !leg.is_empty() {
        let mut child_accu = accu.to_path_buf();
        child_accu.push(leg);
        if let Ok(metadata) = child_accu.metadata() {
            debug!("Exact match {child_accu:?}");
            populate_paths(result, child_accu, Some(metadata), ancestors, leaf_filter);
            return;
        }
    }

    let Some(leg) = leg.to_str() else {
        debug!("Can't match against something that is not a UTF-8 string: {leg:?}");
        return;
    };
    let leg_lc = leg.to_lowercase();

    let Ok(accu_read_dir) = accu.read_dir() else {
        debug!("Not a folder {accu:?}");
        return;
    };

    // Populate "/{accu}/{child}" for every matching child.
    for child in accu_read_dir.filter_map(|child| child.ok()) {
        let child_name = child.file_name();
        if child_name.as_encoded_bytes().starts_with(b".") {
            // Skip "." and ".."
            if let b"." | b".." = child_name.as_encoded_bytes() {
                continue;
            }

            // Only match hidden files if leg starts with '.'
            if !leg.starts_with('.') {
                continue;
            }
        }

        let Some(child_name) = child_name.to_str() else {
            debug!("Can't match child that is not UTF-8 string: {child_name:?}");
            continue;
        };
        if child_name.to_lowercase().contains(&leg_lc) {
            debug!("Child '{child_name}' matches '{leg}'");
            populate_paths(result, child.path(), None, ancestors, leaf_filter);
        } else {
            debug!("Child '{child_name}' does not match '{leg}'");
        }
    }
}

#[nameth]
#[derive(Debug, thiserror::Error)]
pub enum AutoCompleteError {
    #[error("[{n}] {0}", n = self.name())]
    ListDir(std::io::Error),
}

impl IsHttpError for AutoCompleteError {
    fn status_code(&self) -> StatusCode {
        match self {
            AutoCompleteError::ListDir { .. } => StatusCode::NOT_FOUND,
        }
    }
}

#[cfg(test)]
mod tests {
    use std::borrow::Cow;
    use std::path::Path;

    use fluent_asserter::prelude::*;
    use nix::NixPath;
    use trz_gateway_common::tracing::test_utils::enable_tracing_for_tests;

    #[test]
    fn exact_match() {
        enable_tracing_for_tests();
        let root = std::env::var("CARGO_MANIFEST_DIR").unwrap();
        assert_that!(&root).ends_with("/terminal");

        let autocomplete = call_autocomplete(&root, root.clone());
        assert_that!(&autocomplete).contains(&"ROOT/assets".into());
        assert_that!(&autocomplete).contains(&"ROOT/src".into());
        assert_that!(&autocomplete).contains(&"ROOT/build.rs".into());
        assert_that!(&autocomplete).contains(&"ROOT/Cargo.toml".into());
        assert_that!(&autocomplete).does_not_contain_any(&[&"ROOT/xyz".into()]);
    }

    #[test]
    fn fuzzy_match() {
        enable_tracing_for_tests();
        let root = std::env::var("CARGO_MANIFEST_DIR").unwrap();
        let autocomplete = call_autocomplete(&root, format!("{root}/uild"));
        assert_that!(&autocomplete).is_not_empty();
        assert_that!(&autocomplete).contains(&"ROOT/build.rs".into());

        const AUTOCOMPLETE_RS_PATH: &str = "ROOT/src/text_editor/text_editor_ui/autocomplete.rs";
        const PATH_SELECTOR_RS_PATH: &str = "ROOT/src/text_editor/text_editor_ui/path_selector.rs";

        let autocomplete = call_autocomplete(&root, format!("{root}/src/text/ui/path"));
        assert_that!(&autocomplete).is_not_empty();
        assert_that!(&autocomplete).does_not_contain_any(&[&AUTOCOMPLETE_RS_PATH.into()]);
        assert_that!(&autocomplete).contains(&PATH_SELECTOR_RS_PATH.into());

        let autocomplete = call_autocomplete(&root, format!("{root}/src/text/ui/rs"));
        assert_that!(&autocomplete).is_not_empty();
        assert_that!(&autocomplete).contains(&AUTOCOMPLETE_RS_PATH.into());
        assert_that!(&autocomplete).contains(&PATH_SELECTOR_RS_PATH.into());
    }

    #[test]
    fn match_dirs() {
        enable_tracing_for_tests();
        let root = std::env::var("CARGO_MANIFEST_DIR").unwrap();
        let autocomplete = call_autocomplete_dir(&root, format!("{root}/src/text/t"));
        assert_that!(&autocomplete)
            .is_equal_to(&["ROOT/src/text_editor/text_editor_ui".into()].into());
    }

    #[test]
    fn match_files() {
        enable_tracing_for_tests();
        let root = std::env::var("CARGO_MANIFEST_DIR").unwrap();
        let autocomplete = call_autocomplete_files(&root, format!("{root}/src/text/ui"));
        assert_that!(&autocomplete).is_equal_to(
            &[
                "text_editor/text_editor_ui.rs".into(),
                "text_editor/text_editor_ui.scss".into(),
            ]
            .into(),
        );
    }

    fn call_autocomplete(prefix: &str, path: String) -> Vec<String> {
        let mut autocomplete = super::autocomplete_path_impl("".as_ref(), path, |_| true).unwrap();
        autocomplete
            .iter_mut()
            .for_each(|p| *p = p.replace(prefix, "ROOT"));
        return autocomplete;
    }

    fn call_autocomplete_dir(prefix: &str, path: String) -> Vec<String> {
        let mut autocomplete =
            super::autocomplete_path_impl("".as_ref(), path, |m| m.is_dir()).unwrap();
        autocomplete
            .iter_mut()
            .for_each(|p| *p = p.replace(prefix, "ROOT"));
        return autocomplete;
    }

    fn call_autocomplete_files(prefix: &str, path: String) -> Vec<String> {
        let root = std::env::var("CARGO_MANIFEST_DIR").unwrap();
        let mut autocomplete =
            super::autocomplete_path_impl(format!("{root}/src").as_ref(), path, |m| m.is_file())
                .unwrap();
        autocomplete
            .iter_mut()
            .for_each(|p| *p = p.replace(prefix, "ROOT"));
        return autocomplete;
    }

    #[test]
    fn canonicalize() {
        assert_that!(
            super::canonicalize("/a/b".as_ref())
                .iter()
                .map(|leg| leg.to_string_lossy())
                .collect::<Vec<_>>()
        )
        .is_equal_to(["/", "a", "b"].map(Cow::from).into());
        assert_that!(
            super::canonicalize("a/b".as_ref())
                .iter()
                .map(|leg| leg.to_string_lossy())
                .collect::<Vec<_>>()
        )
        .is_equal_to(["a", "b"].map(Cow::from).into());
        assert_that!(
            super::canonicalize("/a/b/.".as_ref())
                .iter()
                .map(|leg| leg.to_string_lossy())
                .collect::<Vec<_>>()
        )
        .is_equal_to(["/", "a", "b"].map(Cow::from).into());
        assert_that!(
            super::canonicalize("/a/./b/.".as_ref())
                .iter()
                .map(|leg| leg.to_string_lossy())
                .collect::<Vec<_>>()
        )
        .is_equal_to(["/", "a", "b"].map(Cow::from).into());
        assert_that!(
            super::canonicalize("/a/./b/../c/.".as_ref())
                .iter()
                .map(|leg| leg.to_string_lossy())
                .collect::<Vec<_>>()
        )
        .is_equal_to(["/", "a", "c"].map(Cow::from).into());
        assert_that!(
            super::canonicalize("/a/../../b/./c/.".as_ref())
                .iter()
                .map(|leg| leg.to_string_lossy())
                .collect::<Vec<_>>()
        )
        .is_equal_to(["/", "b", "c"].map(Cow::from).into());
    }

    #[test]
    fn is_empty_behavior() {
        let empty: &Path = "".as_ref();
        assert_that!(empty.is_empty()).is_true();
        let slash: &Path = "/".as_ref();
        assert_that!(slash.is_empty()).is_false();
    }
}
