use std::ops::Deref;
use std::path::Path;
use std::path::PathBuf;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub(super) struct FilePath<BASE, FILE = BASE> {
    #[cfg_attr(not(feature = "diagnostics"), serde(rename = "b"))]
    pub base: BASE,
    #[cfg_attr(not(feature = "diagnostics"), serde(rename = "f"))]
    pub file: FILE,
}

impl<B: AsRef<Path>, F: AsRef<Path>> FilePath<B, F> {
    #[allow(unused)]
    pub fn full_path(&self) -> PathBuf {
        self.base.as_ref().join(self.file.as_ref())
    }
}

impl<B: Deref, F: Deref> FilePath<B, F> {
    #[allow(unused)]
    pub fn as_ref(&self) -> FilePath<&B::Target, &F::Target> where {
        FilePath {
            base: &self.base,
            file: &self.file,
        }
    }
}

impl<T> FilePath<T> {
    #[allow(unused)]
    pub fn map<U>(self, f: impl Fn(T) -> U) -> FilePath<U> {
        self.map2(&f, &f)
    }
}

impl<B, F> FilePath<B, F> {
    #[allow(unused)]
    pub fn map2<BB, FF>(
        self,
        b: impl FnOnce(B) -> BB,
        f: impl FnOnce(F) -> FF,
    ) -> FilePath<BB, FF> {
        FilePath {
            base: b(self.base),
            file: f(self.file),
        }
    }
}
