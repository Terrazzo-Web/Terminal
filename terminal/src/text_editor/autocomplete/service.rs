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
use tonic::Code;
use tracing::debug;
use tracing::debug_span;

use crate::backend::client_service::grpc_error::GrpcError;
use crate::backend::client_service::grpc_error::IsGrpcError;
use crate::text_editor::autocomplete::AutocompleteItem;
use crate::text_editor::fsio::canonical::canonicalize;
use crate::text_editor::fsio::canonical::concat_base_file_path;
use crate::text_editor::path_selector::PathSelector;

const ROOT: &str = "/";
const MAX_RESULTS: usize = 30;

pub fn autocomplete_path(
    kind: PathSelector,
    prefix: &str,
    input: &str,
) -> Result<Vec<AutocompleteItem>, GrpcError<AutoCompleteError>> {
    let prefix = prefix.trim();
    let input = input.trim();
    let options = Options {
        show_hidden_files: input.ends_with('.'),
        ends_with_slash: input.trim_end_matches('.').ends_with('/'),
    };
    let path = if kind == PathSelector::BasePath && input.is_empty() {
        std::env::home_dir().unwrap_or_else(|| Path::new(ROOT).to_owned())
    } else if Path::new(prefix).is_absolute() {
        concat_base_file_path(prefix, input)
    } else {
        concat_base_file_path(format!("{ROOT}{prefix}"), input)
    };
    return Ok(autocomplete_path_impl(
        prefix.as_ref(),
        &path,
        options,
        |m| kind.accept(m),
    )?);
}

#[derive(Debug, Default)]
struct Options {
    show_hidden_files: bool,
    ends_with_slash: bool,
}

fn autocomplete_path_impl(
    prefix: &Path,
    path: &Path,
    options: Options,
    leaf_filter: impl Fn(&Metadata) -> bool,
) -> Result<Vec<AutocompleteItem>, AutoCompleteError> {
    let _span = debug_span!("Autocomplete", ?path).entered();
    let path = canonicalize(path);
    if let Ok(metadata) = path
        .metadata()
        .inspect_err(|error| debug!("Path does not exist, finding best match. Error={error}"))
    {
        if metadata.is_dir() {
            debug!("List directory");
            return list_folders(prefix, path.parent(), &path, leaf_filter);
        } else {
            debug!("List parent directory");
            let parent = path.parent().unwrap_or(ROOT.as_ref());
            return list_folders(prefix, Some(parent), parent, leaf_filter);
        }
    }
    return resolve_path(prefix, &path, options, leaf_filter);
}

fn list_folders(
    prefix: &Path,
    parent: Option<&Path>,
    path: &Path,
    leaf_filter: impl Fn(&Metadata) -> bool,
) -> Result<Vec<AutocompleteItem>, AutoCompleteError> {
    let mut result = vec![];
    if let Some(parent) = parent {
        result.push(PathInfo {
            path: parent.to_owned(),
            metadata: parent.metadata().ok(),
        })
    }
    if parent != Some(path) {
        result.push(PathInfo {
            path: path.to_owned(),
            metadata: path.metadata().ok(),
        });
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
        let Ok(metadata) = child_path
            .metadata()
            .map_err(|error| debug!("Error when getting metadata for {child_path:?}: {error}"))
        else {
            continue;
        };
        if !leaf_filter(&metadata) {
            continue;
        }
        result.push(PathInfo {
            path: child_path,
            metadata: Some(metadata),
        })
    }
    return Ok(sort_result(prefix, result));
}

fn resolve_path(
    prefix: &Path,
    path: &Path,
    options: Options,
    leaf_filter: impl Fn(&Metadata) -> bool,
) -> Result<Vec<AutocompleteItem>, AutoCompleteError> {
    let mut result = vec![];
    if let Some(parent) = path.parent() {
        result.push(PathInfo {
            path: parent.to_owned(),
            metadata: parent.metadata().ok(),
        });
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
        if options.show_hidden_files {
            ancestors.push(".".as_ref());
        } else if options.ends_with_slash {
            ancestors.push("".as_ref());
        }
        ancestors
    };
    populate_paths(
        &mut result,
        PathBuf::from(ROOT),
        None,
        &ancestors,
        &leaf_filter,
        &options,
    );
    Ok(sort_result(prefix, result))
}

fn populate_paths(
    result: &mut Vec<PathInfo>,
    accu: PathBuf,
    metadata: Option<Metadata>,
    ancestors: &[&OsStr],
    leaf_filter: &impl Fn(&Metadata) -> bool,
    options: &Options,
) {
    let [leg, ancestors @ ..] = &ancestors else {
        let metadata = metadata.or_else(|| accu.metadata().ok());
        if metadata.as_ref().map(leaf_filter).unwrap_or(false) {
            debug!("Found matching leaf {accu:?}");
            result.push(PathInfo {
                path: accu,
                metadata,
            });
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
            populate_paths(
                result,
                child_accu,
                Some(metadata),
                ancestors,
                leaf_filter,
                options,
            );
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
            if !options.show_hidden_files && !leg.starts_with('.') {
                continue;
            }
        }

        let Some(child_name) = child_name.to_str() else {
            debug!("Can't match child that is not UTF-8 string: {child_name:?}");
            continue;
        };
        if child_name.to_lowercase().contains(&leg_lc) {
            debug!("Child '{child_name}' matches '{leg}'");
            populate_paths(result, child.path(), None, ancestors, leaf_filter, options);
        } else {
            debug!("Child '{child_name}' does not match '{leg}'");
        }
    }
}

fn sort_result(prefix: &Path, mut result: Vec<PathInfo>) -> Vec<AutocompleteItem> {
    if result.len() > MAX_RESULTS {
        result.sort_by_key(|path| {
            let age: Option<Duration> = path
                .metadata
                .as_ref()
                .and_then(|metadata| metadata.modified().ok())
                .and_then(|modified| modified.duration_since(SystemTime::UNIX_EPOCH).ok());
            // Sort youngest first / oldest last or error.
            Reverse(age.unwrap_or(Duration::ZERO))
        });
        result = result.into_iter().take(MAX_RESULTS).collect();
    }

    let mut result: Vec<AutocompleteItem> = result
        .into_iter()
        .filter_map(|path_info| {
            let path = path_info.path.strip_prefix(prefix).ok()?;
            let path = path.to_string_lossy().into_owned();
            let is_dir = path_info.metadata.map(|m| m.is_dir()).unwrap_or(false);
            Some(AutocompleteItem { path, is_dir })
        })
        .collect();
    result.sort_by(|a, b| Ord::cmp(&a.path, &b.path));
    return result;
}

struct PathInfo {
    path: PathBuf,
    metadata: Option<Metadata>,
}

#[nameth]
#[derive(Debug, thiserror::Error)]
pub enum AutoCompleteError {
    #[error("[{n}] {0}", n = self.name())]
    ListDir(std::io::Error),
}

impl IsGrpcError for AutoCompleteError {
    fn code(&self) -> Code {
        match self {
            Self::ListDir { .. } => Code::NotFound,
        }
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use fluent_asserter::prelude::*;
    use trz_gateway_common::tracing::test_utils::enable_tracing_for_tests;

    use super::Options;

    #[test]
    fn exact_match() {
        enable_tracing_for_tests();
        let root = std::env::var("CARGO_MANIFEST_DIR").unwrap();
        assert_that!(&root).ends_with("/terminal");

        let autocomplete = call_autocomplete(&root, format!("{root}/src/text_editor"));
        assert_that!(&autocomplete).contains(&"ROOT/src".into());
        assert_that!(&autocomplete).contains(&"ROOT/src/text_editor/autocomplete".into());
        assert_that!(&autocomplete).contains(&"ROOT/src/text_editor/autocomplete.rs".into());
        assert_that!(&autocomplete).contains(&"ROOT/src/text_editor/path_selector".into());
        assert_that!(&autocomplete).contains(&"ROOT/src/text_editor/path_selector.rs".into());
        assert_that!(&autocomplete).does_not_contain_any(&[&"ROOT/xyz".into()]);
    }

    #[test]
    fn fuzzy_match() {
        enable_tracing_for_tests();
        let root = std::env::var("CARGO_MANIFEST_DIR").unwrap();
        let autocomplete = call_autocomplete(&root, format!("{root}/uild"));
        assert_that!(&autocomplete).is_not_empty();
        assert_that!(&autocomplete).contains(&"ROOT/build.rs".into());

        const SERVICE_PATH: &str = "ROOT/src/text_editor/path_selector/service.rs";
        const UI_PATH: &str = "ROOT/src/text_editor/path_selector/ui.rs";
        const PARENT_PATH: &str = "ROOT/src/text/path";

        let autocomplete = call_autocomplete(&root, format!("{root}/src/text/path/ui"));
        assert_that!(&autocomplete).is_not_empty();
        assert_that!(&autocomplete).does_not_contain_any(&[&SERVICE_PATH.into()]);
        assert_that!(&autocomplete).contains(&UI_PATH.into());
        assert_that!(&autocomplete).contains(&PARENT_PATH.into());

        let autocomplete = call_autocomplete(&root, format!("{root}/src/text/path/rs"));
        assert_that!(&autocomplete).is_not_empty();
        assert_that!(&autocomplete).contains(&SERVICE_PATH.into());
        assert_that!(&autocomplete).contains(&UI_PATH.into());
        assert_that!(&autocomplete).contains(&PARENT_PATH.into());
    }

    #[test]
    fn match_dirs() {
        enable_tracing_for_tests();
        let root = std::env::var("CARGO_MANIFEST_DIR").unwrap();
        let autocomplete = call_autocomplete_dir(&root, format!("{root}/src/text/e"));
        assert_that!(&autocomplete).is_equal_to(
            &[
                "ROOT/src/text",
                "ROOT/src/text_editor/autocomplete",
                "ROOT/src/text_editor/path_selector",
                "ROOT/src/text_editor/side",
            ]
            .map(Into::into)
            .into(),
        );
    }

    #[test]
    fn match_files() {
        enable_tracing_for_tests();
        let root = std::env::var("CARGO_MANIFEST_DIR").unwrap();
        let autocomplete = call_autocomplete_files(&root, format!("{root}/src/text/u"));
        assert_that!(&autocomplete).is_equal_to(
            &["text", "text_editor/autocomplete.rs", "text_editor/ui.rs"]
                .map(Into::into)
                .into(),
        );
    }

    fn call_autocomplete(prefix: &str, path: String) -> Vec<String> {
        super::autocomplete_path_impl("".as_ref(), Path::new(&path), Options::default(), |_| true)
            .unwrap()
            .into_iter()
            .map(|p| p.path.replace(prefix, "ROOT"))
            .collect()
    }

    fn call_autocomplete_dir(prefix: &str, path: String) -> Vec<String> {
        super::autocomplete_path_impl("".as_ref(), Path::new(&path), Options::default(), |m| {
            m.is_dir()
        })
        .unwrap()
        .into_iter()
        .map(|p| p.path.replace(prefix, "ROOT"))
        .collect()
    }

    fn call_autocomplete_files(prefix: &str, path: String) -> Vec<String> {
        let root = std::env::var("CARGO_MANIFEST_DIR").unwrap();
        super::autocomplete_path_impl(
            format!("{root}/src").as_ref(),
            Path::new(&path),
            Options::default(),
            |m| m.is_file(),
        )
        .unwrap()
        .iter_mut()
        .map(|p| p.path.replace(prefix, "ROOT"))
        .collect()
    }
}
