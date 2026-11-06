use std::path::{Path, PathBuf};

pub struct Library {
    pub name: String,
    pub directory: PathBuf,
}

pub fn library_files(output: &str) -> Result<Vec<String>, String> {
    let mut files = Vec::new();
    let mut current = String::new();
    let mut quoted = false;
    for character in output.chars() {
        if character == '"' {
            quoted = !quoted;
        } else if character.is_whitespace() && !quoted {
            if !current.is_empty() {
                files.push(std::mem::take(&mut current));
            }
        } else {
            current.push(character);
        }
    }
    if quoted {
        return Err("unterminated quoted library path".to_string());
    }
    if !current.is_empty() {
        files.push(current);
    }
    Ok(files)
}

pub fn resolve_library(file: &str, directories: &[PathBuf]) -> Result<Library, String> {
    let filename = file.rsplit(['/', '\\']).next().unwrap_or(file);
    let Some(name) = filename.strip_suffix(".lib") else {
        return Err(format!("LLVM MSVC library `{file}` must end in .lib"));
    };
    if name.is_empty() || name.contains([':', '=', '\0', '\n', '\r']) {
        return Err(format!("invalid LLVM MSVC library name `{name}`"));
    }
    // Release archives can retain build-machine paths such as S:\build\zstd.lib.
    // Rust parses the drive colon as a library rename, so emit the basename and
    // a separate search directory, resolving relocated files in the installation.
    let original = Path::new(file);
    let resolved = original
        .is_absolute()
        .then(|| original.to_path_buf())
        .filter(|path| path.is_file())
        .or_else(|| {
            directories
                .iter()
                .map(|directory| directory.join(filename))
                .find(|path| path.is_file())
        })
        .ok_or_else(|| {
            format!(
                "LLVM library `{file}` was not found in its original location, the LLVM lib directory, or the MSVC LIB directories; install the matching `{filename}` dependency"
            )
        })?;
    Ok(Library {
        name: name.to_string(),
        directory: resolved
            .parent()
            .expect("resolved library has a parent")
            .to_path_buf(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn quoted_windows_paths_preserve_spaces_and_backslashes() {
        assert_eq!(
            library_files("\"S:\\Build Files\\zstd.lib\" kernel32.lib\r\n").unwrap(),
            ["S:\\Build Files\\zstd.lib", "kernel32.lib"]
        );
        assert!(library_files("\"unterminated.lib").is_err());
    }

    #[test]
    fn relocated_drive_paths_become_names_and_search_directories() {
        let root = std::env::temp_dir().join(format!("tyrus-llvm-link-{}", std::process::id()));
        std::fs::create_dir_all(&root).unwrap();
        std::fs::write(root.join("zstd.lib"), b"archive").unwrap();
        let library = resolve_library("S:\\build\\zstd.lib", std::slice::from_ref(&root)).unwrap();
        assert_eq!(library.name, "zstd");
        assert_eq!(library.directory, root);
        assert!(resolve_library("S:\\build\\missing.lib", std::slice::from_ref(&root)).is_err());
        assert!(resolve_library("bad:name.lib", std::slice::from_ref(&root)).is_err());
        std::fs::remove_dir_all(root).unwrap();
    }
}
