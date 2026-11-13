use super::*;

fn stream_check_diagnostics(source: &str) -> Vec<crate::type_checker::CheckDiagnostic> {
    let tokens = crate::lexer::lex(source).unwrap();
    let program = crate::parser::Parser::new(tokens).parse_program().unwrap();
    let mut types = crate::collections::HashMap::default();
    let mut interfaces = crate::collections::HashMap::default();
    for name in [
        "ReadableStream",
        "WritableStream",
        "DuplexStream",
        "TransformStream",
        "StreamError",
    ] {
        if let Some((ty, fields, _)) = native_export_info("tyrus:stream", name) {
            types.insert(name.to_string(), ty);
            if let Some(fields) = fields {
                interfaces.insert(name.to_string(), fields);
            }
        }
    }
    let behaviors = crate::collections::HashMap::default();
    let type_aliases = crate::collections::HashMap::default();
    let enums = crate::collections::HashMap::default();
    let throws = crate::collections::HashMap::default();
    let db_models = crate::collections::HashMap::default();
    let reachable_interfaces = crate::collections::HashMap::default();
    TypeChecker::new()
        .analyze_program_with_imports(
            &program,
            ImportedProgramSymbols {
                types: &types,
                interfaces: &interfaces,
                behaviors: &behaviors,
                type_aliases: &type_aliases,
                enums: &enums,
                throws: &throws,
                db_models: &db_models,
                validators: &HashMap::default(),
                reachable_interfaces: &reachable_interfaces,
            },
        )
        .diagnostics
}

#[test]
fn resolves_native_http_router_import() {
    let root = std::env::temp_dir().join(format!("tyrus-http-module-test-{}", std::process::id()));
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(&root).unwrap();
    fs::write(
        root.join("main.tyr"),
        r#"
            import { createRouter, HttpError, HttpReply, HttpRouter, HttpStatus } from "tyrus:http";
            interface HealthRequest {}
            interface HealthResponse {
                status: string;
            }
            const handler = (request: HealthRequest, reply: HttpReply<HealthResponse>): void => {
                reply.status(HttpStatus.Ok).send({ status: "ok" });
            };
            const app: HttpRouter = createRouter();
            app.get("/health", { response: { 200: HealthResponse }, handler: handler });
            try {
                app.listen(3000);
            } catch (error: HttpError) {
                console.error(error.code);
            }
        "#,
    )
    .unwrap();

    let program = load_program(&root.join("main.tyr"), None).unwrap();
    assert!(program.items.iter().any(|item| matches!(
        item,
        Item::Stmt(Stmt { kind: StmtKind::VarDecl {
            value: Some(Expr { kind: ExprKind::FnCall { callee, .. }, .. }),
            ..
        }, .. }) if callee == "__tyrus_http_createRouter"
    )));
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn infers_http_error_fields_inside_listen_onfail_without_import() {
    let root = std::env::temp_dir().join(format!(
        "tyrus-http-onfail-error-test-{}",
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(&root).unwrap();
    fs::write(
        root.join("main.tyr"),
        r#"
            import { createRouter, HttpRouter } from "tyrus:http";
            const app: HttpRouter = createRouter();
            const run = (): void => {
                const server = app.listen(3000) onfail {
                    printError(server.code);
                    printError(server.message);
                    return;
                };
            };
        "#,
    )
    .unwrap();

    load_program(&root.join("main.tyr"), None)
        .expect("inferred HttpError fields should be available inside onfail");
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn imported_http_handler_rejects_error_body_without_required_code() {
    let root = std::env::temp_dir().join(format!(
        "tyrus-http-error-handler-context-test-{}",
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(root.join("src/controllers")).unwrap();
    fs::create_dir_all(root.join("src/routes")).unwrap();
    fs::write(
        root.join("src/schemas.tyr"),
        "export interface UserParams { id: string; }\nexport interface ApiError { code: string; message: string; }\n",
    )
    .unwrap();
    fs::write(
        root.join("src/controllers/user.tyr"),
        r#"
            import { HttpStatus } from "tyrus:http";
            export const getUser = async (req, res) => {
                res.status(HttpStatus.InternalServerError).send({ message: "Could not load user" });
            };
        "#,
    )
    .unwrap();
    fs::write(
        root.join("src/routes/user.tyr"),
        r#"
            import { HttpRouter } from "tyrus:http";
            import { ApiError, UserParams } from "../schemas";
            import { getUser } from "../controllers/user";
            export const userRoutes = (router: HttpRouter): void => {
                router.get("/users/:id", {
                    params: UserParams,
                    response: { 500: ApiError },
                    handler: getUser,
                });
            };
        "#,
    )
    .unwrap();
    fs::write(
        root.join("src/main.tyr"),
        r#"
            import { createRouter, HttpRouter } from "tyrus:http";
            import { userRoutes } from "./routes/user";
            const app: HttpRouter = createRouter();
            userRoutes(app);
        "#,
    )
    .unwrap();

    let error = load_program(&root.join("src/main.tyr"), None).unwrap_err();
    assert!(error.contains("E3105"), "unexpected diagnostics: {error}");
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn resolves_public_stream_capability_imports() {
    let root =
        std::env::temp_dir().join(format!("tyrus-stream-module-test-{}", std::process::id()));
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(&root).unwrap();
    fs::write(
        root.join("main.tyr"),
        r#"
            import { DuplexStream, ReadableStream, StreamError, TransformStream, WritableStream } from "tyrus:stream";
            type Input = ReadableStream<u8[]>;
            type Output = WritableStream<u8[]>;
            type Socket = DuplexStream<u8[], u8[]>;
            type Compression = TransformStream<u8[], u8[]>;
            function inspect(input: ReadableStream<u8[]>, output: WritableStream<u8[]>): void {
                const read: Promise<u8[]?> = input.read();
                const write: Promise<void> = output.write([1, 2, 3]);
                const pipe: Promise<void> = input.pipeTo(output);
                const closeInput: Promise<void> = input.close();
                const closeOutput: Promise<void> = output.close();
            }
            function inspectSocket(socket: DuplexStream<u8[], u8[]>): void {
                const closeReadable: Promise<void> = socket.readable.close();
                const closeWritable: Promise<void> = socket.writable.close();
                const closeSocket: Promise<void> = socket.close();
            }
            function inspectTransform(transform: TransformStream<string, u8[]>): void {
                const write: Promise<void> = transform.writable.write("payload");
                const read: Promise<u8[]?> = transform.readable.read();
                const close: Promise<void> = transform.close();
            }
            function report(error: StreamError): void {
                console.error(error.code, error.message);
            }
        "#,
    )
    .unwrap();

    let source = fs::read_to_string(root.join("main.tyr")).unwrap();
    let program = load_program(&root.join("main.tyr"), None).unwrap();
    let aliases = program
        .items
        .iter()
        .filter_map(|item| match item {
            Item::TypeAlias { name, ty, .. } => Some((name.as_str(), ty)),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(aliases.len(), 4);
    assert!(
        aliases
            .iter()
            .all(|(_, ty)| matches!(ty, Type::Apply { .. }))
    );
    let errors = stream_check_diagnostics(&source)
        .into_iter()
        .filter(|diagnostic| diagnostic.severity == crate::type_checker::CheckSeverity::Error)
        .collect::<Vec<_>>();
    assert!(
        errors.is_empty(),
        "stream imports should type check: {errors:#?}"
    );

    fs::write(
        root.join("main.tyr"),
        r#"
            import { ReadableStream, WritableStream } from "tyrus:stream";
            async function misuse(source: ReadableStream<u8[]>, destination: WritableStream<u8[]>): Promise<void> {
                await source.write([1]);
                await destination.read();
                await destination.write("not bytes");
                await source.pipeTo(source);
            }
        "#,
    )
    .unwrap();
    let invalid_errors = load_program(&root.join("main.tyr"), None)
        .expect_err("invalid stream methods and payloads must be rejected");
    assert!(
        invalid_errors.contains("error[E0599]"),
        "stream capabilities must reject unavailable methods: {invalid_errors}"
    );
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn rejects_invalid_stream_generic_arity_after_native_import_resolution() {
    let root = std::env::temp_dir().join(format!("tyrus-stream-arity-test-{}", std::process::id()));
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(&root).unwrap();
    fs::write(
        root.join("main.tyr"),
        r#"import { ReadableStream } from "tyrus:stream"; type Invalid = ReadableStream<u8[], string>;"#,
    )
    .unwrap();

    let arity_error = load_program(&root.join("main.tyr"), None)
        .expect_err("the native stream alias must reject the wrong generic arity");
    assert!(arity_error.contains("error[E3200]"), "got {arity_error}");

    fs::write(
        root.join("main.tyr"),
        "function consume(source: ReadableStream<u8[]>): void {}",
    )
    .unwrap();
    let import_error = load_program(&root.join("main.tyr"), None)
        .expect_err("stream types must be imported explicitly");
    assert!(import_error.contains("error[E0412]"), "got {import_error}");
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn resolves_public_process_imports_and_child_streams() {
    let root =
        std::env::temp_dir().join(format!("tyrus-process-module-test-{}", std::process::id()));
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(&root).unwrap();
    fs::write(
        root.join("main.tyr"),
        r#"
            import { ChildProcess, exec, ProcessError } from "tyrus:process";
            import { ReadableStream, WritableStream } from "tyrus:stream";
            async function run(): Promise<Result<i32, ProcessError>> {
                const child: ChildProcess = exec("tool", ["--version"])?;
                const stdin: WritableStream<u8[]> = child.stdin;
                const stdout: ReadableStream<u8[]> = child.stdout;
                const stderr: ReadableStream<u8[]> = child.stderr;
                await stdin.close();
                return await child.wait();
            }
        "#,
    )
    .unwrap();
    let program = load_program(&root.join("main.tyr"), None).unwrap();
    assert!(!program.items.is_empty());
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn requires_native_http_router_import_for_type_annotations() {
    let root = std::env::temp_dir().join(format!(
        "tyrus-http-router-import-required-{}",
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(&root).unwrap();
    fs::write(
        root.join("main.tyr"),
        r#"
            import { createRouter } from "tyrus:http";
            const app: HttpRouter = createRouter();
        "#,
    )
    .unwrap();

    let error = load_program(&root.join("main.tyr"), None).unwrap_err();
    assert!(
        error.contains("cannot find type `HttpRouter` in this scope"),
        "unexpected diagnostics: {error}"
    );
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn requires_native_http_reply_import_for_type_annotations() {
    let root = std::env::temp_dir().join(format!(
        "tyrus-http-reply-import-required-{}",
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(&root).unwrap();
    fs::write(
        root.join("main.tyr"),
        r#"
            import { createRouter, HttpRouter, HttpStatus } from "tyrus:http";
            interface HealthResponse { status: string; }
            const handler = (request: {}, reply: HttpReply<HealthResponse>): void => {
                reply.status(HttpStatus.Ok).send({ status: "ok" });
            };
            const app: HttpRouter = createRouter();
            app.get("/health", { response: { 200: HealthResponse }, handler });
        "#,
    )
    .unwrap();

    let error = load_program(&root.join("main.tyr"), None).unwrap_err();
    assert!(
        error.contains("cannot find type `HttpReply` in this scope"),
        "unexpected diagnostics: {error}"
    );
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn resolves_native_http_contract_type_imports() {
    let root = std::env::temp_dir().join(format!(
        "tyrus-http-contract-imports-test-{}",
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(&root).unwrap();
    fs::write(
        root.join("main.tyr"),
        r#"
            import { CookieOptions, CorsOptions, CsrfOptions, createRouter, HttpRequest, HttpResponse, HttpRouter, HttpStatus } from "tyrus:http";
            const cors: CorsOptions = { origin: ["https://example.com"], methods: ["GET", "OPTIONS", "QUERY"], allowedHeaders: ["X-Api-Key"], exposedHeaders: [], credentials: false, maxAge: 0 };
            const cookie: CookieOptions = { path: "/", secure: true, httpOnly: true, sameSite: "lax" };
            const csrf: CsrfOptions = { cookie: "__Host-csrf", header: "x-csrf-token" };
            function guard(request: HttpRequest): HttpResponse | true {
                if (request.headers.includes("authorization")) { return true; }
                return { statusCode: HttpStatus.Unauthorized, body: "Authorization required", cookies: "" };
            }
            const app: HttpRouter = createRouter();
            app.cors(cors);
            app.csrf(csrf);
        "#,
    )
    .unwrap();

    load_program(&root.join("main.tyr"), None).unwrap();
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn native_http_option_literal_unions_reject_invalid_values() {
    let root = std::env::temp_dir().join(format!(
        "tyrus-http-option-literals-test-{}",
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(&root).unwrap();

    fs::write(
        root.join("main.tyr"),
        r#"
            import { CorsOptions } from "tyrus:http";
            const cors: CorsOptions = { origin: [], methods: ["get"], allowedHeaders: [], exposedHeaders: [], credentials: false, maxAge: 0 };
        "#,
    )
    .unwrap();
    let error = load_program(&root.join("main.tyr"), None).unwrap_err();
    assert!(error.contains("E0308"), "unexpected diagnostics: {error}");

    fs::write(
        root.join("main.tyr"),
        r#"
            import { CookieOptions } from "tyrus:http";
            const cookie: CookieOptions = { sameSite: "Lax" };
        "#,
    )
    .unwrap();
    let error = load_program(&root.join("main.tyr"), None).unwrap_err();
    assert!(error.contains("E0308"), "unexpected diagnostics: {error}");
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn requires_native_cors_options_import() {
    let root = std::env::temp_dir().join(format!(
        "tyrus-http-cors-import-required-{}",
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(&root).unwrap();
    fs::write(
        root.join("main.tyr"),
        r#"
            import { createRouter, HttpRouter } from "tyrus:http";
            const app: HttpRouter = createRouter();
            app.cors({ origin: [], methods: [], allowedHeaders: [], exposedHeaders: [], credentials: false, maxAge: 0 });
        "#,
    )
    .unwrap();

    let error = load_program(&root.join("main.tyr"), None).unwrap_err();
    assert!(
        error.contains("cannot find type `CorsOptions` in this scope"),
        "unexpected diagnostics: {error}"
    );
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn requires_native_csrf_options_import() {
    let root = std::env::temp_dir().join(format!(
        "tyrus-http-csrf-import-required-{}",
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(&root).unwrap();
    fs::write(
        root.join("main.tyr"),
        r#"
            import { createRouter, HttpRouter } from "tyrus:http";
            const app: HttpRouter = createRouter();
            app.csrf({ cookie: "__Host-csrf", header: "x-csrf-token" });
        "#,
    )
    .unwrap();

    let error = load_program(&root.join("main.tyr"), None).unwrap_err();
    assert!(
        error.contains("cannot find type `CsrfOptions` in this scope"),
        "unexpected diagnostics: {error}"
    );
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn requires_native_raw_http_type_imports() {
    let root = std::env::temp_dir().join(format!(
        "tyrus-http-raw-type-imports-required-{}",
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(&root).unwrap();
    fs::write(
        root.join("main.tyr"),
        r#"
            import { createRouter, HttpRouter } from "tyrus:http";
            function guard(request: HttpRequest): HttpResponse | true {
                return true;
            }
            const app: HttpRouter = createRouter();
        "#,
    )
    .unwrap();

    let error = load_program(&root.join("main.tyr"), None).unwrap_err();
    assert!(
        error.contains("cannot find type `HttpRequest` in this scope"),
        "unexpected diagnostics: {error}"
    );
    assert!(
        error.contains("cannot find type `HttpResponse` in this scope"),
        "unexpected diagnostics: {error}"
    );
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn resolves_native_http_status_import() {
    let root = std::env::temp_dir().join(format!(
        "tyrus-http-status-module-test-{}",
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(&root).unwrap();
    fs::write(
        root.join("main.tyr"),
        r#"
            import { HttpStatus as Status } from "tyrus:http";
            const code: Status = Status.Created;
        "#,
    )
    .unwrap();

    let program = load_program(&root.join("main.tyr"), None).unwrap();
    assert!(program.items.iter().any(|item| matches!(
        item,
        Item::EnumDecl { name, .. } if name == "HttpStatus"
    )));
    let Item::Stmt(Stmt {
        kind:
            StmtKind::VarDecl {
                ty,
                value: Some(value),
                ..
            },
        ..
    }) = program
        .items
        .iter()
        .find(|item| {
            matches!(
                item,
                Item::Stmt(Stmt {
                    kind: StmtKind::VarDecl { .. },
                    ..
                })
            )
        })
        .expect("expected status binding")
    else {
        panic!("expected status binding");
    };
    assert_eq!(ty, &Type::Named("HttpStatus".to_string()));
    assert!(matches!(
        value,
        Expr { kind: ExprKind::FieldAccess { object, field }, .. }
            if matches!(object.as_ref().kind(), ExprKind::Ident(name) if name == "HttpStatus")
                && field == "Created"
    ));
    fs::remove_dir_all(root).unwrap();
}
