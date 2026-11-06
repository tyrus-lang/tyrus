use crate::config::{Category, RULES, Severity};
use crate::{
    LintConfig, lint_paths, lint_rules_source, lint_rules_source_with_analysis, lint_source,
};

#[test]
fn default_config_includes_all_rules() {
    let config = LintConfig::default();
    for rule in RULES {
        assert_eq!(
            config.severity(rule.name),
            rule.default,
            "default severity mismatch for {}",
            rule.name
        );
    }
}

#[test]
fn no_diagnostics_for_clean_file() {
    let source = r#"
const main = (): void => {
    console.log("hello");
};
"#;
    let config = LintConfig::default();
    let diagnostics = lint_source(source, &config);
    assert!(
        diagnostics.is_empty(),
        "expected no diagnostics, got {}: {:?}",
        diagnostics.len(),
        diagnostics
    );
}

#[test]
fn no_unused_vars_does_not_fire_for_wildcard_discard_bindings() {
    let source = r#"
const pair = (): [i32, string] => [1, "a"];
const main = (): void => {
    const [code, _] = pair();
    console.log(code);
    const [_, _] = pair();
};
"#;
    let config = LintConfig::default();
    let diagnostics = lint_source(source, &config);
    assert!(
        diagnostics.is_empty(),
        "expected no diagnostics, got {}: {:?}",
        diagnostics.len(),
        diagnostics
    );
}

#[test]
fn no_unused_vars_does_not_fire_for_a_variable_only_used_as_a_shorthand_object_field() {
    let source = r#"
interface User {
    name: string;
    age: i32;
}
const main = (): void => {
    const name: string = "Neo";
    const age: i32 = 34;
    const user: User = { name, age };
    console.log(user);
};
"#;
    let config = LintConfig::default();
    let diagnostics = lint_source(source, &config);
    assert!(
        diagnostics.is_empty(),
        "expected no diagnostics, got {}: {:?}",
        diagnostics.len(),
        diagnostics
    );
}

#[test]
fn project_lint_resolves_native_imports_before_reporting_compiler_diagnostics() {
    let root =
        std::env::temp_dir().join(format!("tyrus-lint-native-imports-{}", std::process::id()));
    let src = root.join("src");
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&src).unwrap();
    std::fs::write(
        src.join("main.tyr"),
        "import { styleText } from \"tyrus:utils\";\n\n\
         export const runStyleTextExamples = (): void => {\n\
             const banner: string = \"Ready\" |> styleText.greenBright.bold;\n\
             console.log(banner);\n\
         };\n",
    )
    .unwrap();

    let reports = lint_paths(&root, &[], &LintConfig::default()).unwrap();
    let diagnostics = reports
        .iter()
        .flat_map(|report| report.diagnostics.iter())
        .collect::<Vec<_>>();
    assert!(
        diagnostics.is_empty(),
        "expected no diagnostics, got {diagnostics:#?}"
    );
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn project_lint_resolves_implicit_model_timestamp_types() {
    let root = std::env::temp_dir().join(format!(
        "tyrus-lint-implicit-model-timestamps-{}",
        std::process::id()
    ));
    let src = root.join("src");
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&src).unwrap();
    std::fs::write(
        src.join("main.tyr"),
        "@model(\"users\")\nexport interface User {\n    @id id: i64;\n    name: string;\n}\n",
    )
    .unwrap();

    let reports = lint_paths(&root, &[], &LintConfig::default()).unwrap();
    let diagnostics = reports
        .iter()
        .flat_map(|report| report.diagnostics.iter())
        .collect::<Vec<_>>();
    assert!(
        diagnostics
            .iter()
            .all(|diagnostic| diagnostic.code != "E0412"),
        "implicit Model timestamps should resolve `Instant` without a user import: {diagnostics:#?}"
    );
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn no_shadow_does_not_report_the_onfail_error_binding() {
    let source = r#"
const load = async (): Promise<Result<i32, string>> => Ok(1);
export const seed = async (): Promise<Result<i32, string>> => {
    const data = await load() onfail {
        return Err(data);
    };
    return Ok(data);
};
"#;
    let diagnostics = lint_source(source, &LintConfig::default());
    assert!(
        diagnostics
            .iter()
            .all(|diagnostic| diagnostic.code != "no-shadow"),
        "the `onfail` payload intentionally reuses its initializer binding name: {diagnostics:#?}"
    );
}

#[test]
fn project_lint_resolves_native_enums_and_imported_http_handlers() {
    let root = std::env::temp_dir().join(format!("tyrus-lint-http-imports-{}", std::process::id()));
    let src = root.join("src");
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&src).unwrap();
    std::fs::write(
        src.join("handler.tyr"),
        "import { HttpReply, HttpStatus } from \"tyrus:http\";\n\n\
         type EmptyRequest = { };\n\n\
         export interface HealthResponse {\n\
             status: string;\n\
         }\n\n\
         export const healthHandler = (request: EmptyRequest, reply: HttpReply<HealthResponse>): void => {\n\
             reply.status(HttpStatus.Ok).send({ status: \"ok\" });\n\
         };\n",
    )
    .unwrap();
    std::fs::write(
        src.join("server.tyr"),
        "import { createRouter, HttpError, HttpRouter } from \"tyrus:http\";\n\
         import { HealthResponse, healthHandler } from \"./handler\";\n\n\
         export const startServer = (): void => {\n\
             const app: HttpRouter = createRouter();\n\
             app.get(\"/health\", {\n\
                 response: { 200: HealthResponse },\n\
                 handler: healthHandler,\n\
             });\n\
             try {\n\
                 app.listen(3333);\n\
             } catch (error: HttpError) {\n\
                 console.error(error.code);\n\
             }\n\
         };\n",
    )
    .unwrap();

    let reports = lint_paths(&root, &[], &LintConfig::default()).unwrap();
    let diagnostics = reports
        .iter()
        .flat_map(|report| report.diagnostics.iter())
        .collect::<Vec<_>>();
    assert!(
        diagnostics.is_empty(),
        "expected no diagnostics, got {diagnostics:#?}"
    );
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn no_unused_imports_recognizes_a_type_only_used_through_inference() {
    let root = std::env::temp_dir().join(format!(
        "tyrus-lint-type-only-import-{}",
        std::process::id()
    ));
    let src = root.join("src");
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&src).unwrap();
    std::fs::write(
        src.join("main.tyr"),
        "import { CpuInfo, UserInfo, cpus, userInfo } from \"tyrus:os\";\n\n\
         const main = (): void => {\n\
             const info = userInfo();\n\
             console.log(info.username.length > 0);\n\
             console.log(cpus().length > 0);\n\
         };\n",
    )
    .unwrap();

    let reports = lint_paths(&root, &[], &LintConfig::default()).unwrap();
    let diagnostics = reports
        .iter()
        .flat_map(|report| report.diagnostics.iter())
        .collect::<Vec<_>>();
    let unused_user_info = diagnostics
        .iter()
        .any(|d| d.code == "no-unused-imports" && d.message.contains("UserInfo"));
    assert!(
        !unused_user_info,
        "UserInfo is used through inference alone and should not be flagged, got {diagnostics:#?}"
    );
    let unused_cpu_info = diagnostics
        .iter()
        .any(|d| d.code == "no-unused-imports" && d.message.contains("CpuInfo"));
    assert!(
        unused_cpu_info,
        "CpuInfo is genuinely unused and should still be flagged, got {diagnostics:#?}"
    );
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn no_unused_imports_offers_safe_fixes_for_named_import_lists() {
    for source in [
        "import { unused, used } from \"./module\";\nconst main = (): void => { print(used); };\n",
        "import { used, unused } from \"./module\";\nconst main = (): void => { print(used); };\n",
    ] {
        let diagnostic = lint_source(source, &LintConfig::default())
            .into_iter()
            .find(|diagnostic| diagnostic.code == "no-unused-imports")
            .expect("unused import diagnostic");
        let fix = diagnostic.fix.expect("deterministic import removal fix");
        let mut fixed = source.to_string();
        fixed.replace_range(fix.range, &fix.replacement);
        assert!(
            fixed.contains("import { used } from \"./module\";"),
            "{fixed}"
        );
    }
}

#[test]
fn no_unused_imports_skips_fixes_when_import_comments_or_mixed_specifiers_are_present() {
    for source in [
        "import { unused /* keep this comment */, used } from \"./module\";\nconst main = (): void => { print(used); };\n",
        "import defaultValue, { unused } from \"./module\";\nconst main = (): void => { print(defaultValue); };\n",
    ] {
        let diagnostic = lint_source(source, &LintConfig::default())
            .into_iter()
            .find(|diagnostic| diagnostic.code == "no-unused-imports")
            .expect("unused import diagnostic");
        assert!(diagnostic.fix.is_none(), "{diagnostic:#?}");
    }
}

#[test]
fn template_interpolation_counts_as_a_variable_reference() {
    let source = r#"
const describe = (): string => {
    const name = "Tyrus";
    return `language: ${name}`;
};
"#;
    let diagnostics = lint_source(source, &LintConfig::default());
    assert!(
        codes(&diagnostics, "no-unused-vars").is_empty(),
        "template interpolation must count as use: {diagnostics:#?}"
    );
}

#[test]
fn prefer_const_recognizes_mutation_through_a_field() {
    let source = r#"
const increment = (): i32 => {
    let state = { count: 0 };
    state.count = state.count + 1;
    return state.count;
};
"#;
    let diagnostics = lint_source(source, &LintConfig::default());
    assert!(
        codes(&diagnostics, "prefer-const").is_empty(),
        "field assignment mutates the root binding: {diagnostics:#?}"
    );
}

#[test]
fn prefer_const_recognizes_mutations_inside_loop_and_match_blocks() {
    let source = r#"
const main = (): void => {
    let attempts: i32 = 0;
    loop {
        attempts++;
        break;
    }

    let compound: i32 = 0;
    match (true) {
        true => { compound += 1; }
        false => {}
    }

    let reassigned: i32 = 0;
    match (true) {
        true => { reassigned = reassigned + 1; }
        false => {}
    }
    print(attempts + compound + reassigned);
};
"#;
    let diagnostics = lint_source(source, &LintConfig::default());
    assert!(
        codes(&diagnostics, "prefer-const").is_empty(),
        "increments and assignments inside loop and match blocks must count as mutations: {diagnostics:#?}"
    );
}

#[test]
fn prefer_const_still_reports_a_let_binding_that_is_never_modified() {
    let source = r#"
const main = (): void => {
    let unchanged: i32 = 1;
    print(unchanged);
};
"#;
    let diagnostics = lint_source(source, &LintConfig::default());
    let prefer_const = diagnostics
        .iter()
        .filter(|diagnostic| diagnostic.code == "prefer-const")
        .collect::<Vec<_>>();
    assert_eq!(prefer_const.len(), 1, "{diagnostics:#?}");
    assert!(
        prefer_const[0].message.contains("`unchanged`"),
        "the diagnostic must identify the unchanged binding: {prefer_const:#?}"
    );
}

#[test]
fn cached_analysis_tokens_preserve_lint_results_without_relexing() {
    let source = "let value: i32 = 1;\nconsole.log(value);\n";
    let analysis = tyrus_core::analysis::analyze(source);
    assert!(!analysis.tokens.is_empty());
    assert!(analysis.program.is_some());
    let config = LintConfig::default();
    let cached = lint_rules_source_with_analysis(source, &analysis, &config);
    let reference = lint_rules_source(source, &config);
    let signature = |diagnostics: &[crate::LintDiagnostic]| {
        diagnostics
            .iter()
            .map(|diagnostic| {
                (
                    diagnostic.code.clone(),
                    diagnostic.range.clone(),
                    diagnostic.message.clone(),
                )
            })
            .collect::<Vec<_>>()
    };
    assert_eq!(signature(&cached), signature(&reference));
}

#[test]
fn cancellable_lint_stops_without_publishing_partial_rule_results() {
    let source = "let value: i32 = 1;\nconsole.log(value);\n";
    let analysis = tyrus_core::analysis::analyze(source);
    let cancellation = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(true));
    assert!(
        crate::lint_rules_source_with_analysis_cancellable(
            source,
            &analysis,
            &LintConfig::default(),
            cancellation,
        )
        .is_none()
    );
}

#[test]
fn parse_errors_do_not_cancel_lint_when_the_analysis_is_current() {
    let source = "const isLiquid: boolean;\nisLiquid = true;\nprint(isLiquid);\n";
    let analysis = tyrus_core::analysis::analyze(source);
    assert!(analysis.program.is_none(), "the source has a parse error");
    let cancellation = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
    let diagnostics = crate::lint_rules_source_with_analysis_cancellable(
        source,
        &analysis,
        &LintConfig::default(),
        cancellation,
    )
    .expect("a parse error is not a cancellation");
    assert!(diagnostics.is_empty());
}

#[test]
fn cancellable_lint_dispatch_stops_before_running_cancelled_rules() {
    let source = "import { value } from \"./value\";\nlet item = null;\nprint(value!);\n";
    let analysis = tyrus_core::analysis::analyze(source);
    let program = analysis.program.as_ref().expect("program exists");
    let config = LintConfig::default();
    let context = crate::LintContext {
        source,
        file: None,
        program,
        tokens: &analysis.tokens,
        analysis: &analysis,
        config: &config,
    };
    let cancellation = std::sync::atomic::AtomicBool::new(true);

    for rule in [
        "api-no-verb-in-path",
        "api-prefer-resource-nouns",
        "controller-no-try-catch",
        "db-explicit-select-columns",
        "db-no-extra-select-columns",
        "http-no-raw-status-code",
        "export-protect-no-effect",
        "no-layer-skip",
        "no-unused-vars",
        "no-unused-imports",
        "no-unused-functions",
        "no-upward-imports",
        "no-null-literal",
        "no-duplicate-imports",
        "no-force-unwrap",
        "no-unreachable-code",
        "no-duplicate-case",
        "no-shadow",
        "prefer-const",
        "prefer-nullish-coalescing",
        "no-unnecessary-allocation",
        "no-unnecessary-await",
        "no-string-concat-in-loop",
        "no-uninitialized-variable",
        "repository-no-http-error",
        "route-no-business-logic",
        "service-no-reply",
        "no-unsafe-finally",
        "no-empty-catch",
        "no-empty-block",
        "use-caught-error",
    ] {
        assert!(
            crate::rules::run_cancellable(rule, &context, Some(&cancellation)).is_empty(),
            "cancelled rule {rule} must not return partial diagnostics"
        );
    }
}

#[test]
fn project_lint_loads_configured_env_schema_before_reporting_compiler_diagnostics() {
    let root = std::env::temp_dir().join(format!("tyrus-lint-env-schema-{}", std::process::id()));
    let src = root.join("src");
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&src).unwrap();
    std::fs::write(
        root.join("tyrus.json"),
        r#"{
  "project": { "name": "lint-env-schema", "version": "0.1.0" },
  "security": { "envFile": ".env", "envSealed": false }
}
"#,
    )
    .unwrap();
    std::fs::write(root.join(".env"), "@type(u32)\nPORT=3000\n").unwrap();
    std::fs::write(
        src.join("main.tyr"),
        "const main = (): void => {\n\
             const { PORT } = process.env;\n\
             console.log(PORT);\n\
         };\n",
    )
    .unwrap();

    let reports = lint_paths(&root, &[], &LintConfig::default()).unwrap();
    let diagnostics = reports
        .iter()
        .flat_map(|report| report.diagnostics.iter())
        .collect::<Vec<_>>();
    assert!(
        diagnostics
            .iter()
            .all(|diagnostic| diagnostic.code != "E3140"),
        "expected configured env schema to suppress E3140, got {diagnostics:#?}"
    );
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn service_no_reply_fires_in_service_modules() {
    let root =
        std::env::temp_dir().join(format!("tyrus-lint-service-reply-{}", std::process::id()));
    let services = root.join("src/services");
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&services).unwrap();
    let source = r#"
import { HttpReply, HttpStatus } from "tyrus:http";

interface UserResponse {
    id: string;
}

export const getUserService = (reply: HttpReply<UserResponse>): void => {
    reply.status(HttpStatus.Ok).send({ id: "user-1" });
};
"#;
    std::fs::write(services.join("user.tyr"), source).unwrap();

    let reports = lint_paths(&root, &[], &LintConfig::default()).unwrap();
    let diagnostics = reports
        .iter()
        .flat_map(|report| report.diagnostics.iter())
        .filter(|diagnostic| diagnostic.code == "service-no-reply")
        .collect::<Vec<_>>();
    assert!(
        diagnostics.len() >= 3,
        "expected import, parameter, and use diagnostics, got {diagnostics:#?}"
    );
    assert!(diagnostics.iter().any(|diagnostic| {
        diagnostic.message.contains("HttpReply") && &source[diagnostic.range.clone()] == "HttpReply"
    }));
    assert!(diagnostics.iter().any(|diagnostic| {
        diagnostic.message.contains("`reply`") && &source[diagnostic.range.clone()] == "reply"
    }));
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn service_no_reply_is_limited_to_service_modules() {
    let root = std::env::temp_dir().join(format!(
        "tyrus-lint-controller-reply-{}",
        std::process::id()
    ));
    let controllers = root.join("src/controllers");
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&controllers).unwrap();
    std::fs::write(
        controllers.join("user.tyr"),
        r#"
import { HttpReply, HttpStatus } from "tyrus:http";

interface UserResponse {
    id: string;
}

export const getUserController = (reply: HttpReply<UserResponse>): void => {
    reply.status(HttpStatus.Ok).send({ id: "user-1" });
};
"#,
    )
    .unwrap();

    let reports = lint_paths(&root, &[], &LintConfig::default()).unwrap();
    let diagnostics = reports
        .iter()
        .flat_map(|report| report.diagnostics.iter())
        .filter(|diagnostic| diagnostic.code == "service-no-reply")
        .collect::<Vec<_>>();
    assert!(
        diagnostics.is_empty(),
        "controller modules must not trigger service-no-reply: {diagnostics:#?}"
    );
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn http_no_raw_status_code_fires_on_numeric_reply_status() {
    let source = r#"
const getUserController = (reply: HttpReply<UserResponse>): void => {
    reply.status(200).send({ id: "user-1" });
    reply.status(HttpStatus.Created).send({ id: "user-2" });
};
"#;
    let diagnostics = lint_source(source, &LintConfig::default());
    let hits = codes(&diagnostics, "http-no-raw-status-code");
    assert_eq!(hits.len(), 1);
    assert_eq!(&source[hits[0].range.clone()], "200");
    assert!(
        hits[0]
            .help
            .as_deref()
            .unwrap_or("")
            .contains("HttpStatus.Ok")
    );
}

#[test]
fn api_no_verb_in_path_fires_on_action_paths() {
    let source = r#"
const registerRoutes = (app: HttpRouter): void => {
    app.get("/getUsers", { response: { 200: UserResponse }, handler: getUsersController });
    app.post("/users/create", { response: { 201: UserResponse }, handler: createUserController });
    app.get("/users", { response: { 200: UserResponse }, handler: listUsersController });
};
"#;
    let diagnostics = lint_source(source, &LintConfig::default());
    let hits = codes(&diagnostics, "api-no-verb-in-path");
    assert_eq!(hits.len(), 2, "diagnostics: {diagnostics:#?}");
    assert!(
        hits.iter()
            .any(|diagnostic| &source[diagnostic.range.clone()] == "\"/getUsers\"")
    );
    assert!(
        hits.iter()
            .any(|diagnostic| &source[diagnostic.range.clone()] == "\"/users/create\"")
    );
}

#[test]
fn api_prefer_resource_nouns_fires_on_action_first_segment() {
    let source = r#"
const registerRoutes = (app: HttpRouter): void => {
    app.post("/createUser", { response: { 201: UserResponse }, handler: createUserController });
    app.get("/users/:id", { response: { 200: UserResponse }, handler: getUserController });
};
"#;
    let diagnostics = lint_source(source, &LintConfig::default());
    let hits = codes(&diagnostics, "api-prefer-resource-nouns");
    assert_eq!(hits.len(), 1);
    assert_eq!(&source[hits[0].range.clone()], "\"/createUser\"");
}

#[test]
fn no_upward_imports_fires_for_lower_layers_importing_higher_layers() {
    let root =
        std::env::temp_dir().join(format!("tyrus-lint-upward-imports-{}", std::process::id()));
    let services = root.join("src/services");
    let controllers = root.join("src/controllers");
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&services).unwrap();
    std::fs::create_dir_all(&controllers).unwrap();
    std::fs::write(
        controllers.join("user.tyr"),
        "export const getUserController = (): void => {};\n",
    )
    .unwrap();
    let source = "import { getUserController } from \"../controllers/user\";\n\nexport const getUserService = (): void => {\n    getUserController();\n};\n";
    std::fs::write(services.join("user.tyr"), source).unwrap();

    let reports = lint_paths(&root, &[], &LintConfig::default()).unwrap();
    let diagnostics = project_rule_diagnostics(&reports, "no-upward-imports");
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(&source[diagnostics[0].range.clone()], "getUserController");
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn no_layer_skip_fires_for_route_and_controller_layer_jumps() {
    let root = std::env::temp_dir().join(format!("tyrus-lint-layer-skip-{}", std::process::id()));
    let routes = root.join("src/routes");
    let controllers = root.join("src/controllers");
    let services = root.join("src/services");
    let repositories = root.join("src/repositories");
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&routes).unwrap();
    std::fs::create_dir_all(&controllers).unwrap();
    std::fs::create_dir_all(&services).unwrap();
    std::fs::create_dir_all(&repositories).unwrap();
    std::fs::write(
        services.join("user.tyr"),
        "export const getUserService = (): string => \"user-1\";\n",
    )
    .unwrap();
    std::fs::write(
        repositories.join("user.tyr"),
        "export const findUserRepository = (): string => \"user-1\";\n",
    )
    .unwrap();
    std::fs::write(
        routes.join("user.tyr"),
        "import { getUserService } from \"../services/user\";\n\nexport const registerUserRoutes = (): void => {\n    getUserService();\n};\n",
    )
    .unwrap();
    std::fs::write(
        controllers.join("user.tyr"),
        "import { findUserRepository } from \"../repositories/user\";\n\nexport const getUserController = (): void => {\n    findUserRepository();\n};\n",
    )
    .unwrap();

    let reports = lint_paths(&root, &[], &LintConfig::default()).unwrap();
    let diagnostics = project_rule_diagnostics(&reports, "no-layer-skip");
    assert_eq!(diagnostics.len(), 2, "diagnostics: {diagnostics:#?}");
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn route_no_business_logic_fires_for_inline_handlers_and_service_imports() {
    let root =
        std::env::temp_dir().join(format!("tyrus-lint-route-business-{}", std::process::id()));
    let routes = root.join("src/routes");
    let services = root.join("src/services");
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&routes).unwrap();
    std::fs::create_dir_all(&services).unwrap();
    std::fs::write(
        services.join("user.tyr"),
        "export const getUserService = (): string => \"user-1\";\n",
    )
    .unwrap();
    let source = r#"
import { getUserService } from "../services/user";

export const registerUserRoutes = (app: HttpRouter): void => {
    app.get("/users", {
        response: { 200: UserResponse },
        handler: (request, reply) => {
            getUserService();
        },
    });
};
"#;
    std::fs::write(routes.join("user.tyr"), source).unwrap();

    let reports = lint_paths(&root, &[], &LintConfig::default()).unwrap();
    let diagnostics = project_rule_diagnostics(&reports, "route-no-business-logic");
    assert_eq!(diagnostics.len(), 2, "diagnostics: {diagnostics:#?}");
    assert!(diagnostics.iter().any(|diagnostic| {
        diagnostic.message.contains("inline handler")
            && &source[diagnostic.range.clone()] == "handler"
    }));
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn controller_no_try_catch_fires_in_controller_modules() {
    let root = std::env::temp_dir().join(format!(
        "tyrus-lint-controller-try-catch-{}",
        std::process::id()
    ));
    let controllers = root.join("src/controllers");
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&controllers).unwrap();
    let source = r#"
export const getUserController = (): void => {
    try {
        console.log("ok");
    } catch (error: Error) {
        console.log(error);
    }
};
"#;
    std::fs::write(controllers.join("user.tyr"), source).unwrap();

    let reports = lint_paths(&root, &[], &LintConfig::default()).unwrap();
    let diagnostics = project_rule_diagnostics(&reports, "controller-no-try-catch");
    assert_eq!(diagnostics.len(), 2);
    assert!(
        diagnostics
            .iter()
            .all(|diagnostic| matches!(&source[diagnostic.range.clone()], "try" | "catch"))
    );
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn repository_no_http_error_fires_in_repository_modules() {
    let root = std::env::temp_dir().join(format!(
        "tyrus-lint-repository-http-error-{}",
        std::process::id()
    ));
    let repositories = root.join("src/repositories");
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&repositories).unwrap();
    let source = r#"
import { HttpReply } from "tyrus:http";

interface UserResponse {
    id: i32;
}

export const findUserRepository = (reply: HttpReply<UserResponse>): void => {
    console.log(reply);
};
"#;
    std::fs::write(repositories.join("user.tyr"), source).unwrap();

    let reports = lint_paths(&root, &[], &LintConfig::default()).unwrap();
    let diagnostics = project_rule_diagnostics(&reports, "repository-no-http-error");
    assert_eq!(diagnostics.len(), 2, "diagnostics: {diagnostics:#?}");
    assert!(
        diagnostics
            .iter()
            .all(|diagnostic| &source[diagnostic.range.clone()] == "HttpReply")
    );
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn rules_can_be_turned_off() {
    let source = r#"
const unused: string = "x";
"#;
    let config = LintConfig::from_json("{\"no-unused-vars\": \"off\"}").unwrap();
    let diagnostics = lint_source(source, &config);
    let unused: Vec<_> = diagnostics
        .iter()
        .filter(|d| d.code == "no-unused-vars")
        .collect();
    assert!(unused.is_empty(), "no-unused-vars should be off");
}

#[test]
fn no_uninitialized_variable_surfaces_definite_assignment() {
    let never_assigned = r#"
const main = (): void => {
    let message: string;
    console.log("hi");
};
"#;
    let diagnostics = lint_source(never_assigned, &LintConfig::default());
    let hits: Vec<_> = diagnostics
        .iter()
        .filter(|d| d.code == "no-uninitialized-variable")
        .collect();
    assert_eq!(hits.len(), 1);
    assert!(hits[0].message.contains("never assigned"));

    let used_before = r#"
const main = (): void => {
    let message: string;
    console.log(message);
};
"#;
    let diagnostics = lint_source(used_before, &LintConfig::default());
    assert!(
        diagnostics
            .iter()
            .any(|d| d.code == "no-uninitialized-variable" && d.message.contains("used before"))
    );
}

#[test]
fn compiler_type_errors_are_reported_by_lint() {
    let source = r#"
const main = (): void => {
    const message: string = "hello";
    const len: i64 = message.length;
    console.log(len);
};
"#;
    let diagnostics = lint_source(source, &LintConfig::default());
    let hit = diagnostics
        .iter()
        .find(|diagnostic| diagnostic.code == "E0308")
        .unwrap_or_else(|| panic!("diagnostics: {diagnostics:#?}"));
    assert!(hit.message.contains("expected `i64`, found `usize`"));
    assert_eq!(&source[hit.range.clone()], "len");
    assert!(hit.range.start < source.find("console.log(len)").unwrap());
}

#[test]
fn no_null_literal_fires() {
    let source = r#"
const main = (): void => {
    const value = null;
};
"#;
    let diagnostics = lint_source(source, &LintConfig::default());
    let hits = codes(&diagnostics, "no-null-literal");
    assert_eq!(hits.len(), 1);
    assert_eq!(hits[0].severity, Severity::Error);
}

#[test]
fn no_force_unwrap_is_enforced_by_compiler_not_lint() {
    let source = r#"
const main = (value: string?): void => {
    console.log(value!);
};
"#;
    let diagnostics = lint_source(source, &LintConfig::default());
    assert!(
        diagnostics
            .iter()
            .all(|diagnostic| diagnostic.code != "no-force-unwrap")
    );
}

#[test]
fn deferred_let_skips_unused_and_prefer_const() {
    let source = r#"
const main = (): void => {
    let message: string;
    console.log("hi");
};
"#;
    let diagnostics = lint_source(source, &LintConfig::default());
    assert!(
        !diagnostics
            .iter()
            .any(|d| d.code == "prefer-const" || d.code == "no-unused-vars"),
        "deferred `let` must not trigger prefer-const or no-unused-vars: {diagnostics:?}"
    );
}

#[test]
fn no_unused_vars_fires() {
    let source = r#"
const main = (): void => {
    const unused: string = "x";
};
"#;
    let config = LintConfig::default();
    let diagnostics = lint_source(source, &config);
    let unused: Vec<_> = diagnostics
        .iter()
        .filter(|d| d.code == "no-unused-vars")
        .collect();
    assert!(!unused.is_empty(), "expected no-unused-vars diagnostic");
    assert!(unused[0].message.contains("unused"));
    let fix = unused[0]
        .fix
        .as_ref()
        .expect("simple unused variable has a safe rename fix");
    assert_eq!(&source[fix.range.clone()], "unused");
    assert_eq!(fix.replacement, "_unused");
}

#[test]
fn no_unused_vars_omits_rename_fix_for_conflicting_or_destructured_bindings() {
    let source = r#"
interface User { id: i32; }
function main(user: User): void {
    const unused: string = "x";
    const _unused: string = "already bound";
    const { id } = user;
}
"#;
    let diagnostics = lint_source(source, &LintConfig::default());
    let unused = diagnostics
        .iter()
        .filter(|diagnostic| diagnostic.code == "no-unused-vars")
        .collect::<Vec<_>>();
    assert_eq!(unused.len(), 2);
    assert!(unused.iter().all(|diagnostic| diagnostic.fix.is_none()));
}

#[test]
fn no_unused_vars_counts_a_used_onfail_error_as_use_of_its_initializer_binding() {
    let used_in_handler = r#"
function load(): Result<i32, string> { return Ok(1); }
function run(): void {
    const data = load() onfail {
        console.log(data);
    };
}
"#;
    let diagnostics = lint_source(used_in_handler, &LintConfig::default());
    assert!(
        codes(&diagnostics, "no-unused-vars").is_empty(),
        "the initializer binding is used as the `onfail` error payload: {diagnostics:#?}"
    );

    let unused_in_handler = r#"
function load(): Result<i32, string> { return Ok(1); }
function run(): void {
    const data = load() onfail {
        console.log("failed");
    };
}
"#;
    let diagnostics = lint_source(unused_in_handler, &LintConfig::default());
    assert!(
        !codes(&diagnostics, "no-unused-vars").is_empty(),
        "an unused initializer result should still be reported: {diagnostics:#?}"
    );
}

#[test]
fn unknown_rule_name_rejected() {
    let result = LintConfig::from_json(r#"{"does-not-exist": "warn"}"#);
    assert!(result.is_err(), "expected error for unknown rule");
    assert!(result.unwrap_err().contains("unknown"));
}

#[test]
fn removed_route_plural_rule_setting_is_ignored() {
    let config = LintConfig::from_json(r#"{"api-prefer-plural-resources": "info"}"#)
        .expect("a removed rule setting should not break existing project configs");
    assert!(!config.is_enabled("api-prefer-plural-resources"));
}

#[test]
fn invalid_severity_rejected() {
    let result = LintConfig::from_json(r#"{"no-unused-vars": "super-warn"}"#);
    assert!(result.is_err(), "expected error for invalid severity");
}

#[test]
fn rule_meta_roundtrip() {
    for rule in RULES {
        let fetched = crate::config::rule_meta(rule.name);
        assert!(
            fetched.is_some(),
            "rule_meta returned None for {}",
            rule.name
        );
        let fetched = fetched.unwrap();
        assert_eq!(fetched.name, rule.name);
        assert_eq!(fetched.category as usize, rule.category as usize);
    }
}

#[test]
fn labels_are_non_empty() {
    for severity in &[
        Severity::Off,
        Severity::Info,
        Severity::Warn,
        Severity::Error,
    ] {
        assert!(!severity.label().is_empty());
    }
    for category in &[
        Category::Correctness,
        Category::Safety,
        Category::Style,
        Category::Performance,
        Category::Complexity,
    ] {
        assert!(!category.label().is_empty());
    }
}

#[test]
fn no_empty_catch_fires() {
    let source = r#"
const main = (): void => {
    try {
        console.log("ok");
    } catch (error: Error) {
    }
};
"#;
    let config = LintConfig::default();
    let diagnostics = lint_source(source, &config);
    let empty: Vec<_> = diagnostics
        .iter()
        .filter(|d| d.code == "no-empty-catch")
        .collect();
    assert!(!empty.is_empty(), "expected no-empty-catch diagnostic");
}

#[test]
fn no_unreachable_code_fires() {
    let source = r#"
const main = (): void => {
    return;
    console.log("never");
};
"#;
    let config = LintConfig::default();
    let diagnostics = lint_source(source, &config);
    let unreachable: Vec<_> = diagnostics
        .iter()
        .filter(|d| d.code == "no-unreachable-code")
        .collect();
    assert!(
        !unreachable.is_empty(),
        "expected no-unreachable-code diagnostic"
    );
}

#[test]
fn return_await_is_reported_and_fixed_after_direct_promise_adoption() {
    let source = "async function load(value: Promise<i32>): Promise<i32> { return await value; }";
    let diagnostics = lint_source(source, &LintConfig::default());
    let await_diagnostics = codes(&diagnostics, "no-unnecessary-await");
    assert_eq!(await_diagnostics.len(), 1, "diagnostics: {diagnostics:#?}");
    let fix = await_diagnostics[0]
        .fix
        .as_ref()
        .expect("expected safe fix");
    assert_eq!(&source[fix.range.clone()], "await");
    assert_eq!(fix.replacement, "");
}

#[test]
fn return_await_inside_try_is_not_reported_as_redundant() {
    let source = r#"
async function load(value: Promise<i32>): Promise<i32> {
    try {
        return await value;
    } catch (_error: Error) {
        return 0;
    }
}
"#;
    let diagnostics = lint_source(source, &LintConfig::default());
    assert!(
        codes(&diagnostics, "no-unnecessary-await").is_empty(),
        "diagnostics: {diagnostics:#?}"
    );
}

fn codes<'a>(
    diagnostics: &'a [crate::LintDiagnostic],
    code: &str,
) -> Vec<&'a crate::LintDiagnostic> {
    diagnostics.iter().filter(|d| d.code == code).collect()
}

fn project_rule_diagnostics<'a>(
    reports: &'a [crate::FileReport],
    code: &str,
) -> Vec<&'a crate::LintDiagnostic> {
    reports
        .iter()
        .flat_map(|report| report.diagnostics.iter())
        .filter(|diagnostic| diagnostic.code == code)
        .collect()
}

#[test]
fn no_empty_block_fires_on_control_flow() {
    let source = r#"
const main = (): void => {
    if (true) {
    }
};
"#;
    let diagnostics = lint_source(source, &LintConfig::default());
    assert_eq!(codes(&diagnostics, "no-empty-block").len(), 1);
}

#[test]
fn no_empty_block_allows_comment_and_object_literal() {
    let source = r#"
const main = (): void => {
    if (true) {
        // intentionally empty
    }
    const empty = {};
};
"#;
    let diagnostics = lint_source(source, &LintConfig::default());
    assert!(
        codes(&diagnostics, "no-empty-block").is_empty(),
        "comment-explained blocks and object literals must not fire"
    );
}

#[test]
fn empty_catch_is_not_double_reported_as_empty_block() {
    let source = r#"
const main = (): void => {
    try {
        console.log("ok");
    } catch (error: Error) {
    }
};
"#;
    let diagnostics = lint_source(source, &LintConfig::default());
    assert!(
        codes(&diagnostics, "no-empty-block").is_empty(),
        "an empty catch belongs to no-empty-catch only"
    );
    assert_eq!(codes(&diagnostics, "no-empty-catch").len(), 1);
}

#[test]
fn use_caught_error_fires_when_unused() {
    let source = r#"
const main = (): void => {
    try {
        console.log("ok");
    } catch (error: Error) {
        console.log("failed");
    }
};
"#;
    let diagnostics = lint_source(source, &LintConfig::default());
    let caught_errors = codes(&diagnostics, "use-caught-error");
    assert_eq!(caught_errors.len(), 1);
    let fix = caught_errors[0]
        .fix
        .as_ref()
        .expect("unused catch binding has a deterministic fix");
    assert_eq!(&source[fix.range.clone()], "error");
    assert_eq!(fix.replacement, "_error");
}

#[test]
fn use_caught_error_omits_rename_fix_when_the_target_name_conflicts() {
    let source = r#"
const main = (): void => {
    try {
        console.log("ok");
    } catch (error: Error) {
        const _error: string = "already bound";
        console.log("failed");
    }
};
"#;
    let diagnostics = lint_source(source, &LintConfig::default());
    let caught_errors = codes(&diagnostics, "use-caught-error");
    assert_eq!(caught_errors.len(), 1);
    assert!(caught_errors[0].fix.is_none());
}

#[test]
fn use_caught_error_quiet_when_used_or_underscored() {
    let used = r#"
const main = (): void => {
    try {
        console.log("ok");
    } catch (error: Error) {
        console.log(error);
    }
    try {
        console.log("ok");
    } catch (_error: Error) {
        console.log("ignored");
    }
};
"#;
    let diagnostics = lint_source(used, &LintConfig::default());
    assert!(codes(&diagnostics, "use-caught-error").is_empty());
}

#[test]
fn use_caught_error_recognizes_template_interpolation_not_property_names() {
    let source = r#"
const main = (): void => {
    try {
        console.log("ok");
    } catch (error: Error) {
        printError(`${error.code}: ${error.message}`);
    }
    try {
        console.log("ok");
    } catch (error: Error) {
        console.error("failed");
    }
};
"#;
    let diagnostics = lint_source(source, &LintConfig::default());
    let caught_error = codes(&diagnostics, "use-caught-error");
    assert_eq!(caught_error.len(), 1);
}

#[test]
fn no_duplicate_case_fires_for_switch_and_match() {
    let switch = r#"
const main = (x: i32): void => {
    switch (x) {
        case 1:
            console.log("one");
        case 1:
            console.log("again");
        default:
            console.log("other");
    }
};
"#;
    let diagnostics = lint_source(switch, &LintConfig::default());
    assert_eq!(codes(&diagnostics, "no-duplicate-case").len(), 1);

    let matcher = r#"
const main = (x: i32): void => {
    match (x) {
        (1) => { console.log("one"); }
        (1) => { console.log("again"); }
    }
};
"#;
    let diagnostics = lint_source(matcher, &LintConfig::default());
    assert_eq!(codes(&diagnostics, "no-duplicate-case").len(), 1);
}

#[test]
fn no_duplicate_case_quiet_for_distinct_cases() {
    let source = r#"
const main = (x: i32): void => {
    switch (x) {
        case 1:
            console.log("one");
        case 2:
            console.log("two");
    }
};
"#;
    let diagnostics = lint_source(source, &LintConfig::default());
    assert!(codes(&diagnostics, "no-duplicate-case").is_empty());
}

#[test]
fn prefer_nullish_coalescing_fires() {
    let source = r#"
const main = (name: string?): void => {
    const value: string = name !== null ? name : "default";
    console.log(value);
};
"#;
    let diagnostics = lint_source(source, &LintConfig::default());
    assert_eq!(codes(&diagnostics, "prefer-nullish-coalescing").len(), 1);
}

#[test]
fn no_unnecessary_allocation_fires_on_full_slice() {
    let source = r#"
const main = (xs: i32[]): void => {
    const copy: i32[] = xs.slice(0);
    console.log(copy.length);
};
"#;
    let diagnostics = lint_source(source, &LintConfig::default());
    assert_eq!(codes(&diagnostics, "no-unnecessary-allocation").len(), 1);
}

#[test]
fn no_string_concat_in_loop_fires_for_strings_only() {
    let source = r#"
const main = (items: string[]): void => {
    let output: string = "";
    let total: i32 = 0;
    for (let i: i32 = 0; i < items.length; i++) {
        output += items[i];
        total += i;
    }
    console.log(output);
    console.log(total);
};
"#;
    let diagnostics = lint_source(source, &LintConfig::default());
    let hits = codes(&diagnostics, "no-string-concat-in-loop");
    assert_eq!(
        hits.len(),
        1,
        "only the string concat should fire, not the i32 sum"
    );
    assert!(hits[0].message.contains("output"));
}

#[test]
fn exported_bindings_are_not_unused_or_prefer_const() {
    let source = r#"
export let mutableCount: i32 = 0;
export const PI: f64 = 3.14159;
"#;
    let diagnostics = lint_source(source, &LintConfig::default());
    assert!(
        codes(&diagnostics, "no-unused-vars").is_empty(),
        "exported bindings are public API"
    );
    assert!(
        codes(&diagnostics, "prefer-const").is_empty(),
        "an exported let is public mutable API"
    );
}

#[test]
fn prefer_const_skips_method_mutated_and_borrowed() {
    let source = r#"
const main = (): void => {
    let numbers: i32[] = [1, 2, 3];
    numbers.push(4);
    console.log(numbers.length);
};
"#;
    let diagnostics = lint_source(source, &LintConfig::default());
    assert!(
        codes(&diagnostics, "prefer-const").is_empty(),
        "a binding mutated via push must stay let"
    );
}

#[test]
fn prefer_const_recognizes_array_rotation_as_mutation() {
    let source = r#"
const main = (): void => {
    let rotated: i32[] = [1, 2, 3, 4, 5];
    rotated.rotate(-2);
    print(rotated);
};
"#;
    let diagnostics = lint_source(source, &LintConfig::default());
    assert!(
        codes(&diagnostics, "prefer-const").is_empty(),
        "rotate mutates the array value: {diagnostics:#?}"
    );
}

#[test]
fn prefer_const_recognizes_sort_by_as_mutation() {
    let source = r#"
interface Score {
    name: string;
    points: i32;
}
const main = (): void => {
    let scores: Score[] = [
        { name: "Ada", points: 30 },
        { name: "Grace", points: 10 },
        { name: "Linus", points: 20 },
    ];
    scores.sortBy("points");
    print(scores);
};
"#;
    let diagnostics = lint_source(source, &LintConfig::default());
    assert!(
        codes(&diagnostics, "prefer-const").is_empty(),
        "sortBy mutates the array value: {diagnostics:#?}"
    );
}

#[test]
fn destructured_object_bindings_count_as_used() {
    let source = r#"
const main = (): void => {
    const point: Point = { x: 3, y: 7 };
    const { x, y } = point;
    console.log(x);
    console.log(y);
};
"#;
    let diagnostics = lint_source(source, &LintConfig::default());
    assert!(
        codes(&diagnostics, "no-unused-vars").is_empty(),
        "x and y are used after the destructure"
    );
}

#[test]
fn export_protect_in_the_source_root_is_reported_as_having_no_effect() {
    let root = std::env::temp_dir().join(format!("tyrus-lint-protect-{}", std::process::id()));
    let src = root.join("src");
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(src.join("auth")).unwrap();
    std::fs::write(root.join("tyrus.json"), "{}\n").unwrap();
    std::fs::write(
        src.join("main.tyr"),
        "export protect const wide = (): i32 => 1;\n",
    )
    .unwrap();
    std::fs::write(
        src.join("auth/token.tyr"),
        "export protect const narrow = (): i32 => 2;\n",
    )
    .unwrap();

    let reports = lint_paths(&root, &[], &LintConfig::default()).unwrap();
    let hits = reports
        .iter()
        .flat_map(|report| report.diagnostics.iter().map(move |d| (&report.path, d)))
        .filter(|(_, diagnostic)| diagnostic.code == "export-protect-no-effect")
        .collect::<Vec<_>>();
    assert_eq!(
        hits.len(),
        1,
        "only the source-root declaration narrows nothing: {hits:?}"
    );
    let (path, diagnostic) = hits[0];
    assert!(path.ends_with("main.tyr"), "reported in {path}");
    assert!(
        diagnostic.message.contains("`wide`") && diagnostic.message.contains("narrows nothing"),
        "got: {}",
        diagnostic.message
    );
    assert!(
        diagnostic
            .help
            .as_deref()
            .is_some_and(|help| help.contains("drop `protect`")),
        "the help must be concrete: {diagnostic:?}"
    );
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn the_linter_matches_the_compiler_on_protected_imports() {
    let root = std::env::temp_dir().join(format!("tyrus-lint-scope-{}", std::process::id()));
    let src = root.join("src");
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(src.join("auth/session")).unwrap();
    std::fs::create_dir_all(src.join("http")).unwrap();
    std::fs::write(root.join("tyrus.json"), "{}\n").unwrap();
    std::fs::write(
        src.join("auth/token.tyr"),
        "export protect const sign = (id: i32): i32 => id + 1;\n",
    )
    .unwrap();
    std::fs::write(
        src.join("auth/login.tyr"),
        "import { sign } from \"./token\";\n\nexport const login = (id: i32): i32 => sign(id);\n",
    )
    .unwrap();
    // The subfolder case is the one that regressed: the linter resolves imports
    // with a non-canonical path, so `../token` used to leave a `..` in the scope
    // and every descendant looked like an outsider.
    std::fs::write(
        src.join("auth/session/refresh.tyr"),
        "import { sign } from \"../token\";\n\nexport const refresh = (id: i32): i32 => sign(id);\n",
    )
    .unwrap();
    std::fs::write(
        src.join("http/handler.tyr"),
        "import { sign } from \"../auth/token\";\n\nexport const handle = (id: i32): i32 => sign(id);\n",
    )
    .unwrap();

    let reports = lint_paths(&root, &[], &LintConfig::default()).unwrap();
    let offenders = reports
        .iter()
        .filter(|report| {
            report
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic.code == "E3308")
        })
        .map(|report| report.path.clone())
        .collect::<Vec<_>>();
    assert_eq!(
        offenders.len(),
        1,
        "only the sibling folder is out of scope: {offenders:?}"
    );
    assert!(
        offenders[0].ends_with("handler.tyr"),
        "reported in {}",
        offenders[0]
    );
    let diagnostic = reports
        .iter()
        .flat_map(|report| &report.diagnostics)
        .find(|diagnostic| diagnostic.code == "E3308")
        .expect("the sibling import must be reported");
    assert!(
        diagnostic.message.contains("auth/token.tyr")
            && diagnostic.message.contains("limits it to `auth`"),
        "the message must carry the scope with clean paths: {}",
        diagnostic.message
    );
    assert!(
        diagnostic
            .help
            .as_deref()
            .is_some_and(|help| help.contains("drop `protect`") && help.contains("`auth`")),
        "the first help entry must be the actionable one: {diagnostic:?}"
    );
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn db_explicit_select_fix_uses_model_column_mappings() {
    let source = r#"@model("users")
interface User {
    @id
    id: i64;
    @column("email_address")
    email: string;
}

const users = db.query<User>("SELECT * FROM users", []);
"#;
    let diagnostics = lint_source(source, &LintConfig::default());
    let hits = codes(&diagnostics, "db-explicit-select-columns");
    assert_eq!(hits.len(), 1);
    assert_eq!(&source[hits[0].range.clone()], "*");
    let fix = hits[0].fix.as_ref().unwrap();
    assert_eq!(fix.replacement, "\"id\", \"email_address\"");
    assert_eq!(&source[fix.range.clone()], "*");
}

#[test]
fn db_explicit_select_does_not_offer_ambiguous_join_fix() {
    let source = r#"interface JoinedUser {
    userId: i64;
    roleId: i64;
}
const rows = db.query<JoinedUser>("SELECT * FROM users JOIN roles ON users.role_id = roles.id", []);
"#;
    let diagnostics = lint_source(source, &LintConfig::default());
    let hits = codes(&diagnostics, "db-explicit-select-columns");
    assert_eq!(hits.len(), 1);
    assert!(hits[0].fix.is_none());
    assert!(
        hits[0]
            .help
            .as_deref()
            .unwrap()
            .contains("select only the fields")
    );
}

#[test]
fn db_extra_select_column_reports_precise_projection_range() {
    let source = r#"interface UserRow {
    id: i64;
    email: string;
}

const users = db.query<UserRow>("SELECT id, email, password_hash FROM users", []);
"#;
    let diagnostics = lint_source(source, &LintConfig::default());
    let hits = codes(&diagnostics, "db-no-extra-select-columns");
    assert_eq!(hits.len(), 1);
    assert_eq!(&source[hits[0].range.clone()], "password_hash");
    assert!(
        hits[0]
            .help
            .as_deref()
            .unwrap()
            .contains("remove `password_hash`")
    );
}

#[test]
fn db_extra_select_column_respects_model_mappings_and_aliases() {
    let source = r#"@model("users")
interface UserRow {
    @column("user_id")
    id: i64;
    @column("email_address")
    email: string;
}

const users = db.query<UserRow>("SELECT user_id, email_address AS email, internal_note FROM users", []);
"#;
    let diagnostics = lint_source(source, &LintConfig::default());
    let hits = codes(&diagnostics, "db-no-extra-select-columns");
    assert_eq!(hits.len(), 1);
    assert_eq!(&source[hits[0].range.clone()], "internal_note");
}

#[test]
fn db_extra_select_column_skips_select_star_and_unknown_queries() {
    let source = r#"interface UserRow { id: i64; }
const all = db.query<UserRow>("SELECT * FROM users", []);
const malformed = db.query<UserRow>("SELECT id FROM", []);
"#;
    let diagnostics = lint_source(source, &LintConfig::default());
    assert!(codes(&diagnostics, "db-no-extra-select-columns").is_empty());
}
