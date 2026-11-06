use super::*;

#[test]
pub(super) fn emits_native_http_router_calls() {
    let source = r#"
        interface HealthRequest {}
        interface HealthResponse {
            status: string;
        }
        enum HttpStatus { Ok = 200 }
        const handler = (request: HealthRequest, reply: HttpReply<HealthResponse>): void => {
            reply.status(HttpStatus.Ok).send({ status: "ok" });
        };
        const app: HttpRouter = __tyrus_http_createRouter();
        app.cors({
            origin: ["http://localhost:5173"],
            methods: ["GET", "OPTIONS", "QUERY"],
            allowedHeaders: ["Content-Type"],
            exposedHeaders: [],
            credentials: false,
            maxAge: 3600
        });
        app.trustedProxies(["10.0.0.0/8"], "x-client-ip");
        app.rateLimit({ max: 100, globalMax: 250, ban: 3, banWindow: "15 minutes", timeWindow: "1 minute" });
        app.get("/health", {
            response: { 200: HealthResponse },
            config: { rateLimit: { max: 20, globalMax: 50, ban: 2, banWindow: "10 minutes", timeWindow: "1 minute" } },
            handler: handler
        });
        app.options("/health", { response: { 200: HealthResponse }, handler: handler });
        app.query("/health", { response: { 200: HealthResponse }, handler: handler });
        app.listen(3000);
    "#;
    let tokens = Token::lexer(source).filter_map(Result::ok).collect();
    let program = Parser::new(tokens).parse_program().unwrap();
    let mut checker = BorrowChecker::new();
    assert_eq!(checker.check_program(&program), Vec::<String>::new());

    let output = std::env::temp_dir().join(format!("tyrus-http-test-{}", std::process::id()));
    compile(&program, "http_test", output.to_str().unwrap(), false).unwrap();
    let ir_path = output.with_extension("ll");
    let ir = std::fs::read_to_string(&ir_path).unwrap();
    assert!(ir.contains("@tyrus_http_router_new"));
    assert!(ir.contains("@tyrus_http_router_cors"));
    assert!(ir.contains("@tyrus_http_router_trusted_proxies"));
    assert!(ir.contains("@tyrus_http_router_rate_limit"));
    assert!(ir.contains("@tyrus_http_router_get"));
    assert!(ir.contains("@tyrus_http_router_options"));
    assert!(ir.contains("@tyrus_http_router_query"));
    assert!(ir.contains("@tyrus_http_router_listen"));
    assert!(ir.contains("@__tyrus_http_callback_handler"));
    assert!(ir.contains("i64 20"));
    assert!(ir.contains("i64 50"));
    assert!(ir.contains("i64 250"));
    assert!(ir.contains("i64 3"));
    assert!(ir.contains("i64 2"));
    assert!(ir.contains("i64 900000"));
    assert!(ir.contains("i64 600000"));
    assert!(ir.contains("i64 60000"));
    std::fs::remove_file(ir_path).unwrap();
}

#[test]
pub(super) fn emits_http_pre_handler_callback_table() {
    let ir = compile_to_ir(
        r#"
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
        "http_pre_handler",
    );
    assert!(
        ir.contains("@__tyrus_http_pre_handler_requireApiKey"),
        "{ir}"
    );
    assert!(ir.contains("@__tyrus_http_pre_handlers_"), "{ir}");
    assert!(ir.contains("pre_handler.has_response"), "{ir}");
    assert!(ir.contains("pre_handler.response"), "{ir}");
}

#[test]
pub(super) fn emits_http_response_hook_callback() {
    let ir = compile_to_ir(
        r#"
            enum HttpStatus { Ok = 200 }
            function recordAccess(access: HttpAccessLog): void {
                console.log(access.requestId, access.duration.milliseconds);
            }
            const app: HttpRouter = __tyrus_http_createRouter();
            app.onResponse(recordAccess);
        "#,
        "http_response_hook",
    );
    assert!(ir.contains("@tyrus_http_router_on_response"), "{ir}");
    assert!(
        ir.contains("@__tyrus_http_response_hook_recordAccess"),
        "{ir}"
    );
}

#[test]
pub(super) fn emits_http_server_logger_option() {
    let ir = compile_to_ir(
        r#"
            const app: HttpRouter = __tyrus_http_createRouter();
            app.listen(3000, { logger: true });
        "#,
        "http_server_logger",
    );
    assert!(ir.contains("i64 -1, i64 1, { ptr, i64 }"), "{ir}");
    assert!(ir.contains("@tyrus_http_router_listen"), "{ir}");
}

#[test]
pub(super) fn emits_http_csrf_policy() {
    let ir = compile_to_ir(
        r#"
            const app: HttpRouter = __tyrus_http_createRouter();
            app.csrf({ cookie: "__Host-csrf", header: "x-csrf-token" });
        "#,
        "http_csrf",
    );
    assert!(ir.contains("@tyrus_http_router_csrf"), "{ir}");
    assert!(ir.contains("csrf_options"), "{ir}");
}

#[test]
pub(super) fn emits_http_no_content_void_response_contract() {
    let ir = compile_to_ir(
        r#"
            enum HttpStatus { NoContent = 204 }
            const remove = (request: {}, reply: HttpReply<void>): void => {
                reply.status(HttpStatus.NoContent).send();
            };
            const app: HttpRouter = __tyrus_http_createRouter();
            app.delete("/users/:id", {
                response: { 204: void },
                handler: remove
            });
        "#,
        "http_no_content_void",
    );
    assert!(ir.contains("http.reply.empty.body"), "{ir}");
    assert!(ir.contains("@tyrus_http_router_delete"), "{ir}");
}

#[test]
pub(super) fn emits_http_schema_wrapper_validation_and_json_calls() {
    let source = r#"
        interface CreateUserRequest {
            @notEmpty()
            name: string;
            @min(18)
            age: i32;
            amount: decimal;
            tags: string[];
        }
        interface UserResponse {
            @notEmpty()
            id: string;
            name: string;
            nickname?: string;
            adult: boolean;
            amount: decimal;
            tags: string[];
        }
        type CreateUserRouteRequest = {
            body: CreateUserRequest;
        };
        enum HttpStatus { Created = 201 }
        const createUser = (request: CreateUserRouteRequest, reply: HttpReply<UserResponse>): void => {
            reply.status(HttpStatus.Created).send({ id: "user-1", name: request.body.name, adult: true, amount: request.body.amount, tags: request.body.tags });
        };
        const app: HttpRouter = __tyrus_http_createRouter();
        app.post("/users", {
            body: CreateUserRequest,
            response: { 201: UserResponse },
            handler: createUser
        });
    "#;
    let tokens = Token::lexer(source).filter_map(Result::ok).collect();
    let program = Parser::new(tokens).parse_program().unwrap();
    let output =
        std::env::temp_dir().join(format!("tyrus-http-schema-test-{}", std::process::id()));
    compile(
        &program,
        "http_schema_test",
        output.to_str().unwrap(),
        false,
    )
    .unwrap();
    let ir_path = output.with_extension("ll");
    let ir = std::fs::read_to_string(&ir_path).unwrap();

    assert!(ir.contains("@__tyrus_http_callback_createUser"));
    assert!(ir.contains("@tyrus_json_document_parse"));
    assert!(ir.contains("@tyrus_json_document_get_string"));
    assert!(ir.contains("@tyrus_json_document_get_i32"));
    assert!(ir.contains("@tyrus_json_document_get_decimal"));
    assert!(ir.contains("@tyrus_json_document_get_array"));
    assert!(ir.contains("@tyrus_json_array_get_string"));
    assert!(ir.contains("@tyrus_json_array_free"));
    assert!(ir.contains("@tyrus_json_array_builder_new"));
    assert!(ir.contains("@tyrus_json_builder_add_raw"));
    assert!(ir.contains("automatic.json.add_array"));
    assert!(ir.contains("@tyrus_json_document_has_only_fields"));
    assert!(ir.contains("@tyrus_json_document_free"));
    assert!(ir.contains("http.json.known_fields.valid"));
    assert!(ir.contains("@tyrus_validate_CreateUserRequest_ok"));
    assert!(ir.contains("@tyrus_validate_UserResponse_ok"));
    assert!(ir.contains("@tyrus_json_builder_add_string"));
    assert!(ir.contains("@tyrus_json_builder_add_decimal"));
    assert!(ir.contains("automatic.json.optional.has_value"));
    assert!(ir.contains("automatic.json.optional.skip"));
    assert!(ir.contains("automatic.json.finish"));
    assert!(
        !ir.contains("http.json.builder"),
        "HTTP responses must use the same object writer as `toJson()`:\n{ir}"
    );
    assert!(ir.contains("@tyrus_http_set_validation_error"));
    std::fs::remove_file(ir_path).unwrap();
}

#[test]
pub(super) fn emits_http_inline_declarative_route_handlers() {
    let source = r#"
        interface UserParams {
            id: string;
        }
        interface CreateUserBody {
            name: string;
        }
        interface UserQuery {
            active: boolean;
        }
        interface AuthHeaders {
            authorization: string;
        }
        interface UserCookies {
            session: string;
        }
        interface UserResponse {
            id: string;
            name: string;
        }
        interface ApiError {
            message: string;
        }
        interface CookieOptions {
            domain?: string;
            path?: string;
            maxAge?: i64;
            secure?: boolean;
            httpOnly?: boolean;
            sameSite?: string;
        }
        enum HttpStatus { Created = 201 }
        const app: HttpRouter = __tyrus_http_createRouter();
        app.post("/users/:id", {
            params: UserParams,
            query: UserQuery,
            headers: AuthHeaders,
            cookies: UserCookies,
            body: CreateUserBody,
            response: { 201: UserResponse, 409: ApiError },
            handler: (request, reply) => {
                const active: boolean = request.query.active;
                const authorization: string = request.headers.authorization;
                const session: string = request.cookies.session;
                console.log(active);
                console.log(authorization);
                console.log(session);
                reply.status(HttpStatus.Created).send({
                    id: request.params.id,
                    name: request.body.name
                });
            }
        });
    "#;
    let tokens = Token::lexer(source).filter_map(Result::ok).collect();
    let program = Parser::new(tokens).parse_program().unwrap();
    let output = std::env::temp_dir().join(format!(
        "tyrus-http-inline-schema-test-{}",
        std::process::id()
    ));
    compile(
        &program,
        "http_inline_schema_test",
        output.to_str().unwrap(),
        false,
    )
    .unwrap();
    let ir_path = output.with_extension("ll");
    let ir = std::fs::read_to_string(&ir_path).unwrap();

    assert!(ir.contains("@__tyrus_http_inline_handler_"));
    assert!(ir.contains("@tyrus_json_document_get_string"));
    assert!(ir.contains("@tyrus_json_document_get_bool"));
    assert!(ir.contains("@tyrus_validate_UserParams_ok"));
    assert!(ir.contains("@tyrus_validate_UserQuery_ok"));
    assert!(ir.contains("@tyrus_validate_AuthHeaders_ok"));
    assert!(ir.contains("@tyrus_validate_UserCookies_ok"));
    assert!(ir.contains("@tyrus_validate_CreateUserBody_ok"));
    assert!(ir.contains("@tyrus_validate_UserResponse_ok"));
    assert!(ir.contains("i32 201"));
    std::fs::remove_file(ir_path).unwrap();
}

#[test]
pub(super) fn emits_http_reply_status_send_handlers() {
    let source = r#"
        enum HttpStatus { Created = 201, Conflict = 409 }
        interface CreateUserBody {
            name: string;
        }
        interface UserResponse {
            id: string;
            name: string;
        }
        interface ApiError {
            message: string;
        }
        const app: HttpRouter = __tyrus_http_createRouter();
        app.post("/users", {
            body: CreateUserBody,
            response: { 201: UserResponse, 409: ApiError },
            handler: (request, reply) => {
                const conflict: boolean = request.body.name === "conflict";
                if (conflict) {
                    return reply.status(HttpStatus.Conflict).send({ message: "Email already exists" });
                }
                reply
                    .setCookie("session", "abc", { path: "/", secure: true, httpOnly: true, sameSite: "lax" })
                    .clearCookie("preview")
                    .header("x-request-id", "request-42")
                    .appendHeader("link", "</users?page=2>; rel=next")
                    .removeHeader("x-unused")
                    .type("application/json; charset=utf-8")
                    .status(HttpStatus.Created)
                    .send({
                        id: "user-1",
                        name: request.body.name
                    });
            }
        });
    "#;
    let tokens = Token::lexer(source).filter_map(Result::ok).collect();
    let program = Parser::new(tokens).parse_program().unwrap();
    let output = std::env::temp_dir().join(format!(
        "tyrus-http-reply-schema-test-{}",
        std::process::id()
    ));
    compile(
        &program,
        "http_reply_schema_test",
        output.to_str().unwrap(),
        false,
    )
    .unwrap();
    let ir_path = output.with_extension("ll");
    let ir = std::fs::read_to_string(&ir_path).unwrap();

    assert!(ir.contains("@__tyrus_http_inline_handler_"));
    assert!(ir.contains("@__tyrus_http_callback___tyrus_http_inline_handler_"));
    assert!(ir.contains("@tyrus_http_reply_set_cookie"));
    assert!(ir.contains("@tyrus_http_reply_set_cookie_with_options"));
    assert!(ir.contains("@tyrus_http_reply_clear_cookie"));
    assert!(ir.contains("@tyrus_http_reply_header"));
    assert!(ir.contains("@tyrus_http_reply_append_header"));
    assert!(ir.contains("@tyrus_http_reply_remove_header"));
    assert!(ir.contains("@tyrus_http_reply_type"));
    assert!(ir.contains("@tyrus_validate_ApiError_ok"));
    assert!(ir.contains("Email already exists"));
    assert!(ir.contains("http.reply.status.ptr"));
    assert!(ir.contains("http.reply.body.ptr"));
    assert!(ir.contains("@tyrus_validate_CreateUserBody_ok"));
    assert!(ir.contains("@tyrus_validate_UserResponse_ok"));
    assert!(ir.contains("i32 201"));
    let set_cookie = ir.find("call i32 @tyrus_http_reply_set_cookie").unwrap();
    let clear_cookie = ir.find("call i32 @tyrus_http_reply_clear_cookie").unwrap();
    let status = ir.rfind("http.reply.status.ptr").unwrap();
    let body = ir.find("http.reply.body.ptr").unwrap();
    assert!(set_cookie < clear_cookie && clear_cookie < status && status < body);
    std::fs::remove_file(ir_path).unwrap();
}

#[test]
pub(super) fn emits_http_named_declarative_route_handlers() {
    let source = r#"
        interface UserParams {
            id: string;
        }
        interface UserResponse {
            id: string;
        }
        type GetUserRequest = {
            params: UserParams;
        };
        enum HttpStatus { Ok = 200 }
        const getUser = (request: GetUserRequest, reply: HttpReply<UserResponse>): void => {
            reply.status(HttpStatus.Ok).send({ id: request.params.id });
        };
        const app: HttpRouter = __tyrus_http_createRouter();
        app.get("/users/:id", {
            params: UserParams,
            response: { 200: UserResponse },
            handler: getUser
        });
    "#;
    let tokens = Token::lexer(source).filter_map(Result::ok).collect();
    let program = Parser::new(tokens).parse_program().unwrap();
    let output = std::env::temp_dir().join(format!(
        "tyrus-http-named-schema-test-{}",
        std::process::id()
    ));
    compile(
        &program,
        "http_named_schema_test",
        output.to_str().unwrap(),
        false,
    )
    .unwrap();
    let ir_path = output.with_extension("ll");
    let ir = std::fs::read_to_string(&ir_path).unwrap();

    assert!(ir.contains("@__tyrus_http_callback_getUser"));
    assert!(ir.contains("@tyrus_validate_UserParams_ok"));
    assert!(ir.contains("@tyrus_validate_UserResponse_ok"));
    std::fs::remove_file(ir_path).unwrap();
}

#[test]
pub(super) fn emits_a_writable_http_reply_stream_through_the_native_abi() {
    let ir = compile_module_to_ir(
        r#"
        import { createRouter, HttpReply, HttpRouter, HttpStatus, StreamBody } from "tyrus:http";
        import { WritableStream } from "tyrus:stream";

        const app: HttpRouter = createRouter();

        app.get("/bytes", {
            response: { 200: StreamBody },
            handler: async (request, reply): Promise<void> => {
                const body: WritableStream<u8[]> = reply.status(HttpStatus.Ok).stream();
                await body.write([0, 255]);
                await body.close();
            },
        });

        function main(): void {}
        "#,
        "http_reply_writable_stream",
    );
    assert!(
        ir.contains("tyrus_http_reply_writable_stream"),
        "reply.stream must open its writable HTTP body through the native ABI:\n{ir}"
    );
}
