#![allow(unused)]

use std::collections::BTreeMap;
use std::sync::Arc;

use nameth::NamedEnumValues as _;
use nameth::nameth;
use tracing::debug;
use tracing::warn;

use crate::text_editor::fsio::FileMetadata;

mod ui;

#[derive(Clone)]
pub enum SideView {
    Folder {
        name: Arc<str>,
        children: Arc<Children>,
    },
    File(Arc<FileMetadata>),
}

type Children = BTreeMap<Arc<str>, Arc<SideView>>;

impl std::fmt::Debug for SideView {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Folder { name, children } => f
                .debug_struct("Folder")
                .field("name", name)
                .field("children", children)
                .finish(),
            Self::File(file) => f.debug_tuple("File").field(&file.name).finish(),
        }
    }
}

fn add_file(
    into_children: Arc<Children>,
    relative_path: &[&Arc<str>],
    file: Arc<FileMetadata>,
) -> Arc<Children> {
    match relative_path {
        [] => add_file(into_children, &[&"/".into()], file),
        [child_name] => {
            #[cfg(debug_assertions)]
            match into_children.get(*child_name) {
                Some(child) => match &**child {
                    SideView::Folder { .. } => warn!("Replace folder {child_name}"),
                    SideView::File { .. } => debug!("Replace file {child_name}"),
                },
                None => debug!("Add new file {child_name}"),
            }
            let mut into_children = (*into_children).clone();
            into_children.insert((*child_name).clone(), Arc::new(SideView::File(file)));
            Arc::new(into_children)
        }
        [folder_name, rest @ ..] => {
            let children = match into_children.get(*folder_name) {
                Some(child) => match &**child {
                    SideView::Folder { name: _, children } => {
                        debug!("Adding to folder {folder_name}");
                        children.clone()
                    }
                    SideView::File { .. } => {
                        warn!("Replace file {folder_name}");
                        Arc::default()
                    }
                },
                None => {
                    debug!("Add new folder {folder_name}");
                    Arc::default()
                }
            };
            let mut into_children = (*into_children).clone();
            let rec = add_file(children, rest, file);
            into_children.insert(
                (*folder_name).clone(),
                Arc::new(SideView::Folder {
                    name: (**folder_name).clone(),
                    children: rec,
                }),
            );
            Arc::new(into_children)
        }
    }
}

fn remove_file(
    into_children: Arc<Children>,
    relative_path: &[&Arc<str>],
) -> Result<Arc<Children>, RemoveFileError> {
    match relative_path {
        [] => remove_file(into_children, &[&"/".into()]),
        [child_name] => {
            #[cfg(debug_assertions)]
            match into_children.get(*child_name) {
                Some(child) => match &**child {
                    SideView::Folder { .. } => debug!("Remove folder {child_name}"),
                    SideView::File { .. } => debug!("Remove file {child_name}"),
                },
                None => {
                    debug!("The file wasn't here {child_name}");
                    return Err(RemoveFileError::FileNotFound);
                }
            }
            let mut into_children = (*into_children).clone();
            into_children.remove(*child_name);
            Ok(Arc::new(into_children))
        }
        [folder_name, rest @ ..] => {
            let folder_name = *folder_name;
            let children = match into_children.get(folder_name) {
                Some(child) => match &**child {
                    SideView::Folder { name: _, children } => {
                        debug!("Removing from folder {folder_name}");
                        children.clone()
                    }
                    SideView::File(expected_folder) => {
                        return Err(RemoveFileError::ExpectedFolder(
                            expected_folder.name.clone(),
                        ));
                    }
                },
                None => {
                    return Err(RemoveFileError::ParentNotFound(folder_name.clone()));
                }
            };
            let mut into_children = (*into_children).clone();
            let new_children = remove_file(children, rest)?;
            into_children.insert(
                folder_name.clone(),
                Arc::new(SideView::Folder {
                    name: folder_name.clone(),
                    children: new_children,
                }),
            );
            Ok(Arc::new(into_children))
        }
    }
}

#[nameth]
#[derive(thiserror::Error, Debug, serde::Serialize, serde::Deserialize)]
pub enum RemoveFileError {
    #[error("[{n}] File can't be a child of file {0}", n = self.name())]
    ExpectedFolder(Arc<str>),

    #[error("[{n}] Parent folder does not exist: {0}", n = self.name())]
    ParentNotFound(Arc<str>),

    #[error("[{n}] The file was not found", n = self.name())]
    FileNotFound,
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use openssl::x509::store::File;

    use super::BTreeMap;
    use super::Children;
    use super::FileMetadata;
    use super::SideView;
    use super::debug;
    use super::warn;

    #[test]
    fn add_file() {
        let tree = Arc::<Children>::default();
        let make_file = |name: &str| {
            Arc::new(FileMetadata {
                name: Arc::from(name),
                size: Some(12),
                is_dir: false,
                created: None,
                accessed: None,
                modified: None,
                mode: None,
                user: None,
                group: None,
            })
        };
        let tree = super::add_file(
            tree,
            &[&Arc::from("a1"), &Arc::from("b1"), &Arc::from("c1.txt")],
            make_file("c1.txt"),
        );
        assert_eq!(
            r#"
{
    "a1": Folder {
        name: "a1",
        children: {
            "b1": Folder {
                name: "b1",
                children: {
                    "c1.txt": File(
                        "c1.txt",
                    ),
                },
            },
        },
    },
}"#
            .trim(),
            format!("{tree:#?}")
        );

        let tree = super::add_file(
            tree,
            &[&Arc::from("a1"), &Arc::from("b1"), &Arc::from("c2.txt")],
            make_file("c2.txt"),
        );
        assert_eq!(
            r#"
{
    "a1": Folder {
        name: "a1",
        children: {
            "b1": Folder {
                name: "b1",
                children: {
                    "c1.txt": File(
                        "c1.txt",
                    ),
                    "c2.txt": File(
                        "c2.txt",
                    ),
                },
            },
        },
    },
}"#
            .trim(),
            format!("{tree:#?}")
        );

        let tree = super::add_file(
            tree,
            &[&Arc::from("a1"), &Arc::from("b2"), &Arc::from("c3.txt")],
            make_file("c2.txt"),
        );
        assert_eq!(
            r#"
{
    "a1": Folder {
        name: "a1",
        children: {
            "b1": Folder {
                name: "b1",
                children: {
                    "c1.txt": File(
                        "c1.txt",
                    ),
                    "c2.txt": File(
                        "c2.txt",
                    ),
                },
            },
            "b2": Folder {
                name: "b2",
                children: {
                    "c3.txt": File(
                        "c2.txt",
                    ),
                },
            },
        },
    },
}"#
            .trim(),
            format!("{tree:#?}")
        );

        // Folder --> File
        let tree = super::add_file(
            tree,
            &[&Arc::from("a1"), &Arc::from("b1")],
            make_file("b1.txt"),
        );
        assert_eq!(
            r#"
{
    "a1": Folder {
        name: "a1",
        children: {
            "b1": File(
                "b1.txt",
            ),
            "b2": Folder {
                name: "b2",
                children: {
                    "c3.txt": File(
                        "c2.txt",
                    ),
                },
            },
        },
    },
}"#
            .trim(),
            format!("{tree:#?}")
        );

        // File --> Folder
        let tree = super::add_file(
            tree,
            &[&Arc::from("a1"), &Arc::from("b1"), &Arc::from("c1.txt")],
            make_file("c1.txt"),
        );
        assert_eq!(
            r#"
{
    "a1": Folder {
        name: "a1",
        children: {
            "b1": Folder {
                name: "b1",
                children: {
                    "c1.txt": File(
                        "c1.txt",
                    ),
                },
            },
            "b2": Folder {
                name: "b2",
                children: {
                    "c3.txt": File(
                        "c2.txt",
                    ),
                },
            },
        },
    },
}"#
            .trim(),
            format!("{tree:#?}")
        );
    }

    #[test]
    fn remove_file() {
        let tree = Arc::<Children>::default();
        let make_file = |name: &str| {
            Arc::new(FileMetadata {
                name: Arc::from(name),
                size: Some(12),
                is_dir: false,
                created: None,
                accessed: None,
                modified: None,
                mode: None,
                user: None,
                group: None,
            })
        };
        let tree = super::add_file(
            tree,
            &[&Arc::from("a1"), &Arc::from("b1"), &Arc::from("c1.txt")],
            make_file("c1.txt"),
        );
        assert_eq!(
            r#"
{
    "a1": Folder {
        name: "a1",
        children: {
            "b1": Folder {
                name: "b1",
                children: {
                    "c1.txt": File(
                        "c1.txt",
                    ),
                },
            },
        },
    },
}"#
            .trim(),
            format!("{tree:#?}")
        );

        let tree = super::add_file(
            tree,
            &[&Arc::from("a1"), &Arc::from("b1"), &Arc::from("c2.txt")],
            make_file("c2.txt"),
        );
        assert_eq!(
            r#"
{
    "a1": Folder {
        name: "a1",
        children: {
            "b1": Folder {
                name: "b1",
                children: {
                    "c1.txt": File(
                        "c1.txt",
                    ),
                    "c2.txt": File(
                        "c2.txt",
                    ),
                },
            },
        },
    },
}"#
            .trim(),
            format!("{tree:#?}")
        );

        // Remove file: ExpectedFolder
        let error = super::remove_file(
            tree.clone(),
            &[
                &Arc::from("a1"),
                &Arc::from("b1"),
                &Arc::from("c2.txt"),
                &Arc::from("not_found.txt"),
            ],
        )
        .unwrap_err();
        assert_eq!(
            "[ExpectedFolder] File can't be a child of file c2.txt",
            format!("{error}")
        );

        // Remove file: ParentNotFound
        let error = super::remove_file(
            tree.clone(),
            &[
                &Arc::from("a1"),
                &Arc::from("b1"),
                &Arc::from("c3.txt"),
                &Arc::from("not_found.txt"),
            ],
        )
        .unwrap_err();
        assert_eq!(
            "[ParentNotFound] Parent folder does not exist: c3.txt",
            format!("{error}")
        );

        // Remove file: FileNotFound
        let error = super::remove_file(
            tree.clone(),
            &[&Arc::from("a1"), &Arc::from("b1"), &Arc::from("c3.txt")],
        )
        .unwrap_err();
        assert_eq!("[FileNotFound] The file was not found", format!("{error}"));

        // Remove file
        let tree = super::remove_file(
            tree,
            &[&Arc::from("a1"), &Arc::from("b1"), &Arc::from("c2.txt")],
        )
        .unwrap();
        assert_eq!(
            r#"
{
    "a1": Folder {
        name: "a1",
        children: {
            "b1": Folder {
                name: "b1",
                children: {
                    "c1.txt": File(
                        "c1.txt",
                    ),
                },
            },
        },
    },
}"#
            .trim(),
            format!("{tree:#?}")
        );

        // Remove folder
        let tree = super::remove_file(tree, &[&Arc::from("a1"), &Arc::from("b1")]).unwrap();
        assert_eq!(
            r#"
{
    "a1": Folder {
        name: "a1",
        children: {},
    },
}"#
            .trim(),
            format!("{tree:#?}")
        );
    }
}
