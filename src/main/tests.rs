fn strings(args: &[&str]) -> Vec<String> {
    args.iter().map(std::string::ToString::to_string).collect()
}

#[test]
fn test_options_take_one_path() {
    let options = super::test_options_from(&strings(&["tests/a.test.tyr"])).unwrap();
    assert_eq!(
        options,
        super::TestOptions {
            entry: Some(std::path::PathBuf::from("tests/a.test.tyr")),
        }
    );
    assert_eq!(
        super::test_options_from(&[]).unwrap(),
        super::TestOptions::default()
    );
}

#[test]
fn test_options_reject_malformed_input() {
    for flag in [
        "--nope",
        "--release",
        "--filter",
        "--bail",
        "--ci",
        "--color",
        "--jobs",
    ] {
        let unknown = super::test_options_from(&strings(&[flag])).unwrap_err();
        assert!(
            unknown.contains(&format!("unknown option `{flag}`")),
            "{unknown}"
        );
    }
    let two = super::test_options_from(&strings(&["a.test.tyr", "b.test.tyr"])).unwrap_err();
    assert!(two.contains("`b.test.tyr` is the second"), "{two}");
}

#[test]
fn test_report_parses_only_the_exact_shape() {
    assert_eq!(
        super::parse_test_report("3 1 2 4\n"),
        Some(super::TestCounts {
            passed: 3,
            failed: 1,
            skipped: 2,
            todo: 4,
        })
    );
    assert_eq!(super::parse_test_report("3 1 2"), None);
    assert_eq!(super::parse_test_report("3 1 2 4 5"), None);
    assert_eq!(super::parse_test_report("3 x 2 4"), None);
    assert_eq!(super::parse_test_report(""), None);
}

#[test]
fn test_summary_colors_only_nonzero_status_counts() {
    let counts = super::TestCounts {
        passed: 21,
        failed: 1,
        skipped: 1,
        todo: 1,
    };
    assert_eq!(
        super::format_test_counts(counts, false),
        "Tests: 21 passed, 1 failed, 1 skipped, 1 todo"
    );
    assert_eq!(
        super::format_test_counts(counts, true),
        "Tests: \u{1b}[1;32m21\u{1b}[0m passed, \u{1b}[1;31m1\u{1b}[0m failed, \u{1b}[1;33m1\u{1b}[0m skipped, \u{1b}[1;90m1\u{1b}[0m todo"
    );
    assert_eq!(
        super::format_test_count(0, "failed", "31", true),
        "0 failed"
    );
}

#[test]
fn test_discovery_finds_test_files_anywhere_and_skips_build_output() {
    let root = std::env::temp_dir().join(format!("tyrus-discovery-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    for dir in [
        "__tests__/nested",
        "src",
        "lib/deep",
        "target/debug",
        ".cache",
    ] {
        std::fs::create_dir_all(root.join(dir)).unwrap();
    }
    for file in [
        "__tests__/nested/b.test.tyr",
        "__tests__/a.test.tyr",
        "src/c.test.tyr",
        "src/main.tyr",
        "src/test.tyr",
        "lib/deep/d.test.tyr",
        "target/debug/skipped.test.tyr",
        ".cache/skipped.test.tyr",
    ] {
        std::fs::write(root.join(file), "").unwrap();
    }
    let found: Vec<String> = super::discover_test_files(&root)
        .into_iter()
        .map(|path| {
            path.strip_prefix(&root)
                .unwrap()
                .to_string_lossy()
                .into_owned()
        })
        .collect();
    assert_eq!(
        found,
        [
            "__tests__/a.test.tyr",
            "__tests__/nested/b.test.tyr",
            "lib/deep/d.test.tyr",
            "src/c.test.tyr",
        ]
    );
    assert!(tyrus_core::test::is_test_file(std::path::Path::new(
        "x/main.test.tyr"
    )));
    assert!(!tyrus_core::test::is_test_file(std::path::Path::new(
        "x/test.tyr"
    )));
    assert!(!tyrus_core::test::is_test_file(std::path::Path::new(
        "x/main.test.tyr.bak"
    )));
    std::fs::remove_dir_all(&root).unwrap();
}

#[test]
fn pgo_flags_are_steps_of_one_workflow() {
    let generate = ["--release".to_string(), "--pgo-generate".to_string()];
    assert_eq!(
        pgo_mode_from(&generate, true).unwrap(),
        compile::PgoMode::Generate
    );

    let use_profile = ["--release".to_string(), "--pgo-use".to_string()];
    assert_eq!(
        pgo_mode_from(&use_profile, true).unwrap(),
        compile::PgoMode::Use
    );

    let neither = ["--release".to_string()];
    assert_eq!(
        pgo_mode_from(&neither, true).unwrap(),
        compile::PgoMode::Off
    );

    let both = ["--pgo-generate".to_string(), "--pgo-use".to_string()];
    assert!(
        pgo_mode_from(&both, true)
            .unwrap_err()
            .contains("one at a time"),
        "asking for both steps at once must be refused by name"
    );
}

#[test]
fn pgo_requires_release() {
    for flag in ["--pgo-generate", "--pgo-use"] {
        let args = [flag.to_string()];
        let message = pgo_mode_from(&args, false).unwrap_err();
        assert!(
            message.contains("--release"),
            "expected the reason to name `--release`, got {message:?}"
        );
    }
}
use super::*;

#[test]
fn an_unoptimized_runtime_is_recognized_by_its_own_directory() {
    assert!(runtime_library_is_unoptimized(Path::new(
        "/w/target/debug/libtyrus_runtime.a"
    )));
    assert!(!runtime_library_is_unoptimized(Path::new(
        "/w/target/release/libtyrus_runtime.a"
    )));
    // An installed compiler's staticlib sits in neither profile directory.
    assert!(!runtime_library_is_unoptimized(Path::new(
        "/home/user/.local/lib/tyrus/libtyrus_runtime.a"
    )));
    assert!(!runtime_library_is_unoptimized(Path::new(
        "/home/debug/target/release/libtyrus_runtime.a"
    )));
}

#[test]
fn optimized_native_runtime_is_selected_for_release_builds() {
    let executable_dir = Path::new("/home/user/.cargo/bin");

    assert_eq!(
        runtime_library_candidates(executable_dir, false),
        vec![
            PathBuf::from("/home/user/.cargo/bin/libtyrus_runtime.a"),
            PathBuf::from("/home/user/.cargo/bin/../lib/tyrus/libtyrus_runtime.a"),
        ]
    );
    assert_eq!(
        runtime_library_candidates(executable_dir, true),
        vec![
            PathBuf::from("/home/user/.cargo/release/libtyrus_runtime.a"),
            PathBuf::from("/home/user/.cargo/bin/libtyrus_runtime.a"),
            PathBuf::from("/home/user/.cargo/bin/../lib/tyrus/libtyrus_runtime.a"),
        ]
    );
}

#[test]
fn target_parses_known_values_and_round_trips() {
    assert!(matches!(Target::parse("script"), Some(Target::Script)));
    assert!(matches!(Target::parse("cli"), Some(Target::Cli)));
    assert!(matches!(Target::parse("api"), Some(Target::Api)));
    assert!(Target::parse("web").is_none());
    assert_eq!(Target::Cli.as_str(), "cli");
}

#[test]
fn run_args_accept_a_standalone_entry_and_forward_program_args() {
    let args = ["example.tyr", "--release", "--", "--verbose", "report.txt"].map(String::from);

    let (entry, release, rotate_seal_key, forwarded) = parse_run_args(&args).unwrap();
    assert_eq!(entry, Some(PathBuf::from("example.tyr")));
    assert!(release);
    assert!(!rotate_seal_key);
    assert_eq!(forwarded, vec!["--verbose", "report.txt"]);
}

#[test]
fn run_args_reject_multiple_standalone_entries() {
    let args = ["first.tyr", "second.tyr"].map(String::from);
    assert_eq!(
        parse_run_args(&args),
        Err("only one standalone entry file may be specified".to_string())
    );
}

#[test]
fn project_name_rejects_leading_digit_and_invalid_chars() {
    assert!(validate_project_name("my-app").is_ok());
    assert!(validate_project_name("app5").is_ok());
    assert!(validate_project_name("app_2").is_ok());
    assert!(validate_project_name("123app").is_err());
    assert!(validate_project_name("5").is_err());
    assert!(validate_project_name("").is_err());
    assert!(validate_project_name("has space").is_err());
    assert!(validate_project_name("with/slash").is_err());
}

#[test]
fn init_creates_project_config_with_defaults() {
    let _guard = crate::compile::acquire_jit_lock();
    let root = std::env::temp_dir().join(format!("tyrus-init-{}", std::process::id()));
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(&root).unwrap();

    cmd_init(&root);

    let config = fs::read_to_string(root.join("tyrus.json")).unwrap();
    let value: serde_json::Value = serde_json::from_str(&config).unwrap();
    assert_eq!(
        value["project"]["name"],
        root.file_name().unwrap().to_str().unwrap()
    );
    assert_eq!(value["security"]["envFile"], ".env");
    assert_eq!(value["security"]["envSealed"], false);
    assert_eq!(value["imports"]["#"], "./src");
    assert_eq!(value["scripts"]["test"], "tyrus test");
    assert!(value.get("dependencies").is_none());
    fs::remove_dir_all(&root).unwrap();
}

#[test]
fn project_scripts_run_from_the_project_directory() {
    let root = std::env::temp_dir().join(format!(
        "tyrus-run-script-{}-{}",
        std::process::id(),
        line!()
    ));
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(&root).unwrap();
    fs::write(
        root.join("tyrus.json"),
        r#"{"project":{"name":"app","version":"0.1.0"},"scripts":{"example":"echo ran > script-output.txt"}}"#,
    )
    .unwrap();

    let status = run_project_script(&root, "example", &[])
        .unwrap()
        .expect("the configured script must run");
    assert!(status.success());
    assert_eq!(
        fs::read_to_string(root.join("script-output.txt"))
            .unwrap()
            .trim(),
        "ran"
    );
    assert!(run_project_script(&root, "unknown", &[]).unwrap().is_none());
    fs::remove_dir_all(&root).unwrap();
}

#[test]
fn project_script_arguments_do_not_become_shell_commands() {
    let root = std::env::temp_dir().join(format!(
        "tyrus project script args {} {}",
        std::process::id(),
        line!()
    ));
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(&root).unwrap();
    fs::write(
        root.join("tyrus.json"),
        r#"{"project":{"name":"app","version":"0.1.0"},"scripts":{"example":"echo ran > script-output.txt"}}"#,
    )
    .unwrap();

    let status = run_project_script(
        &root,
        "example",
        &[
            "--".to_string(),
            "safe \" & echo injected > injected.txt".to_string(),
        ],
    )
    .unwrap()
    .expect("the configured script must run");
    assert!(status.success());
    assert!(root.join("script-output.txt").is_file());
    assert!(!root.join("injected.txt").exists());
    fs::remove_dir_all(&root).unwrap();
}

#[test]
fn default_resets_custom_project_scripts() {
    let root = std::env::temp_dir().join(format!(
        "tyrus-default-project-{}-{}",
        std::process::id(),
        line!()
    ));
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(&root).unwrap();
    fs::write(
        root.join("tyrus.json"),
        r#"{"project":{"name":"app","version":"0.1.0"},"scripts":{"example":"tyrus run example.tyr"}}"#,
    )
    .unwrap();

    cmd_default(&root);

    let config: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(root.join("tyrus.json")).unwrap()).unwrap();
    assert_eq!(config["scripts"]["fmt"], "tyrus fmt");
    assert_eq!(config["scripts"]["lint"], "tyrus lint");
    assert_eq!(config["scripts"]["test"], "tyrus test");
    assert!(config["scripts"].get("example").is_none());
    fs::remove_dir_all(&root).unwrap();
}

#[test]
fn cli_target_scaffolds_a_prompts_folder() {
    let root = std::env::temp_dir().join(format!("tyrus-new-cli-{}", std::process::id()));
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(root.join("src")).unwrap();
    write_target_sources(&root, Target::Cli);

    let setup = fs::read_to_string(root.join("src/prompts/setup.tyr")).unwrap();
    assert!(setup.contains("from \"tyrus:cli\""));
    assert!(setup.contains("export const runSetup"));
    let main = fs::read_to_string(root.join("src/main.tyr")).unwrap();
    assert!(main.contains("runSetup"));
    fs::remove_dir_all(&root).unwrap();
}

#[test]
fn api_target_scaffolds_an_http_server() {
    let root = std::env::temp_dir().join(format!("tyrus-new-api-{}", std::process::id()));
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(root.join("src")).unwrap();
    write_target_sources(&root, Target::Api);

    let main = fs::read_to_string(root.join("src/main.tyr")).unwrap();
    assert!(main.contains("from \"tyrus:http\""));
    assert!(main.contains("createRouter"));
    assert!(!main.contains("registerUserRoutes"));
    for layer in [
        "routes",
        "controllers",
        "services",
        "repositories",
        "models",
        "schemas",
    ] {
        assert!(root.join("src").join(layer).is_dir());
        assert!(root.join("src").join(layer).join("user.tyr").is_file());
    }
    let routes = fs::read_to_string(root.join("src/routes/user.tyr")).unwrap();
    assert!(routes.contains("export default (router: HttpRouter): void =>"));
    assert!(routes.contains("router.get(\"/users/:id\""));
    assert!(routes.contains("router.post(\"/users\""));
    assert!(routes.contains("params: UserParams"));
    assert!(routes.contains("body: CreateUserBody"));
    assert!(routes.contains("handler: getUserController"));
    assert!(routes.contains("handler: createUserController"));
    assert!(!routes.contains("reply.status("));
    let controller = fs::read_to_string(root.join("src/controllers/user.tyr")).unwrap();
    assert!(controller.contains("from \"tyrus:http\""));
    assert!(controller.contains("HttpReply<UserResponse>"));
    assert!(controller.contains("getUserController"));
    assert!(controller.contains("createUserController"));
    assert!(controller.contains("request: GetUserRequest"));
    assert!(controller.contains("request: CreateUserRequest"));
    assert!(controller.contains("reply.status(HttpStatus.Ok).send(user);"));
    assert!(controller.contains("reply.status(HttpStatus.Created).send(user);"));
    let schemas = fs::read_to_string(root.join("src/schemas/user.tyr")).unwrap();
    assert!(schemas.contains("export interface GetUserRequest"));
    assert!(schemas.contains("export interface CreateUserRequest"));
    assert!(!root.join("src/prompts").exists());
    fs::remove_dir_all(&root).unwrap();
}

#[test]
fn api_target_lint_config_contains_recommended_architecture_rules() {
    let config = api_lint_config_json();
    let value: serde_json::Value = serde_json::from_str(&config).unwrap();
    assert_eq!(value["controller-must-send-response"], "error");
    assert_eq!(value["service-no-reply"], "error");
    assert_eq!(value["api-no-verb-in-path"], "warn");
    tyrus_lint::config::LintConfig::from_json(&config).unwrap();
}
