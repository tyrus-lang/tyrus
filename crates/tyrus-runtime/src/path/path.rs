use tyrus_string::TyrusString;

pub(crate) fn resolve(raw: &str) -> String {
    let separators_fixed = if cfg!(windows) {
        raw.to_string()
    } else {
        raw.replace('\\', "/")
    };
    let path = std::path::Path::new(&separators_fixed);
    let mut resolved = if path.is_absolute() {
        std::path::PathBuf::new()
    } else {
        std::env::current_dir().unwrap_or_default()
    };
    for component in path.components() {
        match component {
            std::path::Component::CurDir => {}
            std::path::Component::ParentDir => {
                if resolved.file_name().is_some() {
                    resolved.pop();
                }
            }
            std::path::Component::Normal(_)
            | std::path::Component::RootDir
            | std::path::Component::Prefix(_) => resolved.push(component.as_os_str()),
        }
    }
    resolved.to_string_lossy().into_owned()
}

fn path_text(value: &TyrusString) -> Result<String, ()> {
    if value.is_null() {
        return Err(());
    }
    Ok(unsafe { value.as_str() }.to_owned())
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_fs_resolve_path(path: TyrusString) -> TyrusString {
    let Ok(text) = path_text(&path) else {
        return TyrusString::null();
    };
    crate::strings::alloc_string(&resolve(&text))
}

#[cfg(test)]
mod tests {
    use super::resolve;
    use std::sync::{Mutex, OnceLock};

    fn cwd_lock() -> &'static Mutex<()> {
        static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
        LOCK.get_or_init(|| Mutex::new(()))
    }

    fn with_cwd<T>(dir: &std::path::Path, run: impl FnOnce() -> T) -> T {
        let _guard = cwd_lock().lock().unwrap();
        let original = std::env::current_dir().unwrap();
        std::env::set_current_dir(dir).unwrap();
        let result = run();
        std::env::set_current_dir(original).unwrap();
        result
    }

    #[test]
    fn absolute_path_normalizes_components_using_native_roots() {
        let path = std::env::temp_dir()
            .join("tyrus-path-normalization")
            .join("..")
            .join("bin");
        let expected = std::env::temp_dir().join("bin");
        assert_eq!(resolve(&path.to_string_lossy()), expected.to_string_lossy());
    }

    #[test]
    fn relative_path_resolves_against_the_current_directory() {
        let tmp = std::env::temp_dir().join("tyrus-path-resolve-relative");
        std::fs::create_dir_all(&tmp).unwrap();
        let (resolved, current_dir) = with_cwd(&tmp, || {
            (
                resolve("data/message.txt"),
                std::env::current_dir().unwrap(),
            )
        });
        assert_eq!(
            resolved,
            current_dir
                .join("data")
                .join("message.txt")
                .to_string_lossy()
        );
    }

    #[test]
    fn current_dir_segments_are_dropped() {
        let tmp = std::env::temp_dir().join("tyrus-path-resolve-curdir");
        std::fs::create_dir_all(&tmp).unwrap();
        let (resolved, current_dir) = with_cwd(&tmp, || {
            (
                resolve("./data/./message.txt"),
                std::env::current_dir().unwrap(),
            )
        });
        assert_eq!(
            resolved,
            current_dir
                .join("data")
                .join("message.txt")
                .to_string_lossy()
        );
    }

    #[test]
    fn parent_dir_segments_walk_up_one_level_each() {
        let tmp = std::env::temp_dir().join("tyrus-path-resolve-parentdir/nested");
        std::fs::create_dir_all(&tmp).unwrap();
        let (resolved, current_dir) = with_cwd(&tmp, || {
            (
                resolve("../sibling/message.txt"),
                std::env::current_dir().unwrap(),
            )
        });
        assert_eq!(
            resolved,
            current_dir
                .parent()
                .unwrap()
                .join("sibling")
                .join("message.txt")
                .to_string_lossy()
        );
    }

    #[test]
    fn relative_paths_with_dot_segments_open_from_the_working_directory() {
        let root = std::env::temp_dir().join(format!("tyrus-path-open-{}", uuid::Uuid::new_v4()));
        let working_dir = root.join("working");
        let sibling_dir = root.join("sibling");
        std::fs::create_dir_all(&working_dir).unwrap();
        std::fs::create_dir_all(&sibling_dir).unwrap();
        std::fs::write(sibling_dir.join("fixture.txt"), "working-directory fixture").unwrap();

        let content = with_cwd(&working_dir, || {
            let parent_path = resolve("../sibling/./fixture.txt");
            let native_path = if cfg!(windows) {
                resolve("..\\sibling\\fixture.txt")
            } else {
                resolve("../sibling/fixture.txt")
            };
            assert_eq!(parent_path, native_path);
            std::fs::read_to_string(parent_path).unwrap()
        });

        assert_eq!(content, "working-directory fixture");
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn parent_dir_beyond_root_does_not_overflow() {
        let mut root = std::path::PathBuf::new();
        for component in std::env::current_dir().unwrap().components() {
            match component {
                std::path::Component::Prefix(_) | std::path::Component::RootDir => {
                    root.push(component.as_os_str());
                }
                _ => break,
            }
        }
        let beyond_root = root.join("..").join("..").join("etc").join("passwd");
        let expected = root.join("etc").join("passwd");
        assert_eq!(
            resolve(&beyond_root.to_string_lossy()),
            expected.to_string_lossy()
        );
    }

    #[test]
    fn backslashes_become_forward_slashes_off_windows() {
        if cfg!(windows) {
            return;
        }
        assert_eq!(resolve("/usr\\local\\bin"), "/usr/local/bin");
    }

    #[cfg(windows)]
    #[test]
    fn unc_paths_preserve_the_server_and_share_root_while_normalizing() {
        assert_eq!(
            resolve(r"\\server\share\folder\..\file.txt"),
            r"\\server\share\file.txt"
        );
        assert_eq!(
            resolve(r"\\server\share\..\file.txt"),
            r"\\server\share\file.txt"
        );
    }

    #[cfg(windows)]
    #[test]
    fn windows_drive_paths_preserve_the_drive_root_while_normalizing() {
        assert_eq!(resolve(r"C:\folder\..\file.txt"), r"C:\file.txt");
        assert_eq!(resolve(r"C:\..\file.txt"), r"C:\file.txt");
    }
}
