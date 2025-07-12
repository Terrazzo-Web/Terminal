//! Conversion utils for the Notify service.

use crate::backend::protos::terrazzo::notify::FilePath as FilePathProto;
use crate::text_editor::file_path::FilePath;

impl<B, F> From<FilePathProto> for FilePath<B, F>
where
    String: Into<B>,
    String: Into<F>,
{
    fn from(proto: FilePathProto) -> Self {
        Self {
            base: proto.base.into(),
            file: proto.file.into(),
        }
    }
}

impl<B, F> From<FilePath<B, F>> for FilePathProto
where
    B: ToString,
    F: ToString,
{
    fn from(proto: FilePath<B, F>) -> Self {
        Self {
            base: proto.base.to_string(),
            file: proto.file.to_string(),
        }
    }
}
