use std::env;
use std::fs;
use std::io::IsTerminal;
use std::path::{Path, PathBuf};

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
struct CleanOptions {
    cache: bool,
    dry_run: bool,
    force: bool,
}

fn parse_options(args: &[String]) -> Result<CleanOptions, String> {
    let mut options = CleanOptions::default();
    for argument in args {
        match argument.as_str() {
            "--cache" if !options.cache => options.cache = true,
            "--dry-run" if !options.dry_run => options.dry_run = true,
            "--force" if !options.force => options.force = true,
            "--cache" | "--dry-run" | "--force" => {
                return Err(format!("option `{argument}` was provided more than once"));
            }
            _ => return Err(format!("unknown option `{argument}`")),
        }
    }
    Ok(options)
}

fn project_root(start: &Path) -> Result<PathBuf, String> {
    let current = start
        .canonicalize()
        .map_err(|error| format!("could not resolve current directory: {error}"))?;
    current
        .ancestors()
        .find(|directory| directory.join("tyrus.json").is_file())
        .map(Path::to_path_buf)
        .ok_or_else(|| "could not find tyrus.json in the current directory or its parents".into())
}

fn path_size(path: &Path) -> std::io::Result<u64> {
    let metadata = fs::symlink_metadata(path)?;
    if metadata.file_type().is_symlink() || metadata.is_file() {
        return Ok(metadata.len());
    }
    let mut total = 0u64;
    for entry in fs::read_dir(path)? {
        total = total.saturating_add(path_size(&entry?.path())?);
    }
    Ok(total)
}

fn remove_path(path: &Path) -> std::io::Result<()> {
    let metadata = fs::symlink_metadata(path)?;
    if metadata.file_type().is_symlink() || metadata.is_file() {
        fs::remove_file(path)
    } else {
        fs::remove_dir_all(path)
    }
}

fn display_size(bytes: u64) -> String {
    const UNITS: [&str; 5] = ["B", "KiB", "MiB", "GiB", "TiB"];
    if bytes < 1024 {
        return format!("{bytes} B");
    }
    let mut value = bytes as f64;
    let mut unit = 0;
    while value >= 1024.0 && unit < UNITS.len() - 1 {
        value /= 1024.0;
        unit += 1;
    }
    format!("{value:.1} {}", UNITS[unit])
}

fn confirm_removal(label: &str, force: bool) -> Result<bool, String> {
    if force {
        return Ok(true);
    }
    if !std::io::stdin().is_terminal() {
        return Err(format!(
            "refusing to remove {label} without an interactive confirmation; rerun with `--force`"
        ));
    }
    inquire::Confirm::new(&format!("Remove {label}?"))
        .with_default(false)
        .prompt()
        .map_err(|error| format!("confirmation failed: {error}"))
}

fn clean_one(path: &Path, label: &str, dry_run: bool, force: bool) -> Result<bool, String> {
    if !path.exists() && fs::symlink_metadata(path).is_err() {
        return Ok(false);
    }
    let size = path_size(path).map_err(|error| format!("could not measure {label}: {error}"))?;
    if dry_run {
        println!("Would remove: {label}");
        println!("Would free:   {}", display_size(size));
        return Ok(true);
    }
    if !confirm_removal(label, force)? {
        println!("Skipped: {label}");
        return Ok(true);
    }
    println!("Removing {label} ({})", display_size(size));
    remove_path(path).map_err(|error| match error.kind() {
        std::io::ErrorKind::PermissionDenied => {
            format!("permission denied removing {label}; check file permissions")
        }
        _ => format!("could not remove {label}: {error}"),
    })?;
    Ok(true)
}

pub(super) fn run(args: &[String]) {
    let result = (|| {
        let options = parse_options(args)?;
        let root = project_root(&env::current_dir().map_err(|error| error.to_string())?)?;
        let target = root.join("target");
        let target_removed = clean_one(&target, "target/", options.dry_run, options.force)?;

        let mut cache_requested = false;
        if options.cache {
            let home = directories::BaseDirs::new()
                .ok_or_else(|| "could not determine the user's home directory".to_string())?
                .home_dir()
                .to_path_buf();
            let tyrus_directory = home.join(".tyrus");
            if fs::symlink_metadata(&tyrus_directory)
                .is_ok_and(|metadata| metadata.file_type().is_symlink())
            {
                return Err(
                    "refusing to clean ~/.tyrus/cache/ because ~/.tyrus is a symbolic link".into(),
                );
            }
            let cache = tyrus_directory.join("cache");
            if !cache.exists() && fs::symlink_metadata(&cache).is_err() {
                eprintln!("Warning: ~/.tyrus/cache/ does not exist. Nothing to remove.");
            } else {
                eprintln!("Warning: removing ~/.tyrus/cache/ can affect other Tyrus projects.");
                cache_requested =
                    clean_one(&cache, "~/.tyrus/cache/", options.dry_run, options.force)?;
            }
        }

        if !target_removed && !cache_requested {
            println!("Nothing to clean. target/ does not exist.");
        } else if options.dry_run {
            println!("(no changes made)");
        } else {
            println!("Done.");
        }
        Ok::<(), String>(())
    })();
    if let Err(message) = result {
        eprintln!("Error: {message}");
        std::process::exit(1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args(values: &[&str]) -> Vec<String> {
        values.iter().map(|value| (*value).to_string()).collect()
    }

    #[test]
    fn parses_clean_options_and_rejects_unknown_or_duplicate_flags() {
        assert_eq!(
            parse_options(&args(&["--cache", "--dry-run", "--force"])).unwrap(),
            CleanOptions {
                cache: true,
                dry_run: true,
                force: true,
            }
        );
        assert!(parse_options(&args(&["--unsafe"])).is_err());
        assert!(parse_options(&args(&["--force", "--force"])).is_err());
    }

    #[test]
    fn measures_and_removes_only_the_requested_tree() {
        let root = tempfile::tempdir().unwrap();
        let target = root.path().join("target");
        let source = root.path().join("src");
        fs::create_dir_all(&target).unwrap();
        fs::create_dir_all(&source).unwrap();
        fs::write(target.join("artifact"), b"artifact").unwrap();
        fs::write(source.join("main.tyr"), b"source").unwrap();

        assert_eq!(path_size(&target).unwrap(), 8);
        remove_path(&target).unwrap();
        assert!(!target.exists());
        assert!(source.join("main.tyr").exists());
    }

    #[test]
    fn size_format_uses_binary_units() {
        assert_eq!(display_size(512), "512 B");
        assert_eq!(display_size(1536), "1.5 KiB");
    }
}
