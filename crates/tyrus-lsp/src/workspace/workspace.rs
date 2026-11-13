use super::*;

pub(super) fn nearest_project_root(start: &Path) -> Option<PathBuf> {
    let mut directory = if start.is_dir() {
        Some(start)
    } else {
        start.parent()
    };
    while let Some(current) = directory {
        if current.join("tyrus.json").is_file() {
            return Some(current.to_path_buf());
        }
        directory = current.parent();
    }
    None
}

pub(super) const MAX_WORKSPACE_FILES: usize = 10_000;
pub(super) const MAX_WORKSPACE_SCAN_DEPTH: usize = 64;
pub(super) const IGNORED_WORKSPACE_DIRECTORIES: &[&str] =
    &[".git", "target", "node_modules", "dist", "build", ".cache"];

#[cfg(test)]
pub(super) fn collect_tyrus_files(directory: &Path, files: &mut Vec<PathBuf>) -> bool {
    collect_tyrus_files_with_options(
        directory,
        files,
        MAX_WORKSPACE_FILES,
        MAX_WORKSPACE_SCAN_DEPTH,
        &IGNORED_WORKSPACE_DIRECTORIES
            .iter()
            .map(|directory| (*directory).to_string())
            .collect(),
        0,
    )
}

pub(super) fn collect_tyrus_files_with_options(
    directory: &Path,
    files: &mut Vec<PathBuf>,
    max_files: usize,
    max_depth: usize,
    excluded_directories: &HashSet<String>,
    depth: usize,
) -> bool {
    if depth > max_depth {
        return true;
    }
    let Ok(entries) = fs::read_dir(directory) else {
        return false;
    };
    let mut entries = entries.flatten().collect::<Vec<_>>();
    entries.sort_by_key(|entry| entry.file_name());
    for entry in entries {
        let Ok(file_type) = entry.file_type() else {
            continue;
        };
        if file_type.is_symlink() {
            continue;
        }
        let path = entry.path();
        if file_type.is_dir() {
            let excluded = path
                .file_name()
                .and_then(|name| name.to_str())
                .is_some_and(|name| excluded_directories.contains(name));
            if !excluded
                && collect_tyrus_files_with_options(
                    &path,
                    files,
                    max_files,
                    max_depth,
                    excluded_directories,
                    depth + 1,
                )
            {
                return true;
            }
        } else if path.extension().and_then(|extension| extension.to_str()) == Some("tyr") {
            if files.len() >= max_files {
                return true;
            }
            files.push(path);
        }
    }
    false
}

pub(super) fn relative_module_source(current: &Path, target: &Path) -> Option<String> {
    let from = current.parent()?;
    let mut from_components = from.components().collect::<Vec<_>>();
    let mut target_components = target.components().collect::<Vec<_>>();
    while from_components.first() == target_components.first() {
        from_components.remove(0);
        target_components.remove(0);
    }
    let mut relative = PathBuf::new();
    for component in from_components {
        if !matches!(component, Component::CurDir) {
            relative.push("..");
        }
    }
    for component in target_components {
        relative.push(component.as_os_str());
    }
    relative.set_extension("");
    let mut source = relative.to_string_lossy().replace('\\', "/");
    if !source.starts_with('.') {
        source = format!("./{source}");
    }
    Some(source)
}

#[cfg(test)]
mod tests {
    use super::{collect_tyrus_files, collect_tyrus_files_with_options};
    use std::collections::HashSet;
    use std::fs;
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicU64, Ordering};

    static NEXT_ROOT: AtomicU64 = AtomicU64::new(0);

    #[test]
    fn workspace_scan_skips_generated_and_dependency_directories() {
        let id = NEXT_ROOT.fetch_add(1, Ordering::Relaxed);
        let root = std::env::temp_dir().join(format!(
            "tyrus-lsp-workspace-scan-{}-{id}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&root);
        for directory in ["src", "target/generated", "node_modules/pkg", "dist"] {
            fs::create_dir_all(root.join(directory)).unwrap();
            fs::write(root.join(directory).join("module.tyr"), "const value = 1;").unwrap();
        }
        let mut files = Vec::<PathBuf>::new();
        collect_tyrus_files(&root, &mut files);
        assert_eq!(files, vec![root.join("src/module.tyr")]);
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn workspace_scan_stops_at_file_and_depth_limits() {
        let id = NEXT_ROOT.fetch_add(1, Ordering::Relaxed);
        let root = std::env::temp_dir().join(format!(
            "tyrus-lsp-workspace-limits-{}-{id}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(root.join("a")).unwrap();
        fs::create_dir_all(root.join("nested/deeper")).unwrap();
        fs::write(root.join("a/first.tyr"), "const value = 1;").unwrap();
        fs::write(root.join("nested/deeper/second.tyr"), "const value = 2;").unwrap();

        let mut files = Vec::<PathBuf>::new();
        let truncated = collect_tyrus_files_with_options(
            &root,
            &mut files,
            1,
            1,
            &HashSet::from_iter(
                super::IGNORED_WORKSPACE_DIRECTORIES
                    .iter()
                    .map(|directory| (*directory).to_string()),
            ),
            0,
        );

        assert!(truncated);
        assert_eq!(files, vec![root.join("a/first.tyr")]);
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn workspace_scan_honors_additional_excluded_directory_names() {
        let id = NEXT_ROOT.fetch_add(1, Ordering::Relaxed);
        let root = std::env::temp_dir().join(format!(
            "tyrus-lsp-workspace-exclusions-{}-{id}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(root.join("src")).unwrap();
        fs::create_dir_all(root.join("vendor/generated")).unwrap();
        fs::write(root.join("src/main.tyr"), "const value = 1;").unwrap();
        fs::write(root.join("vendor/generated/module.tyr"), "const value = 2;").unwrap();

        let excluded = HashSet::from_iter(["vendor".to_string()]);
        let mut files = Vec::<PathBuf>::new();
        let truncated = collect_tyrus_files_with_options(&root, &mut files, 50, 8, &excluded, 0);

        assert!(!truncated);
        assert_eq!(files, vec![root.join("src/main.tyr")]);
        fs::remove_dir_all(root).unwrap();
    }

    #[cfg(unix)]
    #[test]
    fn workspace_scan_does_not_follow_symlinked_directories() {
        use std::os::unix::fs::symlink;

        let id = NEXT_ROOT.fetch_add(1, Ordering::Relaxed);
        let root = std::env::temp_dir().join(format!(
            "tyrus-lsp-workspace-symlink-{}-{id}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(root.join("src")).unwrap();
        fs::write(root.join("src/module.tyr"), "const value = 1;").unwrap();
        symlink(&root, root.join("src/loop")).unwrap();

        let mut files = Vec::<PathBuf>::new();
        collect_tyrus_files(&root, &mut files);

        assert_eq!(files, vec![root.join("src/module.tyr")]);
        fs::remove_dir_all(root).unwrap();
    }
}
