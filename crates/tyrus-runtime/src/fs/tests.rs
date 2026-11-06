use super::*;

#[cfg(test)]
mod sync_result_tests {
    use super::*;

    unsafe fn assert_error_code(result: FsSyncResult, expected: &str) {
        assert_eq!(result.tag, 1);
        let error = unsafe { &*(&result.payload.error as *const _ as *const FsErrorTaken) };
        assert_eq!(unsafe { error.code.as_str() }, expected);
    }

    #[test]
    fn atomic_write_replaces_an_existing_file() {
        let target = std::env::temp_dir().join(format!(
            "tyrus-fs-atomic-replace-{}.txt",
            uuid::Uuid::new_v4()
        ));
        fs::write(&target, "original").unwrap();

        for value in ["first replacement", "second replacement"] {
            let mut result = std::mem::MaybeUninit::<FsSyncResult>::uninit();
            unsafe {
                tyrus_fs_atomic_write_result(
                    TyrusString::lend(&target.to_string_lossy()).unwrap(),
                    TyrusString::lend(value).unwrap(),
                    result.as_mut_ptr(),
                )
            };
            assert_eq!(unsafe { result.assume_init() }.tag, 0);
            assert_eq!(fs::read_to_string(&target).unwrap(), value);
        }

        fs::remove_file(target).unwrap();
    }

    #[test]
    fn read_file_sync_result_contains_success_and_structured_error_variants() {
        let root = std::env::temp_dir().join(format!("tyrus-fs-result-{}", uuid::Uuid::new_v4()));
        fs::create_dir_all(&root).unwrap();
        let path = root.join("value.txt");
        fs::write(&path, "value").unwrap();
        let path_text = path.to_string_lossy();
        let mut success = std::mem::MaybeUninit::<FsSyncResult>::uninit();
        unsafe {
            tyrus_fs_read_file_sync_result(
                TyrusString::lend(&path_text).unwrap(),
                success.as_mut_ptr(),
            )
        };
        let success = unsafe { success.assume_init() };
        assert_eq!(success.tag, 0);
        let value = unsafe { &*(&success.payload.string as *const _ as *const TyrusString) };
        assert_eq!(unsafe { value.as_str() }, "value");
        unsafe { std::ptr::read(value).release() };

        let missing = root.join("missing.txt").to_string_lossy().into_owned();
        let mut failure = std::mem::MaybeUninit::<FsSyncResult>::uninit();
        unsafe {
            tyrus_fs_read_file_sync_result(
                TyrusString::lend(&missing).unwrap(),
                failure.as_mut_ptr(),
            )
        };
        let failure = unsafe { failure.assume_init() };
        assert_eq!(failure.tag, 1);
        let error = unsafe { &*(&failure.payload.error as *const _ as *const FsErrorTaken) };
        assert_eq!(unsafe { error.code.as_str() }, "ERR_FS_READ_FAILED");
        fs::remove_dir_all(root).unwrap();
    }

    #[cfg(any(unix, windows))]
    #[test]
    fn stat_and_metadata_report_symlink_state_and_follow_the_target() {
        let root = std::env::temp_dir().join(format!(
            "tyrus-fs-symlink-metadata-{}",
            uuid::Uuid::new_v4()
        ));
        fs::create_dir_all(&root).unwrap();
        let target = root.join("target.txt");
        let link = root.join("link.txt");
        fs::write(&target, "target data").unwrap();

        #[cfg(unix)]
        std::os::unix::fs::symlink(&target, &link).unwrap();
        #[cfg(windows)]
        {
            if std::env::var("TYRUS_TEST_WINDOWS_SYMLINK").as_deref() == Ok("0") {
                eprintln!(
                    "SKIP: Windows file symlink metadata coverage requires symlink privilege."
                );
                fs::remove_dir_all(&root).unwrap();
                return;
            }
            match std::os::windows::fs::symlink_file(&target, &link) {
                Ok(()) => {}
                Err(error)
                    if error.kind() == std::io::ErrorKind::PermissionDenied
                        || matches!(error.raw_os_error(), Some(5 | 1314)) =>
                {
                    eprintln!(
                        "SKIP: Windows file symlink metadata coverage requires symlink privilege."
                    );
                    fs::remove_dir_all(&root).unwrap();
                    return;
                }
                Err(error) => panic!("Windows file symlink metadata setup failed: {error}"),
            }
        }

        let link = link.to_string_lossy();
        let mut stats = std::mem::MaybeUninit::<FsFileStats>::uninit();
        unsafe { tyrus_fs_stat(TyrusString::lend(&link).unwrap(), stats.as_mut_ptr()) };
        let stats = unsafe { stats.assume_init() };
        assert!(stats.is_file);
        assert!(stats.is_symlink);
        assert_eq!(stats.size, "target data".len() as u64);

        let mut metadata = std::mem::MaybeUninit::<FsMetadata>::uninit();
        unsafe { tyrus_fs_metadata(TyrusString::lend(&link).unwrap(), metadata.as_mut_ptr()) };
        let metadata = unsafe { metadata.assume_init() };
        assert!(metadata.is_file);
        assert!(metadata.is_symlink);
        assert_eq!(metadata.size, "target data".len() as u64);

        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn write_file_sync_result_uses_the_ok_tag_after_writing() {
        let root =
            std::env::temp_dir().join(format!("tyrus-fs-write-result-{}", uuid::Uuid::new_v4()));
        fs::create_dir_all(&root).unwrap();
        let path = root.join("value.txt");
        let path_text = path.to_string_lossy();
        let mut result = std::mem::MaybeUninit::<FsSyncResult>::uninit();
        unsafe {
            tyrus_fs_write_file_sync_result(
                TyrusString::lend(&path_text).unwrap(),
                TyrusString::lend("value").unwrap(),
                result.as_mut_ptr(),
            )
        };
        let result = unsafe { result.assume_init() };
        assert_eq!(result.tag, 0);
        assert_eq!(fs::read_to_string(path).unwrap(), "value");
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn disk_usage_result_contains_structured_error_for_missing_parent() {
        let path = std::env::temp_dir()
            .join(format!("tyrus-fs-disk-usage-{}", uuid::Uuid::new_v4()))
            .join("missing")
            .to_string_lossy()
            .into_owned();
        let mut result = std::mem::MaybeUninit::<FsSyncResult>::uninit();
        unsafe {
            tyrus_fs_disk_usage_result(TyrusString::lend(&path).unwrap(), result.as_mut_ptr())
        };
        unsafe { assert_error_code(result.assume_init(), "ERR_FS_DISK_USAGE_FAILED") };
    }

    #[test]
    fn readdir_returns_a_structured_error_for_a_non_directory_path() {
        let file =
            std::env::temp_dir().join(format!("tyrus-fs-readdir-file-{}", uuid::Uuid::new_v4()));
        fs::write(&file, "not a directory").unwrap();
        let mut result = std::mem::MaybeUninit::<FsSyncResult>::uninit();
        unsafe {
            tyrus_fs_readdir_result(
                TyrusString::lend(&file.to_string_lossy()).unwrap(),
                ptr::null(),
                result.as_mut_ptr(),
            )
        };
        unsafe { assert_error_code(result.assume_init(), "ERR_FS_READ_FAILED") };
        fs::remove_file(file).unwrap();
    }

    #[test]
    fn recursive_copy_propagates_nested_file_failures() {
        let root = std::env::temp_dir().join(format!("tyrus-fs-cp-{}", uuid::Uuid::new_v4()));
        let source = root.join("source");
        let destination = root.join("destination");
        fs::create_dir_all(&source).unwrap();
        fs::write(source.join("child.txt"), "source file").unwrap();
        fs::create_dir_all(destination.join("child.txt")).unwrap();
        let source_text = source.to_string_lossy();
        let destination_text = destination.to_string_lossy();
        let mut recursive = true;
        let options = CpOptionsFfi {
            recursive: &mut recursive,
            force: ptr::null_mut(),
        };
        let mut result = std::mem::MaybeUninit::<FsSyncResult>::uninit();

        unsafe {
            tyrus_fs_cp_result(
                TyrusString::lend(&source_text).unwrap(),
                TyrusString::lend(&destination_text).unwrap(),
                &options,
                result.as_mut_ptr(),
            )
        };

        unsafe { assert_error_code(result.assume_init(), "ERR_FS_CP_FAILED") };
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn filesystem_mutation_results_keep_the_structured_error_codes() {
        let missing = std::env::temp_dir()
            .join(format!("tyrus-fs-missing-{}", uuid::Uuid::new_v4()))
            .to_string_lossy()
            .into_owned();
        let mut result = std::mem::MaybeUninit::<FsSyncResult>::uninit();
        unsafe {
            tyrus_fs_rm_result(
                TyrusString::lend(&missing).unwrap(),
                ptr::null(),
                result.as_mut_ptr(),
            )
        };
        unsafe { assert_error_code(result.assume_init(), "ERR_FS_RM_FAILED") };

        let mut result = std::mem::MaybeUninit::<FsSyncResult>::uninit();
        unsafe {
            tyrus_fs_cp_result(
                TyrusString::lend(&missing).unwrap(),
                TyrusString::lend(&missing).unwrap(),
                ptr::null(),
                result.as_mut_ptr(),
            )
        };
        unsafe { assert_error_code(result.assume_init(), "ERR_FS_CP_FAILED") };

        let mut result = std::mem::MaybeUninit::<FsSyncResult>::uninit();
        unsafe {
            tyrus_fs_mv_result(
                TyrusString::lend(&missing).unwrap(),
                TyrusString::lend(&missing).unwrap(),
                result.as_mut_ptr(),
            )
        };
        unsafe { assert_error_code(result.assume_init(), "ERR_FS_MV_FAILED") };

        let mut result = std::mem::MaybeUninit::<FsSyncResult>::uninit();
        unsafe {
            tyrus_fs_remove_all_result(TyrusString::lend(&missing).unwrap(), result.as_mut_ptr())
        };
        unsafe { assert_error_code(result.assume_init(), "ERR_FS_REMOVE_FAILED") };

        let mut result = std::mem::MaybeUninit::<FsSyncResult>::uninit();
        unsafe {
            tyrus_fs_chmod_result(
                TyrusString::lend(&missing).unwrap(),
                0o600,
                result.as_mut_ptr(),
            )
        };
        unsafe { assert_error_code(result.assume_init(), "ERR_FS_CHMOD_FAILED") };

        let mut result = std::mem::MaybeUninit::<FsSyncResult>::uninit();
        unsafe {
            tyrus_fs_chown_result(
                TyrusString::lend(&missing).unwrap(),
                0,
                0,
                result.as_mut_ptr(),
            )
        };
        unsafe { assert_error_code(result.assume_init(), "ERR_FS_CHOWN_FAILED") };

        let mut result = std::mem::MaybeUninit::<FsSyncResult>::uninit();
        let nested_missing = format!("{missing}/child/link");
        unsafe {
            tyrus_fs_symlink_result(
                TyrusString::lend(&missing).unwrap(),
                TyrusString::lend(&nested_missing).unwrap(),
                result.as_mut_ptr(),
            )
        };
        unsafe { assert_error_code(result.assume_init(), "ERR_FS_SYMLINK_FAILED") };

        let mut result = std::mem::MaybeUninit::<FsSyncResult>::uninit();
        unsafe {
            tyrus_fs_hard_link_result(
                TyrusString::lend(&missing).unwrap(),
                TyrusString::lend(&missing).unwrap(),
                result.as_mut_ptr(),
            )
        };
        unsafe { assert_error_code(result.assume_init(), "ERR_FS_HARDLINK_FAILED") };

        let atomic_target =
            std::env::temp_dir().join(format!("tyrus-fs-atomic-target-{}", uuid::Uuid::new_v4()));
        fs::create_dir(&atomic_target).unwrap();
        let mut result = std::mem::MaybeUninit::<FsSyncResult>::uninit();
        unsafe {
            tyrus_fs_atomic_write_result(
                TyrusString::lend(&atomic_target.to_string_lossy()).unwrap(),
                TyrusString::lend("value").unwrap(),
                result.as_mut_ptr(),
            )
        };
        unsafe { assert_error_code(result.assume_init(), "ERR_FS_WRITE_FAILED") };
        fs::remove_dir_all(atomic_target).unwrap();
    }

    #[cfg(any(unix, windows))]
    #[test]
    fn file_symlink_result_creates_a_link_that_resolves_to_its_target() {
        let root = std::env::temp_dir().join(format!("tyrus-fs-symlink-{}", uuid::Uuid::new_v4()));
        fs::create_dir_all(&root).unwrap();
        let original = root.join("original.txt");
        let link = root.join("link.txt");
        fs::write(&original, "symlink target").unwrap();
        #[cfg(windows)]
        {
            if std::env::var("TYRUS_TEST_WINDOWS_SYMLINK").as_deref() == Ok("0") {
                eprintln!(
                    "SKIP: Windows file symlink creation requires Developer Mode or SeCreateSymbolicLinkPrivilege."
                );
                fs::remove_dir_all(&root).unwrap();
                return;
            }
            let capability_link = root.join("capability-link.txt");
            match std::os::windows::fs::symlink_file(&original, &capability_link) {
                Ok(()) => fs::remove_file(capability_link).unwrap(),
                Err(error)
                    if error.kind() == std::io::ErrorKind::PermissionDenied
                        || matches!(error.raw_os_error(), Some(5 | 1314)) =>
                {
                    eprintln!(
                        "SKIP: Windows file symlink creation requires Developer Mode or SeCreateSymbolicLinkPrivilege."
                    );
                    fs::remove_dir_all(&root).unwrap();
                    return;
                }
                Err(error) => panic!("Windows file symlink capability probe failed: {error}"),
            }
        }
        let original = original.to_string_lossy().into_owned();
        let link = link.to_string_lossy().into_owned();
        let mut result = std::mem::MaybeUninit::<FsSyncResult>::uninit();

        unsafe {
            tyrus_fs_symlink_result(
                TyrusString::lend(&original).unwrap(),
                TyrusString::lend(&link).unwrap(),
                result.as_mut_ptr(),
            )
        };
        let result = unsafe { result.assume_init() };
        assert_eq!(result.tag, 0);
        assert_eq!(fs::read_to_string(&link).unwrap(), "symlink target");
        let mut readlink = std::mem::MaybeUninit::<FsSyncResult>::uninit();
        unsafe {
            tyrus_fs_readlink_result(TyrusString::lend(&link).unwrap(), readlink.as_mut_ptr())
        };
        let readlink = unsafe { readlink.assume_init() };
        assert_eq!(readlink.tag, 0);
        let target = unsafe { &*(&readlink.payload.string as *const _ as *const TyrusString) };
        assert_eq!(unsafe { target.as_str() }, original);
        unsafe { std::ptr::read(target).release() };
        fs::remove_dir_all(root).unwrap();
    }

    #[cfg(any(unix, windows))]
    #[test]
    fn directory_symlink_result_creates_a_link_that_resolves_to_its_target() {
        let root = std::env::temp_dir().join(format!(
            "tyrus-fs-directory-symlink-{}",
            uuid::Uuid::new_v4()
        ));
        fs::create_dir_all(&root).unwrap();
        let target = root.join("target");
        fs::create_dir(&target).unwrap();
        let link = root.join("link");
        #[cfg(windows)]
        {
            if std::env::var("TYRUS_TEST_WINDOWS_SYMLINK").as_deref() == Ok("0") {
                eprintln!(
                    "SKIP: Windows directory symlink creation requires Developer Mode or SeCreateSymbolicLinkPrivilege."
                );
                fs::remove_dir_all(&root).unwrap();
                return;
            }
            let capability_link = root.join("capability-link");
            match std::os::windows::fs::symlink_dir(&target, &capability_link) {
                Ok(()) => fs::remove_dir(&capability_link).unwrap(),
                Err(error)
                    if error.kind() == std::io::ErrorKind::PermissionDenied
                        || matches!(error.raw_os_error(), Some(5 | 1314)) =>
                {
                    eprintln!(
                        "SKIP: Windows directory symlink creation requires Developer Mode or SeCreateSymbolicLinkPrivilege."
                    );
                    fs::remove_dir_all(&root).unwrap();
                    return;
                }
                Err(error) => panic!("Windows directory symlink capability probe failed: {error}"),
            }
        }
        let mut result = std::mem::MaybeUninit::<FsSyncResult>::uninit();

        unsafe {
            tyrus_fs_symlink_dir_result(
                TyrusString::lend(&target.to_string_lossy()).unwrap(),
                TyrusString::lend(&link.to_string_lossy()).unwrap(),
                result.as_mut_ptr(),
            )
        };
        let result = unsafe { result.assume_init() };
        assert_eq!(result.tag, 0);
        assert!(link.join(".").exists());
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn diff_compares_entries_and_returns_stably_ordered_results() {
        let root = std::env::temp_dir().join(format!("tyrus-fs-diff-{}", uuid::Uuid::new_v4()));
        let left = root.join("left");
        let right = root.join("right");
        fs::create_dir_all(&left).unwrap();
        fs::create_dir_all(&right).unwrap();
        fs::write(left.join("added.txt"), "added").unwrap();
        fs::write(left.join("changed.txt"), "left").unwrap();
        fs::write(left.join("same.txt"), "same").unwrap();
        fs::write(right.join("changed.txt"), "right").unwrap();
        fs::write(right.join("removed.txt"), "removed").unwrap();
        fs::write(right.join("same.txt"), "same").unwrap();

        let diff = collect_fs_diff(&left, &right).unwrap();
        assert_eq!(diff.0, ["added.txt"]);
        assert_eq!(diff.1, ["removed.txt"]);
        assert_eq!(diff.2, ["changed.txt"]);
        assert_eq!(diff.3, ["same.txt"]);

        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn glob_returns_sorted_matches_and_propagates_directory_errors() {
        let root = std::env::temp_dir().join(format!("tyrus-fs-glob-{}", uuid::Uuid::new_v4()));
        fs::create_dir_all(root.join("nested")).unwrap();
        fs::write(root.join("z.txt"), "z").unwrap();
        fs::write(root.join("a.txt"), "a").unwrap();
        let pattern = root.join("*.txt").to_string_lossy().into_owned();
        let matches = collect_glob(&pattern).unwrap();
        assert_eq!(
            matches,
            [
                root.join("a.txt").to_string_lossy().into_owned(),
                root.join("z.txt").to_string_lossy().into_owned()
            ]
        );

        let missing_pattern = root.join("missing/*.txt").to_string_lossy().into_owned();
        assert!(collect_glob(&missing_pattern).is_err());
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn glob_result_returns_owned_matches() {
        let root =
            std::env::temp_dir().join(format!("tyrus-fs-glob-result-{}", uuid::Uuid::new_v4()));
        fs::create_dir_all(&root).unwrap();
        fs::write(root.join("b.txt"), "b").unwrap();
        fs::write(root.join("a.txt"), "a").unwrap();
        let pattern = root.join("*.txt").to_string_lossy().into_owned();
        let mut result = std::mem::MaybeUninit::<FsSyncResult>::uninit();
        unsafe { tyrus_fs_glob_result(TyrusString::lend(&pattern).unwrap(), result.as_mut_ptr()) };
        let result = unsafe { result.assume_init() };
        assert_eq!(result.tag, 0);
        let array = unsafe { std::ptr::read(&result.payload.array) };
        let array = std::mem::ManuallyDrop::into_inner(array);
        assert_eq!(array.length, 2);
        let values = unsafe {
            Vec::from_raw_parts(
                array.data.cast::<TyrusString>(),
                array.length,
                array.capacity,
            )
        };
        let names: Vec<String> = values
            .iter()
            .map(|value| unsafe { value.as_str().to_owned() })
            .collect();
        assert_eq!(
            names,
            [
                root.join("a.txt").to_string_lossy().into_owned(),
                root.join("b.txt").to_string_lossy().into_owned()
            ]
        );
        for value in values {
            unsafe { value.release() };
        }
        fs::remove_dir_all(root).unwrap();
    }

    #[cfg(windows)]
    #[test]
    fn glob_pattern_segments_keep_the_windows_drive_prefix() {
        let root = std::env::temp_dir().join(format!("tyrus-fs-glob-{}", uuid::Uuid::new_v4()));
        let pattern = root.join("*.txt").to_string_lossy().into_owned();
        let (start, segments) = glob_pattern_segments(&pattern);
        assert_eq!(start, root);
        assert_eq!(segments, ["*.txt"]);
    }

    #[test]
    fn filesystem_result_wrappers_return_structured_errors_for_remaining_operations() {
        let missing = std::env::temp_dir()
            .join(format!("tyrus-fs-result-missing-{}", uuid::Uuid::new_v4()))
            .to_string_lossy()
            .into_owned();

        let mut result = std::mem::MaybeUninit::<FsSyncResult>::uninit();
        unsafe {
            tyrus_fs_read_file_stream_result(
                TyrusString::lend(&missing).unwrap(),
                result.as_mut_ptr(),
            )
        };
        unsafe { assert_error_code(result.assume_init(), "ERR_FS_READ_FAILED") };

        let blocking_file =
            std::env::temp_dir().join(format!("tyrus-fs-blocking-file-{}", uuid::Uuid::new_v4()));
        fs::write(&blocking_file, "file").unwrap();
        let blocked_stream_path = blocking_file
            .join("child.txt")
            .to_string_lossy()
            .into_owned();
        let mut result = std::mem::MaybeUninit::<FsSyncResult>::uninit();
        unsafe {
            tyrus_fs_write_file_stream_result(
                TyrusString::lend(&blocked_stream_path).unwrap(),
                result.as_mut_ptr(),
            )
        };
        unsafe { assert_error_code(result.assume_init(), "ERR_FS_WRITE_FAILED") };
        fs::remove_file(blocking_file).unwrap();

        let mut result = std::mem::MaybeUninit::<FsSyncResult>::uninit();
        unsafe {
            tyrus_fs_readdir_result(
                TyrusString::lend(&missing).unwrap(),
                ptr::null(),
                result.as_mut_ptr(),
            )
        };
        unsafe { assert_error_code(result.assume_init(), "ERR_FS_READ_FAILED") };

        let mut result = std::mem::MaybeUninit::<FsMetadataResult>::uninit();
        unsafe {
            tyrus_fs_metadata_result(TyrusString::lend(&missing).unwrap(), result.as_mut_ptr())
        };
        let result = unsafe { result.assume_init() };
        assert_eq!(result.tag, 1);
        let error = unsafe { &*(&result.payload.error as *const _ as *const FsErrorTaken) };
        assert_eq!(unsafe { error.code.as_str() }, "ERR_FS_METADATA_FAILED");

        let mut result = std::mem::MaybeUninit::<FsSyncResult>::uninit();
        unsafe {
            tyrus_fs_canonicalize_result(TyrusString::lend(&missing).unwrap(), result.as_mut_ptr())
        };
        unsafe { assert_error_code(result.assume_init(), "ERR_FS_CANONICALIZE_FAILED") };

        let mut result = std::mem::MaybeUninit::<FsSyncResult>::uninit();
        unsafe {
            tyrus_fs_readlink_result(TyrusString::lend(&missing).unwrap(), result.as_mut_ptr())
        };
        unsafe { assert_error_code(result.assume_init(), "ERR_FS_READLINK_FAILED") };

        let mut result = std::mem::MaybeUninit::<FsSyncResult>::uninit();
        unsafe { tyrus_fs_search_result(TyrusString::null(), ptr::null(), result.as_mut_ptr()) };
        unsafe { assert_error_code(result.assume_init(), "ERR_FS_INVALID_PATH") };

        let missing_search_dir =
            std::env::temp_dir().join(format!("tyrus-fs-search-missing-{}", uuid::Uuid::new_v4()));
        let search_pattern = missing_search_dir
            .join("*.txt")
            .to_string_lossy()
            .into_owned();
        let mut result = std::mem::MaybeUninit::<FsSyncResult>::uninit();
        unsafe {
            tyrus_fs_search_result(
                TyrusString::lend(&search_pattern).unwrap(),
                ptr::null(),
                result.as_mut_ptr(),
            )
        };
        unsafe { assert_error_code(result.assume_init(), "ERR_FS_SEARCH_FAILED") };

        let missing_glob_dir =
            std::env::temp_dir().join(format!("tyrus-fs-glob-missing-{}", uuid::Uuid::new_v4()));
        let glob_pattern = missing_glob_dir
            .join("*.txt")
            .to_string_lossy()
            .into_owned();
        let mut result = std::mem::MaybeUninit::<FsSyncResult>::uninit();
        unsafe {
            tyrus_fs_glob_result(
                TyrusString::lend(&glob_pattern).unwrap(),
                result.as_mut_ptr(),
            )
        };
        unsafe { assert_error_code(result.assume_init(), "ERR_FS_GLOB_FAILED") };

        let mut result = std::mem::MaybeUninit::<FsSyncResult>::uninit();
        unsafe { tyrus_fs_watch_result(TyrusString::lend(&missing).unwrap(), result.as_mut_ptr()) };
        unsafe { assert_error_code(result.assume_init(), "ERR_FS_NOT_FOUND") };

        let mut result = std::mem::MaybeUninit::<FsDiffResultUnion>::uninit();
        unsafe {
            tyrus_fs_diff_result(
                TyrusString::null(),
                TyrusString::lend(&missing).unwrap(),
                result.as_mut_ptr(),
            )
        };
        let result = unsafe { result.assume_init() };
        assert_eq!(result.tag, 1);
        let error = unsafe { &*(&result.payload.error as *const _ as *const FsErrorTaken) };
        assert_eq!(unsafe { error.code.as_str() }, "ERR_FS_INVALID_PATH");

        let mut result = std::mem::MaybeUninit::<FsDiffResultUnion>::uninit();
        unsafe {
            tyrus_fs_diff_result(
                TyrusString::lend(&missing).unwrap(),
                TyrusString::lend(&missing).unwrap(),
                result.as_mut_ptr(),
            )
        };
        let result = unsafe { result.assume_init() };
        assert_eq!(result.tag, 1);
        let error = unsafe { &*(&result.payload.error as *const _ as *const FsErrorTaken) };
        assert_eq!(unsafe { error.code.as_str() }, "ERR_FS_DIFF_FAILED");
    }

    #[test]
    fn stat_reports_a_structured_error_for_a_missing_path() {
        let missing = std::env::temp_dir()
            .join(format!("tyrus-fs-stat-missing-{}", uuid::Uuid::new_v4()))
            .to_string_lossy()
            .into_owned();
        let mut result = std::mem::MaybeUninit::<FsFileStatsResult>::uninit();
        unsafe { tyrus_fs_stat_result(TyrusString::lend(&missing).unwrap(), result.as_mut_ptr()) };
        let result = unsafe { result.assume_init() };
        assert_eq!(result.tag, 1);
        let error = unsafe {
            &*(&result.payload.error as *const ManuallyDrop<FsErrorTaken> as *const FsErrorTaken)
        };
        assert_eq!(unsafe { error.code.as_str() }, "ERR_FS_STAT_FAILED");
    }

    #[test]
    fn asynchronous_file_failures_fulfill_promises_with_err_results() {
        let missing = std::env::temp_dir()
            .join(format!("tyrus-fs-async-missing-{}", uuid::Uuid::new_v4()))
            .join("value.txt")
            .to_string_lossy()
            .into_owned();
        let cases = [
            (
                unsafe { tyrus_fs_read_file(TyrusString::lend(&missing).unwrap()) },
                "ERR_FS_READ_FAILED",
            ),
            (
                unsafe {
                    tyrus_fs_write_file(
                        TyrusString::lend(&missing).unwrap(),
                        TyrusString::lend("value").unwrap(),
                    )
                },
                "ERR_FS_WRITE_FAILED",
            ),
            (
                unsafe {
                    tyrus_fs_append_file(
                        TyrusString::lend(&missing).unwrap(),
                        TyrusString::lend("value").unwrap(),
                    )
                },
                "ERR_FS_APPEND_FAILED",
            ),
        ];

        for (promise, expected_code) in cases {
            assert_eq!(
                unsafe { crate::promise::wait_until_settled(promise) },
                crate::promise::TYRUS_PROMISE_FULFILLED
            );
            let result = unsafe { crate::tyrus_promise_await(promise).cast::<FsSyncResult>() };
            assert!(!result.is_null());
            assert_eq!(unsafe { (*result).tag }, 1);
            let error = unsafe { &*(&(*result).payload.error as *const _ as *const FsErrorTaken) };
            assert_eq!(unsafe { error.code.as_str() }, expected_code);
            unsafe { crate::tyrus_promise_free(promise) };
        }
    }
}

#[cfg(test)]
mod stream_tests {
    use super::*;

    #[tokio::test]
    async fn file_streams_copy_binary_data_and_wait_for_flush() {
        let root = std::env::temp_dir().join(format!("tyrus-fs-stream-{}", uuid::Uuid::new_v4()));
        fs::create_dir_all(&root).unwrap();
        let source_path = root.join("source.bin");
        let destination_path = root.join("destination.bin");
        let input = (0..(1024 * 1024 + 137))
            .map(|index| ((index * 31) % 256) as u8)
            .collect::<Vec<_>>();
        fs::write(&source_path, &input).unwrap();

        let source_text = source_path.to_string_lossy();
        let source = unsafe {
            tyrus_fs_read_file_stream(TyrusString::lend(&source_text).expect("valid source path"))
        };
        assert!(!source.is_null());
        let mut copied = Vec::new();
        while let Some(chunk) = unsafe { crate::stream::read_byte_stream(source).await.unwrap() } {
            copied.extend_from_slice(&chunk);
        }
        unsafe { crate::stream::tyrus_readable_stream_free(source) };
        assert_eq!(copied, input);

        let destination_text = destination_path.to_string_lossy();
        let destination = unsafe {
            tyrus_fs_write_file_stream(
                TyrusString::lend(&destination_text).expect("valid destination path"),
            )
        };
        assert!(!destination.is_null());
        for chunk in input.chunks(17_003) {
            unsafe {
                crate::stream::write_byte_stream(destination, chunk.to_vec())
                    .await
                    .unwrap();
            };
        }
        unsafe {
            crate::stream::close_byte_writable_stream(destination)
                .await
                .unwrap();
        };
        unsafe { crate::stream::tyrus_writable_stream_free(destination) };
        assert_eq!(fs::read(&destination_path).unwrap(), input);

        let piped_path = root.join("piped.bin");
        let piped_text = piped_path.to_string_lossy();
        let source = unsafe {
            tyrus_fs_read_file_stream(TyrusString::lend(&source_text).expect("valid source path"))
        };
        let destination = unsafe {
            tyrus_fs_write_file_stream(TyrusString::lend(&piped_text).expect("valid piped path"))
        };
        let pipe = unsafe { crate::stream::tyrus_readable_stream_pipe_to(source, destination) };
        unsafe {
            assert_eq!(crate::promise::wait_until_settled(pipe), 1);
            crate::tyrus_promise_free(pipe);
            crate::stream::tyrus_readable_stream_free(source);
            crate::stream::tyrus_writable_stream_free(destination);
        }
        assert_eq!(fs::read(piped_path).unwrap(), input);
        fs::remove_dir_all(root).unwrap();
    }

    #[cfg(target_os = "linux")]
    fn open_file_descriptors_for(path: &std::path::Path) -> usize {
        fs::read_dir("/proc/self/fd")
            .unwrap()
            .filter_map(Result::ok)
            .filter(|entry| fs::read_link(entry.path()).is_ok_and(|target| target == path))
            .count()
    }

    #[cfg(target_os = "linux")]
    #[tokio::test]
    async fn dropping_a_file_readable_releases_its_owned_descriptor() {
        let root =
            std::env::temp_dir().join(format!("tyrus-fs-stream-cancel-{}", uuid::Uuid::new_v4()));
        fs::create_dir_all(&root).unwrap();
        let source_path = root.join("source.bin");
        fs::write(&source_path, vec![19_u8; 32 * 1024 * 1024]).unwrap();
        let source_text = source_path.to_string_lossy();
        let source = unsafe {
            tyrus_fs_read_file_stream(TyrusString::lend(&source_text).expect("valid source path"))
        };
        assert!(!source.is_null());
        assert_eq!(open_file_descriptors_for(&source_path), 1);
        unsafe { crate::stream::tyrus_readable_stream_free(source) };

        tokio::time::timeout(std::time::Duration::from_secs(2), async {
            while open_file_descriptors_for(&source_path) != 0 {
                tokio::task::yield_now().await;
            }
        })
        .await
        .expect("dropping the readable stream must close its owned file descriptor");
        fs::remove_dir_all(root).unwrap();
    }

    #[tokio::test]
    async fn dropping_a_file_writable_stream_closes_and_flushes_it() {
        let root =
            std::env::temp_dir().join(format!("tyrus-fs-stream-drop-{}", uuid::Uuid::new_v4()));
        fs::create_dir_all(&root).unwrap();
        let destination_path = root.join("dropped.bin");
        let expected = vec![0, 255, 17, 42, 0, 99];
        let destination_text = destination_path.to_string_lossy();
        let destination = unsafe {
            tyrus_fs_write_file_stream(
                TyrusString::lend(&destination_text).expect("valid destination path"),
            )
        };
        assert!(!destination.is_null());
        unsafe {
            crate::stream::write_byte_stream(destination, expected.clone())
                .await
                .unwrap();
            crate::stream::tyrus_writable_stream_free(destination);
        }

        tokio::time::timeout(std::time::Duration::from_secs(1), async {
            loop {
                if fs::read(&destination_path).ok().as_deref() == Some(expected.as_slice()) {
                    return;
                }
                tokio::task::yield_now().await;
            }
        })
        .await
        .expect("dropping the writable stream must flush its accepted bytes");
        fs::remove_dir_all(root).unwrap();
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn a_full_device_preserves_the_stream_failure_through_close() {
        let path =
            unsafe { TyrusString::lend("/dev/full") }.expect("the Unix full device path is valid");
        let destination = unsafe { tyrus_fs_write_file_stream(path) };
        assert!(!destination.is_null());

        unsafe {
            crate::stream::write_byte_stream(destination, vec![7; 4096])
                .await
                .unwrap();
        }
        let error = unsafe { crate::stream::close_byte_writable_stream(destination).await }
            .expect_err("closing /dev/full must preserve the failed write or flush");
        assert_eq!(error.code, "ERR_STREAM_DESTINATION_FAILED");
        unsafe { crate::stream::tyrus_writable_stream_free(destination) };
    }
}

#[cfg(test)]
mod glob_path_tests {
    use super::glob_path_match;

    #[test]
    fn double_star_spans_any_number_of_directories() {
        assert!(glob_path_match("**/*.map", "app.js.map"));
        assert!(glob_path_match("**/*.map", "dist/js/app.js.map"));
        assert!(!glob_path_match("**/*.map", "dist/js/app.js"));
        assert!(glob_path_match("node_modules", "node_modules"));
        assert!(!glob_path_match("node_modules", "a/node_modules"));
        assert!(glob_path_match("**/node_modules", "a/node_modules"));
        assert!(glob_path_match("src/*.ts", "src/main.ts"));
        assert!(!glob_path_match("src/*.ts", "src/nested/main.ts"));
    }
}

#[cfg(all(test, unix))]
mod disk_usage_tests {
    use super::*;

    fn usage(path: &str) -> (FsDiskUsage, Option<String>) {
        let mut out = FsDiskUsage {
            total: 0,
            used: 0,
            free: 0,
        };
        unsafe { tyrus_fs_disk_usage(TyrusString::lend(path).unwrap(), &mut out) };
        let code = LAST_FS_ERROR.with(|slot| slot.take()).map(|(code, _)| {
            code.to_str()
                .expect("a static error literal is UTF-8")
                .to_owned()
        });
        (out, code)
    }

    #[test]
    fn free_is_what_an_unprivileged_process_can_still_write() {
        let directory = std::env::temp_dir();
        let path = directory.to_string_lossy().into_owned();
        let (out, code) = usage(&path);
        assert_eq!(code, None);
        assert!(out.total > 0);
        assert!(out.used.saturating_add(out.free) <= out.total);

        let c_path = CString::new(path).unwrap();
        let mut stats: libc::statvfs = unsafe { std::mem::zeroed() };
        assert_eq!(unsafe { libc::statvfs(c_path.as_ptr(), &mut stats) }, 0);
        #[allow(clippy::useless_conversion)]
        let available = u64::from(stats.f_bavail) * u64::from(stats.f_frsize);
        // Other processes write to the same file system between the two
        // calls; a few MiB of drift is noise, reserved blocks are not.
        assert!(out.free.abs_diff(available) < 64 * 1024 * 1024);
    }

    #[test]
    fn a_path_with_a_nul_byte_is_an_error_not_a_panic() {
        let (_, code) = usage("/tmp/a\0b/c");
        assert_eq!(code.as_deref(), Some("ERR_FS_INVALID_PATH"));
    }
}

#[cfg(all(test, windows))]
mod windows_disk_usage_tests {
    use super::*;

    #[test]
    fn disk_usage_reads_a_unicode_directory() {
        let directory =
            std::env::temp_dir().join(format!("tyrus-disk-usage-東京-{}", uuid::Uuid::new_v4()));
        fs::create_dir_all(&directory).unwrap();
        let path = directory.to_string_lossy().into_owned();
        let mut usage = FsDiskUsage {
            total: 0,
            used: 0,
            free: 0,
        };

        unsafe { tyrus_fs_disk_usage(TyrusString::lend(&path).unwrap(), &mut usage) };
        let error = LAST_FS_ERROR.with(|slot| slot.take());

        assert!(error.is_none());
        assert!(usage.total > 0);
        assert!(usage.used.saturating_add(usage.free) <= usage.total);
        fs::remove_dir_all(directory).unwrap();
    }
}
