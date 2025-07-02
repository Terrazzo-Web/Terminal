#![cfg(feature = "server")]

use std::collections::HashMap;
use std::ffi::CStr;
use std::fs::Metadata;
use std::os::unix::fs::MetadataExt as _;
use std::path::Path;
use std::sync::Arc;
use std::time::SystemTime;

use libc::getgrgid;
use libc::getpwuid;

use crate::text_editor::fsio::FileMetadata;
use crate::utils::more_path::MorePath as _;

impl FileMetadata {
    pub fn single(path: &Path, metadata: &Metadata) -> Self {
        Self::make(
            path.file_name()
                .map(|n| n.to_owned_string().into())
                .unwrap_or_else(|| "/".into()),
            Ok(metadata),
            &mut HashMap::new(),
            &mut HashMap::new(),
        )
    }

    pub fn of(
        file: std::fs::DirEntry,
        gids: &mut HashMap<u32, Option<Arc<str>>>,
        uids: &mut HashMap<u32, Option<Arc<str>>>,
    ) -> Self {
        Self::make(
            file.file_name().to_string_lossy().to_string().into(),
            file.metadata().as_ref(),
            gids,
            uids,
        )
    }

    fn make(
        name: Arc<str>,
        metadata: Result<&Metadata, &std::io::Error>,
        gids: &mut HashMap<u32, Option<Arc<str>>>,
        uids: &mut HashMap<u32, Option<Arc<str>>>,
    ) -> Self {
        let metadata = metadata.ok();
        let size = metadata.map(|m| m.len());

        FileMetadata {
            name,
            size,
            is_dir: metadata.map(|m| m.is_dir()).unwrap_or_default(),
            created: get_date_since_epoch(|m| m.created(), metadata),
            accessed: get_date_since_epoch(|m| m.accessed(), metadata),
            modified: get_date_since_epoch(|m| m.modified(), metadata),
            mode: metadata.map(|m| m.mode()),
            user: metadata.and_then(|m| {
                uids.entry(m.uid())
                    .or_insert_with(|| uid_to_username(m.uid()))
                    .clone()
            }),
            group: metadata.and_then(|m| {
                gids.entry(m.gid())
                    .or_insert_with(|| gid_to_groupname(m.gid()))
                    .clone()
            }),
        }
    }
}

fn get_date_since_epoch(
    g: impl FnOnce(&Metadata) -> std::io::Result<SystemTime>,
    metadata: Option<&Metadata>,
) -> Option<std::time::Duration> {
    metadata
        .and_then(|m| g(m).ok())
        .and_then(|m| m.duration_since(SystemTime::UNIX_EPOCH).ok())
}

/// Convert UID to username
fn uid_to_username(uid: u32) -> Option<Arc<str>> {
    unsafe {
        let pw = getpwuid(uid);
        if pw.is_null() {
            return None;
        }
        let name = CStr::from_ptr((*pw).pw_name);
        name.to_str().ok().map(|s| s.to_owned().into())
    }
}

/// Convert GID to group name
fn gid_to_groupname(gid: u32) -> Option<Arc<str>> {
    unsafe {
        let gr = getgrgid(gid);
        if gr.is_null() {
            return None;
        }
        let name = CStr::from_ptr((*gr).gr_name);
        name.to_str().ok().map(|s| s.to_owned().into())
    }
}
