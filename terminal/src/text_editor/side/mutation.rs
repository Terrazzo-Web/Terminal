#![cfg(feature = "client")]

use std::sync::Arc;

use nameth::NamedEnumValues as _;
use nameth::nameth;
use terrazzo::prelude::diagnostics;

use self::diagnostics::debug;
use self::diagnostics::warn;
use super::SideViewList;
use super::SideViewNode;

pub fn add_file(
    tree: Arc<SideViewList>,
    relative_path: &[Arc<str>],
    node: SideViewNode,
) -> Arc<SideViewList> {
    match relative_path {
        [] => add_file(tree, &["/".into()], node),
        [child_name] => {
            #[cfg(debug_assertions)]
            #[cfg(debug_assertions)]
            match tree.get(child_name) {
                Some(child) => match &**child {
                    SideViewNode::Folder { .. } => warn!("Replace folder {child_name}"),
                    SideViewNode::File { .. } => debug!("Replace file {child_name}"),
                },
                None => debug!("Add new file {child_name}"),
            }
            let mut new_tree = (*tree).clone();
            new_tree.insert((*child_name).clone(), Arc::new(node));
            Arc::new(new_tree)
        }
        [folder_name, rest @ ..] => {
            let children = match tree.get(folder_name) {
                Some(child) => match &**child {
                    SideViewNode::Folder { name: _, children } => {
                        debug!("Adding to folder {folder_name}");
                        children.clone()
                    }
                    SideViewNode::File { .. } => {
                        warn!("Replace file {folder_name}");
                        Arc::default()
                    }
                },
                None => {
                    debug!("Add new folder {folder_name}");
                    Arc::default()
                }
            };
            let mut new_tree = (*tree).clone();
            let rec = add_file(children, rest, node);
            new_tree.insert(
                (*folder_name).clone(),
                Arc::new(SideViewNode::Folder {
                    name: folder_name.clone(),
                    children: rec,
                }),
            );
            Arc::new(new_tree)
        }
    }
}

pub fn remove_file(
    tree: Arc<SideViewList>,
    relative_path: &[Arc<str>],
) -> Result<Arc<SideViewList>, RemoveFileError> {
    match relative_path {
        [] => remove_file(tree, &["/".into()]),
        [child_name] => {
            #[cfg(debug_assertions)]
            match tree.get(child_name) {
                Some(child) => match &**child {
                    SideViewNode::Folder { .. } => debug!("Remove folder {child_name}"),
                    SideViewNode::File { .. } => debug!("Remove file {child_name}"),
                },
                None => {
                    debug!("The file wasn't here {child_name}");
                    return Err(RemoveFileError::FileNotFound);
                }
            }
            let mut new_tree = (*tree).clone();
            new_tree.remove(child_name);
            Ok(Arc::new(new_tree))
        }
        [folder_name, rest @ ..] => {
            let children = match tree.get(folder_name) {
                Some(child) => match &**child {
                    SideViewNode::Folder { name: _, children } => {
                        debug!("Removing from folder {folder_name}");
                        children.clone()
                    }
                    SideViewNode::File(expected_folder) => {
                        return Err(RemoveFileError::ExpectedFolder(
                            expected_folder.name.clone(),
                        ));
                    }
                },
                None => {
                    return Err(RemoveFileError::ParentNotFound(folder_name.clone()));
                }
            };
            let mut new_tree = (*tree).clone();
            let new_children = remove_file(children, rest)?;
            new_tree.insert(
                folder_name.clone(),
                Arc::new(SideViewNode::Folder {
                    name: folder_name.clone(),
                    children: new_children,
                }),
            );
            Ok(Arc::new(new_tree))
        }
    }
}

#[nameth]
#[derive(thiserror::Error, Debug, serde::Serialize, serde::Deserialize)]
pub enum RemoveFileError {
    #[error("[{n}] File can't be a child of file {0}", n = self.name())]
    #[cfg_attr(not(feature = "diagnostics"), serde(rename = "E"))]
    ExpectedFolder(Arc<str>),

    #[error("[{n}] Parent folder does not exist: {0}", n = self.name())]
    #[cfg_attr(not(feature = "diagnostics"), serde(rename = "P"))]
    ParentNotFound(Arc<str>),

    #[error("[{n}] The file was not found", n = self.name())]
    #[cfg_attr(not(feature = "diagnostics"), serde(rename = "F"))]
    FileNotFound,
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use super::SideViewList;
    use super::SideViewNode;
    use crate::text_editor::fsio::FileMetadata;

    #[test]
    fn add_file() {
        let tree = Arc::<SideViewList>::default();
        let make_file = |name: &str| {
            SideViewNode::File(Arc::new(FileMetadata {
                name: Arc::from(name),
                size: Some(12),
                is_dir: false,
                created: None,
                accessed: None,
                modified: None,
                mode: None,
                user: None,
                group: None,
            }))
        };
        let tree = super::add_file(
            tree,
            &[Arc::from("a1"), Arc::from("b1"), Arc::from("c1.txt")],
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
            &[Arc::from("a1"), Arc::from("b1"), Arc::from("c2.txt")],
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
            &[Arc::from("a1"), Arc::from("b2"), Arc::from("c3.txt")],
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
            &[Arc::from("a1"), Arc::from("b1")],
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
            &[Arc::from("a1"), Arc::from("b1"), Arc::from("c1.txt")],
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
        let tree = Arc::<SideViewList>::default();
        let make_file = |name: &str| {
            SideViewNode::File(Arc::new(FileMetadata {
                name: Arc::from(name),
                size: Some(12),
                is_dir: false,
                created: None,
                accessed: None,
                modified: None,
                mode: None,
                user: None,
                group: None,
            }))
        };
        let tree = super::add_file(
            tree,
            &[Arc::from("a1"), Arc::from("b1"), Arc::from("c1.txt")],
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
            &[Arc::from("a1"), Arc::from("b1"), Arc::from("c2.txt")],
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
                Arc::from("a1"),
                Arc::from("b1"),
                Arc::from("c2.txt"),
                Arc::from("not_found.txt"),
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
                Arc::from("a1"),
                Arc::from("b1"),
                Arc::from("c3.txt"),
                Arc::from("not_found.txt"),
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
            &[Arc::from("a1"), Arc::from("b1"), Arc::from("c3.txt")],
        )
        .unwrap_err();
        assert_eq!("[FileNotFound] The file was not found", format!("{error}"));

        // Remove file
        let tree = super::remove_file(
            tree,
            &[Arc::from("a1"), Arc::from("b1"), Arc::from("c2.txt")],
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
        let tree = super::remove_file(tree, &[Arc::from("a1"), Arc::from("b1")]).unwrap();
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
