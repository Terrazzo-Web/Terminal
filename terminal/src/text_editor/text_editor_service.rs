#![cfg(feature = "server")]

use std::fmt::Debug;
use std::ops::Deref;
use std::path::Path;

use nameth::NamedEnumValues as _;
use nameth::nameth;
use terrazzo::http::StatusCode;
use tracing::debug;
use tracing::debug_span;
use trz_gateway_common::http_error::HttpError;
use trz_gateway_common::http_error::IsHttpError;

const ROOT: &str = "/";

pub fn base_path_autocomplete(path: String) -> Result<Vec<String>, HttpError<AutoCompleteError>> {
    Ok(base_path_autocomplete_impl(path.as_ref())?)
}

pub fn base_path_autocomplete_impl(path: &Path) -> Result<Vec<String>, AutoCompleteError> {
    let _span = debug_span!("Autocomplete", path = %path.to_string_lossy()).entered();
    match path.metadata() {
        Ok(metadata) => {
            if metadata.is_dir() {
                debug!("List directory");
                return list_dir(&path);
            } else {
                debug!("List parent directory");
                return path
                    .parent()
                    .map(list_dir)
                    .unwrap_or_else(|| list_dir(ROOT.as_ref()));
            }
        }
        Err(error) => {
            debug!("Path does not exist, finding best match. Error={error}");
            return resolve_path(path);
        }
    }
}

fn list_dir(path: &Path) -> Result<Vec<String>, AutoCompleteError> {
    let mut result = vec![];
    for child in path.read_dir().map_err(AutoCompleteError::ListDir)? {
        match child {
            Ok(child) => result.push(child.path().to_string_lossy().into_owned()),
            Err(error) => debug!("Error when reading {path:?}: {error}"),
        }
    }
    return Ok(result);
}

fn resolve_path(path: &Path) -> Result<Vec<String>, AutoCompleteError> {
    let ancestors = {
        let mut ancestors = vec![];
        for ancestor in path.ancestors() {
            let ancestor_name = ancestor.file_name().unwrap_or_default().to_string_lossy();
            ancestors.push(ancestor_name);
        }
        if ancestors.is_empty() {
            ancestors.push(ROOT.into());
        } else {
            ancestors.reverse();
        }
        ancestors
    };
    let mut result = vec![];
    populate_paths(&mut result, ROOT.as_ref(), &ancestors);
    Ok(result)
}

fn populate_paths<'t>(
    result: &mut Vec<String>,
    path: &Path,
    ancestors: &[impl Deref<Target = str> + Debug],
) {
    let &[first, rest @ ..] = &ancestors else {
        debug!("Found {path:?}");
        result.push(path.to_string_lossy().into_owned());
        return;
    };

    let first = &**first;
    debug!(?path, first, "Populate path. ancestors={ancestors:?}");

    // If "/{path}/{first}" exists, return it.
    {
        let mut child_path = path.to_path_buf();
        child_path.push(first);
        if child_path.exists() {
            debug!("Exact match {child_path:?}");
            populate_paths(result, &child_path, rest);
            return;
        }
    }

    let Ok(dir) = path.read_dir() else {
        debug!("Not a folder {path:?}");
        return;
    };

    // Populate "/{path}/{child}" for every matching child.
    for child in dir.filter_map(|child| child.ok()) {
        let child = child.file_name();
        let child = child.to_string_lossy();
        if child.starts_with(".") {
            match child.as_ref() {
                ".." | "." => continue,
                _ => (),
            }
            if !first.starts_with(".") {
                continue;
            }
        }
        if child.contains(first) {
            debug!("Child '{child}' matches '{first}'");
            let mut child_path = path.to_path_buf();
            child_path.push(child.as_ref());
            populate_paths(result, &child_path, rest);
        } else {
            debug!("Child '{child}' does not match '{first}'");
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
    use std::path::Path;

    use fluent_asserter::prelude::*;
    use trz_gateway_common::tracing::test_utils::enable_tracing_for_tests;

    #[test]
    fn exact_match() {
        enable_tracing_for_tests();
        let root = std::env::var("CARGO_MANIFEST_DIR").unwrap();
        assert_that!(&root).ends_with("/terminal");

        let autocomplete = call_autocomplete(&root, &root);
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

    fn call_autocomplete(prefix: &str, path: impl AsRef<Path>) -> Vec<String> {
        let path = path.as_ref();
        let mut autocomplete = super::base_path_autocomplete_impl(path).unwrap();
        autocomplete
            .iter_mut()
            .for_each(|p| *p = p.replace(prefix, "ROOT"));
        return autocomplete;
    }
}
