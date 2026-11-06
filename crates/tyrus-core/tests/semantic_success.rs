use tyrus_core::parser::Parser;
use tyrus_core::type_checker::{CheckSeverity, TypeChecker};

#[test]
fn accepts_an_object_literal_nested_under_a_generic_type_argument() {
    assert_well_typed(
        r#"
            interface Inner {
                n: i32;
            }
            interface Boxed<T> {
                label: string;
                value: T;
            }
            const boxed: Boxed<Inner> = { label: "okay", value: { n: 1 } };
            console.log(boxed.value.n);
        "#,
    );
}

#[test]
fn preserves_native_handle_types_in_result_match_bindings() {
    assert_well_typed(
        r#"
            interface TcpStream {}
            interface TcpListener {}
            interface NetError {
                code: string;
                message: string;
            }
            function useStream(result: Result<TcpStream, NetError>): void {
                match (result) {
                    Ok(stream) => { stream.close(); }
                    Err(error) => { console.error(error.code); }
                }
            }
            function useListener(result: Result<TcpListener, NetError>): void {
                match (result) {
                    Ok(listener) => { listener.localAddress(); }
                    Err(error) => { console.error(error.code); }
                }
            }
        "#,
    );
}

fn assert_well_typed(source: &str) {
    let tokens = tyrus_core::lexer::lex(source).unwrap();
    let program = Parser::new(tokens).parse_program().unwrap();
    let errors = TypeChecker::new()
        .check_program(&program)
        .into_iter()
        .filter(|diagnostic| diagnostic.severity == CheckSeverity::Error)
        .collect::<Vec<_>>();
    assert!(errors.is_empty(), "unexpected errors: {errors:#?}");
}

fn assert_has_error(source: &str, expected_code: &str) {
    let tokens = tyrus_core::lexer::lex(source).unwrap();
    let program = Parser::new(tokens).parse_program().unwrap();
    let errors = TypeChecker::new()
        .check_program(&program)
        .into_iter()
        .filter(|diagnostic| diagnostic.severity == CheckSeverity::Error)
        .collect::<Vec<_>>();
    assert!(
        errors.iter().any(|e| e.code == expected_code),
        "expected error code `{expected_code}`, got: {errors:#?}"
    );
}

fn assert_db_backend_well_typed(source: &str, backend: tyrus_core::db::DatabaseBackend) {
    let source = format!("interface Instant {{ seconds: i64; nanoseconds: u32; }}\n{source}");
    let tokens = tyrus_core::lexer::lex(&source).unwrap();
    let program = Parser::new(tokens).parse_program().unwrap();
    let mut checker = TypeChecker::new();
    checker.set_db_backend(Some(backend));
    let errors = checker
        .check_program(&program)
        .into_iter()
        .filter(|diagnostic| diagnostic.severity == CheckSeverity::Error)
        .collect::<Vec<_>>();
    assert!(errors.is_empty(), "unexpected errors: {errors:#?}");
}

#[test]
fn accepts_native_identifier_declarations_for_each_database_backend() {
    assert_db_backend_well_typed(
        r#"@model("users") interface User { name: string; }"#,
        tyrus_core::db::DatabaseBackend::Sqlite,
    );
    assert_db_backend_well_typed(
        r#"@model("users") interface User { name: string; }"#,
        tyrus_core::db::DatabaseBackend::Mongodb,
    );
    assert_db_backend_well_typed(
        r#"interface Uuid {}
@model("auto_users") interface AutoUser { @id({ strategy: "auto_increment" }) id: i64; }
@model("uuid_users") interface UuidUser { @id({ strategy: "uuid" }) id: Uuid; }
@model("short_users") interface ShortUser { @id({ strategy: "uuid_short" }) id: u64; }"#,
        tyrus_core::db::DatabaseBackend::Mysql,
    );
    assert_db_backend_well_typed(
        r#"@model("users") interface User { name: string; }"#,
        tyrus_core::db::DatabaseBackend::Mysql,
    );
    assert_db_backend_well_typed(
        r#"interface Uuid {}
interface Uuid7 {}
@model("identity_users") interface IdentityUser { @id id: i64; }
@model("uuid_users") interface UuidUser { @id({ strategy: "uuid" }) id: Uuid; }
@model("uuid7_users") interface Uuid7User { @id({ strategy: "uuid7" }) id: Uuid7; }"#,
        tyrus_core::db::DatabaseBackend::Postgresql,
    );
    assert_db_backend_well_typed(
        r#"@model("users") interface User { name: string; }"#,
        tyrus_core::db::DatabaseBackend::Postgresql,
    );
}

#[test]
fn direct_analysis_applies_sqlite_implicit_model_id_before_schema_validation() {
    use tyrus_core::analysis::{AnalyzeContext, analyze_with_context};
    use tyrus_core::db::{DatabaseBackend, DbColumnSchema, DbSchemaSnapshot, DbTableSchema};

    let schema = DbSchemaSnapshot::new_for_backend(
        DatabaseBackend::Sqlite,
        "data/app.db".to_string(),
        1,
        vec![DbTableSchema {
            name: "users".to_string(),
            engine: None,
            columns: vec![
                DbColumnSchema {
                    name: "id".to_string(),
                    logical_type: "i64".to_string(),
                    nullable: false,
                    ordinal: 0,
                    default: None,
                    native_type: Some("INTEGER".to_string()),
                },
                DbColumnSchema {
                    name: "name".to_string(),
                    logical_type: "string".to_string(),
                    nullable: false,
                    ordinal: 1,
                    default: None,
                    native_type: Some("TEXT".to_string()),
                },
            ],
            primary_key: vec!["id".to_string()],
            unique: Vec::new(),
            foreign_keys: Vec::new(),
            indexes: Vec::new(),
            triggers: Vec::new(),
        }],
    );
    let source = "@model(\"users\", { timestamps: false }) export interface User { name: string; }";
    let analysis = analyze_with_context(
        source,
        AnalyzeContext {
            db_schema: Some(&schema),
            db_backend: Some(&DatabaseBackend::Sqlite),
            ..AnalyzeContext::default()
        },
    );

    assert!(
        analysis.diagnostics.is_empty(),
        "SQLite Models may omit the generated id field: {:#?}",
        analysis.diagnostics
    );
}

#[test]
fn accepts_none_as_explicit_optional_absence() {
    assert_well_typed(
        r#"
            interface User { name: string; }
            const absent: User? = None;
            const find = (id: i32): User? => {
                if (id === 1) { return { name: "Ada" }; }
                return None;
            };
            const result = find(2);
            if (result) { console.log(result.name); }
        "#,
    );
}

#[test]
fn accepts_result_success_error_propagation_match_and_void_success() {
    assert_well_typed(
        r#"
            function load(): Result<i32, string> { return Ok(7); }
            function fail(): Result<i32, string> { return Err("failed"); }
            function use_result(): Result<i32, string> {
                let value = load()?;
                return Ok(value);
            }
            function done(): Result<void, string> { return Ok(); }
            function narrowWithIf(result: Result<i32, string>): Result<i32, string> {
                if (result) {
                    return Ok(result);
                } else {
                    return Err(result);
                }
            }
            const main = (): void => {
                let result = fail();
                match (result) {
                    Ok(value) => { console.log(value); }
                    Err(error) => { console.log(error); }
                }
            };
        "#,
    );
}

#[test]
fn accepts_result_in_generic_and_async_function_returns() {
    assert_well_typed(
        r#"
            type Outcome<T> = Result<T, string>;
            function identity<T>(value: T): Result<T, string> {
                return Ok(value);
            }
            function load(): Outcome<i32> { return Ok(7); }
            function pass(): Outcome<i32> {
                return Ok(load()?);
            }
            async function passAsync(): Promise<Outcome<i32>> {
                const value = load()?;
                return Ok(value);
            }
            const result: Result<i32, string> = identity<i32>(7);
        "#,
    );
}

#[test]
fn accepts_none_as_an_optional_match_pattern() {
    assert_well_typed(
        r#"
            const describe = (value: string?): void => {
                match (value) {
                    None => { console.log("missing"); }
                    text => { console.log(text); }
                }
            };
            describe(None);
        "#,
    );
}

#[test]
fn accepts_well_typed_language_constructs() {
    assert_well_typed(
        r#"
            const add = (left: i64, right: i64): i64 => left + right;
            const total: i64 = add(20, 22);
            const ratio: f32 = 1.5;
            const message: string = "total: " + "42";
            interface Message {
                text?: string;
            }
            const entry: Message = {};
            const optional: string? = entry.text;
            console.log(total);
            console.log(ratio);
            console.log(message);
            console.log(optional ?? "none");
        "#,
    );
}

#[test]
fn accepts_typed_numeric_literals_and_infers_their_exact_types() {
    assert_well_typed(
        r#"
            const byte = 10u8;
            const wide = 1000u64;
            const ratio = 3.14f32;
            const values = [0u64, 1u64, 2u64];
            const converted: i32 = 10u8 as i32;
            console.log(byte, wide, ratio, values, converted);
        "#,
    );
}

#[test]
fn accepts_nominal_enum_and_member_access() {
    assert_well_typed(
        r#"
            enum Status { Success, Error, Pending }
            const s: Status = Status.Success;
            const ok: boolean = s === Status.Success;
            console.log(s as i32);
        "#,
    );
}

#[test]
fn accepts_primitive_enums_inferred() {
    assert_well_typed(
        r#"
            enum HttpStatus { Ok = 200, NotFound = 404 }
            enum Role { Admin = "admin", User = "user" }
            enum Flag { Enabled = true, Disabled = false }
            const code: HttpStatus = HttpStatus.NotFound;
            const role: Role = Role.Admin;
            const flag: Flag = Flag.Enabled;
            console.log(code as i32);
            console.log(role as string);
            console.log(flag as boolean);
        "#,
    );
}

#[test]
fn accepts_explicitly_typed_enum() {
    assert_well_typed(
        r#"
            enum HttpStatus: i64 { Ok = 200, NotFound = 404 }
            const code: HttpStatus = HttpStatus.Ok;
            console.log(code as i64);
        "#,
    );
}

#[test]
fn accepts_tuple_operations() {
    assert_well_typed(
        r#"
            const sum = (p: &[i32, i32]): i32 => p[0] + p[1];
            const pair: [i32, string] = [1, "one"];
            const copy: [i32, i32] = [3, 4];
            const same: [i32, i32] = copy; // Copy: tuple of Copy types
            console.log(copy[0]);
            console.log(same[0]);
            console.log(pair);
            console.log(sum(&copy));
            let mutable: [i32, string] = [1, "a"];
            mutable[0] = 5;
            mutable[1] = "b";
            console.log(mutable);
        "#,
    );
}

#[test]
fn accepts_wildcard_discard_in_array_and_tuple_destructuring() {
    assert_well_typed(
        r#"
            const pair = (): [i32, string] => [1, "a"];
            const main = (): void => {
                const [code, _] = pair();
                console.log(code);
                const [_, message] = pair();
                console.log(message);
                const [_, _] = pair();
                const [first, _] = [10, 20];
                console.log(first);
            };
        "#,
    );
}

#[test]
fn accepts_string_format_specifiers_in_template_interpolations() {
    assert_well_typed(
        r#"
            const main = (): void => {
                const pi: f64 = 3.141592;
                const id: i32 = 42;
                console.log(`Valor: ${pi:.2f} | Codigo: ${id:04d}`);
                console.log(`Mistura: ${pi:.1f} e ${id:02d}`);
                console.log(`Sem spec: ${pi} ${id}`);
            };
        "#,
    );
}

#[test]
fn accepts_shorthand_object_fields() {
    assert_well_typed(
        r#"
            interface User {
                name: string;
                age: i32;
            }
            const main = (): void => {
                const name: string = "Neo";
                const age: i32 = 34;
                const user: User = { name, age };
                console.log(user.name);
                console.log(user.age);
                const updated: User = { ...user, age };
                console.log(updated.age);
            };
        "#,
    );
}

#[test]
fn accepts_bitwise_operators() {
    assert_well_typed(
        r#"
            const main = (): void => {
                const a: i32 = 5 & 3;
                const b: i32 = a | 2;
                const c: i32 = b ^ 1;
                const d: i32 = c << 2;
                const e: i32 = d >> 1;
                const f: i32 = e >>> 1;
                const g: i32 = ~f;
                let h: i32 = 5;
                h &= 3;
                h |= 2;
                h ^= 1;
                h <<= 1;
                h >>= 1;
                h >>>= 1;
                console.log(g);
                console.log(h);
            };
        "#,
    );
}

#[test]
fn accepts_recursion_and_exhaustive_returns() {
    assert_well_typed(
        r#"
            const factorial = (n: i32): i32 => {
                if (n <= 1) {
                    return 1;
                } else {
                    return n * factorial(n - 1);
                }
            };
            console.log(factorial(5));
        "#,
    );
}

#[test]
fn accepts_lexical_shadowing() {
    assert_well_typed(
        r#"
            const main = (): void => {
                const value: i32 = 1;
                if (true) {
                    const value: string = "inner";
                    console.log(value);
                }
                console.log(value);
            };
        "#,
    );
}

#[test]
fn accepts_optional_coalescing_in_branches() {
    assert_well_typed(
        r#"
            const show = (value: string?): void => {
                console.log(value ?? "none");
            };
        "#,
    );
}

#[test]
fn accepts_logical_assignments() {
    assert_well_typed(
        r#"
            const main = (): void => {
                let active: boolean = true;
                active &&= false;
                active ||= true;
                let name: string? = "Ada";
                name ??= "guest";
                console.log(active);
                console.log(name ?? "missing");
            };
        "#,
    );
}

#[test]
fn accepts_ternary_with_nullish_else_branch() {
    assert_well_typed(
        r#"
            interface Settings {
                fallback?: i32;
            }

            const settings: Settings = {};
            const fallback: i32? = settings.fallback;
            const value: i32 = true ? 1 : fallback ?? 2;
            console.log(value);
        "#,
    );
}

#[test]
fn accepts_interface_typed_objects() {
    assert_well_typed(
        r#"
            interface User {
                name: string;
                active: boolean;
            }
            const user: User = {
                name: 'Ada',
                active: true
            };
            console.log(user.name);
        "#,
    );
}

#[test]
fn accepts_extension_methods_at_their_declared_receiver_type() {
    assert_well_typed(
        r#"
        interface User { age: u32; }
        interface UserMethods at User {
            isAdult(user: User): boolean { return user.age >= 18; }
        }
        const main = (): void => {
            const user: User = { age: 30 };
            console.log(user.isAdult());
        };
        "#,
    );
}

#[test]
fn accepts_a_named_handler_whose_reply_unions_the_declared_responses() {
    assert_well_typed(
        r#"
            interface HttpReply<T> {}
            interface HttpRouter {}
            interface UserParams { id: string; }
            interface UserResponse { id: string; }
            interface ApiError { code: string; message: string; }
            type GetUserRequest = { params: UserParams };
            enum HttpStatus { Ok = 200, NotFound = 404 }
            const getUser = (request: GetUserRequest, reply: HttpReply<UserResponse | ApiError>): void => {
                if (request.params.id === "user-404") {
                    return reply.status(HttpStatus.NotFound).send({ code: "user_not_found", message: "User not found" });
                }
                reply.status(HttpStatus.Ok).send({ id: request.params.id });
            };
            const main = (): void => {
                const app: HttpRouter = __tyrus_http_createRouter();
                app.get("/users/:id", {
                    params: UserParams,
                    response: { 404: ApiError, 200: UserResponse },
                    handler: getUser
                });
            };
        "#,
    );
}

#[test]
fn accepts_array_and_primitive_http_response_schemas() {
    assert_well_typed(
        r#"
            interface HttpReply<T> {}
            interface HttpRouter {}
            interface User { id: i64; name: string; }
            interface Health { ok: boolean; }
            interface ApiError { code: string; message: string; }
            enum HttpStatus { Ok = 200, InternalServerError = 500 }
            const listUsers = (request: {}, reply: HttpReply<User[]>): void => {
                reply.status(HttpStatus.Ok).send([]);
            };
            const getHealth = (request: {}, reply: HttpReply<string>): void => {
                reply.status(HttpStatus.Ok).send("ok");
            };
            const listUsersWithError = (
                request: {},
                reply: HttpReply<User[] | ApiError>
            ): void => {
                reply.status(HttpStatus.Ok).send([]);
            };
            const main = (): void => {
                const app: HttpRouter = __tyrus_http_createRouter();
                app.get("/users", { response: { 200: User[] }, handler: listUsers });
                app.get("/users-with-error", {
                    response: { 200: User[], 500: ApiError },
                    handler: listUsersWithError
                });
                app.get("/health", { response: { 200: string }, handler: getHealth });
                app.get("/health-object", {
                    response: { 200: Health },
                    handler: (request, reply) => reply.status(HttpStatus.Ok).send({ ok: true })
                });
            };
        "#,
    );
}

#[test]
fn accepts_optional_scalar_fields_in_a_request_schema() {
    assert_well_typed(
        r#"
            interface HttpReply<T> {}
            interface HttpRouter {}
            interface CreateBody {
                name: string;
                nickname?: string;
                age?: i32;
            }
            interface CreatedResponse { name: string; }
            enum HttpStatus { Created = 201 }
            const main = (): void => {
                const app: HttpRouter = __tyrus_http_createRouter();
                app.post("/users", {
                    body: CreateBody,
                    response: { 201: CreatedResponse },
                    handler: (request, reply) => {
                        reply.status(HttpStatus.Created).send({ name: request.body.name });
                    }
                });
            };
        "#,
    );
}

#[test]
fn accepts_narrowing_an_optional_field_through_an_immutable_root() {
    assert_well_typed(
        r#"
            interface Inner { name?: string; }
            interface Outer { inner: Inner; }
            const take = (value: string): string => value;
            const main = (): void => {
                const outer: Outer = { inner: { name: "Ada" } };
                if (outer.inner.name) {
                    console.log(take(outer.inner.name));
                }
            };
        "#,
    );
}

#[test]
fn accepts_http_router_configuration() {
    assert_well_typed(
        r#"
            interface ServerOptions { host?: string; requestTimeout?: string; maxConcurrentRequests?: i32; headerTimeout?: string; maxHeaderSize?: string; maxConnections?: i32; keepAlive?: boolean; tls?: TlsOptions; }
            interface RateLimit { max: i32; timeWindow: string; }
            interface TlsOptions { cert: string; key: string; }
            interface HttpError { code: string; message: string; }
            interface HttpServer {}
            interface CorsOptions { origin: string[]; methods: string[]; allowedHeaders: string[]; exposedHeaders: string[]; credentials: boolean; maxAge: i32; }
            interface HttpReply<T> {}
            interface HttpRouter {}
            interface HealthRequest {}
            interface HealthResponse {
                status: string;
                role: Role;
                level: Level;
            }
            enum Role { Admin = "admin", Member = "member" }
            enum Level { Low = 1, High = 9 }
            enum HttpStatus { Ok = 200 }
            const health = (request: HealthRequest, reply: HttpReply<HealthResponse>): void => {
                reply.status(HttpStatus.Ok).send({ status: "ok", role: Role.Admin, level: Level.High });
            };
            const app: HttpRouter = __tyrus_http_createRouter();
            app.trustedProxies(["10.0.0.0/8", "127.0.0.1", "::1/128"], "x-client-ip");
            app.cors({
                origin: ["http://localhost:5173"],
                methods: ["GET", "POST", "OPTIONS", "QUERY"],
                allowedHeaders: ["Content-Type", "Authorization"],
                exposedHeaders: [],
                credentials: true,
                maxAge: 3600
            });
            app.get("/health", {
                response: { 200: HealthResponse },
                config: {
                    bodyLimit: "5mb",
                    rateLimit: { max: 20, globalMax: 100, ban: 3, banWindow: "15 minutes", timeWindow: "1 minute" }
                },
                handler: health
            });
            app.post("/health", {
                response: { 200: HealthResponse },
                config: { bodyLimit: "off" },
                handler: health
            });
            app.rateLimit({ max: 100, globalMax: 500, ban: 3, banWindow: "15 minutes", timeWindow: "1 minute" });
            app.listen(3000, { host: "127.0.0.1", requestTimeout: "30s", maxConcurrentRequests: 1024, headerTimeout: "10s", maxHeaderSize: "32kib", maxConnections: 512, keepAlive: true, tls: { cert: "certs/server.crt", key: "certs/server.key" } });
            app.head("/health", { response: { 200: HealthResponse }, handler: health });
            app.options("/health", { response: { 200: HealthResponse }, handler: health });
            app.connect("/health", { response: { 200: HealthResponse }, handler: health });
            app.query("/health", { response: { 200: HealthResponse }, handler: health });
            app.trace("/health", { response: { 200: HealthResponse }, handler: health });
            app.listen(3000);
        "#,
    );
}

#[test]
fn accepts_the_http_server_lifecycle_handle() {
    assert_well_typed(
        r#"
            interface ServerOptions { host?: string; requestTimeout?: string; maxConcurrentRequests?: i32; shutdownTimeout?: string; }
            interface HttpError { code: string; message: string; }
            interface HttpServer {}
            interface HttpReply<T> {}
            interface HttpRouter {}
            interface HealthResponse { status: string; }
            enum HttpStatus { Ok = 200 }
            const health = (request: HealthRequest, reply: HttpReply<HealthResponse>): void => {
                reply.status(HttpStatus.Ok).send({ status: "ok" });
            };
            interface HealthRequest {}
            const run = async (): Promise<void> => {
                const app: HttpRouter = __tyrus_http_createRouter();
                app.get("/health", { response: { 200: HealthResponse }, handler: health });
                const server = app.listen(3000, { host: "127.0.0.1", shutdownTimeout: "10s" }) onfail {
                    printError(server.code);
                };
                console.log(server.port());
                await server.close();
            };
        "#,
    );
}

#[test]
fn accepts_http_route_pre_handlers() {
    assert_well_typed(
        r#"
            interface HttpRequest { method: string; path: string; body: string; params: string; query: string; headers: string; cookies: string; }
            interface HttpResponse { statusCode: HttpStatus; body: string; cookies: string; }
            interface HttpReply<T> {}
            interface HttpRouter {}
            interface HealthResponse { status: string; }
            enum HttpStatus { Ok = 200, Unauthorized = 401 }
            function requireApiKey(request: HttpRequest): HttpResponse | true {
                if (request.headers.includes("x-api-key")) { return true; }
                return { statusCode: HttpStatus.Unauthorized, body: "Unauthorized", cookies: "" };
            }
            const health = (request: {}, reply: HttpReply<HealthResponse>): void => {
                reply.status(HttpStatus.Ok).send({ status: "ok" });
            };
            const app: HttpRouter = __tyrus_http_createRouter();
            app.get("/health", {
                response: { 200: HealthResponse },
                preHandler: [requireApiKey],
                handler: health
            });
        "#,
    );
}

#[test]
fn accepts_named_http_response_hooks() {
    assert_well_typed(
        r#"
            interface HttpRouter {}
            interface HttpAccessLog { readonly requestId: string; readonly method: string; readonly path: string; readonly status: HttpStatus; readonly duration: Duration; }
            interface Duration { readonly milliseconds: i64; }
            enum HttpStatus { Ok = 200 }
            function recordAccess(access: HttpAccessLog): void {
                console.log(access.requestId, access.duration.milliseconds);
            }
            const app: HttpRouter = __tyrus_http_createRouter();
            app.onResponse(recordAccess);
        "#,
    );
}

#[test]
fn accepts_named_and_inferred_inline_async_http_reply_handlers() {
    assert_well_typed(
        r#"
            interface HttpReply<T> {}
            interface HttpRouter {}
            interface HealthResponse {
                status: string;
            }
            enum HttpStatus { Ok = 200 }
            async function named(request: {}, reply: HttpReply<HealthResponse>): Promise<void> {
                await Promise.resolve();
                reply.status(HttpStatus.Ok).send({ status: "named" });
            }
            const app: HttpRouter = __tyrus_http_createRouter();
            app.get("/named", {
                response: { 200: HealthResponse },
                handler: named
            });
            app.get("/inline", {
                response: { 200: HealthResponse },
                handler: async (request, reply) => {
                    await Promise.resolve();
                    reply.status(HttpStatus.Ok).send({ status: "inline" });
                }
            });
        "#,
    );
}

#[test]
fn accepts_http_router_schema_handlers() {
    assert_well_typed(
        r#"
            interface HttpReply<T> {}
            interface HttpRouter {}
            interface CreateUserRequest {
                @notEmpty()
                name: string;
                @min(18)
                age: i32;
                tags: string[];
                weights: decimal[];
            }
            interface UserResponse {
                @notEmpty()
                id: string;
                name: string;
                adult: boolean;
            }
            type CreateUserRouteRequest = {
                body: CreateUserRequest;
            };
            enum HttpStatus { Created = 201 }
            const createUser = (request: CreateUserRouteRequest, reply: HttpReply<UserResponse>): void => {
                console.log(request.body.tags.length);
                reply.status(HttpStatus.Created).send({ id: "user-1", name: request.body.name, adult: true });
            };
            const app: HttpRouter = __tyrus_http_createRouter();
            app.post("/users", {
                body: CreateUserRequest,
                response: { 201: UserResponse },
                handler: createUser
            });
        "#,
    );
}

#[test]
fn accepts_http_router_inferred_route_request_handlers() {
    assert_well_typed(
        r#"
            interface HttpReply<T> {}
            interface HttpRouter {}
            interface CreateUserBody {
                name: string;
            }
            interface UserParams {
                id: i64;
            }
            interface AuthHeaders {
                authorization: string;
            }
            interface UserResponse {
                id: string;
                name: string;
            }
            interface DeleteParams {
                id: string;
            }
            enum HttpStatus { Created = 201 }
            const app: HttpRouter = __tyrus_http_createRouter();
            app.post("/users/:id", {
                params: UserParams,
                headers: AuthHeaders,
                body: CreateUserBody,
                response: { 201: UserResponse },
                handler: (request, reply) => {
                    const id: i64 = request.params.id;
                    const token: string = request.headers.authorization;
                    console.log(token);
                    reply.status(HttpStatus.Created).send({ id: "user-1", name: request.body.name });
                }
            });
        "#,
    );
}

#[test]
fn accepts_http_reply_status_send_handlers() {
    assert_well_typed(
        r#"
            interface HttpReply<T> {}
            interface HttpRouter {}
            enum HttpStatus { Ok = 200, Created = 201, NoContent = 204 }
            interface CreateUserBody {
                name: string;
            }
            interface UserResponse {
                id: string;
                name: string;
            }
            interface DeleteParams {
                id: string;
            }
            const app: HttpRouter = __tyrus_http_createRouter();
            app.post("/users", {
                body: CreateUserBody,
                response: { 201: UserResponse, 204: void },
                handler: (request, reply) => {
                    reply.status(HttpStatus.Created).send({
                        id: "user-1",
                        name: request.body.name
                    });
                }
            });
            app.delete("/users/:id", {
                params: DeleteParams,
                response: { 204: void },
                handler: (request, reply) => {
                    reply.status(HttpStatus.NoContent).send();
                }
            });
        "#,
    );
}

#[test]
fn accepts_optional_scalar_http_response_fields() {
    assert_well_typed(
        r#"
            interface HttpReply<T> {}
            interface HttpRouter {}
            enum HttpStatus { Ok = 200 }
            interface UserResponse {
                id: string;
                nickname?: string;
            }
            const app: HttpRouter = __tyrus_http_createRouter();
            app.get("/users/1", {
                response: { 200: UserResponse },
                handler: (request, reply) => {
                    reply.status(HttpStatus.Ok).send({ id: "user-1" });
                }
            });
        "#,
    );
}

#[test]
fn accepts_http_reply_cookie_metadata_before_status_send() {
    assert_well_typed(
        r#"
            interface HttpReply<T> {}
            interface HttpRouter {}
            interface CookieOptions { domain?: string; path?: string; maxAge?: i64; secure?: boolean; httpOnly?: boolean; sameSite?: string; }
            interface HttpCookie { name: string; value: string; options?: CookieOptions; }
            enum HttpStatus { Ok = 200 }
            interface UserResponse {
                id: string;
            }
            const app: HttpRouter = __tyrus_http_createRouter();
            app.get("/users", {
                response: { 200: UserResponse },
                handler: (request, reply) => {
                    reply
                        .setCookie("session", "abc", { path: "/", secure: true, httpOnly: true, sameSite: "lax" })
                        .clearCookie("preview")
                        .setCookies([{ name: "theme", value: "dark" }, { name: "lang", value: "en", options: { path: "/" } }])
                        .clearCookies()
                        .header("x-request-id", "request-42")
                        .appendHeader("link", "</users?page=2>; rel=next")
                        .removeHeader("x-unused")
                        .type("application/json; charset=utf-8")
                        .status(HttpStatus.Ok)
                        .send({ id: "user-1" });
                }
            });
        "#,
    );
}

#[test]
fn accepts_application_error_reply_paths() {
    assert_well_typed(
        r#"
            interface HttpReply<T> {}
            interface HttpRouter {}
            enum HttpStatus { Ok = 200, NotFound = 404, Conflict = 409 }
            interface UserResponse {
                id: string;
            }
            interface ApiError {
                code: string;
                message: string;
            }
            const app: HttpRouter = __tyrus_http_createRouter();
            app.get("/users", {
                response: { 200: UserResponse, 404: ApiError },
                handler: (request, reply) => {
                    const missing: boolean = true;
                    if (missing) {
                        return reply.status(HttpStatus.NotFound).send({ code: "not_found", message: "User not found" });
                    }
                    reply.status(HttpStatus.Ok).send({ id: "user-1" });
                }
            });
            app.post("/users", {
                response: { 200: UserResponse, 409: ApiError },
                handler: (request, reply) => {
                    return reply.status(HttpStatus.Conflict).send({ code: "email_in_use", message: "Email already exists" });
                }
            });
        "#,
    );
}

#[test]
fn accepts_http_reply_branch_flow_handlers() {
    assert_well_typed(
        r#"
            interface HttpReply<T> {}
            interface HttpRouter {}
            enum HttpStatus { Ok = 200, NotFound = 404 }
            interface UserResponse {
                id: string;
            }
            const app: HttpRouter = __tyrus_http_createRouter();
            app.get("/users", {
                response: { 200: UserResponse },
                handler: (request, reply) => {
                    const invalid: boolean = false;
                    if (invalid) {
                        console.log("invalid");
                    }
                    reply.status(HttpStatus.Ok).send({ id: "user-1" });
                }
            });
            app.get("/fallback", {
                response: { 200: UserResponse },
                handler: (request, reply) => {
                    const cached: boolean = true;
                    if (cached) {
                        console.log("cached");
                    }
                    reply.status(HttpStatus.Ok).send({ id: "fresh" });
                }
            });
            app.get("/return", {
                response: { 200: UserResponse },
                handler: (request, reply) => {
                    const invalid: boolean = false;
                    if (invalid) {
                        console.log("invalid");
                    }
                    return reply.status(HttpStatus.Ok).send({ id: "ok" });
                }
            });
        "#,
    );
}

#[test]
fn accepts_typed_arrays_indexing_and_length() {
    assert_well_typed(
        r#"
            const names: string[] = ["Ada", "Linus"];
            const first: string = names[0];
            const count: usize = names.length;
            const capacity: usize = names.capacity;
            const empty: Array<string> = [];
            let mutable: i32[] = [1, 2];
            mutable[0] = 3;
            console.log(first);
            console.log(count);
            console.log(capacity);
        "#,
    );
}

#[test]
fn accepts_generic_tuple_optional_and_range_type_annotations() {
    assert_well_typed(
        r#"
            const pair: Tuple<string, i32> = ["one", 1];
            const first: string = pair[0];
            const count: Optional<i32> = 1;
            const span: Range<i32> = 1..4;
            const containsTwo: boolean = 2 in span;
            console.log(first);
            console.log(count ?? 0);
            console.log(containsTwo);
        "#,
    );
}

#[test]
fn accepts_in_bounds_and_unprovable_array_indices() {
    assert_well_typed(
        r#"
            const a: i32[] = [1, 2, 3];
            console.log(a[2]);

            let grown: i32[] = [1, 2, 3];
            grown.push(4);
            console.log(grown[3]);

            let reassigned: i32[] = [1, 2, 3];
            reassigned = [1, 2, 3, 4, 5];
            console.log(reassigned[4]);

            const read = (xs: i32[], i: i32): void => {
                console.log(xs[i]);
                console.log(xs[99]);
            };
            read(a, 0);
        "#,
    );
}

#[test]
fn array_length_tracking_stays_sound() {
    assert_well_typed(
        r#"
            const conditional = (flag: boolean): void => {
                let xs: i32[] = [1, 2, 3, 4, 5];
                if (flag) {
                    xs = [1];
                }
                console.log(xs[3]);
            };

            const looped = (): void => {
                let xs: i32[] = [1, 2, 3];
                for (let i: i32 = 0; i < 3; i++) {
                    xs.push(i);
                }
                console.log(xs[3]);
            };

            const captured = (): void => {
                let xs: i32[] = [1, 2, 3];
                const grow = (): void => {
                    xs.push(9);
                };
                grow();
                console.log(xs[3]);
            };

            conditional(true);
            looped();
            captured();
        "#,
    );
}

#[test]
fn accepts_typescript_style_array_methods() {
    assert_well_typed(
        r#"
            let values: i32[] = [1, 2, 3];
            const isArray: boolean = Array.isArray(values);
            const isNotArray: boolean = Array.isArray("values");
            const pushed: usize = values.push(4, 5);
            const unshifted: usize = values.unshift(0);
            const includes: boolean = values.includes(3);
            const firstIndex: i64 = values.indexOf(2);
            const lastIndex: i64 = values.lastIndexOf(2);
            const part: i32[] = values.slice(1, 3);
            const combined: i32[] = values.concat(part);
            values.reverse();
            values.fill(9, 0, 1);
            values.copyWithin(1, 2, 3);
            const reversed: i32[] = values.toReversed();
            const replaced: i32[] = values.with(0, 7);
            const random: i32? = values.random();
            const shuffled: i32[] = values.shuffle();
            const sample: i32[] = values.sample(2);
            const repeated: i32[] = values.sample(2, { repeat: true });
            const words: string[] = ["tyrus", "array"];
            const shuffledWords: string[] = words.shuffle();
            const joined: string = words.join("-");
        "#,
    );
}

#[test]
fn accepts_explicit_and_inferred_generic_calls() {
    assert_well_typed(
        r#"
            const identity = <T>(value: T): T => value;
            const explicit: i32 = identity<i32>(42);
            const inferred: string = identity("tyrus");
        "#,
    );
}

#[test]
fn accepts_contextually_typed_callbacks_and_functional_array_methods() {
    assert_well_typed(
        r#"
            const transform: (i32) => i32 = value => value * 2;
            const values: i32[] = [1, 2, 3, 4];
            const mapped: i32[] = values.map((value, index) => {
                console.log(index);
                return value + 1;
            });
            const filtered: i32[] = mapped.filter(value => value > 2);
            const some: boolean = filtered.some(value => value === 4);
            const every: boolean = filtered.every(value => value > 0);
            const found: i32? = filtered.find(value => value === 4);
            const last: i32? = filtered.findLast(value => value > 0);
            const index: i64 = filtered.findIndex(value => value === 4);
            const lastIndex: i64 = filtered.findLastIndex(value => value > 0);
            filtered.forEach(value => { console.log(value); });
            const total: i32 = filtered.reduce((sum, value) => sum + value, 0);
            const right: i32 = filtered.reduceRight((sum, value) => sum - value);
            let mutable: i32[] = [3, 1, 2];
            mutable.sort((left, right) => left - right);
            const sorted: i32[] = mutable.toSorted((left, right) => right - left);
            const nested: i32[][] = [[1], [2, 3]];
            const flat: i32[] = nested.flat();
            const expanded: i32[] = values.flatMap(value => [value, value]);
            const chained: i32[] = values.map(transform).filter(value => value > 2);
        "#,
    );
}

#[test]
fn accepts_string_length_and_methods() {
    assert_well_typed(
        r#"
            const name: string = "tyrus";
            const len: usize = name.length;
            const ch: string = name.charAt(0);
            const code: i32 = name.charCodeAt(1);
            const found: boolean = name.includes("r");
            const starts: boolean = name.startsWith("rs");
            const ends: boolean = name.endsWith("ts");
            const idx: i64 = name.indexOf("s");
            const last: i64 = name.lastIndexOf("s");
            const sliced: string = name.slice(0, 2);
            const sub: string = name.substring(1, 3);
            const trimmed: string = name.trim();
            const leftTrimmed: string = name.trimStart();
            const rightTrimmed: string = name.trimEnd();
            const lower: string = name.toLowerCase();
            const upper: string = name.toUpperCase();
            const replaced: string = name.replace("r", "R");
            const regexMatch: string[]? = name.match(/r(st)/);
            const regexMatchAll: string[][] = name.matchAll(/[rs]/);
            const regexSearch: i32 = name.search(/st/);
            const regexReplaced: string = name.replace(/s+/, "S");
            const regexSplit: string[] = name.split(/s/);
            const repeated: string = name.repeat(3);
            const padded: string = name.padStart(10, "-");
            const paddedEnd: string = name.padEnd(10, "-");
            const concated: string = name.concat(" is", " great");
            console.log(len);
            console.log(ch);
            console.log(code);
            console.log(trimmed);
            console.log(lower);
            console.log(upper);
            console.log(replaced);
            if (regexMatch) {
                console.log(regexMatch[0]);
                console.log(regexMatch[1]);
            }
            console.log(regexSearch);
            console.log(regexReplaced);
            console.log(repeated);
            console.log(padded);
            console.log(paddedEnd);
            console.log(concated);
        "#,
    );
}

#[test]
fn accepts_else_if_chaining() {
    assert_well_typed(
        r#"
            const score: i32 = 75;
            let grade: string = "";
            if (score >= 90) {
                grade = "A";
            } else if (score >= 80) {
                grade = "B";
            } else if (score >= 70) {
                grade = "C";
            } else {
                grade = "D";
            }
            console.log(grade);
        "#,
    );
}

#[test]
fn accepts_unary_minus() {
    assert_well_typed(
        r#"
            const neg: i32 = -1;
            const negFloat: f64 = -2.5;
            const value: i64 = 42;
            const result: i64 = -value;
            const combined: i64 = -value + 10;
        "#,
    );
}

#[test]
fn accepts_string_length_on_literal() {
    assert_well_typed(
        r#"
            const len: usize = "hello".length;
        "#,
    );
}

#[test]
fn accepts_unknown_type() {
    assert_well_typed(
        r#"
            const a: unknown = 42;
            const b: unknown = "hello";
            const c: unknown = true;
            const e: unknown = 3.14;
        "#,
    );
}

#[test]
fn accepts_literal_types() {
    assert_well_typed(
        r#"
            const a: "hello" = "hello";
            const b: 42 = 42;
            const c: true = true;
            const d: false = false;
        "#,
    );
}

#[test]
fn accepts_common_union_fields() {
    assert_well_typed(
        r#"
            type Shape =
                | { kind: "circle"; radius: f64 }
                | { kind: "square"; size: f64 };

            const shape: Shape = { kind: "circle", radius: 2.0 };
            console.log(shape.kind);
        "#,
    );
}

#[test]
fn accepts_readonly_field() {
    assert_well_typed(
        r#"
            interface Config {
                readonly name: string;
                value: i32;
            }
            const cfg: Config = {
                name: "fixed",
                value: 42
            };
        "#,
    );
}

#[test]
fn narrows_unknown_to_the_type_a_typeof_category_proves() {
    assert_well_typed(
        r#"
            const main = (): void => {
                const value: unknown = "hello";
                if (typeof value === "string") {
                    const size: usize = value.length;
                }
                if ("string" === typeof value) {
                    const upper: string = value.toUpperCase();
                }
                if (typeof value !== "string") {
                    console.log("other");
                } else {
                    const size: usize = value.length;
                }
                if (typeof value === "boolean") { const v: boolean = !value; }
                if (typeof value === "char") { const v: char = value; }
                if (typeof value === "i8") { const v: i8 = value; }
                if (typeof value === "i16") { const v: i16 = value; }
                if (typeof value === "i32") { const v: i32 = value; }
                if (typeof value === "i64") { const v: i64 = value; }
                if (typeof value === "i128") { const v: i128 = value; }
                if (typeof value === "isize") { const v: isize = value; }
                if (typeof value === "u8") { const v: u8 = value; }
                if (typeof value === "u16") { const v: u16 = value; }
                if (typeof value === "u32") { const v: u32 = value; }
                if (typeof value === "u64") { const v: u64 = value; }
                if (typeof value === "u128") { const v: u128 = value; }
                if (typeof value === "usize") { const v: usize = value; }
                if (typeof value === "f32") { const v: f32 = value; }
                if (typeof value === "f64") { const v: f64 = value; }
                if (typeof value === "f128") { const v: f128 = value; }
                if (typeof value === "decimal") { const v: decimal = value; }
            };
        "#,
    );
}

#[test]
fn accepts_scalar_and_scalar_union_values_in_unknown() {
    assert_well_typed(
        r#"
            const main = (): void => {
                const either: string | i32 = 3;
                const wide: unknown = either;
                const status: "active" | "inactive" = "active";
                const label: unknown = status;
                const letter: char = 'a';
                const held: unknown = letter;
                const price: unknown = 1.5d;
            };
        "#,
    );
}

#[test]
fn keeps_typeof_narrowing_of_known_unions_in_both_branches() {
    assert_well_typed(
        r#"
            const main = (): void => {
                const value: string | i32 | boolean = 4;
                if (typeof value !== "string") {
                    if (typeof value === "i32") {
                        const n: i32 = value;
                    } else {
                        const flag: boolean = value;
                    }
                } else {
                    const text: string = value;
                }
            };
        "#,
    );
}

#[test]
fn accepts_array_destructure_param() {
    assert_well_typed(
        r#"
            const main = (): void => {
                const f = ([a, b]: i32[]): i32 => a + b;
                const r: i32 = f([1, 2]);
            };
        "#,
    );
}

#[test]
fn accepts_object_destructure_param_with_named_interface() {
    assert_well_typed(
        r#"
            interface Point { x: i32; y: i32 }
            const main = (): void => {
                const f = ({x, y}: Point): i32 => x + y;
                const r: i32 = f({x: 1, y: 2});
            };
        "#,
    );
}

#[test]
fn accepts_object_destructure_param_with_inline_type() {
    assert_well_typed(
        r#"
            const main = (): void => {
                const f = ({x, y}: {x: i32, y: i32}): i32 => x + y;
                const r: i32 = f({x: 1, y: 2});
            };
        "#,
    );
}

#[test]
fn accepts_named_fn_with_array_destructure_params() {
    assert_well_typed(
        r#"
            const sumPair = ([a, b]: i32[]): i32 => a + b;
            const main = (): void => {
                const r: i32 = sumPair([10, 20]);
            };
        "#,
    );
}

#[test]
fn accepts_named_fn_with_object_destructure_params() {
    assert_well_typed(
        r#"
            interface Config { name: string; limit: i32 }
            const greet = ({name, limit}: Config): void => {
                console.log(name);
            };
            const main = (): void => {
                greet({name: "test", limit: 5});
            };
        "#,
    );
}

#[test]
fn accepts_named_fn_with_inline_object_destructure_params() {
    assert_well_typed(
        r#"
            const greet = ({name, limit}: {name: string, limit: i32}): void => {
                console.log(name);
            };
            const main = (): void => {
                greet({name: "test", limit: 5});
            };
        "#,
    );
}

#[test]
fn accepts_type_assertions() {
    assert_well_typed(
        r#"
            const main = (): void => {
                const x: i32 = 42;
                const y: f64 = x as f64;
                const z: i32 = 3.14 as i32;
                const a: i32 = (x as f64) as i32;
                console.log(y);
                console.log(z);
                console.log(a);
            };
        "#,
    );
}

#[test]
fn accepts_decimal_as_a_primitive_type() {
    assert_well_typed(
        r#"
            const identity = (value: decimal): decimal => value;
        "#,
    );
}

#[test]
fn accepts_explicit_and_contextual_decimal_literals() {
    assert_well_typed(
        r#"
            const explicit = 0.1d;
            const contextual: decimal = 0.2;
        "#,
    );
}

#[test]
fn accepts_typeof_narrowing() {
    assert_well_typed(
        r#"
            const main = (): void => {
                const value: string | i32 = 42;
                if (typeof value === "string") {
                    console.log(value);
                } else if (typeof value === "i32") {
                    const numeric: i32 = value;
                    console.log(numeric);
                }
            };
        "#,
    );
}

#[test]
fn accepts_typeof_narrowing_with_more_than_two_variants() {
    assert_well_typed(
        r#"
            const main = (): void => {
                const value: string | i32 | boolean = 42;
                if (typeof value !== "string") {
                    if (typeof value === "i32") {
                        const numeric: i32 = value;
                        console.log(numeric);
                    } else {
                        const flag: boolean = value;
                        console.log(flag);
                    }
                } else {
                    const text: string = value;
                    console.log(text);
                }
            };
        "#,
    );
}

#[test]
fn accepts_typescript_style_tuples() {
    assert_well_typed(
        r#"
            const lengthSquared = (point: [f64, f64]): f64 => {
                const x: f64 = point[0];
                const y: f64 = point[1];
                return x * x + y * y;
            };

            const main = (): void => {
                const point: [f64, f64] = [3.0, 4.0];
                const [x, y] = point;
                console.log(x);
                console.log(y);
                console.log(lengthSquared(point));
            };
        "#,
    );
}

#[test]
fn accepts_tagged_union_discriminant_narrowing() {
    assert_well_typed(
        r#"
            type Shape =
                | { kind: "circle"; radius: f64 }
                | { kind: "square"; size: f64 }
                | { kind: "rect"; width: f64; height: f64 };

            const main = (): void => {
                const shape: Shape = { kind: "rect", width: 10.0, height: 5.0 };
                if (shape.kind === "circle") {
                    console.log(shape.radius);
                } else if (shape.kind === "square") {
                    console.log(shape.size);
                } else {
                    console.log(shape.width);
                    console.log(shape.height);
                }
            };
        "#,
    );
}

#[test]
fn accepts_expanded_tagged_union_value_as_alias_argument() {
    assert_well_typed(
        r#"
            type Shape =
                | { kind: "circle"; radius: f64 }
                | { kind: "square"; size: f64 }
                | { kind: "rect"; width: f64; height: f64 };

            const printShape = (shape: Shape): void => {
                console.log(shape.kind);
            };

            const main = (): void => {
                const shape: Shape = { kind: "rect", width: 10.0, height: 5.0 };
                printShape(shape);
            };
        "#,
    );
}

#[test]
fn accepts_negative_tagged_union_discriminant_narrowing() {
    assert_well_typed(
        r#"
            type Shape =
                | { kind: "circle"; radius: f64 }
                | { kind: "square"; size: f64 }
                | { kind: "rect"; width: f64; height: f64 };

            const main = (): void => {
                const shape: Shape = { kind: "rect", width: 10.0, height: 5.0 };
                if (shape.kind !== "circle") {
                    if (shape.kind === "square") {
                        console.log(shape.size);
                    } else {
                        console.log(shape.width);
                    }
                } else {
                    console.log(shape.radius);
                }
            };
        "#,
    );
}

#[test]
fn accepts_typeof_type_query() {
    assert_well_typed(
        r#"
            type MyType = typeof 42;
            const main = (): void => {
                const x: MyType = 42;
                console.log(x);
            };
        "#,
    );
}

#[test]
fn accepts_typeof_returning_string() {
    assert_well_typed(
        r#"
            const main = (): void => {
                const t: string = typeof 42;
                console.log(t);
            };
        "#,
    );
}

#[test]
fn accepts_destructure_param_with_mut() {
    assert_well_typed(
        r#"
            const main = (): void => {
                let f = (mut [a, b]: i32[]): void => {
                    a = a + 1;
                };
                f([1, 2]);
            };
        "#,
    );
}

fn env_fields(env_source: &str) -> Vec<tyrus_core::ast::Field> {
    let (vars, diagnostics) = tyrus_core::env::parse_env_source(env_source);
    assert!(
        diagnostics.is_empty(),
        "invalid env fixture: {diagnostics:?}"
    );
    tyrus_core::env::EnvSchema {
        env_file: ".env".to_string(),
        file_path: std::path::PathBuf::from(".env"),
        source: env_source.to_string(),
        vars,
    }
    .object_type()
}

fn assert_well_typed_with_env(env_source: &str, source: &str) {
    let tokens = tyrus_core::lexer::lex(source).unwrap();
    let program = Parser::new(tokens).parse_program().unwrap();
    let mut checker = TypeChecker::new();
    checker.set_env_schema(env_fields(env_source));
    let errors = checker
        .check_program(&program)
        .into_iter()
        .filter(|diagnostic| diagnostic.severity == CheckSeverity::Error)
        .collect::<Vec<_>>();
    assert!(errors.is_empty(), "unexpected errors: {errors:#?}");
}

#[test]
fn accepts_typed_process_env_access() {
    assert_well_typed_with_env(
        "@type(u32)\nPORT=3000\nTYRUS_ENV=development\n",
        r#"
            const environment: string = process.env.TYRUS_ENV;
            const port: u32 = process.env.PORT;
            const doubled: u32 = port * 2;
            console.log(environment);
            console.log(doubled);
        "#,
    );
}

#[test]
fn accepts_process_env_destructuring() {
    assert_well_typed_with_env(
        "@type(u32)\nPORT=3000\nTYRUS_ENV=development\n",
        r#"
            const { TYRUS_ENV, PORT } = process.env;
            const port: u32 = PORT;
            const environment: string = TYRUS_ENV;
            console.log(environment);
            console.log(port);
        "#,
    );
}

#[test]
fn accepts_user_declared_process_variable_shadowing_the_namespace() {
    assert_well_typed(
        r#"
            interface Job { env: string; }
            const process: Job = { env: "batch" };
            const label: string = process.env;
            console.log(label);
        "#,
    );
}

#[test]
fn accepts_pipeline_operator() {
    assert_well_typed(
        r#"
            const double = (n: i64): i64 => n * 2;
            const add = (a: i64, b: i64): i64 => a + b;
            const result: i64 = 5 |> double |> double;
            const summed: i64 = 3 |> add(4);
            const mixed: i64 = 5 |> double |> add(%, 4) |> double;
            console.log(result);
            console.log(summed);
            console.log(mixed);
        "#,
    );
}

#[test]
fn accepts_pipeline_to_console_log() {
    assert_well_typed(
        r#"
            const shout = (message: string): string => message + "!";
            const main = (): void => {
                "hi" |> shout |> console.log;
            };
        "#,
    );
}

fn cli_imports() -> tyrus_core::collections::HashMap<String, tyrus_core::ast::Type> {
    let mut imports: tyrus_core::collections::HashMap<String, tyrus_core::ast::Type> =
        tyrus_core::cli::prompt_builtins()
            .iter()
            .map(|builtin| {
                (
                    builtin.name.to_string(),
                    tyrus_core::cli::prompt_type(builtin),
                )
            })
            .collect();
    imports.insert("isTTY".to_string(), tyrus_core::cli::is_tty_type());
    imports
}

fn assert_well_typed_with_cli(body: &str) {
    let source = format!(
        "import {{ input, password, confirm, select, multiselect, number, integer, isTTY }} from \"tyrus:cli\";\n{body}"
    );
    let tokens = tyrus_core::lexer::lex(&source).unwrap();
    let program = Parser::new(tokens).parse_program().unwrap();
    let errors = TypeChecker::new()
        .check_program_with_imports(&program, &cli_imports())
        .into_iter()
        .filter(|diagnostic| diagnostic.severity == CheckSeverity::Error)
        .collect::<Vec<_>>();
    assert!(errors.is_empty(), "unexpected errors: {errors:#?}");
}

#[test]
fn accepts_cli_prompts_with_option_objects() {
    assert_well_typed_with_cli(
        r#"
            const name: string = input({
                message: "Project name?",
                default: "app",
                pattern: /^[a-z][a-z0-9-]*$/,
                patternMessage: "lowercase, start with a letter",
            });
            const secret: string = password({
                message: "Token?",
                confirm: true,
                confirmMessage: "Confirm token",
                pattern: /.{8,}/,
            });
            const git: boolean = confirm({
                message: "Init git?",
                default: true,
                yes: "Yes",
                no: "No",
            });
            const target: string = select({
                message: "Target?",
                choices: ["script", "cli"],
                default: "cli",
            });
            const features: string[] = multiselect({
                message: "Features?",
                choices: ["eslint", "tests"],
                pageSize: 10,
                required: true,
            });
            const ratio: f64 = number({ message: "Ratio?", default: 1.5, min: 0.0, max: 10.0 });
            const port: i64 = integer({ message: "Port?", default: 3000, min: 1, max: 65535 });
            const validPort: boolean = port > 0;
            console.log(name);
            console.log(secret);
            console.log(target);
            console.log(features[0]);
            console.log(git);
            console.log(ratio);
            console.log(port);
            console.log(validPort);
        "#,
    );
}

#[test]
fn accepts_cli_prompts_with_only_required_fields() {
    assert_well_typed_with_cli(
        r#"
            const name: string = input({ message: "Name?" });
            const target: string = select({ message: "Target?", choices: ["a", "b"] });
            console.log(name);
            console.log(target);
        "#,
    );
}

#[test]
fn accepts_is_tty_as_a_zero_argument_boolean_query() {
    assert_well_typed_with_cli(
        r#"
            const interactive: boolean = isTTY();
            const repeated: boolean = isTTY();
            console.log(interactive);
            console.log(repeated);
            console.log("assert:", false);
        "#,
    );
}

fn assert_well_typed_with_style_text(body: &str) {
    let source = format!("import {{ styleText }} from \"tyrus:utils\";\n{body}");
    let tokens = tyrus_core::lexer::lex(&source).unwrap();
    let program = Parser::new(tokens).parse_program().unwrap();
    let mut imports = tyrus_core::collections::HashMap::default();
    imports.insert(
        "styleText".to_string(),
        tyrus_core::ast::Type::Named("__StyleText".to_string()),
    );
    let errors = TypeChecker::new()
        .check_program_with_imports(&program, &imports)
        .into_iter()
        .filter(|diagnostic| diagnostic.severity == CheckSeverity::Error)
        .collect::<Vec<_>>();
    assert!(errors.is_empty(), "unexpected errors: {errors:#?}");
}

fn assert_well_typed_with_figlet(body: &str) {
    let source = format!("import {{ figlet }} from \"tyrus:utils\";\n{body}");
    let tokens = tyrus_core::lexer::lex(&source).unwrap();
    let program = Parser::new(tokens).parse_program().unwrap();
    let mut imports = tyrus_core::collections::HashMap::default();
    imports.insert(
        "figlet".to_string(),
        tyrus_core::ast::Type::Named("__Figlet".to_string()),
    );
    let errors = TypeChecker::new()
        .check_program_with_imports(&program, &imports)
        .into_iter()
        .filter(|diagnostic| diagnostic.severity == CheckSeverity::Error)
        .collect::<Vec<_>>();
    assert!(errors.is_empty(), "unexpected errors: {errors:#?}");
}

#[test]
fn accepts_style_text_three_syntaxes() {
    assert_well_typed_with_style_text(
        r#"
            const a: string = styleText("green", "hi");
            const b: string = styleText(["bold", "red"], "hi");
            const c: string = styleText.cyan.underline("hi");
            const d: string = "hi" |> styleText.yellow.bold;
            const e: string = "hi" |> styleText(["magenta", "italic"]);
            console.log(a);
            console.log(b);
            console.log(c);
            console.log(d);
            console.log(e);
        "#,
    );
}

#[test]
fn accepts_figlet_builtin_fonts() {
    assert_well_typed_with_figlet(
        r#"
            const a: string = figlet("HELLO");
            const b: string = figlet("HELLO", "standard");
            const c: string = figlet("HELLO", "small");
            const d: string = figlet("HELLO", "big");
            const e: string = figlet("HELLO", "slant");
            console.log(a + b + c + d + e);
        "#,
    );
}

#[test]
fn accepts_style_text_with_dynamic_text() {
    assert_well_typed_with_style_text(
        r#"
            const name: string = "world";
            const greeting: string = styleText("blue", "hello " + name);
            console.log(greeting);
        "#,
    );
}

#[test]
fn accepts_calling_function_typed_interface_field() {
    assert_well_typed(
        r#"
            interface Handlers {
                onClick: (x: i32) => i32;
                combine: (a: i32, b: i32) => i32;
                label: string;
            }
            const run = (h: Handlers): void => {
                const clicked: i32 = h.onClick(5);
                const sum: i32 = h.combine(3, 4);
                console.log(clicked);
                console.log(sum);
                console.log(h.label);
            };
        "#,
    );
}

#[test]
fn accepts_interface_inheritance() {
    assert_well_typed(
        r#"
            interface User { name: string; age: i32; }
            interface Timestamps { createdAt: i64; }
            interface Admin extends User, Timestamps { role: string; }
            const run = (a: Admin): void => {
                console.log(a.name);
                console.log(a.role);
                console.log(a.createdAt);
            };
        "#,
    );
}

#[test]
fn accepts_generic_alias_application() {
    assert_well_typed(
        r#"
            type Box<T> = { value: T; };
            type IntBox = Box<i32>;
            const run = (): void => {
                const b: IntBox = { value: 5 };
                console.log(b.value);
            };
        "#,
    );
}

#[test]
fn accepts_literal_union_and_keyof_values() {
    assert_well_typed(
        r#"
            interface User { name: string; age: i32; }
            type Choice = "a" | "b";
            type Keys = keyof User;
            const run = (): void => {
                const c: Choice = "a";
                const k: Keys = "name";
                console.log(c);
                console.log(k);
            };
        "#,
    );
}

#[test]
fn accepts_structural_utility_types() {
    assert_well_typed(
        r#"
            interface User { name: string; age: i32; email: string; }
            type UserUpdate = Partial<User>;
            type NameOnly = Pick<User, "name">;
            type NoEmail = Omit<User, "email">;
            type CredentialsUpdate = Optional<User, "email">;
            type Unchanged = Optional<User, never>;
            type AllOptional = Optional<User, keyof User>;
            type NameUpdate = Optional<Pick<User, "name" | "age">, "name">;
            type Flags = Record<"active" | "admin", boolean>;
            const applyCredentials = (update: CredentialsUpdate): CredentialsUpdate => update;
            const run = (): void => {
                const patch: UserUpdate = { name: "Ada" };
                const n: NameOnly = { name: "Linus" };
                const u: NoEmail = { name: "Grace", age: 40 };
                const credentials: CredentialsUpdate = { name: "Edsger", age: 54 };
                const unchanged: Unchanged = { name: "Margaret", age: 38, email: "m@example.com" };
                const all: AllOptional = {};
                const nameUpdate: NameUpdate = { age: 31 };
                const applied: CredentialsUpdate = applyCredentials(credentials);
                const f: Flags = { active: true, admin: false };
                console.log(patch.name ?? "none");
                console.log(n.name);
                console.log(u.age);
                console.log(credentials.email ?? "none");
                console.log(unchanged.email);
                console.log(all.name ?? "none");
                console.log(nameUpdate.age);
                console.log(applied.name);
                console.log(f.active);
            };
        "#,
    );
}

#[test]
fn accepts_static_object_pick_and_omit_methods() {
    assert_well_typed(
        r#"
            interface User { id: i32; name: string; email: string; password: string; }
            type PublicKeys = "id" | "name";
            const run = (user: User): void => {
                const publicUser = user.pick(["id", "name"]);
                const keys: PublicKeys[] = ["id", "name"];
                const fromKeys = user.pick(keys);
                const stored = user.omit(["password"]);
                const copied = user.omit([]);
                console.log(publicUser.id);
                console.log(fromKeys.name);
                console.log(stored.email);
                console.log(copied.password);
            };
        "#,
    );
}

#[test]
fn accepts_dynamic_map_operations() {
    assert_well_typed(
        r#"
            type Scores = Record<string, i32>;
            const run = (): void => {
                let scores: Scores = { alice: 10, bob: 20 };
                scores["carol"] = 30;
                const alice: i32? = scores["alice"];
                console.log(scores.size);
                console.log(scores.has("bob"));
                console.log(scores.delete("carol"));
                const names: string[] = scores.keys();
                console.log(names.length);
                if (alice) {
                    console.log(alice);
                }
            };
        "#,
    );
}

#[test]
fn accepts_map_get_or_insert_with_a_mutable_value_reference() {
    assert_well_typed(
        r#"
            type Counters = Map<string, i32>;
            const run = (): void => {
                let counters: Counters = Map<string, i32>();
                let counter = counters.getOrInsert("visits", 0);
                counter += 1;
                console.log(counter);
            };
        "#,
    );
}

#[test]
fn accepts_primitive_string_conversions_and_parsing() {
    assert_well_typed(
        r#"
            const run = (): void => {
                const n: i32 = 42;
                const a: string = n.toString();
                const b: string = true.toString();
                const c: string = String(3.14);
                console.log(a);
                console.log(b);
                console.log(c);
                const i: i64? = parseInt("42");
                const f: f64? = parseFloat("3.5");
                const num: f64? = Number("2.5");
                console.log(i ?? -1);
                console.log(f ?? 0.0);
                console.log(num ?? 0.0);
            };
        "#,
    );
}

#[test]
fn accepts_automatic_copy_and_equality_for_named_data_interfaces() {
    assert_well_typed(
        r#"
            interface User {
                name: string;
                age: i32;
            }
            const run = (): void => {
                const user: User = { name: "Ada", age: 30 };
                const copied: User = user.copy();
                const same: boolean = user.equals(copied);
                console.log(same);
            };
        "#,
    );
}

#[test]
fn accepts_automatic_equality_for_optional_fields() {
    assert_well_typed(
        r#"
            interface User { name: string; nickname?: string; }
            const run = (): void => {
                const first: User = { name: "Ada" };
                const second: User = { name: "Ada" };
                const same: boolean = first.equals(second);
                console.log(same);
            };
        "#,
    );
}

#[test]
fn accepts_automatic_debug_for_named_data_interfaces() {
    assert_well_typed(
        r#"
            interface User {
                name: string;
                age: i32;
            }
            const run = (): void => {
                const user: User = { name: "Ada", age: 30 };
                const text: string = user.toString();
                console.log(text);
            };
        "#,
    );
}

#[test]
fn accepts_automatic_debug_for_decimal_fields() {
    assert_well_typed(
        r#"
            interface Balance { amount: decimal; }
            const run = (): void => {
                const balance: Balance = { amount: 12.50d };
                const text: string = balance.toString();
                console.log(text);
            };
        "#,
    );
}

#[test]
fn accepts_automatic_json_encoding_for_data_interfaces() {
    assert_well_typed(
        r#"
            interface User { name: string; age: i32; active: boolean; }
            const run = (): void => {
                const user: User = { name: "Ada", age: 30, active: true };
                const json: string = user.toJson();
                console.log(json);
            };
        "#,
    );
}

#[test]
fn accepts_pretty_automatic_json_encoding_for_data_interfaces() {
    assert_well_typed(
        r#"
            interface User { name: string; }
            const run = (): void => {
                const user: User = { name: "Ada" };
                const json: string = user.toJson({ pretty: true });
                console.log(json);
            };
        "#,
    );
}

#[test]
fn accepts_automatic_json_decoding_for_data_interfaces() {
    assert_well_typed(
        r#"
            interface User { name: string; age: i32; active: boolean; }

            const main = (): void => {
                const result: Result<User, JsonError> = User.fromJson("{\\\"name\\\":\\\"Ada\\\",\\\"age\\\":37,\\\"active\\\":true}");
                if (result) {
                    const user: User = result;
                    console.log(user.name);
                }
            };
        "#,
    );
}

#[test]
fn accepts_automatic_json_for_optional_fields() {
    assert_well_typed(
        r#"
            interface User { name: string; nickname?: string; }

            const main = (): void => {
                const user: User = { name: "Ada" };
                const encoded: string = user.toJson();
                const decoded: Result<User, JsonError> = User.fromJson(encoded);
            };
        "#,
    );
}

#[test]
fn accepts_automatic_json_for_arrays_and_nested_data_interfaces() {
    assert_well_typed(
        r#"
            interface Address { city: string; }
            interface User { address: Address; scores: i32[]; }
            const main = (): void => {
                const user: User = { address: { city: "Ada" }, scores: [10, 20] };
                const encoded: string = user.toJson();
            };
        "#,
    );
}

#[test]
fn accepts_automatic_json_for_arrays_of_data_interfaces() {
    assert_well_typed(
        r#"
            interface Address { city: string; }
            interface Directory { entries: Address[]; }
            const main = (): void => {
                const directory: Directory = { entries: [{ city: "Ada" }] };
                const encoded: string = directory.toJson();
                const decoded: Result<Directory, JsonError> = Directory.fromJson(encoded);
            };
        "#,
    );
}

#[test]
fn accepts_automatic_debug_with_nested_structural_objects() {
    assert_well_typed(
        r#"
            interface User {
                address: { city: string; zip: i32; };
            }
            const run = (): void => {
                const user: User = { address: { city: "São Paulo", zip: 1000 } };
                const text: string = user.toString();
                console.log(text);
            };
        "#,
    );
}

#[test]
fn accepts_automatic_debug_with_optional_fields() {
    assert_well_typed(
        r#"
            interface User {
                name: string;
                email?: string;
            }
            const run = (): void => {
                const user: User = { name: "Ada" };
                const text: string = user.toString();
                console.log(text);
            };
        "#,
    );
}

#[test]
fn accepts_automatic_debug_with_array_fields() {
    assert_well_typed(
        r#"
            interface Report {
                title: string;
                scores: i32[];
            }
            const run = (): void => {
                const report: Report = { title: "result", scores: [7, 9, 10] };
                const text: string = report.toString();
                console.log(text);
            };
        "#,
    );
}

#[test]
fn accepts_automatic_debug_with_union_fields() {
    assert_well_typed(
        r#"
            interface Event {
                detail: string | i32;
            }
            const run = (): void => {
                const event: Event = { detail: "ready" };
                const text: string = event.toString();
                console.log(text);
            };
        "#,
    );
}

#[test]
fn accepts_automatic_debug_with_tuple_fields() {
    assert_well_typed(
        r#"
            interface Point {
                coordinate: [i32, i32];
            }
            const run = (): void => {
                const point: Point = { coordinate: [10, 20] };
                const text: string = point.toString();
                console.log(text);
            };
        "#,
    );
}

#[test]
fn accepts_new_array_methods() {
    assert_well_typed(
        r#"
            const run = (): void => {
                let a: i32[] = [10, 20, 30];
                const first: i32? = a.at(0);
                const last: i32? = a.at(-1);
                const ks: usize[] = a.keys();
                const vs: i32[] = a.values();
                const es: [usize, i32][] = a.entries();
                const built: i32[] = Array.of(1, 2, 3);
                const copied: i32[] = Array.from(a);
                const spliced: i32[] = a.splice(0, 1, 40, 50);
                const popped: i32? = a.pop();
                const shifted: i32? = a.shift();
                console.log(first ?? 0);
                console.log(last ?? 0);
                console.log(ks.length);
                console.log(vs.length);
                console.log(es.length);
                console.log(built.length);
                console.log(copied.length);
                console.log(spliced.length);
                console.log(popped ?? 0);
                console.log(shifted ?? 0);
                for (const entry of es) {
                    console.log(entry[0]);
                }
            };
        "#,
    );
}

#[test]
fn accepts_test_assertions() {
    let source = r#"
                const testMath = (): void => {
            assert(1 + 1 === 2);
            assert(2 > 1, "two beats one");
            assertEqual(2 + 3, 5);
            assertEqual("a" + "b", "ab");
        };
    "#;
    let tokens = tyrus_core::lexer::lex(source).unwrap();
    let program = Parser::new(tokens).parse_program().unwrap();
    let errors = TypeChecker::new()
        .check_program(&program)
        .into_iter()
        .filter(|diagnostic| diagnostic.severity == CheckSeverity::Error)
        .collect::<Vec<_>>();
    assert!(errors.is_empty(), "unexpected errors: {errors:#?}");
}

#[test]
fn accepts_process_exit() {
    assert_well_typed(
        r#"
            const run = (code: i32): void => {
                if (code > 0) {
                    process.exit(code);
                }
                console.log("ok");
            };
        "#,
    );
}

#[test]
fn accepts_set_and_map_collections() {
    assert_well_typed(
        r#"
            type IntSet = Set<i32>;
            type StringSet = Set<string>;
            type ScoreMap = Map<string, i32>;
            type NameMap = Map<i32, string>;
            const run = (): void => {
                let seen: IntSet = Set<i32>();
                seen.add(1);
                seen.add(1);
                console.log(seen.size);
                console.log(seen.has(1));
                console.log(seen.delete(1));
                const tags: StringSet = Set(["a", "b"]);
                for (const tag of tags) {
                    console.log(tag);
                }

                let scores: ScoreMap = Map<string, i32>();
                scores.set("ada", 10);
                const ada: i32? = scores.get("ada");
                console.log(scores.size);
                console.log(scores.has("ada"));
                const keys: string[] = scores.keys();
                const values: i32[] = scores.values();
                const entries: [string, i32][] = scores.entries();
                console.log(keys.length);
                console.log(values.length);
                console.log(entries.length);
                for (const entry of scores) {
                    console.log(entry[0]);
                }
                if (ada) {
                    console.log(ada);
                }

                let byId: NameMap = Map<i32, string>();
                byId.set(7, "seven");
                console.log(byId.get(7) ?? "none");
            };
        "#,
    );
}

#[test]
fn accepts_conditional_and_function_utility_types() {
    assert_well_typed(
        r#"
            type Fn = (i32) => string;
            type Result = ReturnType<Fn>;
            type Args = Parameters<Fn>;
            type Colors = "red" | "green" | "blue";
            type Safe = Exclude<Colors, "red">;
            type IsString<T> = T extends string ? boolean : i32;
            type FromString = IsString<string>;
            const run = (): void => {
                const r: Result = "hello";
                const a: Args = [42];
                const c: Safe = "green";
                const flag: FromString = true;
                console.log(r);
                console.log(a[0]);
                console.log(c);
                console.log(flag);
            };
        "#,
    );
}

#[test]
fn coerces_literal_return_to_union_member() {
    assert_well_typed(
        r#"
            interface Payload {
                value: i32;
            }
            const guard = (deny: boolean): Payload | false => {
                if (deny) {
                    return { value: 1 };
                }
                return false;
            };
            const run = (): void => {
                const result: Payload | false = guard(true);
            };
        "#,
    );
}

#[test]
fn coerces_numeric_literals_to_optional_annotations() {
    assert_well_typed(
        r#"
            const count: i32? = 5;
            const ratio: f64? = 1.5;
            const offset: isize? = -1;
            const length: usize? = 3;
            const name: string? = "ada";
            const enabled: boolean? = true;
            console.log(count ?? 0);
            console.log(ratio ?? 0.0);
        "#,
    );
}

#[test]
fn types_optional_field_access_as_an_optional_of_the_field_type() {
    assert_well_typed(
        r#"
            interface Address {
                city: string;
            }
            interface Profile {
                nickname?: string;
                address?: Address;
            }
            const profile: Profile = {};
            const nickname: string? = profile.nickname;
            const size: usize? = nickname?.length;
            const city: string? = profile.address?.city;
            console.log(size ?? 0);
            console.log(city ?? "unknown");
        "#,
    );
}

#[test]
fn produces_absence_without_a_null_literal() {
    assert_well_typed(
        r#"
            interface User {
                id: i32;
                nickname?: string;
            }
            const findUser = (found: boolean): User? => {
                if (found) {
                    return { id: 1 };
                }
                return;
            };
            const run = (): void => {
                const user: User? = findUser(false);
                console.log(user?.id ?? -1);

                let pending: i32?;
                console.log(pending ?? 0);
                pending = 5;
                console.log(pending ?? 0);

                const partial: User = { id: 2 };
                console.log(partial.nickname ?? "anon");
            };
        "#,
    );
}

#[test]
fn assigns_object_and_array_literals_to_optional_annotations() {
    assert_well_typed(
        r#"
            interface Point {
                x: i32;
                y: i32;
            }
            const origin: Point? = { x: 0, y: 0 };
            const makePoint = (): Point? => {
                return { x: 1, y: 2 };
            };
            const takesPoint = (p: Point?): void => {
                console.log(p?.x ?? -1);
            };
            const run = (): void => {
                takesPoint({ x: 3, y: 4 });
                console.log(origin?.x ?? -1);
                console.log(makePoint()?.y ?? -1);
            };
        "#,
    );
}

#[test]
fn accepts_rest_parameter_call_forms() {
    assert_well_typed(
        r#"
            const sum = (...xs: i32[]): i32 => {
                let total: i32 = 0;
                for (const x of xs) {
                    total += x;
                }
                return total;
            };
            const withBase = (base: i32, ...xs: i32[]): i32 => {
                return base + sum(...xs);
            };
            const run = (): void => {
                console.log(sum(1, 2, 3));
                console.log(sum());
                const values: i32[] = [4, 5];
                console.log(sum(...values));
                console.log(withBase(10, 1, 2));
                console.log(withBase(10, ...values));
            };
        "#,
    );
}

#[test]
fn uses_named_functions_as_first_class_values() {
    assert_well_typed(
        r#"
            const double = (x: i32): i32 => x * 2;
            function isEven(x: i32): boolean {
                return x % 2 === 0;
            }
            const apply = (f: (i32) => i32, v: i32): i32 => f(v);
            const run = (): void => {
                console.log(apply(double, 21));
                const alias = double;
                console.log(alias(10));
                const numbers: i32[] = [1, 2, 3, 4];
                const doubled: i32[] = numbers.map(double);
                const evens: i32[] = numbers.filter(isEven);
                console.log(doubled[0], evens[0]);
            };
        "#,
    );
}

#[test]
fn accepts_optionals_as_conditions_with_narrowing() {
    assert_well_typed(
        r#"
            interface User {
                name: string;
            }
            const find = (ok: boolean): User? => {
                if (ok) {
                    return { name: "Ada" };
                }
                return;
            };
            const run = (): void => {
                const user: User? = find(true);
                const label: string = user ? user.name : "anon";
                const shown: boolean = user && user.name === "Ada";
                const missing: boolean = !user;
                console.log(label, shown, missing);
                while (user) {
                    console.log("present");
                    break;
                }
            };
        "#,
    );
}

#[test]
fn accepts_logical_expression_statements() {
    assert_well_typed(
        r#"
            const save = (): void => {
                console.log("saved");
            };
            const run = (x: i32): void => {
                const isAdmin: boolean = x > 0;
                isAdmin && save();
                isAdmin || save();
                x > 0 && save();
            };
        "#,
    );
}

#[test]
fn accepts_for_of_with_destructuring() {
    assert_well_typed(
        r#"
            interface Point {
                x: i32;
                y: i32;
            }
            const run = (): void => {
                const pairs: [i32, i32][] = [[1, 2], [3, 4]];
                for (const [a, b] of pairs) {
                    console.log(a + b);
                }
                const points: Point[] = [{ x: 1, y: 2 }];
                for (const { x, y } of points) {
                    console.log(x * y);
                }
                const scores = Map<string, i32>();
                scores.set("ada", 1);
                for (const [name, score] of scores.entries()) {
                    console.log(name, score);
                }
            };
        "#,
    );
}

#[test]
fn accepts_negative_integer_literals_in_arithmetic_and_bitwise() {
    assert_well_typed(
        r#"
            const run = (): void => {
                console.log((-8) >>> 1);
                console.log((-16) >> 2);
                console.log((-8) << 1);
                console.log((-6) & 3);
                console.log(-8 + 3);
                const wide: i64 = 100;
                console.log(wide + (-8));
            };
        "#,
    );
}

#[test]
fn resolves_member_access_through_a_generic_constraint() {
    assert_well_typed(
        r#"
            interface Named {
                name: string;
            }
            interface HasId {
                id: i32;
                label: string;
            }
            const greet = <T extends Named>(value: T): string => {
                return value.name;
            };
            const idOf = <T extends HasId>(value: T): i32 => {
                return value.id;
            };
            const run = (): void => {
                const person: Named = { name: "Ada" };
                console.log(greet(person));
                const record: HasId = { id: 1, label: "x" };
                console.log(idOf(record));
            };
        "#,
    );
}

#[test]
fn accepts_literal_types_inside_unions_and_fields() {
    assert_well_typed(
        r#"
            type Status = "active" | "inactive";
            type Result =
                | { tag: "ok"; data: i32 }
                | { tag: "err"; code: i32 };
            interface Fields {
                a: i32;
                b: i32;
            }
            type Key = keyof Fields;
            const setMode = (mode: "on" | "off"): void => {
                console.log(mode);
            };
            const run = (): void => {
                const status: Status = "active";
                const result: Result = { tag: "ok", data: 1 };
                const key: Key = "a";
                console.log(status, result.tag, key);
                setMode("on");
            };
        "#,
    );
}

#[test]
fn interface_extends_inherits_required_fields() {
    assert_well_typed(
        r#"
            interface Age {
                age: u32;
            }
            interface Person extends Age {
                name: string;
            }
            interface JustAge extends Age {}
            const run = (): void => {
                const person: Person = { age: 30, name: "Ada" };
                console.log(person.name, person.age);
                const only: JustAge = { age: 25 };
                console.log(only.age);
            };
        "#,
    );
}

#[test]
fn never_function_via_infinite_loop_is_accepted() {
    assert_well_typed(
        r#"
            const spin = (): never => {
                while (true) {}
            };
            const loopReturn = (): i32 => {
                while (true) {
                    return 5;
                }
            };
            const run = (): void => {
                console.log("declared");
            };
        "#,
    );
}

#[test]
fn parse_builtins_adopt_the_annotated_numeric_width() {
    assert_well_typed(
        r#"
            const port: i32? = parseInt("8080");
            const count: u32? = parseInt("7");
            const size: usize? = parseInt("9");
            const wide: i64? = parseInt("9999999999");
            const huge: i128? = parseInt("170141183460469231731687303715884105727");
            const ratio: f32? = parseFloat("3.5");
            const whole: f32? = Number("2.5");
            const fallback = parseInt("42");
            console.log(port ?? -1);
            console.log(count ?? 0);
            console.log(size ?? 0);
            console.log(wide ?? -1);
            console.log(huge ?? -1);
            console.log(ratio ?? 0.0);
            console.log(whole ?? 0.0);
            console.log(fallback ?? -1);
        "#,
    );
}

#[test]
fn accepts_the_numeric_and_enum_type_assertions() {
    assert_well_typed(
        r#"
            enum Code {
                Ok = 200,
                NotFound = 404,
            }
            const value: i32 = 300;
            const widened: f64 = value as f64;
            const wrapped: u32 = value as u32;
            const wide: i64 = 5_000_000_000;
            const narrowed = wide as i32?;
            const truncated = parseFloat("2.9") as i32?;
            const status: i64 = Code.NotFound as i64;
            const same: i32 = value as i32;
            console.log(widened);
            console.log(wrapped);
            console.log(narrowed ?? -1);
            console.log(truncated ?? -1);
            console.log(status);
            console.log(same);
        "#,
    );
}

#[test]
fn accepts_assert_throws_on_a_function_that_declares_throws() {
    assert_well_typed(
        r#"
            function boom(): void throws string {
                throw "kaboom";
            }

                        const detectsAThrow = (): void => {
                assertThrows(boom);
                assertThrows(boom, "kaboom");
            };
        "#,
    );
}

#[test]
fn accepts_process_argv_as_a_string_array() {
    assert_well_typed(
        r#"
            const main = (): void => {
                const args: string[] = process.argv;
                const rest: string[] = args.slice(1);
                console.log(args.length);
                console.log(rest.at(0) ?? "(none)");
            };
        "#,
    );
}

#[test]
fn accepts_calling_the_result_of_a_call() {
    assert_well_typed(
        r#"
            const makeAdder = (base: i32): (i32) => i32 => {
                return (value: i32): i32 => base + value;
            };

            const main = (): void => {
                console.log(makeAdder(5)(3));
            };
        "#,
    );
}

#[test]
fn accepts_methods_and_fields_directly_on_an_array_literal() {
    assert_well_typed(
        r#"
            const main = (): void => {
                const doubled: i32[] = [1, 2, 3].map((value: i32): i32 => value * 2);
                console.log(doubled.length);
                console.log([1, 2, 3].length);
                console.log(["a", "b"].join("-"));
                console.log([3, 1, 2].includes(2));
            };
        "#,
    );
}

#[test]
fn an_array_literal_still_widens_to_the_annotated_element_type() {
    assert_well_typed(
        r#"
            const takeWide = (values: i64[]): usize => values.length;

            const main = (): void => {
                const wide: i64[] = [1, 2];
                const wider: i128[] = [1, 2];
                const floats: f64[] = [1.5, 2.5];
                const unsigned: u32[] = [1, 2];
                const nested: i64[][] = [[1], [2]];
                console.log(wide[0]);
                console.log(wider[0]);
                console.log(floats[0]);
                console.log(unsigned[0]);
                console.log(nested.length);
                console.log(takeWide([1, 2]));
            };
        "#,
    );
}

#[test]
fn accepts_chained_relational_and_equality_comparisons() {
    assert_well_typed(
        r#"
            const minimo = (): i32 => 5;
            const maximo = (): i32 => 50;
            const main = (): void => {
                const temp: u32 = 25;
                const valor: i32 = 25;
                const a: i32 = 5;
                const b: i32 = 5;
                const c: i32 = 5;
                console.log(0 < temp < 100);
                console.log(minimo() < valor < maximo());
                console.log(18 <= valor < 65);
                console.log(a === b === c);
            };
        "#,
    );
}

#[test]
fn accepts_zip_under_a_catch_and_under_throws() {
    assert_well_typed(
        r#"
            const main = (): void => {
                const names: string[] = ["a", "b"];
                const scores: i32[] = [1, 2];
                try {
                    const pairs: [string, i32][] = names.zip(scores);
                    console.log(pairs.length);
                    console.log(pairs[0][0]);
                    console.log(pairs[0][1]);
                } catch (error: ZipError) {
                    console.log(error.code);
                    console.log(error.message);
                }
            };
        "#,
    );
    assert_well_typed(
        r#"
            const zipOrEmpty = (left: i32[], right: i32[]): [i32, i32][] throws ZipError => {
                return left.zip(right);
            };
            const main = (): void => {
                try {
                    const pairs: [i32, i32][] = zipOrEmpty([1], [2]);
                    console.log(pairs.length);
                } catch (error: ZipError) {
                    console.log(error.code);
                }
            };
        "#,
    );
}

#[test]
fn accepts_membership_checks_over_array_range_map_set_and_object() {
    assert_well_typed(
        r#"
            interface Point {
                x: i32;
                y: i32;
            }
            const main = (): void => {
                const nums: i32[] = [1, 2, 3];
                console.log(2 in nums);
                console.log(5 not in nums);

                const status: i32 = 204;
                console.log(status in 200..299);
                console.log(status not in 400..=599);

                const r = 0..10;
                console.log(r.contains(5));

                const names = Set<string>(["a", "b"]);
                console.log("a" in names);

                const scores = Map<string, i32>();
                console.log("alice" in scores);

                const point: Point = { x: 1, y: 2 };
                console.log("x" in point);
            };
        "#,
    );
}

#[test]
fn accepts_for_of_over_a_range_and_array_slicing() {
    assert_well_typed(
        r#"
            const main = (): void => {
                let sum: i32 = 0;
                for (const i of 0..5) {
                    sum += i;
                }
                console.log(sum);

                const nums: i32[] = [10, 20, 30, 40, 50];
                const middle: i32[] = nums[1..4];
                const fromTwo: i32[] = nums[2..];
                const upToThree: i32[] = nums[..3];
                const upToThreeInclusive: i32[] = nums[..=3];
                const whole: i32[] = nums[..];
                const single: i32 = nums[2];
                console.log(middle);
                console.log(fromTwo);
                console.log(upToThree);
                console.log(upToThreeInclusive);
                console.log(whole);
                console.log(single);
            };
        "#,
    );
}

#[test]
fn accepts_range_patterns_in_match_over_signed_unsigned_and_float_subjects() {
    assert_well_typed(
        r#"
            const letterGrade = (score: i32): string => {
                match (score) {
                    90..=100 => { return "A"; }
                    80..89   => { return "B"; }
                    70..79   => { return "C"; }
                    60..69   => { return "D"; }
                    ..60     => { return "F"; }
                    100..    => { return "F"; }
                }
                return "?";
            };
            const ageCategory = (age: u8): string => {
                match (age) {
                    0..2   => { return "infant"; }
                    3..12  => { return "child"; }
                    13..17 => { return "teenager"; }
                    18..64 => { return "adult"; }
                    65..   => { return "senior"; }
                }
                return "?";
            };
            const tempState = (celsius: f64): string => {
                match (celsius) {
                    ..0.0       => { return "freezing"; }
                    0.0..=100.0 => { return "liquid"; }
                    100.0..     => { return "boiling"; }
                }
                return "?";
            };
            const main = (): void => {
                console.log(letterGrade(95));
                console.log(ageCategory(30));
                console.log(tempState(50.0));
            };
        "#,
    );
}

#[test]
fn accepts_compact_match_arms_as_implicit_returns() {
    assert_well_typed(
        r#"
            const letter_grade = (score: i32): string => {
                match (score) {
                    90..=100 => "A";
                    80..=89 => "B";
                    _ => "F";
                }
            };
        "#,
    );
}

#[test]
fn accepts_split_at_with_a_statically_known_index_and_an_unknown_one() {
    assert_well_typed(
        r#"
            const main = (): void => {
                const nums: i32[] = [1, 2, 3, 4, 5];
                const parts: [i32[], i32[]] = nums.splitAt(2);
                console.log(parts[0]);
                console.log(parts[1]);

                const whole = nums.splitAt(nums.length);
                console.log(whole[0]);

                let index: usize = 2;
                index = index + 1;
                const dynamic = nums.splitAt(index);
                console.log(dynamic[0]);

                const words: string[] = ["a", "b", "c"];
                const wordParts = words.splitAt(1);
                console.log(wordParts[0]);
                console.log(wordParts[1]);
            };
        "#,
    );
}

#[test]
fn accepts_string_and_array_count_in_every_form() {
    assert_well_typed(
        r#"
            const main = (): void => {
                const text: string = "banana";
                const len: usize = text.count();
                const an: usize = text.count("an");
                console.log(len);
                console.log(an);

                const numbers: i32[] = [1, 2, 3, 4, 5, 6];
                const total: usize = numbers.count();
                const threes: usize = numbers.count(3);
                const evens: usize = numbers.count(number % 2 === 0);
                const highs: usize = numbers.count(number > 3);
                console.log(total);
                console.log(threes);
                console.log(evens);
                console.log(highs);

                const empty: i32[] = [];
                console.log(empty.count());
                console.log(empty.count(1));
                console.log(empty.count(x > 0));

                const words: string[] = ["ada", "grace"];
                console.log(words.count(w.length > 4));
            };
        "#,
    );
}

#[test]
fn accepts_math_constant_imports_and_usage() {
    assert_well_typed(
        r#"
            import { PI, E, TAU, SQRT2, PHI } from "tyrus:math";
            const circumference: f64 = TAU * 3.0;
            const area: f64 = PI * SQRT2;
            const golden: f64 = PHI;
            const natural: f64 = E;
            console.log(circumference);
            console.log(area);
            console.log(golden);
            console.log(natural);
        "#,
    );
}

#[test]
fn accepts_math_function_imports_and_usage() {
    assert_well_typed(
        r#"
            import { abs, sign, sqrt, cbrt, floor, ceil, trunc, round, sin, cos, tan, asin, acos, atan, atan2, sinh, cosh, tanh, exp, log, log10, log2, pow, max, min, imul, clz32 } from "tyrus:math";
            const main = (): void => {
                console.log(abs(-4.5));
                console.log(sign(-4.5));
                console.log(sqrt(9.0));
                console.log(cbrt(27.0));
                console.log(floor(4.9));
                console.log(ceil(4.1));
                console.log(trunc(4.7));
                console.log(round(4.5));
                console.log(sin(0.5));
                console.log(cos(0.5));
                console.log(tan(0.5));
                console.log(asin(0.0));
                console.log(acos(1.0));
                console.log(atan(1.0));
                console.log(atan2(1.0, 1.0));
                console.log(sinh(0.0));
                console.log(cosh(0.0));
                console.log(tanh(0.0));
                console.log(exp(1.0));
                console.log(log(2.718281828459045));
                console.log(log10(100.0));
                console.log(log2(8.0));
                console.log(pow(2.0, 3.0));
                console.log(max(3.0, 7.0));
                console.log(min(3.0, 7.0));
                console.log(imul(5, 3));
                console.log(clz32(8));
            };
        "#,
    );
}

#[test]
fn accepts_math_advanced_function_imports_and_usage() {
    assert_well_typed(
        r#"
            import { approxEqual, clamp, clampMin, clampMax, lerp, inverseLerp, remap, degToRad, radToDeg, gcd, lcm, randomRange, smoothstep, roundTo, mean, median, mode, variance, stdDev, percentile, factorial, permutation, combination, isPrime, nextPrime, primeFactors, fibonacci, hypot, copysign, fmod, logBase, powi, toPolar, toCartesian } from "tyrus:math";
            const main = (): void => {
                console.log(approxEqual(0.1 + 0.2, 0.3));
                console.log(clamp(15.0, 0.0, 10.0));
                console.log(clampMin(-5.0, 0.0));
                console.log(clampMax(150.0, 100.0));
                console.log(lerp(0.0, 100.0, 0.5));
                console.log(inverseLerp(0.0, 100.0, 50.0));
                console.log(remap(5.0, 0.0, 10.0, 0.0, 100.0));
                console.log(degToRad(180.0));
                console.log(radToDeg(3.141592653589793));
                console.log(gcd(12, 8));
                console.log(lcm(4, 6));
                console.log(randomRange(0.0, 1.0));
                console.log(smoothstep(0.0, 1.0, 0.5));
                console.log(roundTo(3.14159, 2));
                console.log(mean([1.0, 2.0, 3.0]));
                console.log(median([1.0, 2.0, 3.0, 4.0, 5.0]));
                console.log(mode([1.0, 2.0, 2.0, 3.0]));
                console.log(variance([2.0, 4.0, 4.0, 4.0, 5.0, 5.0, 7.0, 9.0]));
                console.log(stdDev([2.0, 4.0, 4.0, 4.0, 5.0, 5.0, 7.0, 9.0]));
                console.log(percentile(50.0, [1.0, 2.0, 3.0, 4.0, 5.0]));
                console.log(factorial(5));
                console.log(permutation(5, 3));
                console.log(combination(5, 3));
                console.log(isPrime(7));
                console.log(nextPrime(7));
                console.log(primeFactors(12));
                console.log(fibonacci(10));
                console.log(hypot(3.0, 4.0));
                console.log(copysign(-5.0, 3.0));
                console.log(fmod(7.5, 2.0));
                console.log(logBase(8.0, 2.0));
                console.log(powi(2.0, 3));
                console.log(toPolar(3.0, 4.0));
                console.log(toCartesian(5.0, 0.9272952180016122));
            };
        "#,
    );
}

#[test]
fn accepts_maps_and_sets_that_take_their_types_from_the_context_or_the_entries() {
    assert_well_typed(
        r#"
            type Counters = Map<string, i32>;
            type Rates = Map<string, f64>;
            type Marks = Set<f64>;
            const main = (): void => {
                const inferred = Map([["ada", 1], ["grace", 2]]);
                console.log(inferred.size);

                const annotated: Counters = Map([["ada", 1]]);
                console.log(annotated.size);

                const widened: Rates = Map([["half", 1]]);
                console.log(widened.size);

                const empty: Counters = Map();
                console.log(empty.size);

                const marks: Marks = Set([1, 2]);
                console.log(marks.size);
            };
        "#,
    );
}

#[test]
fn accepts_algebraic_enum_with_tuple_variants() {
    assert_well_typed(
        r#"
            enum Command {
                Quit,
                Move(i32, i32),
                Write(string),
            }
            const cmd: Command = Command.Move(10, 20);
        "#,
    );
}

#[test]
fn accepts_algebraic_enum_with_struct_variants() {
    assert_well_typed(
        r#"
            enum WebEvent {
                PageLoad,
                KeyPress(char),
                Click(i32, i32),
                Paste { text: string, source: string }
            }
            const event: WebEvent = WebEvent.Paste { text: "hello", source: "clipboard" };
        "#,
    );
}

#[test]
fn accepts_algebraic_enum_with_mixed_variants() {
    assert_well_typed(
        r#"
            enum PaymentStatus {
                Pending,
                Paid(u32),
                Failed { reason: string }
            }
            const status: PaymentStatus = PaymentStatus.Paid(150);
        "#,
    );
}

#[test]
fn accepts_match_on_algebraic_enum_with_exhaustive_patterns() {
    assert_well_typed(
        r#"
            enum Command {
                Quit,
                Move(i32, i32),
                Write(string),
            }
            const cmd: Command = Command.Move(10, 20);
            match (cmd) {
                Command.Quit => { console.log("quit"); }
                Command.Move(x, y) => { console.log("move"); }
                Command.Write(text) => { console.log("write"); }
            }
        "#,
    );
}

#[test]
fn binds_algebraic_enum_tuple_and_struct_payloads() {
    assert_well_typed(
        r#"
            enum Command {
                Move(i32, i32),
                ChangeColor { r: u8, g: u8, b: u8 },
            }
            const move: Command = Command.Move(10, 20);
            const color: Command = Command.ChangeColor { r: 255u8, g: 0u8, b: 0u8 };
            match (move) {
                Command.Move(x, y) => { const sum: i32 = x + y; console.log(sum); }
                Command.ChangeColor { r, g, b } => { console.log(r + g + b); }
            }
            match (color) {
                Command.Move(x, y) => { console.log(x + y); }
                Command.ChangeColor { r: red, g, b } => { console.log(red + g + b); }
            }
        "#,
    );
}

#[test]
fn accepts_at_least_one_when_a_selected_field_is_present() {
    assert_well_typed(
        r#"
            interface Form { name: string; email?: string; phone?: string; }
            type ValidForm = AtLeastOne<Form, "email" | "phone">;
            const byEmail: ValidForm = { name: "Ada", email: "ada@example.com" };
            const byPhone: ValidForm = { name: "Grace", phone: "555-0100" };
            const byBoth: ValidForm = { name: "Lin", email: "lin@example.com", phone: "555-0101" };
            type RequiredEmail = AtLeastOne<Form, "email">;
            const singleSelected: RequiredEmail = { name: "Mina", email: "mina@example.com" };
            interface Search { status?: string; start?: string; category?: string; }
            type PeriodSearch = AtLeastOne<Search, "status" | "start">;
            const partialSelection: PeriodSearch = { category: "news", status: "active" };
        "#,
    );
}

#[test]
fn rejects_algebraic_enum_with_explicit_repr() {
    assert_has_error(
        r#"
            enum Command: i32 {
                Quit,
                Move(i32, i32),
            }
        "#,
        "E0706",
    );
}

#[test]
fn accepts_toggle_on_number_array() {
    assert_well_typed(
        r#"
            const main = (): void => {
                let values: i32[] = [1, 2, 3];
                const result: i32[] = values.toggle(4);
                const removed: i32[] = values.toggle(2);
            };
        "#,
    );
}

#[test]
fn accepts_toggle_on_string_array() {
    assert_well_typed(
        r#"
            const main = (): void => {
                let names: string[] = ["alice", "bob"];
                const added: string[] = names.toggle("charlie");
                const removed: string[] = names.toggle("alice");
            };
        "#,
    );
}

#[test]
fn accepts_toggle_with_key() {
    assert_well_typed(
        r#"
            interface Item {
                id: i32;
                name: string;
            }
            const main = (): void => {
                let items: Item[] = [{ id: 1, name: "a" }];
                const result: Item[] = items.toggle({ id: 2, name: "b" }, "id");
            };
        "#,
    );
}

#[test]
fn accepts_toggle_all() {
    assert_well_typed(
        r#"
            const main = (): void => {
                let values: i32[] = [1, 2, 3];
                const result: i32[] = values.toggleAll([2, 4]);
            };
        "#,
    );
}

#[test]
fn accepts_toggle_on_boolean_array() {
    assert_well_typed(
        r#"
            const main = (): void => {
                let flags: boolean[] = [true, false];
                const result: boolean[] = flags.toggle(true);
            };
        "#,
    );
}

#[test]
fn accepts_catching_the_rejection_of_an_awaited_call() {
    assert_well_typed(
        r#"
            const failing = async (): Promise<i32> throws string => {
                throw "no";
            };

            const wrapper = async (value: Promise<i32>): Promise<i32> throws string => {
                return await value;
            };

            const main = async (): Promise<void> => {
                try {
                    const direct: i32 = await failing();
                    console.log(direct);
                    const nested: i32 = await wrapper(failing());
                    console.log(nested);
                } catch (error: string) {
                    console.log(error);
                }
            };
        "#,
    );
}

#[test]
fn accepts_an_array_of_objects_with_a_unit_enum_field_across_an_await() {
    // A payload-free enum is an integer tag, as copyable as the integer the
    // code generator stores it as; the frame check once refused every
    // interface that held one, so a list of such records could not outlive
    // the next `await`.
    assert_well_typed(
        r#"
            enum Color { Red, Green }
            interface Paint {
                name: string;
                color: Color;
            }
            const later = async (): Promise<void> => {};
            const run = async (): Promise<void> => {
                const paints: Paint[] = [{ name: "leaf", color: Color.Green }];
                await later();
                console.log(paints[0].color === Color.Green);
            };
        "#,
    );
}
