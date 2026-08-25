use std::collections::HashMap;
use std::ffi::OsStr;
use std::path::{Path, PathBuf};

use ignore::WalkBuilder;

use crate::error::Error;
use crate::model::Language;

#[derive(Debug)]
pub(crate) struct DiscoveryResult {
    pub root: DiscoveredDirectory,
    pub errors: Vec<Error>,
}

#[derive(Debug)]
pub(crate) struct DiscoveredDirectory {
    pub path: PathBuf,
    pub walk_depth: usize,
    pub depth_limited: bool,
    pub entries: Vec<DiscoveredEntry>,
}

#[derive(Debug)]
pub(crate) enum DiscoveredEntry {
    Directory(DiscoveredDirectory),
    File(PathBuf),
}

#[derive(Debug)]
struct ChildEntry {
    path: PathBuf,
    depth: usize,
    is_directory: bool,
}

pub(crate) fn discover(root: &Path, walk_depth: usize) -> DiscoveryResult {
    let mut builder = WalkBuilder::new(root);
    builder
        .hidden(false)
        .follow_links(false)
        .max_depth(Some(walk_depth))
        .filter_entry(|entry| entry.file_name() != OsStr::new(".git"));

    let mut children: HashMap<PathBuf, Vec<ChildEntry>> = HashMap::new();
    let mut errors = Vec::new();

    for result in builder.build() {
        let entry = match result {
            Ok(entry) => entry,
            Err(error) => {
                errors.push(Error::DirectoryTraversal {
                    path: root.to_path_buf(),
                    message: error.to_string(),
                });
                continue;
            }
        };

        if entry.depth() == 0 {
            continue;
        }

        let Some(file_type) = entry.file_type() else {
            continue;
        };
        if file_type.is_symlink() {
            continue;
        }

        let depth = entry.depth();
        let path = entry.into_path();
        let Some(parent) = path.parent() else {
            continue;
        };

        if file_type.is_dir() {
            children
                .entry(parent.to_path_buf())
                .or_default()
                .push(ChildEntry {
                    path,
                    depth,
                    is_directory: true,
                });
        } else if file_type.is_file() && Language::from_path(&path).is_some() {
            children
                .entry(parent.to_path_buf())
                .or_default()
                .push(ChildEntry {
                    path,
                    depth,
                    is_directory: false,
                });
        }
    }

    let root = build_directory(root.to_path_buf(), 0, walk_depth, &mut children);
    DiscoveryResult { root, errors }
}

fn build_directory(
    path: PathBuf,
    depth: usize,
    walk_depth: usize,
    children: &mut HashMap<PathBuf, Vec<ChildEntry>>,
) -> DiscoveredDirectory {
    let depth_limited = depth >= walk_depth;
    let mut entries = Vec::new();

    if !depth_limited {
        let mut child_entries = children.remove(&path).unwrap_or_default();
        child_entries.sort_by(|left, right| {
            display_name(&left.path)
                .cmp(&display_name(&right.path))
                .then_with(|| left.path.cmp(&right.path))
        });

        for child in child_entries {
            if child.is_directory {
                let directory = build_directory(child.path, child.depth, walk_depth, children);
                if directory.depth_limited || !directory.entries.is_empty() {
                    entries.push(DiscoveredEntry::Directory(directory));
                }
            } else {
                entries.push(DiscoveredEntry::File(child.path));
            }
        }
    }

    DiscoveredDirectory {
        path,
        walk_depth,
        depth_limited,
        entries,
    }
}

fn display_name(path: &Path) -> String {
    path.file_name()
        .unwrap_or(path.as_os_str())
        .to_string_lossy()
        .into_owned()
}
