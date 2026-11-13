use super::*;

#[test]
pub(super) fn emits_native_file_operations_for_node_compatible_fs_methods() {
    let source = r#"
        __tyrus_fs_writeFileSync("message.txt", "hello");
        const message: string = __tyrus_fs_readFileSync("message.txt");
        console.log(message);
    "#;
    let tokens = Token::lexer(source).filter_map(Result::ok).collect();
    let program = Parser::new(tokens).parse_program().unwrap();
    let mut checker = BorrowChecker::new();
    assert_eq!(checker.check_program(&program), Vec::<String>::new());

    let output = std::env::temp_dir().join(format!("tyrus-fs-test-{}", std::process::id()));
    compile(&program, "fs_test", output.to_str().unwrap(), false).unwrap();
    let ir_path = output.with_extension("ll");
    let ir = read_ir(&ir_path).unwrap();
    assert!(ir.contains("@tyrus_fs_read_file_sync_result"));
    assert!(ir.contains("@tyrus_fs_write_file_sync_result"));
    std::fs::remove_file(ir_path).unwrap();
}

#[test]
pub(super) fn fs_read_file_sync_resolves_the_path_before_opening_and_frees_it_after() {
    let source = r#"
        __tyrus_fs_readFileSync("message.txt");
    "#;
    let tokens = Token::lexer(source).filter_map(Result::ok).collect();
    let program = Parser::new(tokens).parse_program().unwrap();
    let output = std::env::temp_dir().join(format!("tyrus-fs-resolve-read-{}", std::process::id()));
    compile(&program, "fs_resolve_read", output.to_str().unwrap(), false).unwrap();
    let ir_path = output.with_extension("ll");
    let ir = read_ir(&ir_path).unwrap();
    assert!(
        ir.contains("call void @tyrus_fs_read_file_sync_result("),
        "expected the path to be resolved inside the runtime wrapper:\n{ir}"
    );
    std::fs::remove_file(ir_path).unwrap();
}

#[test]
pub(super) fn fs_read_file_sync_return_value_is_registered_as_owned() {
    // `readFileSync`'s returned buffer is a fresh `calloc`, but nothing
    // registered it for cleanup, a leak on every call. Asserted here at the
    // IR level; the actual leak is guarded by `tests/leaks/clean/`.
    let source = r#"
        function main() {
            const message: string = __tyrus_fs_readFileSync("message.txt");
            console.log(message);
        }
    "#;
    let tokens = Token::lexer(source).filter_map(Result::ok).collect();
    let program = Parser::new(tokens).parse_program().unwrap();
    let output = std::env::temp_dir().join(format!("tyrus-fs-owned-{}", std::process::id()));
    compile(&program, "fs_owned", output.to_str().unwrap(), false).unwrap();
    let ir_path = output.with_extension("ll");
    let ir = read_ir(&ir_path).unwrap();
    assert!(
        ir.contains(string_release_call()),
        "expected readFileSync's result to be released at scope exit:\n{ir}"
    );
    std::fs::remove_file(ir_path).unwrap();
}

#[test]
pub(super) fn fs_readdir_emits_call_to_tyrus_fs_readdir() {
    let source = r#"
        __tyrus_fs_readdir("./");
    "#;
    let tokens = Token::lexer(source).filter_map(Result::ok).collect();
    let program = Parser::new(tokens).parse_program().unwrap();
    let mut checker = BorrowChecker::new();
    assert_eq!(checker.check_program(&program), Vec::<String>::new());

    let output = std::env::temp_dir().join(format!("tyrus-fs-readdir-{}", std::process::id()));
    compile(&program, "fs_readdir", output.to_str().unwrap(), false).unwrap();
    let ir_path = output.with_extension("ll");
    let ir = read_ir(&ir_path).unwrap();
    assert!(
        ir.contains("call void @tyrus_fs_readdir_result("),
        "expected call to tyrus_fs_readdir_result:\n{ir}"
    );
    std::fs::remove_file(ir_path).unwrap();
}

#[test]
pub(super) fn fs_readdir_with_options_passes_options_pointer() {
    let source = r#"
        __tyrus_fs_readdir("./", { recursive: true });
    "#;
    let tokens = Token::lexer(source).filter_map(Result::ok).collect();
    let program = Parser::new(tokens).parse_program().unwrap();
    let mut checker = BorrowChecker::new();
    assert_eq!(checker.check_program(&program), Vec::<String>::new());

    let output = std::env::temp_dir().join(format!("tyrus-fs-readdir-opts-{}", std::process::id()));
    compile(&program, "fs_readdir_opts", output.to_str().unwrap(), false).unwrap();
    let ir_path = output.with_extension("ll");
    let ir = read_ir(&ir_path).unwrap();
    assert!(
        ir.contains("call void @tyrus_fs_readdir_result("),
        "expected call to tyrus_fs_readdir_result:\n{ir}"
    );
    std::fs::remove_file(ir_path).unwrap();
}

#[test]
pub(super) fn fs_mkdir_emits_call_to_tyrus_fs_mkdir_result() {
    let source = r#"
        __tyrus_fs_mkdir("./test-dir");
    "#;
    let tokens = Token::lexer(source).filter_map(Result::ok).collect();
    let program = Parser::new(tokens).parse_program().unwrap();
    let mut checker = BorrowChecker::new();
    assert_eq!(checker.check_program(&program), Vec::<String>::new());

    let output = std::env::temp_dir().join(format!("tyrus-fs-mkdir-{}", std::process::id()));
    compile(&program, "fs_mkdir", output.to_str().unwrap(), false).unwrap();
    let ir_path = output.with_extension("ll");
    let ir = read_ir(&ir_path).unwrap();
    assert!(
        ir.contains("call void @tyrus_fs_mkdir_result("),
        "expected call to tyrus_fs_mkdir_result:\n{ir}"
    );
    std::fs::remove_file(ir_path).unwrap();
}

#[test]
pub(super) fn fs_rm_emits_call_to_tyrus_fs_rm_result() {
    let source = r#"
        __tyrus_fs_rm("./test.txt");
    "#;
    let tokens = Token::lexer(source).filter_map(Result::ok).collect();
    let program = Parser::new(tokens).parse_program().unwrap();
    let mut checker = BorrowChecker::new();
    assert_eq!(checker.check_program(&program), Vec::<String>::new());

    let output = std::env::temp_dir().join(format!("tyrus-fs-rm-{}", std::process::id()));
    compile(&program, "fs_rm", output.to_str().unwrap(), false).unwrap();
    let ir_path = output.with_extension("ll");
    let ir = read_ir(&ir_path).unwrap();
    assert!(
        ir.contains("call void @tyrus_fs_rm_result("),
        "expected call to tyrus_fs_rm_result:\n{ir}"
    );
    std::fs::remove_file(ir_path).unwrap();
}

#[test]
pub(super) fn fs_rm_with_options_passes_options_pointer() {
    let source = r#"
        __tyrus_fs_rm("./test-dir", { recursive: true, force: true });
    "#;
    let tokens = Token::lexer(source).filter_map(Result::ok).collect();
    let program = Parser::new(tokens).parse_program().unwrap();
    let mut checker = BorrowChecker::new();
    assert_eq!(checker.check_program(&program), Vec::<String>::new());

    let output = std::env::temp_dir().join(format!("tyrus-fs-rm-opts-{}", std::process::id()));
    compile(&program, "fs_rm_opts", output.to_str().unwrap(), false).unwrap();
    let ir_path = output.with_extension("ll");
    let ir = read_ir(&ir_path).unwrap();
    assert!(
        ir.contains("call void @tyrus_fs_rm_result("),
        "expected call to tyrus_fs_rm_result:\n{ir}"
    );
    std::fs::remove_file(ir_path).unwrap();
}

#[test]
pub(super) fn fs_cp_emits_call_to_tyrus_fs_cp_result() {
    let source = r#"
        __tyrus_fs_cp("./a.txt", "./b.txt");
    "#;
    let tokens = Token::lexer(source).filter_map(Result::ok).collect();
    let program = Parser::new(tokens).parse_program().unwrap();
    let mut checker = BorrowChecker::new();
    assert_eq!(checker.check_program(&program), Vec::<String>::new());

    let output = std::env::temp_dir().join(format!("tyrus-fs-cp-{}", std::process::id()));
    compile(&program, "fs_cp", output.to_str().unwrap(), false).unwrap();
    let ir_path = output.with_extension("ll");
    let ir = read_ir(&ir_path).unwrap();
    assert!(
        ir.contains("call void @tyrus_fs_cp_result("),
        "expected call to tyrus_fs_cp_result:\n{ir}"
    );
    std::fs::remove_file(ir_path).unwrap();
}

#[test]
pub(super) fn fs_mv_emits_call_to_tyrus_fs_mv_result() {
    let source = r#"
        __tyrus_fs_mv("./a.txt", "./b.txt");
    "#;
    let tokens = Token::lexer(source).filter_map(Result::ok).collect();
    let program = Parser::new(tokens).parse_program().unwrap();
    let mut checker = BorrowChecker::new();
    assert_eq!(checker.check_program(&program), Vec::<String>::new());

    let output = std::env::temp_dir().join(format!("tyrus-fs-mv-{}", std::process::id()));
    compile(&program, "fs_mv", output.to_str().unwrap(), false).unwrap();
    let ir_path = output.with_extension("ll");
    let ir = read_ir(&ir_path).unwrap();
    assert!(
        ir.contains("call void @tyrus_fs_mv_result("),
        "expected call to tyrus_fs_mv_result:\n{ir}"
    );
    std::fs::remove_file(ir_path).unwrap();
}

#[test]
pub(super) fn fs_remove_all_emits_call_to_tyrus_fs_remove_all_result() {
    let source = r#"
        __tyrus_fs_removeAll("./test-dir");
    "#;
    let tokens = Token::lexer(source).filter_map(Result::ok).collect();
    let program = Parser::new(tokens).parse_program().unwrap();
    let mut checker = BorrowChecker::new();
    assert_eq!(checker.check_program(&program), Vec::<String>::new());

    let output = std::env::temp_dir().join(format!("tyrus-fs-removeAll-{}", std::process::id()));
    compile(&program, "fs_remove_all", output.to_str().unwrap(), false).unwrap();
    let ir_path = output.with_extension("ll");
    let ir = read_ir(&ir_path).unwrap();
    assert!(
        ir.contains("call void @tyrus_fs_remove_all_result("),
        "expected call to tyrus_fs_remove_all_result:\n{ir}"
    );
    std::fs::remove_file(ir_path).unwrap();
}

#[test]
pub(super) fn fs_stat_emits_call_to_tyrus_fs_stat_result() {
    let source = r#"
        __tyrus_fs_stat("./message.txt");
    "#;
    let tokens = Token::lexer(source).filter_map(Result::ok).collect();
    let program = Parser::new(tokens).parse_program().unwrap();
    let mut checker = BorrowChecker::new();
    assert_eq!(checker.check_program(&program), Vec::<String>::new());

    let output = std::env::temp_dir().join(format!("tyrus-fs-stat-{}", std::process::id()));
    compile(&program, "fs_stat", output.to_str().unwrap(), false).unwrap();
    let ir_path = output.with_extension("ll");
    let ir = read_ir(&ir_path).unwrap();
    assert!(
        ir.contains("call void @tyrus_fs_stat_result("),
        "expected call to tyrus_fs_stat_result:\n{ir}"
    );
    std::fs::remove_file(ir_path).unwrap();
}

#[test]
pub(super) fn fs_metadata_emits_call_to_tyrus_fs_metadata() {
    let source = r#"
        __tyrus_fs_metadata("./message.txt");
    "#;
    let tokens = Token::lexer(source).filter_map(Result::ok).collect();
    let program = Parser::new(tokens).parse_program().unwrap();
    let mut checker = BorrowChecker::new();
    assert_eq!(checker.check_program(&program), Vec::<String>::new());

    let output = std::env::temp_dir().join(format!("tyrus-fs-metadata-{}", std::process::id()));
    compile(&program, "fs_metadata", output.to_str().unwrap(), false).unwrap();
    let ir_path = output.with_extension("ll");
    let ir = read_ir(&ir_path).unwrap();
    assert!(
        ir.contains("call void @tyrus_fs_metadata_result("),
        "expected call to tyrus_fs_metadata_result:\n{ir}"
    );
    std::fs::remove_file(ir_path).unwrap();
}

#[test]
pub(super) fn fs_disk_usage_emits_call_to_tyrus_fs_disk_usage() {
    let source = r#"
        __tyrus_fs_disk_usage("./");
    "#;
    let tokens = Token::lexer(source).filter_map(Result::ok).collect();
    let program = Parser::new(tokens).parse_program().unwrap();
    let mut checker = BorrowChecker::new();
    assert_eq!(checker.check_program(&program), Vec::<String>::new());

    let output = std::env::temp_dir().join(format!("tyrus-fs-disk-usage-{}", std::process::id()));
    compile(&program, "fs_disk_usage", output.to_str().unwrap(), false).unwrap();
    let ir_path = output.with_extension("ll");
    let ir = read_ir(&ir_path).unwrap();
    assert!(
        ir.contains("call void @tyrus_fs_disk_usage_result("),
        "expected call to tyrus_fs_disk_usage_result:\n{ir}"
    );
    std::fs::remove_file(ir_path).unwrap();
}

#[test]
pub(super) fn fs_mime_emits_call_to_tyrus_fs_mime() {
    let source = r#"
        __tyrus_fs_mime("./file.txt");
    "#;
    let tokens = Token::lexer(source).filter_map(Result::ok).collect();
    let program = Parser::new(tokens).parse_program().unwrap();
    let mut checker = BorrowChecker::new();
    assert_eq!(checker.check_program(&program), Vec::<String>::new());

    let output = std::env::temp_dir().join(format!("tyrus-fs-mime-{}", std::process::id()));
    compile(&program, "fs_mime", output.to_str().unwrap(), false).unwrap();
    let ir_path = output.with_extension("ll");
    let ir = read_ir(&ir_path).unwrap();
    assert!(
        ir.contains("call ptr @tyrus_fs_mime("),
        "expected call to tyrus_fs_mime:\n{ir}"
    );
    std::fs::remove_file(ir_path).unwrap();
}

#[test]
pub(super) fn fs_chmod_emits_call_to_tyrus_fs_chmod() {
    let source = r#"
        __tyrus_fs_chmod("./file.txt", 0);
    "#;
    let tokens = Token::lexer(source).filter_map(Result::ok).collect();
    let program = Parser::new(tokens).parse_program().unwrap();
    let mut checker = BorrowChecker::new();
    assert_eq!(checker.check_program(&program), Vec::<String>::new());

    let output = std::env::temp_dir().join(format!("tyrus-fs-chmod-{}", std::process::id()));
    compile(&program, "fs_chmod", output.to_str().unwrap(), false).unwrap();
    let ir_path = output.with_extension("ll");
    let ir = read_ir(&ir_path).unwrap();
    assert!(
        ir.contains("call void @tyrus_fs_chmod_result("),
        "expected call to tyrus_fs_chmod_result:\n{ir}"
    );
    std::fs::remove_file(ir_path).unwrap();
}

#[test]
pub(super) fn fs_chown_emits_call_to_tyrus_fs_chown() {
    let source = r#"
        __tyrus_fs_chown("./file.txt", 1000, 1000);
    "#;
    let tokens = Token::lexer(source).filter_map(Result::ok).collect();
    let program = Parser::new(tokens).parse_program().unwrap();
    let mut checker = BorrowChecker::new();
    assert_eq!(checker.check_program(&program), Vec::<String>::new());

    let output = std::env::temp_dir().join(format!("tyrus-fs-chown-{}", std::process::id()));
    compile(&program, "fs_chown", output.to_str().unwrap(), false).unwrap();
    let ir_path = output.with_extension("ll");
    let ir = read_ir(&ir_path).unwrap();
    assert!(
        ir.contains("call void @tyrus_fs_chown_result("),
        "expected call to tyrus_fs_chown_result:\n{ir}"
    );
    std::fs::remove_file(ir_path).unwrap();
}

#[test]
pub(super) fn fs_readlink_emits_call_to_tyrus_fs_readlink_result() {
    let source = r#"
        __tyrus_fs_readlink("./file.txt");
    "#;
    let tokens = Token::lexer(source).filter_map(Result::ok).collect();
    let program = Parser::new(tokens).parse_program().unwrap();
    let mut checker = BorrowChecker::new();
    assert_eq!(checker.check_program(&program), Vec::<String>::new());

    let output = std::env::temp_dir().join(format!("tyrus-fs-readlink-{}", std::process::id()));
    compile(&program, "fs_readlink", output.to_str().unwrap(), false).unwrap();
    let ir_path = output.with_extension("ll");
    let ir = read_ir(&ir_path).unwrap();
    assert!(
        ir.contains("call void @tyrus_fs_readlink_result("),
        "expected call to tyrus_fs_readlink_result:\n{ir}"
    );
    std::fs::remove_file(ir_path).unwrap();
}

#[test]
pub(super) fn fs_symlink_emits_call_to_tyrus_fs_symlink() {
    let source = r#"
        __tyrus_fs_symlink("./target.txt", "./link.txt");
    "#;
    let tokens = Token::lexer(source).filter_map(Result::ok).collect();
    let program = Parser::new(tokens).parse_program().unwrap();
    let mut checker = BorrowChecker::new();
    assert_eq!(checker.check_program(&program), Vec::<String>::new());

    let output = std::env::temp_dir().join(format!("tyrus-fs-symlink-{}", std::process::id()));
    compile(&program, "fs_symlink", output.to_str().unwrap(), false).unwrap();
    let ir_path = output.with_extension("ll");
    let ir = read_ir(&ir_path).unwrap();
    assert!(
        ir.contains("call void @tyrus_fs_symlink_result("),
        "expected call to tyrus_fs_symlink_result:\n{ir}"
    );
    std::fs::remove_file(ir_path).unwrap();
}

#[test]
pub(super) fn fs_hard_link_emits_call_to_tyrus_fs_hard_link() {
    let source = r#"
        __tyrus_fs_hard_link("./target.txt", "./link.txt");
    "#;
    let tokens = Token::lexer(source).filter_map(Result::ok).collect();
    let program = Parser::new(tokens).parse_program().unwrap();
    let mut checker = BorrowChecker::new();
    assert_eq!(checker.check_program(&program), Vec::<String>::new());

    let output = std::env::temp_dir().join(format!("tyrus-fs-hard-link-{}", std::process::id()));
    compile(&program, "fs_hard_link", output.to_str().unwrap(), false).unwrap();
    let ir_path = output.with_extension("ll");
    let ir = read_ir(&ir_path).unwrap();
    assert!(
        ir.contains("call void @tyrus_fs_hard_link_result("),
        "expected call to tyrus_fs_hard_link_result:\n{ir}"
    );
    std::fs::remove_file(ir_path).unwrap();
}

#[test]
pub(super) fn fs_canonicalize_emits_call_to_tyrus_fs_canonicalize() {
    let source = r#"
        __tyrus_fs_canonicalize("./file.txt");
    "#;
    let tokens = Token::lexer(source).filter_map(Result::ok).collect();
    let program = Parser::new(tokens).parse_program().unwrap();
    let mut checker = BorrowChecker::new();
    assert_eq!(checker.check_program(&program), Vec::<String>::new());

    let output = std::env::temp_dir().join(format!("tyrus-fs-canonicalize-{}", std::process::id()));
    compile(&program, "fs_canonicalize", output.to_str().unwrap(), false).unwrap();
    let ir_path = output.with_extension("ll");
    let ir = read_ir(&ir_path).unwrap();
    assert!(
        ir.contains("call void @tyrus_fs_canonicalize_result("),
        "expected call to tyrus_fs_canonicalize_result:\n{ir}"
    );
    std::fs::remove_file(ir_path).unwrap();
}

#[test]
pub(super) fn fs_glob_emits_call_to_tyrus_fs_glob_result() {
    let source = r#"
        __tyrus_fs_glob("*.txt");
    "#;
    let tokens = Token::lexer(source).filter_map(Result::ok).collect();
    let program = Parser::new(tokens).parse_program().unwrap();
    let mut checker = BorrowChecker::new();
    assert_eq!(checker.check_program(&program), Vec::<String>::new());

    let output = std::env::temp_dir().join(format!("tyrus-fs-glob-{}", std::process::id()));
    compile(&program, "fs_glob", output.to_str().unwrap(), false).unwrap();
    let ir_path = output.with_extension("ll");
    let ir = read_ir(&ir_path).unwrap();
    assert!(
        ir.contains("call void @tyrus_fs_glob_result("),
        "expected call to tyrus_fs_glob_result:\n{ir}"
    );
    std::fs::remove_file(ir_path).unwrap();
}

#[test]
pub(super) fn fs_search_emits_call_to_tyrus_fs_search() {
    let source = r#"
        __tyrus_fs_search("*.txt");
    "#;
    let tokens = Token::lexer(source).filter_map(Result::ok).collect();
    let program = Parser::new(tokens).parse_program().unwrap();
    let mut checker = BorrowChecker::new();
    assert_eq!(checker.check_program(&program), Vec::<String>::new());

    let output = std::env::temp_dir().join(format!("tyrus-fs-search-{}", std::process::id()));
    compile(&program, "fs_search", output.to_str().unwrap(), false).unwrap();
    let ir_path = output.with_extension("ll");
    let ir = read_ir(&ir_path).unwrap();
    assert!(
        ir.contains("call void @tyrus_fs_search_result("),
        "expected call to tyrus_fs_search_result:\n{ir}"
    );
    std::fs::remove_file(ir_path).unwrap();
}

#[test]
pub(super) fn fs_diff_emits_call_to_tyrus_fs_diff() {
    let source = r#"
        __tyrus_fs_diff("./a.txt", "./b.txt");
    "#;
    let tokens = Token::lexer(source).filter_map(Result::ok).collect();
    let program = Parser::new(tokens).parse_program().unwrap();
    let mut checker = BorrowChecker::new();
    assert_eq!(checker.check_program(&program), Vec::<String>::new());

    let output = std::env::temp_dir().join(format!("tyrus-fs-diff-{}", std::process::id()));
    compile(&program, "fs_diff", output.to_str().unwrap(), false).unwrap();
    let ir_path = output.with_extension("ll");
    let ir = read_ir(&ir_path).unwrap();
    assert!(
        ir.contains("call void @tyrus_fs_diff_result("),
        "expected call to tyrus_fs_diff_result:\n{ir}"
    );
    std::fs::remove_file(ir_path).unwrap();
}

#[test]
pub(super) fn fs_atomic_write_emits_call_to_tyrus_fs_atomic_write() {
    let source = r#"
        __tyrus_fs_atomic_write("./file.txt", "hello");
    "#;
    let tokens = Token::lexer(source).filter_map(Result::ok).collect();
    let program = Parser::new(tokens).parse_program().unwrap();
    let mut checker = BorrowChecker::new();
    assert_eq!(checker.check_program(&program), Vec::<String>::new());

    let output = std::env::temp_dir().join(format!("tyrus-fs-atomic-write-{}", std::process::id()));
    compile(&program, "fs_atomic_write", output.to_str().unwrap(), false).unwrap();
    let ir_path = output.with_extension("ll");
    let ir = read_ir(&ir_path).unwrap();
    assert!(
        ir.contains("call void @tyrus_fs_atomic_write_result("),
        "expected call to tyrus_fs_atomic_write_result:\n{ir}"
    );
    std::fs::remove_file(ir_path).unwrap();
}

#[test]
pub(super) fn fs_temp_dir_emits_call_to_tyrus_fs_temp_dir() {
    let source = r#"
        __tyrus_fs_temp_dir();
    "#;
    let tokens = Token::lexer(source).filter_map(Result::ok).collect();
    let program = Parser::new(tokens).parse_program().unwrap();
    let mut checker = BorrowChecker::new();
    assert_eq!(checker.check_program(&program), Vec::<String>::new());

    let output = std::env::temp_dir().join(format!("tyrus-fs-temp-dir-{}", std::process::id()));
    compile(&program, "fs_temp_dir", output.to_str().unwrap(), false).unwrap();
    let ir_path = output.with_extension("ll");
    let ir = read_ir(&ir_path).unwrap();
    assert!(
        ir.contains("call void @tyrus_fs_temp_dir("),
        "expected call to tyrus_fs_temp_dir:\n{ir}"
    );
    std::fs::remove_file(ir_path).unwrap();
}

#[test]
pub(super) fn fs_temp_file_emits_call_to_tyrus_fs_temp_file() {
    let source = r#"
        __tyrus_fs_temp_file();
    "#;
    let tokens = Token::lexer(source).filter_map(Result::ok).collect();
    let program = Parser::new(tokens).parse_program().unwrap();
    let mut checker = BorrowChecker::new();
    assert_eq!(checker.check_program(&program), Vec::<String>::new());

    let output = std::env::temp_dir().join(format!("tyrus-fs-temp-file-{}", std::process::id()));
    compile(&program, "fs_temp_file", output.to_str().unwrap(), false).unwrap();
    let ir_path = output.with_extension("ll");
    let ir = read_ir(&ir_path).unwrap();
    assert!(
        ir.contains("call void @tyrus_fs_temp_file_result("),
        "expected call to tyrus_fs_temp_file_result:\n{ir}"
    );
    std::fs::remove_file(ir_path).unwrap();
}

#[test]
pub(super) fn fs_watch_emits_call_to_tyrus_fs_watch() {
    let source = r#"
        __tyrus_fs_watch("./tmp");
    "#;
    let tokens = Token::lexer(source).filter_map(Result::ok).collect();
    let program = Parser::new(tokens).parse_program().unwrap();
    let mut checker = BorrowChecker::new();
    assert_eq!(checker.check_program(&program), Vec::<String>::new());

    let output = std::env::temp_dir().join(format!("tyrus-fs-watch-{}", std::process::id()));
    compile(&program, "fs_watch", output.to_str().unwrap(), false).unwrap();
    let ir_path = output.with_extension("ll");
    let ir = read_ir(&ir_path).unwrap();
    assert!(
        ir.contains("call void @tyrus_fs_watch_result("),
        "expected call to tyrus_fs_watch_result:\n{ir}"
    );
    std::fs::remove_file(ir_path).unwrap();
}
