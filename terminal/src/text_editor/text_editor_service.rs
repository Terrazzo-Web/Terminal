#![cfg(feature = "server")]
#![allow(unused)]

use std::ops::Deref;
use std::path::Path;

use nameth::NamedEnumValues as _;
use nameth::nameth;
use terrazzo::http::StatusCode;
use tracing::debug;
use trz_gateway_common::http_error::HttpError;
use trz_gateway_common::http_error::IsHttpError;

const ROOT: &str = "/";

pub fn base_path_autocomplete(path: String) -> Result<Vec<String>, HttpError<AutoCompleteError>> {
    Ok(base_path_autocomplete_impl(path.as_ref())?)
}

pub fn base_path_autocomplete_impl(path: &Path) -> Result<Vec<String>, AutoCompleteError> {
    match path.metadata() {
        Ok(metadata) => {
            if metadata.is_dir() {
                return list_dir(&path);
            } else {
                return path
                    .parent()
                    .map(list_dir)
                    .unwrap_or_else(|| list_dir(ROOT.as_ref()));
            }
        }
        Err(error) => {
            debug!("Path does not exist: path='{path:?}' error={error}");
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
            let ancestor_name = ancestor
                .file_name()
                .ok_or(AutoCompleteError::InvalidInput)?
                .to_string_lossy();
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
    ancestors: &[impl Deref<Target = str>],
) {
    let &[first, rest @ ..] = &ancestors else {
        result.push(path.to_string_lossy().into_owned());
        return;
    };

    let first = &**first;

    // If "/{path}/{first}" exists, return it.
    {
        let mut child_path = path.to_path_buf();
        child_path.push(first);
        if child_path.exists() {
            populate_paths(result, &child_path, rest);
            return;
        }
    }

    let Ok(dir) = path.read_dir() else {
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
            let mut child_path = path.to_path_buf();
            child_path.push(first);
            populate_paths(result, &child_path, ancestors);
        }
    }
}

#[nameth]
#[derive(Debug, thiserror::Error)]
pub enum AutoCompleteError {
    #[error("[{n}] The input does not parse as path or a prefix", n = self.name())]
    InvalidInput,

    #[error("[{n}] {0}", n = self.name())]
    ListDir(std::io::Error),
}

impl IsHttpError for AutoCompleteError {
    fn status_code(&self) -> StatusCode {
        match self {
            AutoCompleteError::InvalidInput => StatusCode::PRECONDITION_FAILED,
            AutoCompleteError::ListDir { .. } => StatusCode::NOT_FOUND,
        }
    }
}
