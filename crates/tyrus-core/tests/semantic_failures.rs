#[path = "semantic_failures/db_model_failures.rs"]
mod db_model_failures;

use logos::Logos;
use tyrus_core::lexer::Token;
use tyrus_core::parser::Parser;
use tyrus_core::span::Span;
use tyrus_core::type_checker::{CheckSeverity, TypeChecker};

fn error_codes(source: &str) -> Vec<&'static str> {
    let tokens = tyrus_core::lexer::lex(source).unwrap();
    let program = Parser::new(tokens).parse_program().unwrap();
    TypeChecker::new()
        .check_program(&program)
        .into_iter()
        .filter(|diagnostic| diagnostic.severity == CheckSeverity::Error)
        .map(|diagnostic| diagnostic.code)
        .collect()
}

fn fs_error_codes(body: &str) -> Vec<&'static str> {
    let source = format!(
        "import {{ readFileSync, writeFileSync }} from \"tyrus:fs\";\nconst main = (): void => {{ {body} }};"
    );
    let tokens = tyrus_core::lexer::lex(&source).unwrap();
    let program = Parser::new(tokens).parse_program().unwrap();
    let imports: tyrus_core::collections::HashMap<String, tyrus_core::ast::Type> =
        tyrus_core::collections::HashMap::default();
    let mut checker = TypeChecker::new();
    checker.set_import_targets(
        [
            (
                "readFileSync".to_string(),
                "__tyrus_fs_readFileSync".to_string(),
            ),
            (
                "writeFileSync".to_string(),
                "__tyrus_fs_writeFileSync".to_string(),
            ),
        ]
        .into_iter()
        .collect(),
    );
    let _ = imports;
    checker
        .check_program(&program)
        .into_iter()
        .filter(|diagnostic| diagnostic.severity == CheckSeverity::Error)
        .map(|diagnostic| diagnostic.code)
        .collect()
}

fn cli_error_codes(body: &str) -> Vec<&'static str> {
    let source = format!(
        "import {{ input, password, confirm, select, multiselect, number, isTTY }} from \"tyrus:cli\";\n{body}"
    );
    let tokens = tyrus_core::lexer::lex(&source).unwrap();
    let program = Parser::new(tokens).parse_program().unwrap();
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
    TypeChecker::new()
        .check_program_with_imports(&program, &imports)
        .into_iter()
        .filter(|diagnostic| diagnostic.severity == CheckSeverity::Error)
        .map(|diagnostic| diagnostic.code)
        .collect()
}

#[test]
fn rejects_is_tty_with_arguments() {
    let codes = cli_error_codes(
        "const interactive: boolean = isTTY(\"anything\");\nconsole.log(interactive);",
    );
    assert!(codes.contains(&"E0061"), "got {codes:?}");
}

#[test]
fn rejects_cli_prompt_wrong_return_binding() {
    let codes = cli_error_codes(
        "const port: f64 = select({ message: \"m\", choices: [\"a\"] });\nconsole.log(port);",
    );
    assert!(codes.contains(&"E0308"), "got {codes:?}");
}

#[test]
fn rejects_mixed_typed_numeric_array_elements() {
    let codes = error_codes("const values = [0u8, 1u16]; console.log(values);");
    assert!(codes.contains(&"E0310"), "got {codes:?}");
}

#[test]
fn rejects_cli_prompt_wrong_option_field_type() {
    let codes = cli_error_codes(
        "const t: string = select({ message: \"m\", choices: [\"a\"], pageSize: \"big\" });\nconsole.log(t);",
    );
    assert!(codes.contains(&"E0308"), "got {codes:?}");
}

fn all_codes(source: &str) -> Vec<&'static str> {
    let tokens = tyrus_core::lexer::lex(source).unwrap();
    let program = Parser::new(tokens).parse_program().unwrap();
    TypeChecker::new()
        .check_program(&program)
        .into_iter()
        .map(|diagnostic| diagnostic.code)
        .collect()
}

#[test]
fn flags_use_of_possibly_uninitialized_variable() {
    let flagged = [
        "const run = (): void => { let x: i32; console.log(x); };",
        "const run = (ok: boolean): void => { let x: i32; if (ok) { x = 1; } console.log(x); };",
        "const run = (): void => { let x: i32; for (let i: i32 = 0; i < 3; i++) { x = i; } console.log(x); };",
        "const run = (): void => { let x: i32; x += 1; console.log(x); };",
    ];
    for source in flagged {
        assert!(
            all_codes(source).contains(&"E0381"),
            "expected E0381 for `{source}`"
        );
    }
}

#[test]
fn flags_declared_but_never_assigned() {
    let source = "const run = (): void => { let message: string; console.log(\"hi\"); };";
    let codes = all_codes(source);
    assert!(codes.contains(&"E0382"), "expected E0382 for `{source}`");
    assert!(!codes.contains(&"E0381"));
}

#[test]
fn accepts_definitely_assigned_deferred_variables() {
    let ok = [
        "const run = (): void => { let x: i32; x = 1; console.log(x); };",
        "const run = (ok: boolean): void => { let x: i32; if (ok) { x = 1; } else { x = 2; } console.log(x); };",
        "const run = (ok: boolean): void => { let x: i32; if (ok) { x = 1; } else { return; } console.log(x); };",
        "const run = (): void => { let x: i32; do { x = 1; } while (false); console.log(x); };",
        "const fetch = (): i32 => { return 1; }; const run = (): void => { let x: i32; x = fetch(); console.log(x); };",
        "const run = (): void => { let x: i32; const assign = (): void => { x = 5; }; assign(); console.log(x); };",
    ];
    for source in ok {
        assert!(
            !all_codes(source).contains(&"E0381"),
            "unexpected E0381 for `{source}`"
        );
    }
}

#[test]
fn const_cannot_be_deferred() {
    let tokens = Token::lexer("const x: i32;")
        .collect::<Result<Vec<_>, _>>()
        .unwrap();
    let error = Parser::new(tokens).parse_program().unwrap_err();
    assert!(error.message.contains("const"), "got: {error}");
}

#[test]
fn let_can_be_deferred() {
    let tokens = Token::lexer("const run = (): void => { let x: i32; x = 1; };")
        .collect::<Result<Vec<_>, _>>()
        .unwrap();
    assert!(Parser::new(tokens).parse_program().is_ok());
}

#[test]
#[allow(clippy::too_many_lines)]
fn rejects_semantically_invalid_programs() {
    let cases = [
        ("const value: i32 = \"text\";", "E0308"),
        ("const value: i32 = 1 + false;", "E0369"),
        ("const f = (x: i32): i32 => x; f();", "E0061"),
        ("const f = (x: i32): i32 => x; f(\"x\");", "E0308"),
        ("const f = (): i32 => { return \"x\"; };", "E0308"),
        ("const f = (): i32 => { const x: i32 = 1; };", "E0310"),
        ("missing();", "E0425"),
        ("const id: i32 = 42; const m = `Valor: ${id:.2f}`;", "E0717"),
        (
            "const pi: f64 = 3.14; const m = `Valor: ${pi:04d}`;",
            "E0717",
        ),
        (
            "const main = (): void => { const bad = { naoDeclarado }; console.log(bad); };",
            "E0425",
        ),
        (
            "const pair = (): [i32, string] => [1, \"a\"]; const main = (): void => { const [x, _] = pair(); console.log(_); };",
            "E0425",
        ),
        (
            "const main = (): void => { if (true) { const hidden: i32 = 1; } console.log(hidden); };",
            "E0425",
        ),
        (
            "const main = (): void => { const x: i32 = 1; const x: i32 = 2; };",
            "E0428",
        ),
        (
            "const main = (): void => { if (1) { console.log(1); } };",
            "E0308",
        ),
        (
            "const main = (): void => { const x: i32 = 1; x = 2; };",
            "E0384",
        ),
        (
            "const main = (): void => { const x: i32 = 1; x(); };",
            "E0618",
        ),
        ("const value: i32 = 1!;", "E0277"),
        (
            "const value: string? = \"x\"; const forced: string = value!;",
            "E0716",
        ),
        ("const value: string = null;", "E3080"),
        (
            "const main = (): void => { let value: boolean = true; value &&= 1; };",
            "E0308",
        ),
        (
            "const main = (): void => { let value: i32 = 1; value ||= true; };",
            "E0368",
        ),
        (
            "const main = (): void => { let value: string = \"x\"; value ??= \"y\"; };",
            "E0368",
        ),
        (
            "interface User { name: string; active: boolean; } const user: User = { name: 42 };",
            "E0308",
        ),
        (
            "interface User { id: i32; name: string; } const user: User = { id: 1 };",
            "E2741",
        ),
        (
            "interface User { name: string; } const user: User = { name: 'Ada' }; console.log(user.missing);",
            "E0609",
        ),
        (
            "const app: HttpRouter = __tyrus_http_createRouter(); app.missing();",
            "E0599",
        ),
        (
            "const app: HttpRouter = __tyrus_http_createRouter(); app.get('/health');",
            "E0061",
        ),
        (
            "const invalid = (name: string): string => name; const app: HttpRouter = __tyrus_http_createRouter(); app.post('/users', invalid);",
            "E0308",
        ),
        (
            "interface Request { name?: string; } interface Response { ok: boolean; } const handler = (request: &Request): Response => ({ ok: true }); const app: HttpRouter = __tyrus_http_createRouter(); app.post('/users', { response: { 200: Response }, handler: handler });",
            "E3108",
        ),
        (
            "interface Request { name: string; } interface Response { ok: boolean; } const handler = (request: &Request): Response => ({ ok: true }); const app: HttpRouter = __tyrus_http_createRouter(); app.post('/users', { response: { 200: Response } });",
            "E2741",
        ),
        (
            "interface Request { name: string; } interface Response { ok: boolean; } const handler = (request: &Request): Response => ({ ok: true }); const app: HttpRouter = __tyrus_http_createRouter(); app.post('/users', { response: { ok: Response }, handler: handler });",
            "E0308",
        ),
        (
            "interface Response { ok: boolean; } const app: HttpRouter = __tyrus_http_createRouter(); app.post('/users', { body: MissingBody, response: { 200: Response }, handler: (request) => ({ ok: true }) });",
            "E0308",
        ),
        (
            "enum HttpStatus { Ok = 200 } interface Response { ok: boolean; } const app: HttpRouter = __tyrus_http_createRouter(); app.post('/users', { response: { 200: Response }, handler: (request, reply) => { reply.status(HttpStatus.Ok).send({ ok: request.body.name }); } });",
            "E0609",
        ),
        (
            "interface Response { ok: boolean; } const app: HttpRouter = __tyrus_http_createRouter(); app.get('/users', { response: { 99: Response }, handler: (request) => ({ ok: true }) });",
            "E0308",
        ),
        (
            "interface Response { ok: boolean; } const app: HttpRouter = __tyrus_http_createRouter(); app.get('/users', { response: { 200: Response, 200: Response }, handler: (request) => ({ ok: true }) });",
            "E2300",
        ),
        (
            "interface Response { ok: boolean; } const app: HttpRouter = __tyrus_http_createRouter(); app.get('/users/:id', { response: { 200: Response }, handler: (request) => ({ ok: true }) });",
            "E2741",
        ),
        (
            "interface Params { userId: string; } interface Response { ok: boolean; } const app: HttpRouter = __tyrus_http_createRouter(); app.get('/users/:id', { params: Params, response: { 200: Response }, handler: (request) => ({ ok: true }) });",
            "E2741",
        ),
        (
            "enum HttpStatus { Ok = 200 } interface Response { ok: boolean; } const app: HttpRouter = __tyrus_http_createRouter(); app.get('/users', { response: { 200: Response }, handler: (request, reply) => { reply.status(200).send({ ok: true }); } });",
            "E3107",
        ),
        (
            "enum HttpStatus { Ok = 200, Created = 201 } interface Response { ok: boolean; } const app: HttpRouter = __tyrus_http_createRouter(); app.get('/users', { response: { 200: Response }, handler: (request, reply) => { reply.status(HttpStatus.Created).send({ ok: true }); } });",
            "E3106",
        ),
        (
            "enum HttpStatus { Ok = 200 } interface Response { ok: boolean; } const app: HttpRouter = __tyrus_http_createRouter(); app.get('/users', { response: { 200: Response }, handler: (request, reply) => { reply.status(HttpStatus.Ok).send('wrong'); } });",
            "E3105",
        ),
        (
            "enum HttpStatus { NotFound = 404 } interface ApiError { code: string; message: string; } const app: HttpRouter = __tyrus_http_createRouter(); app.get('/users', { response: { 404: ApiError }, handler: (request, reply) => { reply.status(HttpStatus.NotFound).send({ message: 'User not found' }); } });",
            "E3105",
        ),
        (
            "interface HttpReply<T> {} interface HttpRouter {} interface ApiError { code: string; message: string; } enum HttpStatus { NotFound = 404 } const getUser = (request: { params: { id: string } }, reply: HttpReply<ApiError>): void => { reply.status(HttpStatus.NotFound).send({ message: 'User not found' }); }; const app: HttpRouter = __tyrus_http_createRouter(); app.get('/users/:id', { response: { 404: ApiError }, handler: getUser });",
            "E3105",
        ),
        (
            "enum HttpStatus { NoContent = 204 } interface Response { ok: boolean; } const app: HttpRouter = __tyrus_http_createRouter(); app.delete('/users', { response: { 204: Response }, handler: (request, reply) => { reply.status(HttpStatus.NoContent).send({ ok: true }); } });",
            "E0308",
        ),
        (
            "enum HttpStatus { NoContent = 204 } interface Response { ok: boolean; } const app: HttpRouter = __tyrus_http_createRouter(); app.delete('/users', { response: { 204: Response }, handler: (request, reply) => { reply.status(HttpStatus.NoContent).send(); } });",
            "E0308",
        ),
        (
            "enum HttpStatus { Ok = 200 } const app: HttpRouter = __tyrus_http_createRouter(); app.get('/users', { response: { 200: void }, handler: (request, reply) => { reply.status(HttpStatus.Ok).send(); } });",
            "E0308",
        ),
        (
            "enum HttpStatus { Ok = 200 } interface Response { ok: boolean; } const app: HttpRouter = __tyrus_http_createRouter(); app.get('/users', { response: { 200: Response }, handler: (request, reply) => { reply.send({ ok: true }); } });",
            "E3101",
        ),
        (
            "const app: HttpRouter = __tyrus_http_createRouter(); app.trustedProxies(['10.0.0.0/33']);",
            "E3116",
        ),
        (
            "interface Q { name?: string; } const take = (value: string): string => value; const main = (): void => { let q: Q = { name: 'Ada' }; if (q.name) { console.log(take(q.name)); } };",
            "E0308",
        ),
        (
            "interface HttpReply<T> {} interface Tag { name: string; } interface CreateBody { name: string; tags?: Tag[]; } interface Created { name: string; } enum HttpStatus { Created = 201 } const app: HttpRouter = __tyrus_http_createRouter(); app.post('/users', { body: CreateBody, response: { 201: Created }, handler: (request, reply) => { reply.status(HttpStatus.Created).send({ name: 'x' }); } });",
            "E0308",
        ),
        (
            "interface HttpServer {} interface HttpError { code: string; message: string; } const app: HttpRouter = __tyrus_http_createRouter(); const run = (): void => { const server = app.listen(3000) onfail { return; }; server.stop(); };",
            "E0599",
        ),
        (
            "const app: HttpRouter = __tyrus_http_createRouter(); app.start(3000);",
            "E0599",
        ),
        (
            "interface HttpServer {} interface HttpError { code: string; message: string; } const app: HttpRouter = __tyrus_http_createRouter(); const main = (): void => { const server = app.listen(3000) onfail { return; }; server.closed(); };",
            "E0599",
        ),
        (
            "interface HttpError { code: string; message: string; } const app: HttpRouter = __tyrus_http_createRouter(); app.listen(70000);",
            "E3111",
        ),
        (
            "interface ServerOptions { host?: string; requestTimeout?: string; maxConcurrentRequests?: i32; } const options: ServerOptions = { host: '127.0.0.1' }; const app: HttpRouter = __tyrus_http_createRouter(); app.listen(3000, options);",
            "E3113",
        ),
        (
            "interface ServerOptions { host?: string; requestTimeout?: string; maxConcurrentRequests?: i32; } const app: HttpRouter = __tyrus_http_createRouter(); app.listen(3000, { requestTimeout: '10 fortnights' });",
            "E3112",
        ),
        (
            "interface ServerOptions { host?: string; requestTimeout?: string; maxConcurrentRequests?: i32; } const app: HttpRouter = __tyrus_http_createRouter(); app.listen(3000, { host: 'not-an-address' });",
            "E3112",
        ),
        (
            "interface ServerOptions { host?: string; requestTimeout?: string; maxConcurrentRequests?: i32; } const app: HttpRouter = __tyrus_http_createRouter(); app.listen(3000, { maxConcurrentRequests: 0 });",
            "E3112",
        ),
        (
            "const main = (): void => { const n: i32 = 1; const m: i32 = n.copy(); };",
            "E0599",
        ),
        (
            "const main = (): void => { const a: i32[] = [1, 2]; const b: i32[] = a.copy(2); };",
            "E0061",
        ),
        (
            "interface ServerOptions { host?: string; requestTimeout?: string; maxConcurrentRequests?: i32; headerTimeout?: string; maxHeaderSize?: string; } const app: HttpRouter = __tyrus_http_createRouter(); app.listen(3000, { headerTimeout: '0s' });",
            "E3112",
        ),
        (
            "interface ServerOptions { host?: string; requestTimeout?: string; maxConcurrentRequests?: i32; headerTimeout?: string; maxHeaderSize?: string; } const app: HttpRouter = __tyrus_http_createRouter(); app.listen(3000, { headerTimeout: '2 fortnights' });",
            "E3112",
        ),
        (
            "interface ServerOptions { host?: string; requestTimeout?: string; maxConcurrentRequests?: i32; headerTimeout?: string; maxHeaderSize?: string; } const app: HttpRouter = __tyrus_http_createRouter(); app.listen(3000, { maxHeaderSize: 'off' });",
            "E3112",
        ),
        (
            "interface ServerOptions { host?: string; requestTimeout?: string; maxConcurrentRequests?: i32; headerTimeout?: string; maxHeaderSize?: string; } const app: HttpRouter = __tyrus_http_createRouter(); app.listen(3000, { maxHeaderSize: '1kb' });",
            "E3112",
        ),
        (
            "interface HttpReply<T> {} enum Shape { Circle(f64), Square(f64) } interface Body { shape: Shape; } const app: HttpRouter = __tyrus_http_createRouter(); app.get('/x', { response: { 200: Body }, handler: (request: Body, reply: HttpReply<Body>): void => {} });",
            "E0308",
        ),
        (
            "interface ServerOptions { host?: string; maxConnections?: i32; keepAlive?: boolean; tls?: TlsOptions; } interface TlsOptions { cert: string; key: string; } const app: HttpRouter = __tyrus_http_createRouter(); app.listen(3000, { maxConnections: 0 });",
            "E3112",
        ),
        (
            "interface ServerOptions { host?: string; maxConnections?: i32; keepAlive?: boolean; tls?: TlsOptions; } interface TlsOptions { cert: string; key: string; } const app: HttpRouter = __tyrus_http_createRouter(); app.listen(3000, { tls: { cert: '', key: 'server.key' } });",
            "E3112",
        ),
        (
            "interface ServerOptions { host?: string; tls?: TlsOptions; } const app: HttpRouter = __tyrus_http_createRouter(); app.listen(3000, { tls: { cert: 'a.crt', key: 'a.key' } });",
            "E0412",
        ),
        (
            "const app: HttpRouter = __tyrus_http_createRouter(); app.rateLimit({ max: 20 });",
            "E2741",
        ),
        (
            "const app: HttpRouter = __tyrus_http_createRouter(); app.rateLimit({ max: 0, timeWindow: '1 minute' });",
            "E0707",
        ),
        (
            "const app: HttpRouter = __tyrus_http_createRouter(); app.rateLimit({ max: 20, globalMax: 0, timeWindow: '1 minute' });",
            "E0707",
        ),
        (
            "const app: HttpRouter = __tyrus_http_createRouter(); app.rateLimit({ max: 20, ban: -1, timeWindow: '1 minute' });",
            "E0707",
        ),
        (
            "interface RateLimit { max: i32; timeWindow: string; ban?: i32; banWindow?: string; } const app: HttpRouter = __tyrus_http_createRouter(); app.rateLimit({ max: 20, banWindow: '1 minute', timeWindow: '1 minute' });",
            "E2741",
        ),
        (
            "const app: HttpRouter = __tyrus_http_createRouter(); app.rateLimit({ max: 20, timeWindow: '2 fortnights' });",
            "E3115",
        ),
        (
            "const app: HttpRouter = __tyrus_http_createRouter(); app.rateLimit({ max: 20, timeWindow: '1 minute', key: 'authorization' });",
            "E0609",
        ),
        (
            "enum HttpStatus { Ok = 200 } interface Response { ok: boolean; } const app: HttpRouter = __tyrus_http_createRouter(); app.get('/users', { response: { 200: Response }, config: { rateLimit: { max: 20, timeWindow: '1 minute', key: 'authorization' } }, handler: (request, reply) => { reply.status(HttpStatus.Ok).send({ ok: true }); } });",
            "E0609",
        ),
        (
            "const app: HttpRouter = __tyrus_http_createRouter(); app.trustedProxies(['not-an-address']);",
            "E3116",
        ),
        (
            "const app: HttpRouter = __tyrus_http_createRouter(); app.trustedProxies(['10.0.0.0/8'], 'x client ip');",
            "E3117",
        ),
        (
            "interface HttpReply<T> {} interface Response { ok: boolean; } enum HttpStatus { Ok = 200 } const handler = (request: MissingRequest, reply: HttpReply<Response>): void => { reply.status(HttpStatus.Ok).send({ ok: true }); }; const app: HttpRouter = __tyrus_http_createRouter(); app.get('/users', { response: { 200: Response }, handler: handler });",
            "E3110",
        ),
        (
            "enum HttpStatus { Ok = 200 } interface Response { ok: boolean; } const app: HttpRouter = __tyrus_http_createRouter(); app.get('/users', { response: { 200: Response }, handler: (request, reply) => { reply.header('x demo', 'ok').status(HttpStatus.Ok).send({ ok: true }); } });",
            "E3117",
        ),
        (
            "enum HttpStatus { Ok = 200 } interface Response { ok: boolean; } const app: HttpRouter = __tyrus_http_createRouter(); app.get('/users', { response: { 200: Response }, handler: (request, reply) => { reply.header('set-cookie', 'a=b').status(HttpStatus.Ok).send({ ok: true }); } });",
            "E3118",
        ),
        (
            "enum HttpStatus { Ok = 200 } interface Response { ok: boolean; } const app: HttpRouter = __tyrus_http_createRouter(); app.get('/users', { response: { 200: Response }, handler: (request, reply) => { reply.appendHeader('Content-Length', '1').status(HttpStatus.Ok).send({ ok: true }); } });",
            "E3118",
        ),
        (
            "enum HttpStatus { Ok = 200 } interface Response { ok: boolean; } const app: HttpRouter = __tyrus_http_createRouter(); app.get('/users', { response: { 200: Response }, handler: (request, reply) => { reply.removeHeader('transfer-encoding').status(HttpStatus.Ok).send({ ok: true }); } });",
            "E3118",
        ),
        (
            "enum HttpStatus { Ok = 200 } interface Response { ok: boolean; } const app: HttpRouter = __tyrus_http_createRouter(); app.get('/users', { response: { 200: Response }, handler: (request, reply) => { reply.header('x-demo', 'ok').send({ ok: true }); } });",
            "E3101",
        ),
        (
            "enum HttpStatus { Ok = 200 } interface Response { ok: boolean; } const app: HttpRouter = __tyrus_http_createRouter(); app.get('/users', { response: { 200: Response }, handler: (request, reply) => { return reply.send({ ok: true }); } });",
            "E3101",
        ),
        (
            "enum HttpStatus { Ok = 200 } interface Response { ok: boolean; } const app: HttpRouter = __tyrus_http_createRouter(); app.get('/users', { response: { 200: Response }, handler: (request, reply) => { reply.status(HttpStatus.Ok); } });",
            "E3102",
        ),
        (
            "enum HttpStatus { Ok = 200 } interface Response { ok: boolean; } const app: HttpRouter = __tyrus_http_createRouter(); app.get('/users', { response: { 200: Response }, handler: (request, reply) => { reply.setCookie('session', 1).status(HttpStatus.Ok).send({ ok: true }); } });",
            "E0308",
        ),
        (
            "enum HttpStatus { Ok = 200 } interface Response { ok: boolean; } const app: HttpRouter = __tyrus_http_createRouter(); app.get('/users', { response: { 200: Response }, handler: (request, reply) => { reply.setCookie('session', 'abc', {}).status(HttpStatus.Ok).send({ ok: true }); } });",
            "E0412",
        ),
        (
            "enum HttpStatus { Ok = 200 } interface Response { ok: boolean; } const app: HttpRouter = __tyrus_http_createRouter(); app.get('/users', { response: { 200: Response }, handler: (request, reply) => { reply.setCookies([{ name: 'session', value: 'abc' }]).status(HttpStatus.Ok).send({ ok: true }); } });",
            "E0412",
        ),
        (
            "enum HttpStatus { Ok = 200 } interface HttpCookie { name: string; value: string; } interface Response { ok: boolean; } const app: HttpRouter = __tyrus_http_createRouter(); app.get('/users', { response: { 200: Response }, handler: (request, reply) => { reply.setCookies(['session']).status(HttpStatus.Ok).send({ ok: true }); } });",
            "E0308",
        ),
        (
            "enum HttpStatus { Ok = 200 } interface Response { ok: boolean; } const app: HttpRouter = __tyrus_http_createRouter(); app.get('/users', { response: { 200: Response }, handler: (request, reply) => { reply.clearCookies('session').status(HttpStatus.Ok).send({ ok: true }); } });",
            "E0061",
        ),
        (
            "enum HttpStatus { Ok = 200 } interface Response { ok: boolean; } const app: HttpRouter = __tyrus_http_createRouter(); app.get('/users', { response: { 200: Response }, handler: (request, reply) => { reply.header('x-request-id', 1).status(HttpStatus.Ok).send({ ok: true }); } });",
            "E0308",
        ),
        (
            "enum HttpStatus { Ok = 200 } interface Response { ok: boolean; } const app: HttpRouter = __tyrus_http_createRouter(); app.get('/users', { response: { 200: Response }, handler: (request, reply) => { reply.status(HttpStatus.Ok).appendHeader('link', 'next').send({ ok: true }); } });",
            "E0599",
        ),
        (
            "enum HttpStatus { Ok = 200 } interface Response { ok: boolean; } const app: HttpRouter = __tyrus_http_createRouter(); app.get('/users', { response: { 200: Response }, handler: (request, reply) => { reply.status(HttpStatus.Ok).setCookie('session', 'abc').send({ ok: true }); } });",
            "E0599",
        ),
        (
            "enum HttpStatus { Ok = 200 } interface Response { ok: boolean; } const app: HttpRouter = __tyrus_http_createRouter(); app.get('/users', { response: { 200: Response }, handler: (request, reply) => { reply.error('wrong'); } });",
            "E0599",
        ),
        (
            "interface CorsOptions { origin: string[]; methods: string[]; allowedHeaders: string[]; exposedHeaders: string[]; credentials: boolean; maxAge: i32; } const app: HttpRouter = __tyrus_http_createRouter(); app.cors({ origin: ['*'], methods: ['GET'], allowedHeaders: [], exposedHeaders: [], credentials: true, maxAge: 0 });",
            "E2742",
        ),
        (
            "interface CorsOptions { origin: string[]; methods: string[]; allowedHeaders: string[]; exposedHeaders: string[]; credentials: boolean; maxAge: i32; } const app: HttpRouter = __tyrus_http_createRouter(); app.cors({ origin: ['https://example.com'], methods: ['GET'], allowedHeaders: ['*'], exposedHeaders: [], credentials: true, maxAge: 0 });",
            "E2742",
        ),
        (
            "interface CorsOptions { origin: string[]; methods: string[]; allowedHeaders: string[]; exposedHeaders: string[]; credentials: boolean; maxAge: i32; } const cors: CorsOptions = { origin: ['*'], methods: ['GET'], allowedHeaders: [], exposedHeaders: [], credentials: true, maxAge: 0 }; const app: HttpRouter = __tyrus_http_createRouter(); app.cors(cors);",
            "E2742",
        ),
        (
            "interface CorsOptions { origin: string[]; methods: string[]; allowedHeaders: string[]; exposedHeaders: string[]; credentials: boolean; maxAge: i32; } const app: HttpRouter = __tyrus_http_createRouter(); app.cors({ origin: ['https://example.com/'], methods: ['GET'], allowedHeaders: [], exposedHeaders: [], credentials: false, maxAge: 0 });",
            "E2743",
        ),
        (
            "interface CorsOptions { origin: string[]; methods: string[]; allowedHeaders: string[]; exposedHeaders: string[]; credentials: boolean; maxAge: i32; } const app: HttpRouter = __tyrus_http_createRouter(); app.cors({ origin: ['example.com'], methods: ['GET'], allowedHeaders: [], exposedHeaders: [], credentials: false, maxAge: 0 });",
            "E2743",
        ),
        (
            "interface CorsOptions { origin: string[]; methods: string[]; allowedHeaders: string[]; exposedHeaders: string[]; credentials: boolean; maxAge: i32; } const app: HttpRouter = __tyrus_http_createRouter(); app.cors({ origin: [], methods: ['GET'], allowedHeaders: [], exposedHeaders: [], credentials: false, maxAge: 0 });",
            "E2743",
        ),
        (
            "interface CorsOptions { origin: string[]; methods: string[]; allowedHeaders: string[]; exposedHeaders: string[]; credentials: boolean; maxAge: i32; } const app: HttpRouter = __tyrus_http_createRouter(); app.cors({ origin: ['https://example.com'], methods: ['GET'], allowedHeaders: ['Content Type'], exposedHeaders: [], credentials: false, maxAge: 0 });",
            "E2743",
        ),
        (
            "enum HttpStatus { Ok = 200 } interface Response { ok: boolean; } const app: HttpRouter = __tyrus_http_createRouter(); app.get('/users', { response: { 200: Response }, handler: (request, reply) => { const ok: boolean = true; } });",
            "E3100",
        ),
        (
            "enum HttpStatus { Ok = 200 } interface Response { ok: boolean; } const app: HttpRouter = __tyrus_http_createRouter(); app.get('/users', { response: { 200: Response }, handler: (request, reply) => { reply.status(HttpStatus.Ok).send({ ok: true }); reply.status(HttpStatus.Ok).send({ ok: false }); } });",
            "E3104",
        ),
        (
            "interface HttpReply<T> {} enum HttpStatus { Ok = 200 } interface Response { ok: boolean; } async function handler(request: {}, reply: HttpReply<Response>): Promise<void> { await Promise.resolve(); } const app: HttpRouter = __tyrus_http_createRouter(); app.get('/users', { response: { 200: Response }, handler: handler });",
            "E3100",
        ),
        (
            "interface HttpReply<T> {} enum HttpStatus { Ok = 200 } interface Response { ok: boolean; } async function handler(request: {}, reply: HttpReply<Response>): Promise<void> { reply.status(HttpStatus.Ok).send({ ok: true }); await Promise.resolve(); reply.status(HttpStatus.Ok).send({ ok: false }); } const app: HttpRouter = __tyrus_http_createRouter(); app.get('/users', { response: { 200: Response }, handler: handler });",
            "E3104",
        ),
        (
            "enum HttpStatus { Ok = 200 } interface Response { ok: boolean; } const app: HttpRouter = __tyrus_http_createRouter(); app.get('/users', { response: { 200: Response }, handler: (request, reply) => { reply.status(HttpStatus.Ok).send({ ok: true }).send({ ok: false }); } });",
            "E3104",
        ),
        (
            "enum HttpStatus { Ok = 200 } interface Response { ok: boolean; } const app: HttpRouter = __tyrus_http_createRouter(); app.get('/users', { response: { 200: Response }, handler: (request, reply) => { reply.status(HttpStatus.Ok).status(HttpStatus.Ok).send({ ok: true }); } });",
            "E3103",
        ),
        (
            "enum HttpStatus { Ok = 200 } interface Response { ok: boolean; } const app: HttpRouter = __tyrus_http_createRouter(); app.get('/users', { response: { 200: Response }, handler: (request, reply) => { const found: boolean = true; if (found) { reply.status(HttpStatus.Ok).send({ ok: true }); } } });",
            "E3100",
        ),
        (
            "enum HttpStatus { Ok = 200 } interface Response { ok: boolean; } const app: HttpRouter = __tyrus_http_createRouter(); app.get('/users', { response: { 200: Response }, handler: (request, reply) => { const found: boolean = true; if (found) { reply.status(HttpStatus.Ok).send({ ok: true }); } else { const ok: boolean = false; } } });",
            "E3100",
        ),
        (
            "enum HttpStatus { Ok = 200 } interface Response { ok: boolean; } const app: HttpRouter = __tyrus_http_createRouter(); app.get('/users', { response: { 200: Response }, handler: (request, reply) => { return; } });",
            "E3100",
        ),
        (
            "enum HttpStatus { Ok = 200 } interface Response { ok: boolean; } const app: HttpRouter = __tyrus_http_createRouter(); app.get('/users', { response: { 200: Response }, handler: (request, reply) => { const found: boolean = true; if (found) { reply.status(HttpStatus.Ok).send({ ok: true }); } reply.status(HttpStatus.Ok).send({ ok: false }); } });",
            "E3104",
        ),
        (
            "enum HttpStatus { Ok = 200 } interface Response { ok: boolean; } const app: HttpRouter = __tyrus_http_createRouter(); app.get('/users', { response: { 200: Response }, config: { bogus: 1 }, handler: (request, reply) => { reply.status(HttpStatus.Ok).send({ ok: true }); } });",
            "E0609",
        ),
        (
            "enum HttpStatus { Ok = 200 } interface Response { ok: boolean; } const app: HttpRouter = __tyrus_http_createRouter(); app.get('/users', { response: { 200: Response }, config: { bodyLimit: 'huge' }, handler: (request, reply) => { reply.status(HttpStatus.Ok).send({ ok: true }); } });",
            "E3114",
        ),
        (
            "enum HttpStatus { Ok = 200 } interface Response { ok: boolean; } const app: HttpRouter = __tyrus_http_createRouter(); app.get('/users', { response: { 200: Response }, config: { bodyLimit: 5 }, handler: (request, reply) => { reply.status(HttpStatus.Ok).send({ ok: true }); } });",
            "E0308",
        ),
        (
            "enum HttpStatus { Ok = 200 } interface Response { ok: boolean; } const app: HttpRouter = __tyrus_http_createRouter(); app.get('/users', { response: { 200: Response }, config: { rateLimit: { max: 20 } }, handler: (request, reply) => { reply.status(HttpStatus.Ok).send({ ok: true }); } });",
            "E2741",
        ),
        (
            "enum HttpStatus { Ok = 200 } interface Response { ok: boolean; } const app: HttpRouter = __tyrus_http_createRouter(); app.get('/users', { response: { 200: Response }, config: { rateLimit: { max: 'x', timeWindow: '1 minute' } }, handler: (request, reply) => { reply.status(HttpStatus.Ok).send({ ok: true }); } });",
            "E0308",
        ),
        (
            "enum HttpStatus { Ok = 200 } interface Response { ok: boolean; } const app: HttpRouter = __tyrus_http_createRouter(); app.get('/users', { response: { 200: Response }, config: { rateLimit: { max: 0, timeWindow: '1 minute' } }, handler: (request, reply) => { reply.status(HttpStatus.Ok).send({ ok: true }); } });",
            "E0707",
        ),
        (
            "enum HttpStatus { Ok = 200 } interface Response { ok: boolean; } const app: HttpRouter = __tyrus_http_createRouter(); app.get('/users', { response: { 200: Response }, config: { rateLimit: { max: 1, timeWindow: '0ms' } }, handler: (request, reply) => { reply.status(HttpStatus.Ok).send({ ok: true }); } });",
            "E0707",
        ),
        (
            "enum HttpStatus { Ok = 200 } interface Response { ok: boolean; } const app: HttpRouter = __tyrus_http_createRouter(); app.get('/users', { response: { 200: Response }, config: { rateLimit: { max: 20, timeWindow: '2 fortnights' } }, handler: (request, reply) => { reply.status(HttpStatus.Ok).send({ ok: true }); } });",
            "E3115",
        ),
        (
            "interface HttpRequest { method: string; path: string; body: string; params: string; query: string; headers: string; cookies: string; } interface HttpResponse { statusCode: HttpStatus; body: string; cookies: string; } interface HttpRouter {} enum HttpStatus { Ok = 200 } interface Response { ok: boolean; } const app: HttpRouter = __tyrus_http_createRouter(); app.get('/users', { response: { 200: Response }, preHandler: [(request: HttpRequest): HttpResponse | true => true], handler: (request, reply) => { reply.status(HttpStatus.Ok).send({ ok: true }); } });",
            "E0308",
        ),
        (
            "interface HttpRequest { method: string; path: string; body: string; params: string; query: string; headers: string; cookies: string; } interface HttpResponse { statusCode: HttpStatus; body: string; cookies: string; } interface HttpRouter {} enum HttpStatus { Ok = 200 } interface Response { ok: boolean; } function guard(request: HttpRequest): true { return true; } const app: HttpRouter = __tyrus_http_createRouter(); app.get('/users', { response: { 200: Response }, preHandler: [guard], handler: (request, reply) => { reply.status(HttpStatus.Ok).send({ ok: true }); } });",
            "E0308",
        ),
        ("const values: i32[] = [1, 'two'];", "E0308"),
        ("const values: i32[] = [1]; values[0] = 2;", "E0384"),
        ("const values: i32[] = [1]; values.push(2);", "E0384"),
        (
            "let values: i32[] = [1]; const found: boolean = values.includes('1');",
            "E0308",
        ),
        ("const value: boolean = Array.isArray();", "E0061"),
        ("const value: boolean = Array.notArray(\"text\");", "E0599"),
        (
            "const values: string[] = ['one']; const value: string = values[1.5];",
            "E0277",
        ),
        ("const value: i32 = 1; const item: i32 = value[0];", "E0608"),
        (
            "const xs: i32[] = [1, 2, 3]; const x: i32 = xs[3];",
            "E0606",
        ),
        ("const x: i32 = [1, 2, 3][7];", "E0606"),
        (
            r#"const matchResult: string[]? = "hello@example.com".match(/^([a-z]+)@([a-z]+)\.([a-z]{2,})$/); if (matchResult) { console.log(matchResult[4]); }"#,
            "E0606",
        ),
        (
            r#"const matchResult: string[]? = "hello@example.com".match(/^([a-z]+)@([a-z]+)\.([a-z]{2,})$/); const matchArr: string[] = matchResult!; console.log(matchArr[4]);"#,
            "E0606",
        ),
        (
            "const xs: i32[] = [1, 2, 3]; const x: i32 = xs[-1];",
            "E0606",
        ),
        ("const xs: i32[] = []; const x: i32 = xs[0];", "E0606"),
        (
            "let xs: i32[] = [1, 2, 3]; xs.push(4); const x: i32 = xs[4];",
            "E0606",
        ),
        (
            "let xs: i32[] = [1, 2, 3]; xs.push(4, 5); const x: i32 = xs[5];",
            "E0606",
        ),
        (
            "const run = (): void => { let xs: i32[] = [1, 2, 3]; const ys: i32[] = xs.toReversed(); console.log(ys[5]); };",
            "E0606",
        ),
        (
            "const identity = <T>(value: T): T => value; const value: i32 = identity<i32, string>(1);",
            "E0107",
        ),
        (
            "const identity = <T extends string>(value: T): T => value; const value: i32 = identity<i32>(1);",
            "E0277",
        ),
        (
            "const values: i32[] = [1]; const result: i32[] = values.filter(value => value);",
            "E0308",
        ),
        ("const values: i32[] = [1]; values.sort();", "E0384"),
        (
            "const values: i32[][] = [[1]]; const depth: i64 = 1; const result: i32[] = values.flat(depth);",
            "E0747",
        ),
        (
            "const name: string = \"tyrus\"; name.unknownMethod();",
            "E0599",
        ),
        (
            "const name: string = \"tyrus\"; const len: string = name.length;",
            "E0308",
        ),
        (
            "const name: string = \"tyrus\"; name.charAt(0, 1);",
            "E0061",
        ),
        ("const name: string = \"tyrus\"; name.indexOf();", "E0061"),
        (
            "const name: string = \"tyrus\"; const neg: string = -name;",
            "E0308",
        ),
        (
            "const value: boolean = true; const r: i32 = value & 1;",
            "E0369",
        ),
        (
            "const value: boolean = true; const r: i32 = value | 1;",
            "E0369",
        ),
        (
            "const value: boolean = true; const r: i32 = value ^ 1;",
            "E0369",
        ),
        (
            "const value: boolean = true; const r: i32 = value << 1;",
            "E0369",
        ),
        (
            "const value: boolean = true; const r: i32 = value >> 1;",
            "E0369",
        ),
        (
            "const value: boolean = true; const r: i32 = value >>> 1;",
            "E0369",
        ),
        (
            "const value: boolean = true; const r: i32 = ~value;",
            "E0308",
        ),
        ("let x: string = \"a\"; x &= 1;", "E0368"),
        ("let x: i32 = 5; x &= true;", "E0308"),
        ("const x: never = 42;", "E0308"),
        ("const x: never = \"hello\";", "E0308"),
        ("const x: \"hello\" = \"goodbye\";", "E0308"),
        ("const x: 42 = 43;", "E0308"),
        (
            "type Shape = | { kind: \"circle\"; radius: f64 } | { kind: \"square\"; size: f64 }; const shape: Shape = { kind: \"circle\", radius: 2.0 }; console.log(shape.size);",
            "E0609",
        ),
        ("const f = ([a, b]: i32): void => {};", "E0308"),
        ("const f = ({x, y}: i32): void => {};", "E0308"),
        ("enum Mix { A = 1, B = \"two\" }", "E0702"),
        ("enum Mix { A, B = 2 }", "E0703"),
        ("enum F { A = 1.5 }", "E0701"),
        ("enum Big { A = 9999999999 }", "E0705"),
        ("enum Opcode: u8 { Over = 256 }", "E0705"),
        ("enum Signed: i8 { Under = -129 }", "E0705"),
        ("enum Pointer: usize { A = 1 }", "E0701"),
        (
            "enum Status { Success } const x: i32 = Status.Nope as i32;",
            "E0609",
        ),
        (
            "enum Role { Admin = \"admin\" } const s: string = Role.Admin;",
            "E0308",
        ),
        ("enum Bad: f64 { A = 1 }", "E0701"),
        ("let t: [i32, i32] = [1, 2]; t[0] = \"x\";", "E0308"),
        ("const t: [i32, i32] = [1, 2]; t[0] = 9;", "E0384"),
        ("let t: [i32, i32] = [1, 2]; t[5] = 9;", "E2493"),
        (
            "let t: [i32, i32] = [1, 2]; let i: i32 = 0; t[i] = 9;",
            "E2493",
        ),
        (
            "const xs: i32[] = [1, 2, 3]; console.log(\"x\" in xs);",
            "E0308",
        ),
        ("const r = 0..10; console.log(\"x\" in r);", "E0308"),
        (
            "const s = Set<string>([\"a\"]); console.log(1 in s);",
            "E0308",
        ),
        (
            "const m = Map<string, i32>(); console.log(1 in m);",
            "E0308",
        ),
        ("const n: i32 = 5; console.log(\"x\" in n);", "E0308"),
        ("const r = 0..\"x\";", "E0369"),
        (
            "const main = (): void => { const n: i32 = 5; for (const x of n) { console.log(x); } };",
            "E0308",
        ),
        (
            "const main = (): void => { const x: string = \"hello\"; match (x) { 0..10 => { console.log(1); } _ => { console.log(2); } } };",
            "E0308",
        ),
    ];

    for (source, expected) in cases {
        let codes = error_codes(source);
        assert!(
            codes.contains(&expected),
            "expected {expected} for `{source}`, got {codes:?}"
        );
    }
}

#[test]
fn rejects_duplicate_enum_variant_with_a_precise_diagnostic() {
    let error = Parser::from_source("enum State { Ready, Ready }")
        .unwrap()
        .parse_program()
        .unwrap_err();
    assert_eq!(error.code, Some("E0428"));
    assert!(error.message.contains("variant `Ready`"), "{error}");
    assert_eq!(error.span, Some(20..25));
    assert_eq!(error.help, vec!["rename the duplicate variant `Ready`"]);
}

#[test]
fn at_least_one_rejects_an_object_without_a_selected_field() {
    let codes = error_codes(
        "interface Form { name: string; email?: string; phone?: string; } type ValidForm = AtLeastOne<Form, \"email\" | \"phone\">; const invalid: ValidForm = { name: \"Ada\" };",
    );
    assert!(codes.contains(&"E0308"), "got {codes:?}");
}

#[test]
fn rejects_at_least_one_with_an_unknown_selected_key() {
    let codes = error_codes(
        "interface Form { email?: string; phone?: string; } type ValidForm = AtLeastOne<Form, \"email\" | \"missing\">; const valid: ValidForm = { email: \"ada@example.com\" };",
    );
    assert!(codes.contains(&"E3212"), "got {codes:?}");
}

#[test]
fn rejects_at_least_one_with_non_literal_or_non_object_arguments() {
    let non_literal = error_codes(
        "interface Form { email?: string; } type ValidForm = AtLeastOne<Form, string>; const valid: ValidForm = { email: \"ada@example.com\" };",
    );
    assert!(non_literal.contains(&"E3209"), "got {non_literal:?}");

    let non_object =
        error_codes("type Invalid = AtLeastOne<i32, \"email\">; const value: Invalid = 1;");
    assert!(non_object.contains(&"E3211"), "got {non_object:?}");
}

fn error_codes_with_env(env_source: &str, source: &str) -> Vec<&'static str> {
    let (vars, diagnostics) = tyrus_core::env::parse_env_source(env_source);
    assert!(
        diagnostics.is_empty(),
        "invalid env fixture: {diagnostics:?}"
    );
    let fields = tyrus_core::env::EnvSchema {
        env_file: ".env".to_string(),
        file_path: std::path::PathBuf::from(".env"),
        source: env_source.to_string(),
        vars,
    }
    .object_type();
    let tokens = tyrus_core::lexer::lex(source).unwrap();
    let program = Parser::new(tokens).parse_program().unwrap();
    let mut checker = TypeChecker::new();
    checker.set_env_schema(fields);
    checker
        .check_program(&program)
        .into_iter()
        .filter(|diagnostic| diagnostic.severity == CheckSeverity::Error)
        .map(|diagnostic| diagnostic.code)
        .collect()
}

#[test]
fn flags_undeclared_process_env_variable() {
    let codes = error_codes_with_env(
        "PORT=3000\n",
        "const host: string = process.env.HOST;\nconsole.log(host);",
    );
    assert_eq!(codes, ["E3141"]);
}

#[test]
fn flags_undeclared_process_env_destructuring() {
    let codes = error_codes_with_env(
        "PORT=3000\n",
        "const { HOST } = process.env;\nconsole.log(HOST);",
    );
    assert_eq!(codes, ["E3141"]);
}

#[test]
fn flags_process_env_without_configured_env_file() {
    let codes = error_codes("const port = process.env.PORT;\nconsole.log(port);");
    assert_eq!(codes, ["E3140"]);
}

#[test]
fn flags_invalid_process_use() {
    let codes = error_codes("const id = process.bogus;\nconsole.log(id);");
    assert_eq!(codes, ["E3145"]);

    let codes = error_codes("console.log(process);");
    assert!(codes.contains(&"E3145"));
}

#[test]
fn flags_process_env_type_mismatch() {
    let codes = error_codes_with_env(
        "@type(u32)\nPORT=3000\n",
        "const port: string = process.env.PORT;\nconsole.log(port);",
    );
    assert_eq!(codes, ["E0308"]);
}

#[test]
fn flags_process_env_index_access() {
    let codes = error_codes_with_env(
        "@type(string)\nKEY=v\n",
        "const x = process.env[\"KEY\"];\nconsole.log(x);",
    );
    assert_eq!(codes, ["E3144"]);
}

#[test]
fn flags_non_callable_pipeline_target() {
    assert_eq!(
        error_codes("const r = 5 |> 10;\nconsole.log(r);"),
        ["E3290"]
    );
}

#[test]
fn flags_topic_token_outside_pipeline() {
    assert_eq!(error_codes("const r = %;\nconsole.log(r);"), ["E3291"]);
}

#[test]
fn topic_token_does_not_cross_arrow_function_boundary() {
    assert_eq!(
        error_codes(
            "const apply = (seed: i64, f: (i64) => i64): i64 => f(seed);\nconst r = 5 |> apply(%, (n: i64): i64 => n + %);\nconsole.log(r);"
        ),
        ["E3291"]
    );
}

#[test]
fn does_not_treat_nested_pipeline_topic_as_outer_topic() {
    assert_eq!(
        error_codes("const f = (n: i64): i64 => n;\nconst r = 1 |> (2 |> f(%));\nconsole.log(r);"),
        ["E3290"]
    );
}

#[test]
fn flags_pipeline_argument_type_mismatch() {
    let codes =
        error_codes("const f = (s: string): void => {};\nconst main = (): void => { 5 |> f; };");
    assert!(codes.contains(&"E0308"), "expected E0308, got {codes:?}");
}

fn style_text_error_codes(body: &str) -> Vec<&'static str> {
    let source = format!("import {{ styleText }} from \"tyrus:utils\";\n{body}");
    let tokens = tyrus_core::lexer::lex(&source).unwrap();
    let program = Parser::new(tokens).parse_program().unwrap();
    let mut imports = tyrus_core::collections::HashMap::default();
    imports.insert(
        "styleText".to_string(),
        tyrus_core::ast::Type::Named("__StyleText".to_string()),
    );
    TypeChecker::new()
        .check_program_with_imports(&program, &imports)
        .into_iter()
        .filter(|diagnostic| diagnostic.severity == CheckSeverity::Error)
        .map(|diagnostic| diagnostic.code)
        .collect()
}

fn figlet_error_codes(body: &str) -> Vec<&'static str> {
    let source = format!("import {{ figlet }} from \"tyrus:utils\";\n{body}");
    let tokens = tyrus_core::lexer::lex(&source).unwrap();
    let program = Parser::new(tokens).parse_program().unwrap();
    let mut imports = tyrus_core::collections::HashMap::default();
    imports.insert(
        "figlet".to_string(),
        tyrus_core::ast::Type::Named("__Figlet".to_string()),
    );
    TypeChecker::new()
        .check_program_with_imports(&program, &imports)
        .into_iter()
        .filter(|diagnostic| diagnostic.severity == CheckSeverity::Error)
        .map(|diagnostic| diagnostic.code)
        .collect()
}

#[test]
fn flags_unknown_style() {
    assert_eq!(
        style_text_error_codes("const a: string = styleText(\"nope\", \"hi\");\nconsole.log(a);"),
        ["E3273"]
    );
}

#[test]
fn flags_non_literal_style() {
    let codes = style_text_error_codes(
        "const fmt: string = \"green\";\nconst a: string = styleText(fmt, \"hi\");\nconsole.log(a);",
    );
    assert!(codes.contains(&"E3272"), "got {codes:?}");
}

#[test]
fn flags_incomplete_style_text() {
    let codes = style_text_error_codes("const a = styleText.green;\nconsole.log(a);");
    assert!(codes.contains(&"E3271"), "got {codes:?}");
}

#[test]
fn flags_non_string_style_text_target() {
    let codes =
        style_text_error_codes("const a: string = styleText(\"green\", 42);\nconsole.log(a);");
    assert!(codes.contains(&"E0308"), "got {codes:?}");
}

#[test]
fn flags_unknown_figlet_font() {
    assert_eq!(
        figlet_error_codes("const a: string = figlet(\"hi\", \"banner\");\nconsole.log(a);"),
        ["E3277"]
    );
}

#[test]
fn flags_non_literal_figlet_font() {
    let codes = figlet_error_codes(
        "const font: string = \"slant\";\nconst a: string = figlet(\"hi\", font);\nconsole.log(a);",
    );
    assert!(codes.contains(&"E3276"), "got {codes:?}");
}

#[test]
fn flags_non_string_figlet_target() {
    let codes = figlet_error_codes("const a: string = figlet(42, \"small\");\nconsole.log(a);");
    assert!(codes.contains(&"E0308"), "got {codes:?}");
}

#[test]
fn rejects_calling_non_function_field_and_bad_args() {
    let codes = error_codes(
        "interface H { label: string; }\nconst run = (h: H): void => { console.log(h.label(1)); };",
    );
    assert!(codes.contains(&"E0599"), "got {codes:?}");
    let codes = error_codes(
        "interface H { f: (x: i32) => i32; }\nconst run = (h: H): void => { console.log(h.f(\"x\")); };",
    );
    assert!(codes.contains(&"E0308"), "got {codes:?}");
}

#[test]
fn rejects_conflicting_inherited_fields() {
    let codes = error_codes(
        "interface A { x: i32; }\ninterface B { x: string; }\ninterface C extends A, B { }\nconst run = (c: C): void => { console.log(c.x); };",
    );
    assert!(codes.contains(&"E3204"), "got {codes:?}");
}

#[test]
fn rejects_pick_of_unknown_key() {
    let codes = error_codes(
        "interface User { name: string; }\ntype Bad = Pick<User, \"missing\">;\nconst run = (u: Bad): void => { console.log(u); };",
    );
    assert!(codes.contains(&"E3212"), "got {codes:?}");
}

#[test]
fn rejects_invalid_selective_optional_types() {
    let unknown_key = error_codes(
        "interface User { name: string; }\ntype Bad = Optional<User, \"missing\">;\nconst run = (u: Bad): void => { console.log(u); };",
    );
    assert!(unknown_key.contains(&"E3212"), "got {unknown_key:?}");

    let non_literal_key = error_codes(
        "interface User { name: string; }\ntype Bad = Optional<User, string>;\nconst run = (u: Bad): void => { console.log(u); };",
    );
    assert!(
        non_literal_key.contains(&"E3209"),
        "got {non_literal_key:?}"
    );

    let non_object = error_codes(
        "type Bad = Optional<i32, \"name\">;\nconst run = (u: Bad): void => { console.log(u); };",
    );
    assert!(non_object.contains(&"E3211"), "got {non_object:?}");
}

#[test]
fn rejects_invalid_object_pick_and_omit_methods() {
    let unknown_key = error_codes(
        "interface User { name: string; }\nconst run = (user: User): void => { user.pick([\"missing\"]); };",
    );
    assert!(unknown_key.contains(&"E0609"), "got {unknown_key:?}");

    let excluded_field = error_codes(
        "interface User { id: i32; email: string; }\nconst run = (user: User): void => { const summary = user.pick([\"id\"]); console.log(summary.email); };",
    );
    assert!(excluded_field.contains(&"E0609"), "got {excluded_field:?}");

    let dynamic_keys = error_codes(
        "interface User { name: string; }\nconst run = (user: User, keys: string[]): void => { user.omit(keys); };",
    );
    assert!(dynamic_keys.contains(&"E0308"), "got {dynamic_keys:?}");

    let non_literal_key = error_codes(
        "interface User { name: string; }\nconst run = (user: User, key: string): void => { user.pick([key]); };",
    );
    assert!(
        non_literal_key.contains(&"E0308"),
        "got {non_literal_key:?}"
    );

    let non_object = error_codes("const run = (count: i32): void => { count.pick([\"value\"]); };");
    assert!(non_object.contains(&"E0599"), "got {non_object:?}");
}

#[test]
fn rejects_wrong_map_value_type() {
    let codes = error_codes(
        "type Scores = Record<string, i32>;\nconst run = (): void => { let s: Scores = { a: 1 }; s[\"b\"] = \"oops\"; console.log(s.size); };",
    );
    assert!(codes.contains(&"E0308"), "got {codes:?}");
}

#[test]
fn rejects_wrong_map_get_or_insert_value_type() {
    let codes = error_codes(
        "const run = (): void => { let m: Map<string, i32> = Map<string, i32>(); const v = m.getOrInsert(\"key\", \"oops\"); console.log(v); };",
    );
    assert!(codes.contains(&"E0308"), "got {codes:?}");
}

#[test]
fn rejects_unindexed_map_read_as_non_optional() {
    let codes = error_codes(
        "type Scores = Record<string, i32>;\nconst run = (): void => { let s: Scores = { a: 1 }; const v: i32 = s[\"a\"]; console.log(v); };",
    );
    assert!(codes.contains(&"E0308"), "got {codes:?}");
}

#[test]
fn rejects_unknown_map_method() {
    let codes = error_codes(
        "type Scores = Record<string, i32>;\nconst run = (): void => { let s: Scores = { a: 1 }; console.log(s.clear()); };",
    );
    assert!(codes.contains(&"E0599"), "got {codes:?}");
}

#[test]
fn rejects_wrong_set_element_type() {
    let codes = error_codes(
        "const run = (): void => { let s: Set<i32> = Set<i32>(); s.add(\"x\"); console.log(s.size); };",
    );
    assert!(codes.contains(&"E0308"), "got {codes:?}");
}

#[test]
fn rejects_unknown_set_method() {
    let codes = error_codes(
        "const run = (): void => { let s: Set<i32> = Set<i32>(); console.log(s.clear()); };",
    );
    assert!(codes.contains(&"E0599"), "got {codes:?}");
}

#[test]
fn rejects_wrong_map_key_type() {
    let codes = error_codes(
        "const run = (): void => { let m: Map<string, i32> = Map<string, i32>(); m.set(1, 2); console.log(m.size); };",
    );
    assert!(codes.contains(&"E0308"), "got {codes:?}");
}

#[test]
fn rejects_assert_non_boolean() {
    let codes = error_codes("const t = (): void => { assert(42); };");
    assert!(codes.contains(&"E0308"), "got {codes:?}");
}

#[test]
fn rejects_assert_equal_type_mismatch() {
    let codes = error_codes("const t = (): void => { assertEqual(1, \"x\"); };");
    assert!(codes.contains(&"E0308"), "got {codes:?}");
}

#[test]
fn rejects_optional_chaining_on_a_non_optional_receiver() {
    let codes = error_codes(
        r#"
            const name: string = "ada";
            const size: usize? = name?.length;
        "#,
    );
    assert!(codes.contains(&"E0277"), "unexpected codes: {codes:?}");
}

#[test]
fn rejects_a_wrong_type_for_an_optional_field_access() {
    let codes = error_codes(
        r#"
            const name: string? = "ada";
            const size: string? = name?.length;
        "#,
    );
    assert!(codes.contains(&"E0308"), "unexpected codes: {codes:?}");
}

#[test]
fn rejects_bare_return_in_a_non_optional_function() {
    let codes = error_codes("const f = (): i32 => { return; };");
    assert!(codes.contains(&"E0069"), "unexpected codes: {codes:?}");
}

#[test]
fn rejects_compact_match_arms_with_inconsistent_return_types() {
    let codes = error_codes(
        r#"
            const classify = (value: i32): string => {
                match (value) {
                    1 => "one";
                    _ => 2;
                }
            };
        "#,
    );
    assert!(codes.contains(&"E0308"), "unexpected codes: {codes:?}");
}

#[test]
fn rejects_non_exhaustive_optional_match_without_a_present_arm() {
    let codes = error_codes(
        r#"
            const describe = (value: string?): void => {
                match (value) {
                    None => { console.log("missing"); }
                }
            };
        "#,
    );
    assert!(codes.contains(&"E0004"), "unexpected codes: {codes:?}");
}

#[test]
fn rejects_comparing_optional_presence_with_none() {
    let codes = error_codes(
        r#"
            const check = (value: string?): boolean => value === None;
        "#,
    );
    assert!(codes.contains(&"E3079"), "unexpected codes: {codes:?}");
}

#[test]
fn still_rejects_the_null_literal() {
    let codes = error_codes("const f = (): i32? => { return null; };");
    assert!(codes.contains(&"E3080"), "unexpected codes: {codes:?}");
}

#[test]
fn rejects_arithmetic_on_an_uninitialized_optional() {
    let codes = error_codes(
        "const run = (): void => { let x: i32?; const y: i32 = x + 1; console.log(y); };",
    );
    assert!(codes.contains(&"E0369"), "unexpected codes: {codes:?}");
}

#[test]
fn rejects_a_rest_argument_of_the_wrong_element_type() {
    let codes = error_codes(
        "const sum = (...xs: i32[]): i32 => { return 0; }; const run = (): void => { console.log(sum(1, \"x\", 3)); };",
    );
    assert!(codes.contains(&"E0308"), "unexpected codes: {codes:?}");
}

#[test]
fn rejects_a_negative_literal_against_an_unsigned_type() {
    let codes =
        error_codes("const run = (): void => { const a: u32 = 8; console.log(a & (-8)); };");
    assert!(codes.contains(&"E0369"), "unexpected codes: {codes:?}");
}

#[test]
fn rejects_a_field_absent_from_a_generic_constraint() {
    let codes = error_codes(
        "interface Named { name: string; } const bad = <T extends Named>(value: T): string => { return value.age; };",
    );
    assert!(codes.contains(&"E0609"), "unexpected codes: {codes:?}");
}

#[test]
fn rejects_bare_literal_type_annotations() {
    for source in [
        r#"const run = (): void => { let name: "TitenQ" = "TitenQ"; console.log(name); };"#,
        "const run = (): void => { let age: true = true; console.log(age); };",
        "const run = (): void => { const n: 42 = 42; console.log(n); };",
        r#"const only = (p: "yes"): void => { console.log(p); };"#,
        r#"type Only = "y"; const run = (): void => { let x: Only = "y"; console.log(x); };"#,
    ] {
        let codes = error_codes(source);
        assert!(
            codes.contains(&"E0316"),
            "expected E0316 for `{source}`, got {codes:?}"
        );
    }
}

#[test]
fn nominal_interface_mismatch_explains_and_stays_e0308() {
    let codes = error_codes(
        "interface A { x: i32; } interface B { x: i32; } const run = (): void => { const a: A = { x: 1 }; const b: B = a; console.log(b.x); };",
    );
    assert!(codes.contains(&"E0308"), "unexpected codes: {codes:?}");
}

#[test]
fn rejects_automatic_equality_for_a_matching_but_different_interface() {
    let codes = error_codes(
        "interface User { name: string; } interface OtherUser { name: string; } const run = (): void => { const user: User = { name: \"Ada\" }; const other: OtherUser = { name: \"Ada\" }; console.log(user.equals(other)); };",
    );
    assert!(codes.contains(&"E0308"), "unexpected codes: {codes:?}");
}

#[test]
fn interface_extends_still_requires_inherited_fields() {
    let codes = error_codes(
        "interface Age { age: u32; } interface Person extends Age { name: string; } const run = (): void => { const p: Person = { name: \"Ada\" }; console.log(p.name); };",
    );
    assert!(codes.contains(&"E2741"), "unexpected codes: {codes:?}");
}

#[test]
fn parse_builtins_reject_a_non_numeric_annotation() {
    assert_eq!(
        error_codes(r#"const wrong: string? = parseInt("42");"#),
        vec!["E0308"]
    );
    assert_eq!(
        error_codes(r#"const wrong: i32? = parseFloat("3.5");"#),
        vec!["E0308"]
    );
    assert_eq!(
        error_codes(r#"const wrong: i32? = parseInt(42);"#),
        vec!["E0308"]
    );
}

/// `as` converts; it does not reinterpret.
///
/// Code generation can only build the target value for a numeric conversion.
/// Every other pair used to fall through it and hand the source bits to code
/// that reads them as the target type, so `1 as string` compiled to a
/// dereference of the address `0x1` and segfaulted, and `"text" as i32`
/// printed the string's pointer. The checker is what keeps those bits apart.
#[test]
fn type_assertion_rejects_unrelated_types() {
    assert_eq!(error_codes(r#"const wrong = 1 as string;"#), vec!["E0605"]);
    assert_eq!(
        error_codes(r#"const wrong = "text" as i32;"#),
        vec!["E0605"]
    );
    assert_eq!(error_codes(r#"const wrong = true as i32;"#), vec!["E0605"]);
    assert_eq!(error_codes(r#"const wrong = 1 as boolean;"#), vec!["E0605"]);
}

#[test]
fn type_assertion_rejects_decimal_and_binary_float_conversions() {
    assert_eq!(
        error_codes(r#"const value: decimal = 1d; const wrong = value as f64;"#),
        vec!["E0605"]
    );
    assert_eq!(
        error_codes(r#"const value: f64 = 1.5; const wrong = value as decimal;"#),
        vec!["E0605"]
    );
}

#[test]
fn type_assertion_cannot_launder_a_value_into_a_validated_interface() {
    assert_eq!(
        error_codes(
            r#"
                interface Loose {
                    age: i32;
                }
                interface User {
                    @min(18)
                    age: i32;
                }
                const read = (): i32 => 1;
                const loose: Loose = { age: read() };
                const user: User = loose as User;
            "#
        ),
        vec!["E0605"]
    );
}

#[test]
fn type_assertion_rejects_building_an_enum_from_a_number() {
    assert_eq!(
        error_codes(
            r#"
                enum Code {
                    Ok = 200,
                    NotFound = 404,
                }
                const wrong: Code = 999 as Code;
            "#
        ),
        vec!["E0605"]
    );
}

#[test]
fn unknown_type_names_are_reported() {
    assert_eq!(
        error_codes(r#"const identity = (value: Bogus): Bogus => value;"#),
        vec!["E0412", "E0412"]
    );
    assert_eq!(
        error_codes(r#"const missing: Bogus[] = [];"#),
        vec!["E0412"]
    );
    assert_eq!(
        error_codes(
            r#"
                interface Holder {
                    value: Bogus;
                }
            "#
        ),
        vec!["E0412"]
    );
}

#[test]
fn an_object_literal_cannot_be_built_with_an_assertion() {
    assert_eq!(
        error_codes(
            r#"
                interface User {
                    age: i32;
                }
                const user = { age: 30 } as User;
            "#
        ),
        vec!["E0605"]
    );
}

#[test]
fn rejects_assert_throws_on_a_function_that_cannot_throw() {
    let codes = error_codes(
        "function quiet(): void { console.log(\"x\"); }\n\
         const t = (): void => { assertThrows(quiet); };",
    );
    assert!(codes.contains(&"E0277"), "got: {codes:?}");
}

#[test]
fn rejects_assert_throws_on_a_function_that_requires_arguments() {
    let codes = error_codes(
        "function boom(value: i32): void throws string { throw \"x\"; }\n\
         const t = (): void => { assertThrows(boom); };",
    );
    assert!(codes.contains(&"E0061"), "got: {codes:?}");
}

#[test]
fn rejects_an_assert_throws_message_when_the_thrown_value_is_not_a_string() {
    let codes = error_codes(
        "interface Failure { code: i32 }\n\
         function boom(): void throws Failure { throw { code: 1 }; }\n\
         const t = (): void => { assertThrows(boom, \"x\"); };",
    );
    assert!(codes.contains(&"E0308"), "got: {codes:?}");
}

#[test]
fn rejects_an_unknown_process_member() {
    let codes = error_codes("const main = (): void => { console.log(process.nope); };");
    assert!(codes.contains(&"E3145"), "got: {codes:?}");
}

#[test]
fn rejects_process_argv_used_as_a_non_string_array() {
    let codes = error_codes("const main = (): void => { const args: i32[] = process.argv; };");
    assert!(codes.contains(&"E0308"), "got: {codes:?}");
}

#[test]
fn rejects_calling_the_result_of_a_call_that_is_not_a_function() {
    let codes = error_codes(
        "const makeNumber = (): i32 => 5;\n\
         const main = (): void => { console.log(makeNumber()(1)); };",
    );
    assert!(codes.contains(&"E0618"), "got: {codes:?}");
}

#[test]
fn rejects_a_value_call_with_the_wrong_arity() {
    let codes = error_codes(
        "const makeAdder = (b: i32): (i32) => i32 => { return (v: i32): i32 => b + v; };\n\
         const main = (): void => { console.log(makeAdder(1)(2, 3)); };",
    );
    assert!(codes.contains(&"E0061"), "got: {codes:?}");
}

#[test]
fn rejects_a_closure_capturing_from_two_functions_out() {
    let codes = error_codes(
        "const triple = (a: i32): (i32) => (i32) => i32 => {\n\
             return (b: i32): (i32) => i32 => {\n\
                 return (c: i32): i32 => a + b + c;\n\
             };\n\
         };",
    );
    assert!(codes.contains(&"E0435"), "got: {codes:?}");
}

#[test]
fn a_capture_from_the_enclosing_function_stays_accepted() {
    let codes = error_codes(
        "const makeAdder = (base: i32): (i32) => i32 => {\n\
             return (value: i32): i32 => base + value;\n\
         };",
    );
    assert!(!codes.contains(&"E0435"), "false positive: {codes:?}");
}

#[test]
fn rejects_an_unsupported_file_encoding_before_code_generation() {
    let codes = fs_error_codes(r#"console.log(readFileSync("a.txt", "latin1"));"#);
    assert!(codes.contains(&"E0790"), "got: {codes:?}");
}

#[test]
fn rejects_a_non_literal_file_encoding_before_code_generation() {
    let codes = fs_error_codes(
        "const encoding: string = \"utf8\";\nconsole.log(readFileSync(\"a.txt\", encoding));",
    );
    assert!(codes.contains(&"E0790"), "got: {codes:?}");
}

#[test]
fn accepts_the_supported_file_encodings() {
    for encoding in ["utf8", "utf-8"] {
        let codes = fs_error_codes(&format!(
            r#"console.log(readFileSync("a.txt", "{encoding}"));"#
        ));
        assert!(
            !codes.contains(&"E0790"),
            "`{encoding}` must be accepted, got: {codes:?}"
        );
    }
}

fn entry_error_codes(source: &str) -> Vec<&'static str> {
    let tokens = tyrus_core::lexer::lex(source).unwrap();
    let program = Parser::new(tokens).parse_program().unwrap();
    let mut checker = TypeChecker::new();
    checker.set_entry_point(true);
    checker
        .check_program(&program)
        .into_iter()
        .filter(|diagnostic| diagnostic.severity == CheckSeverity::Error)
        .map(|diagnostic| diagnostic.code)
        .collect()
}

#[test]
fn rejects_an_entry_module_whose_main_takes_parameters() {
    let codes =
        entry_error_codes("const main = async (x: i32): Promise<void> => { console.log(x); };");
    assert!(codes.contains(&"E0580"), "got: {codes:?}");
}

#[test]
fn rejects_an_entry_module_whose_main_returns_a_value() {
    let codes = entry_error_codes("const main = (): i32 => { return 1; };");
    assert!(codes.contains(&"E0580"), "got: {codes:?}");
}

#[test]
fn rejects_an_entry_module_without_main() {
    let codes = entry_error_codes("const helper = (): void => { console.log(1); };");
    assert!(codes.contains(&"E0601"), "got: {codes:?}");
}

#[test]
fn accepts_both_supported_main_signatures() {
    for source in [
        "const main = (): void => { console.log(1); };",
        "const main = async (): Promise<void> => { console.log(1); };",
    ] {
        let codes = entry_error_codes(source);
        assert!(
            !codes.contains(&"E0580") && !codes.contains(&"E0601"),
            "`{source}` must be accepted, got: {codes:?}"
        );
    }
}

#[test]
fn accepts_an_omitted_main_return_type_that_infers_to_void() {
    for source in [
        "function main() { console.log(1); }",
        "async function main() { console.log(1); }",
    ] {
        let codes = entry_error_codes(source);
        assert!(
            !codes.contains(&"E0580") && !codes.contains(&"E0601"),
            "`{source}` must be accepted, got: {codes:?}"
        );
    }
}

#[test]
fn rejects_an_omitted_main_return_type_that_infers_to_a_value() {
    let codes = entry_error_codes("function main() { return 1; }");
    assert!(codes.contains(&"E0580"), "got: {codes:?}");
}

#[test]
fn a_module_that_is_not_the_entry_needs_no_main() {
    let tokens = tyrus_core::lexer::lex("export const helper = (): void => {};").unwrap();
    let program = Parser::new(tokens).parse_program().unwrap();
    let codes = TypeChecker::new()
        .check_program(&program)
        .into_iter()
        .filter(|diagnostic| diagnostic.severity == CheckSeverity::Error)
        .map(|diagnostic| diagnostic.code)
        .collect::<Vec<_>>();
    assert!(!codes.contains(&"E0601"), "got: {codes:?}");
}

#[test]
fn rejects_a_chained_comparison_mixing_incompatible_types() {
    let source = "const main = (): void => { const n: i32 = 1; const s: string = \"x\"; const r = 0 < n < s; };";
    let tokens = tyrus_core::lexer::lex(source).unwrap();
    let program = Parser::new(tokens).parse_program().unwrap();
    let diagnostics = TypeChecker::new()
        .check_program(&program)
        .into_iter()
        .filter(|diagnostic| diagnostic.severity == CheckSeverity::Error)
        .collect::<Vec<_>>();
    let diagnostic = diagnostics
        .iter()
        .find(|diagnostic| diagnostic.code == "E0369")
        .unwrap_or_else(|| panic!("expected E0369, got: {diagnostics:#?}"));
    assert_eq!(diagnostic.subject.as_deref(), Some("s"));
}

#[test]
fn in_element_type_mismatch_anchors_on_the_left_operand() {
    let source = r#"const main = (): void => { const nums: i32[] = [1, 2, 3]; const guest: string = "x"; console.log(guest in nums); console.log(nums.length); };"#;
    let tokens = tyrus_core::lexer::lex(source).unwrap();
    let program = Parser::new(tokens).parse_program().unwrap();
    let diagnostics = TypeChecker::new()
        .check_program(&program)
        .into_iter()
        .filter(|diagnostic| diagnostic.severity == CheckSeverity::Error)
        .collect::<Vec<_>>();
    let diagnostic = diagnostics
        .iter()
        .find(|diagnostic| diagnostic.code == "E0308")
        .unwrap_or_else(|| panic!("expected E0308, got: {diagnostics:#?}"));
    assert_eq!(diagnostic.subject.as_deref(), Some("guest"));
}

#[test]
fn range_bound_type_mismatch_anchors_on_the_offending_bound() {
    let source = r#"const main = (): void => { const label: string = "x"; const r = 0..label; };"#;
    let tokens = tyrus_core::lexer::lex(source).unwrap();
    let program = Parser::new(tokens).parse_program().unwrap();
    let diagnostics = TypeChecker::new()
        .check_program(&program)
        .into_iter()
        .filter(|diagnostic| diagnostic.severity == CheckSeverity::Error)
        .collect::<Vec<_>>();
    let diagnostic = diagnostics
        .iter()
        .find(|diagnostic| diagnostic.code == "E0369")
        .unwrap_or_else(|| panic!("expected E0369, got: {diagnostics:#?}"));
    assert_eq!(diagnostic.subject.as_deref(), Some("label"));
}

#[test]
fn match_range_pattern_type_mismatch_anchors_on_each_offending_bound() {
    let source = r#"const main = (): void => { const x: string = "hello"; match (x) { 0..10 => { console.log(1); } _ => { console.log(2); } } };"#;
    let program = Parser::from_source(source)
        .unwrap()
        .parse_program()
        .unwrap();
    let diagnostics = TypeChecker::new()
        .check_program(&program)
        .into_iter()
        .filter(|diagnostic| {
            diagnostic.severity == CheckSeverity::Error && diagnostic.code == "E0308"
        })
        .collect::<Vec<_>>();
    assert_eq!(diagnostics.len(), 2, "got: {diagnostics:#?}");
    let spans = diagnostics
        .iter()
        .map(|diagnostic| {
            let span = diagnostic
                .span
                .expect("range pattern bound must have a real span");
            &source[span.to_range()]
        })
        .collect::<Vec<_>>();
    assert_eq!(spans, vec!["0", "10"]);
}

#[test]
fn inverted_range_pattern_warns_without_erroring() {
    let source = r#"const main = (): void => { const n: i32 = 5; match (n) { 10..5 => { console.log(1); } _ => { console.log(2); } } };"#;
    let tokens = tyrus_core::lexer::lex(source).unwrap();
    let program = Parser::new(tokens).parse_program().unwrap();
    let diagnostics = TypeChecker::new().check_program(&program);
    assert!(
        !diagnostics
            .iter()
            .any(|diagnostic| diagnostic.severity == CheckSeverity::Error),
        "got: {diagnostics:#?}"
    );
    assert!(
        diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "W0003"),
        "expected W0003, got: {diagnostics:#?}"
    );
}

#[test]
fn split_at_constant_index_out_of_bounds_anchors_on_the_literal() {
    let source = r#"const main = (): void => { const xs: i32[] = [1, 2, 3]; const parts = xs.splitAt(5); };"#;
    let program = Parser::from_source(source)
        .unwrap()
        .parse_program()
        .unwrap();
    let diagnostics = TypeChecker::new()
        .check_program(&program)
        .into_iter()
        .filter(|diagnostic| {
            diagnostic.severity == CheckSeverity::Error && diagnostic.code == "E0606"
        })
        .collect::<Vec<_>>();
    assert_eq!(diagnostics.len(), 1, "got: {diagnostics:#?}");
    let span = diagnostics[0]
        .span
        .expect("splitAt out-of-bounds index must have a real span");
    assert_eq!(&source[span.to_range()], "5");
}

#[test]
fn split_at_accepts_an_index_equal_to_the_known_length() {
    let source = r#"const main = (): void => { const xs: i32[] = [1, 2, 3]; const parts = xs.splitAt(3); console.log(parts[0]); };"#;
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
fn count_implicit_condition_with_two_free_variables_is_rejected() {
    let source = r#"const main = (): void => { const numbers: i32[] = [1, 2, 3]; const bad = numbers.count(a > b); console.log(bad); };"#;
    let program = Parser::from_source(source)
        .unwrap()
        .parse_program()
        .unwrap();
    let diagnostics = TypeChecker::new()
        .check_program(&program)
        .into_iter()
        .filter(|diagnostic| {
            diagnostic.severity == CheckSeverity::Error && diagnostic.code == "E1356"
        })
        .collect::<Vec<_>>();
    assert_eq!(diagnostics.len(), 1, "got: {diagnostics:#?}");
    assert!(
        diagnostics[0].message.contains('a') && diagnostics[0].message.contains('b'),
        "expected both free names cited: {}",
        diagnostics[0].message
    );
    let span = diagnostics[0]
        .span
        .expect("count's multi-free-variable error must have a real span");
    assert_eq!(&source[span.to_range()], "a > b");
}

#[test]
fn count_implicit_condition_shadowed_by_an_outer_binding_is_a_type_mismatch_with_help() {
    let source = r#"const main = (): void => { const numbers: i32[] = [1, 2, 3]; const number: i32 = 10; const bad = numbers.count(number % 2 === 0); console.log(bad); };"#;
    let program = Parser::from_source(source)
        .unwrap()
        .parse_program()
        .unwrap();
    let diagnostics = TypeChecker::new()
        .check_program(&program)
        .into_iter()
        .filter(|diagnostic| {
            diagnostic.severity == CheckSeverity::Error && diagnostic.code == "E0308"
        })
        .collect::<Vec<_>>();
    assert_eq!(diagnostics.len(), 1, "got: {diagnostics:#?}");
    assert!(
        diagnostics[0]
            .help
            .iter()
            .any(|line| line.contains("`number`")),
        "expected a help line naming the shadowed identifier: {:#?}",
        diagnostics[0].help
    );
}

#[test]
fn rejects_unwrapped_ambiguous_result_return_and_offers_explicit_variants() {
    let source = r#"function ambiguous(): Result<string, string> { return "value"; }"#;
    let codes = error_codes(source);
    assert!(codes.contains(&"E3074"), "got: {codes:?}");
    let program = Parser::from_source(source)
        .unwrap()
        .parse_program()
        .unwrap();
    let diagnostic = TypeChecker::new()
        .check_program(&program)
        .into_iter()
        .find(|diagnostic| diagnostic.code == "E3074")
        .expect("expected Result constructor diagnostic");
    assert!(diagnostic.help.iter().any(|help| help.contains("Ok")));
    assert!(diagnostic.help.iter().any(|help| help.contains("Err")));
}

#[test]
fn rejects_bare_void_result_return_and_requires_ok() {
    let source = "function done(): Result<void, string> { return; }";
    let program = Parser::from_source(source)
        .unwrap()
        .parse_program()
        .unwrap();
    let diagnostic = TypeChecker::new()
        .check_program(&program)
        .into_iter()
        .find(|diagnostic| diagnostic.code == "E3075")
        .expect("expected bare Result<void, E> return diagnostic");
    assert_eq!(
        diagnostic.span,
        Some(Span::new(
            source.find("return;").unwrap(),
            source.find("return;").unwrap() + "return;".len(),
        ))
    );
    assert!(diagnostic.help.iter().any(|help| help.contains("Ok()")));
}

#[test]
fn rejects_optional_result_error_types() {
    let codes = error_codes("function invalid(): Result<string, string?> { return Ok(\"x\"); }");
    assert!(codes.contains(&"E3072"), "got: {codes:?}");
}

#[test]
fn accepts_optional_result_success_types() {
    let codes =
        error_codes("function find(): Result<string?, string> { return Err(\"missing\"); }");
    assert!(codes.is_empty(), "got: {codes:?}");
}

#[test]
fn rejects_result_propagation_with_a_non_result_enclosing_return() {
    let codes = error_codes(
        "function load(): Result<i32, string> { return Ok(1); }\nfunction invalid(): i32 {\n    let value = load()?;\n    return value;\n}",
    );
    assert!(
        codes.contains(&"E3078") || codes.contains(&"E3076"),
        "got: {codes:?}"
    );
}

#[test]
fn rejects_result_propagation_with_a_different_error_type() {
    let codes = error_codes(
        "function load(): Result<i32, string> { return Ok(1); }\nfunction invalid(): Result<i32, boolean> { let value = load()?; return Ok(value); }",
    );
    assert!(codes.contains(&"E3071"), "got: {codes:?}");
}

#[test]
fn rejects_result_propagation_outside_a_result_function() {
    let codes = error_codes(
        "function load(): Result<i32, string> { return Ok(1); }\nfunction invalid(): void { let value = load()?; }",
    );
    assert!(
        codes.contains(&"E3078") || codes.contains(&"E3076"),
        "got: {codes:?}"
    );
}

#[test]
fn rejects_non_exhaustive_result_match() {
    let codes = error_codes(
        "function invalid(result: Result<i32, string>): void { match (result) { value => { console.log(value); } } }",
    );
    assert!(codes.contains(&"E0004"), "got: {codes:?}");
}

#[test]
fn rejects_result_match_that_omits_err() {
    let codes = error_codes(
        "function invalid(result: Result<i32, string>): void { match (result) { Ok(value) => { console.log(value); } } }",
    );
    assert!(codes.contains(&"E0004"), "got: {codes:?}");
}

#[test]
fn rejects_a_non_object_json_format_option() {
    let codes = error_codes(
        r#"
            interface User { name: string; }
            const main = (): void => {
                const user: User = { name: "Ada" };
                const json = user.toJson(true);
            };
        "#,
    );
    assert!(codes.contains(&"E0308"), "got: {codes:?}");
}

#[test]
fn count_value_form_rejects_a_mismatched_element_type() {
    let source = r#"const main = (): void => { const words: string[] = ["hello", "world"]; const bad = words.count(42); console.log(bad); };"#;
    let tokens = tyrus_core::lexer::lex(source).unwrap();
    let program = Parser::new(tokens).parse_program().unwrap();
    let diagnostics = TypeChecker::new()
        .check_program(&program)
        .into_iter()
        .filter(|diagnostic| {
            diagnostic.severity == CheckSeverity::Error && diagnostic.code == "E0308"
        })
        .collect::<Vec<_>>();
    assert_eq!(diagnostics.len(), 1, "got: {diagnostics:#?}");
}

#[test]
fn string_count_rejects_a_non_string_argument() {
    let source = r#"const main = (): void => { const text: string = "banana"; const bad = text.count(42); console.log(bad); };"#;
    let tokens = tyrus_core::lexer::lex(source).unwrap();
    let program = Parser::new(tokens).parse_program().unwrap();
    let diagnostics = TypeChecker::new()
        .check_program(&program)
        .into_iter()
        .filter(|diagnostic| {
            diagnostic.severity == CheckSeverity::Error && diagnostic.code == "E0308"
        })
        .collect::<Vec<_>>();
    assert_eq!(diagnostics.len(), 1, "got: {diagnostics:#?}");
}

#[test]
fn chained_comparison_reports_every_mismatched_pair_independently() {
    let source = "const main = (): void => { const n: i32 = 1; const s: string = \"x\"; const r = n < s < true; };";
    let tokens = tyrus_core::lexer::lex(source).unwrap();
    let program = Parser::new(tokens).parse_program().unwrap();
    let count = TypeChecker::new()
        .check_program(&program)
        .into_iter()
        .filter(|diagnostic| {
            diagnostic.severity == CheckSeverity::Error && diagnostic.code == "E0369"
        })
        .count();
    assert_eq!(count, 2, "expected one E0369 per mismatched pair");
}

#[test]
fn overlapping_behavior_interfaces_require_unambiguous_method_names() {
    let source = r#"
        interface User { age: u32; }
        interface FirstUserMethods at User {
            label(user: User): string { return user.age.toString(); }
        }
        interface SecondUserMethods at User {
            label(user: User): string { return "user"; }
        }
        const main = (): void => {
            const user: User = { age: 21 };
            console.log(user.label());
        };
    "#;
    let program = Parser::from_source(source)
        .unwrap()
        .parse_program()
        .unwrap();
    let diagnostic = TypeChecker::new()
        .check_program(&program)
        .into_iter()
        .find(|diagnostic| diagnostic.code == "E0034")
        .expect("overlapping behavior methods must be rejected");
    assert!(diagnostic.message.contains("`FirstUserMethods`"));
    assert!(diagnostic.message.contains("`SecondUserMethods`"));
    assert!(diagnostic.help.iter().any(|help| help.contains("`label`")));
    assert_eq!(
        &source[diagnostic.span.unwrap().to_range()],
        "label",
        "the primary range must point at the ambiguous method"
    );
}

#[test]
fn rejects_map_entries_that_do_not_determine_one_pair_of_types() {
    for entries in [
        r#"[["a", 1], ["b", "two"]]"#,
        r#"[["a", 1], [2, 3]]"#,
        r#"[["a", 1, 2]]"#,
        "[]",
        "[[[1], 2]]",
    ] {
        let source = format!("const main = (): void => {{ const m = Map({entries}); }};");
        assert!(
            error_codes(&source).contains(&"E0282"),
            "`Map({entries})` must report E0282"
        );
    }
}

#[test]
fn rejects_enum_variant_arity_mismatch() {
    let codes = error_codes(
        r#"
            enum Command {
                Move(i32, i32),
            }
            function main(): void {
                const cmd: Command = Command.Move(10, 20);
                match (cmd) {
                    Command.Move(x, y, z) => { console.log(x); }
                }
            }
            "#,
    );
    assert!(
        codes.contains(&"E3292"),
        "must report E3292 for arity mismatch, got: {codes:?}"
    );
}

#[test]
fn rejects_a_single_binding_for_a_multi_field_variant() {
    let codes = error_codes(
        r#"
            enum Command {
                Move(i32, i32),
            }
            function main(): void {
                const cmd: Command = Command.Move(10, 20);
                match (cmd) {
                    Command.Move(position) => { console.log(1); }
                }
            }
            "#,
    );
    assert!(
        codes.contains(&"E3292"),
        "must report E3292 for a single binding on a multi-field variant, got: {codes:?}"
    );
}

#[test]
fn accepts_a_single_binding_for_a_one_field_variant() {
    let codes = error_codes(
        r#"
            enum Command {
                Move(i32),
            }
            function main(): void {
                const cmd: Command = Command.Move(10);
                match (cmd) {
                    Command.Move(distance) => { console.log(distance); }
                }
            }
            "#,
    );
    assert!(
        !codes.contains(&"E3292"),
        "a one-field payload takes a single binding, got: {codes:?}"
    );
}

#[test]
fn rejects_binding_on_unit_variant() {
    assert!(
        error_codes(
            r#"
            enum Command {
                Quit,
            }
            function main(): void {
                const cmd: Command = Command.Quit;
                match (cmd) {
                    Command.Quit(x) => { console.log(x); }
                }
            }
            "#
        )
        .contains(&"E3292"),
        "must report E3292 for binding on unit variant"
    );
}

#[test]
fn rejects_toggle_with_wrong_type() {
    assert!(
        error_codes(
            r#"
                const main = (): void => {
                    let values: i32[] = [1, 2, 3];
                    values.toggle("wrong");
                };
            "#
        )
        .contains(&"E0308"),
        "must report E0308 for type mismatch in toggle"
    );
}

#[test]
fn rejects_toggle_with_too_many_arguments() {
    assert!(
        error_codes(
            r#"
                const main = (): void => {
                    let values: i32[] = [1, 2, 3];
                    values.toggle(1, "id", "extra");
                };
            "#
        )
        .contains(&"E0061"),
        "must report E0061 for too many arguments in toggle"
    );
}

#[test]
fn rejects_toggle_all_with_wrong_type() {
    assert!(
        error_codes(
            r#"
                const main = (): void => {
                    let values: i32[] = [1, 2, 3];
                    values.toggleAll(["wrong"]);
                };
            "#
        )
        .contains(&"E0308"),
        "must report E0308 for type mismatch in toggleAll"
    );
}

#[test]
fn rejects_toggle_with_nonexistent_key() {
    assert!(
        error_codes(
            r#"
                interface Item {
                    id: i32;
                    name: string;
                }
                const main = (): void => {
                    let items: Item[] = [];
                    items.toggle({ id: 1, name: "a" }, "nonexistent");
                };
            "#
        )
        .contains(&"E0609"),
        "must report E0609 for nonexistent key in toggle"
    );
}

#[test]
fn rejects_toggle_key_not_string_literal() {
    assert!(
        error_codes(
            r#"
                interface Item {
                    id: i32;
                    name: string;
                }
                const main = (): void => {
                    let items: Item[] = [];
                    const key: string = "id";
                    items.toggle({ id: 1, name: "a" }, key);
                };
            "#
        )
        .contains(&"E0308"),
        "must report E0308 for non-literal key in toggle"
    );
}

#[test]
fn flags_uncaught_zip_as_unhandled_throw() {
    assert!(
        error_codes(
            r#"
                const main = (): void => {
                    const names: string[] = ["a", "b"];
                    const scores: i32[] = [1, 2];
                    const pairs: [string, i32][] = names.zip(scores);
                    console.log(pairs.length);
                };
            "#
        )
        .contains(&"E3011"),
        "must report unhandled-throw for a zip without catch or throws"
    );
}

#[test]
fn flags_zip_without_array_argument() {
    assert!(
        error_codes(
            r#"
                const main = (): void => {
                    const values: i32[] = [1, 2];
                    try {
                        const pairs = values.zip(3);
                        console.log(pairs);
                    } catch (error: ZipError) {
                        console.log(error.code);
                    }
                };
            "#
        )
        .contains(&"E0277"),
        "must report E0277 for a non-array zip argument"
    );
}

type TestModuleError = (&'static str, String, usize, String, Vec<String>);

fn test_module_errors(source: &str) -> Vec<TestModuleError> {
    use tyrus_core::test::{
        DESCRIBE_TARGET, EXPECT_TARGET, IT_TARGET, TEST_TARGET, describe_type, expect_type,
        test_type,
    };
    let program = Parser::from_source(source)
        .unwrap()
        .parse_program()
        .unwrap();
    let imports: tyrus_core::collections::HashMap<String, tyrus_core::ast::Type> = [
        ("describe".to_string(), describe_type()),
        ("test".to_string(), test_type()),
        ("it".to_string(), test_type()),
        ("expect".to_string(), expect_type()),
    ]
    .into_iter()
    .collect();
    let mut checker = TypeChecker::new();
    checker.set_import_targets(
        [
            ("describe".to_string(), DESCRIBE_TARGET.to_string()),
            ("test".to_string(), TEST_TARGET.to_string()),
            ("it".to_string(), IT_TARGET.to_string()),
            ("expect".to_string(), EXPECT_TARGET.to_string()),
        ]
        .into_iter()
        .collect(),
    );
    checker
        .check_program_with_imports(&program, &imports)
        .into_iter()
        .filter(|diagnostic| diagnostic.severity == CheckSeverity::Error)
        .map(|diagnostic| {
            let span = diagnostic
                .span
                .unwrap_or_else(|| panic!("`{}` has no span", diagnostic.message));
            (
                diagnostic.code,
                diagnostic.message,
                span.start,
                source[span.start..span.end].to_string(),
                diagnostic.help,
            )
        })
        .collect()
}

fn test_module_error<'a>(
    errors: &'a [TestModuleError],
    code: &str,
    fragment: &str,
) -> &'a TestModuleError {
    errors
        .iter()
        .find(|(c, message, ..)| *c == code && message.contains(fragment))
        .unwrap_or_else(|| panic!("no {code} containing `{fragment}` in {errors:#?}"))
}

fn offset_in(source: &str, anchor: &str, skip: usize) -> usize {
    source
        .find(anchor)
        .unwrap_or_else(|| panic!("`{anchor}` not in source"))
        + skip
}

#[test]
fn test_modifier_errors_anchor_at_the_member_and_carry_help() {
    let source = r#"import { describe, test, it, expect } from "tyrus:test";

test.concurrent("unsupported", () => {});
test.wat("unknown", () => {});
describe.todo("no todo on suites");
it.skip("missing callback");
test.todo(42);
test.skip("fine", () => {});
const main = (): void => {};
"#;
    let errors = test_module_errors(source);
    assert_eq!(errors.len(), 5, "{errors:#?}");

    let (_, _, start, text, help) =
        test_module_error(&errors, "E0599", "`test.concurrent` is not implemented yet");
    assert_eq!(
        (*start, text.as_str()),
        (offset_in(source, "test.concurrent", 5), "concurrent")
    );
    assert_eq!(
        help,
        &["the available modifiers are `test.skip`, `test.only`, `test.todo` or `test.each`"]
    );

    let (_, _, start, text, _) =
        test_module_error(&errors, "E0599", "no modifier named `wat` on `test`");
    assert_eq!(
        (*start, text.as_str()),
        (offset_in(source, "test.wat", 5), "wat")
    );

    let (_, _, start, text, help) =
        test_module_error(&errors, "E0599", "no modifier named `todo` on `describe`");
    assert_eq!(
        (*start, text.as_str()),
        (offset_in(source, "describe.todo", 9), "todo")
    );
    assert_eq!(
        help,
        &["the available modifiers are `describe.skip` or `describe.only`"]
    );

    let (_, _, start, text, help) = test_module_error(
        &errors,
        "E0061",
        "`it.skip` expects 2 arguments, but 1 were provided",
    );
    assert_eq!(
        (*start, text.as_str()),
        (offset_in(source, "it.skip(\"missing", 3), "skip")
    );
    assert_eq!(
        help,
        &["pass a name and a callback, as in `it.skip(\"name\", () => { ... })`"]
    );

    let (_, _, start, text, help) = test_module_error(
        &errors,
        "E0308",
        "the name passed to `test.todo` must be a `string`, found `integer literal`",
    );
    assert_eq!(
        (*start, text.as_str()),
        (offset_in(source, "test.todo(42)", 10), "42")
    );
    assert_eq!(
        help,
        &["write the name as a string, as in `test.todo(\"name\")`"]
    );
}

#[test]
fn to_throw_and_assert_throws_errors_anchor_at_the_argument() {
    let source = r#"import { it, expect } from "tyrus:test";

const plain = (): void => {};
const needsArg = (n: i32): void throws string => {
    throw "x";
};

it("no throws", () => { expect(plain).toThrow(); });
it("needs an argument", () => { expect(needsArg).toThrow(); });
const direct = (): void => { assertThrows(plain); };
it("too many", () => { expect(needsArg).toThrow("a", "b"); });

const main = (): void => {
    plain();
    try {
        needsArg(1);
    } catch (error: string) {
        console.log(error);
    }
};
"#;
    let errors = test_module_errors(source);
    assert_eq!(errors.len(), 5, "{errors:#?}");

    let plain: Vec<(usize, &str)> = errors
        .iter()
        .filter(|(code, message, ..)| *code == "E0277" && message.contains("`plain`"))
        .map(|(_, _, start, text, _)| (*start, text.as_str()))
        .collect();
    assert_eq!(
        plain,
        [
            (offset_in(source, "expect(plain)", 7), "plain"),
            (offset_in(source, "assertThrows(plain)", 13), "plain"),
        ]
    );

    let needs_arg: Vec<(usize, &str)> = errors
        .iter()
        .filter(|(code, message, ..)| {
            *code == "E0061"
                && message == "`toThrow` calls `needsArg` with no arguments, but it requires 1"
        })
        .map(|(_, _, start, text, _)| (*start, text.as_str()))
        .collect();
    assert_eq!(
        needs_arg,
        [
            (
                offset_in(source, "expect(needsArg).toThrow()", 7),
                "needsArg"
            ),
            (
                offset_in(source, "expect(needsArg).toThrow(\"a\"", 7),
                "needsArg"
            ),
        ]
    );

    let (_, _, start, text, _) = test_module_error(
        &errors,
        "E0061",
        "`.toThrow()` expects 0 or 1 arguments, but 2 were provided",
    );
    assert_eq!(
        (*start, text.as_str()),
        (offset_in(source, "toThrow(\"a\"", 0), "toThrow")
    );
}

#[test]
fn rejects_matchers_that_do_not_apply_to_the_language() {
    let source = r#"import { it, expect } from "tyrus:test";

it("property", () => { expect(1).toHaveProperty("x"); });
it("instance", () => { expect(1).toBeInstanceOf(2); });
it("typo", () => { expect(1).toBeGreat(2); });
"#;
    let errors = test_module_errors(source);

    let (_, _, start, text, help) = test_module_error(
        &errors,
        "E0599",
        "`toHaveProperty` does not apply in tyrus: a field is resolved at compile time, so this can only ever be true",
    );
    assert_eq!(
        (*start, text.as_str()),
        (offset_in(source, "toHaveProperty", 0), "toHaveProperty")
    );
    assert_eq!(
        help,
        &["assert the field itself, as in `expect(user.email).toBeDefined()`"]
    );

    let (_, _, _, _, help) = test_module_error(
        &errors,
        "E0599",
        "`toBeInstanceOf` does not apply in tyrus: tyrus has no class instances to test for",
    );
    assert_eq!(
        help,
        &[
            "narrow the value instead, with `match` or an optional check, and assert what the narrowed value holds"
        ]
    );

    let (_, _, _, _, help) = test_module_error(
        &errors,
        "E0599",
        "no test matcher named `toBeGreat` on the value returned by `expect`",
    );
    assert!(
        !help.iter().any(|line| line.contains("does not apply")),
        "{help:?}"
    );
}

#[test]
fn rejects_promise_modifiers_on_a_value_that_is_not_a_promise() {
    let source = r#"import { it, expect } from "tyrus:test";

const nothing = (): Promise<void> => {
    return Promise.resolve<void>();
};

it("not a promise", () => { expect(1).resolves.toBe(1); });
it("nor for rejects", () => { expect(1).rejects.toThrow(); });
it("no value to assert", () => { expect(nothing()).resolves.toBe(1); });
it("but rejects is fine", () => { expect(nothing()).not.rejects.toThrow(); });
"#;
    let errors = test_module_errors(source);
    assert_eq!(errors.len(), 3, "{errors:#?}");

    let (_, _, start, text, _) = test_module_error(
        &errors,
        "E0277",
        "`.resolves` expects a promise, but `expect` was given `i32`",
    );
    assert_eq!(
        (*start, text.as_str()),
        (offset_in(source, "expect(1).resolves", 7), "1")
    );

    let (_, _, start, text, _) = test_module_error(
        &errors,
        "E0277",
        "`.rejects` expects a promise, but `expect` was given `i32`",
    );
    assert_eq!(
        (*start, text.as_str()),
        (offset_in(source, "expect(1).rejects", 7), "1")
    );

    let (_, _, start, text, help) = test_module_error(
        &errors,
        "E0277",
        "`.resolves` needs a promise that produces a value, and this one produces `void`",
    );
    assert_eq!(
        (*start, text.as_str()),
        (
            offset_in(source, "expect(nothing()).resolves", 7),
            "nothing()"
        )
    );
    assert_eq!(
        help,
        &[
            "assert that it did not fail with `.rejects` and `.not`, as in `expect(run()).not.rejects.toThrow()`"
        ]
    );
}

#[test]
fn promise_modifiers_do_not_double_report_the_value_under_test() {
    let source = r#"import { it, expect } from "tyrus:test";

it("undeclared", () => { expect(missing()).resolves.toBe(1); });
"#;
    let errors = test_module_errors(source);
    let undeclared: Vec<_> = errors
        .iter()
        .filter(|(code, ..)| *code == "E0425")
        .collect();
    assert_eq!(undeclared.len(), 1, "{errors:#?}");
}

#[test]
fn rejects_contradictory_promise_modifiers() {
    let source = r#"import { it, expect } from "tyrus:test";

const load = (): Promise<i32> => {
    return Promise.resolve<i32>(1);
};

it("throws after resolves", () => { expect(load()).resolves.toThrow(); });
it("both at once", () => { expect(load()).resolves.rejects.toThrow(); });
"#;
    let errors = test_module_errors(source);
    assert_eq!(errors.len(), 2, "{errors:#?}");

    let (_, _, start, text, help) =
        test_module_error(&errors, "E0599", "`.toThrow()` does not follow `.resolves`");
    assert_eq!(
        (*start, text.as_str()),
        (offset_in(source, "resolves.toThrow()", 9), "toThrow")
    );
    assert_eq!(
        help,
        &["assert that a promise failed with `.rejects.toThrow(...)`"]
    );

    let (_, _, start, text, _) = test_module_error(
        &errors,
        "E0599",
        "`.resolves` and `.rejects` ask opposite questions of the same promise",
    );
    assert_eq!(
        (*start, text.as_str()),
        (offset_in(source, "rejects.toThrow()", 8), "toThrow")
    );
}

#[test]
fn rejects_array_equals_on_non_scalar_elements() {
    let codes = error_codes(
        "const main = (): void => { const grid: i32[][] = [[1], [2]]; console.log(grid.equals([[1], [2]])); };",
    );
    assert!(codes.contains(&"E0277"), "got: {codes:?}");
    let scalar = error_codes(
        "const main = (): void => { const left: i32[] = [1, 2]; console.log(left.equals([1, 2])); };",
    );
    assert!(scalar.is_empty(), "got: {scalar:?}");
}

#[test]
fn rejects_equality_on_arrays_and_objects() {
    let source = "const main = (): void => { const a: i32[] = [1]; const b: i32[] = [1]; console.log(a === b); };";
    let program = Parser::from_source(source)
        .unwrap()
        .parse_program()
        .unwrap();
    let diagnostics = TypeChecker::new().check_program(&program);
    let diagnostic = diagnostics
        .iter()
        .find(|diagnostic| diagnostic.code == "E0369")
        .unwrap_or_else(|| panic!("diagnostics: {diagnostics:#?}"));
    assert_eq!(
        diagnostic.message,
        "`===` cannot compare `i32[]` with `i32[]`, because arrays are compared by their contents"
    );
    assert_eq!(
        diagnostic.help,
        ["compare them with `left.equals(right)`, which compares arrays by their contents"]
    );
    let span = diagnostic.span.expect("the comparison has a span");
    assert_eq!(&source[span.start..span.end], "a === b");

    let objects = error_codes(
        "interface Point { x: i32 }\nconst main = (): void => { const p: Point = { x: 1 }; const q: Point = { x: 1 }; console.log(p !== q); };",
    );
    assert!(objects.contains(&"E0369"), "got: {objects:?}");
}

#[test]
fn unknown_stays_unknown_outside_the_proven_branch() {
    let codes = error_codes(
        r#"const main = (): void => {
            const value: unknown = "a";
            if (typeof value === "string") {
                console.log(value.length);
            }
            console.log(value.length);
        };"#,
    );
    assert_eq!(codes, ["E0609"]);
}

#[test]
fn unknown_is_not_narrowed_by_a_negated_or_composite_check() {
    let negated = error_codes(
        r#"const main = (): void => {
            const value: unknown = "a";
            if (typeof value !== "string") {
                console.log(value.length);
            }
        };"#,
    );
    assert_eq!(negated, ["E0609"]);
    let composite = error_codes(
        r#"const main = (): void => {
            const value: unknown = "a";
            if (typeof value === "object") {
                console.log(value.length);
            }
        };"#,
    );
    assert_eq!(composite, ["E0609"]);
}

#[test]
fn rejects_composite_values_stored_in_unknown() {
    for source in [
        "const list: unknown = [1, 2, 3];",
        "const cfg: unknown = { name: \"test\", value: 42 };",
        "interface User { name: string; }\nconst user: User = { name: \"a\" };\nconst held: unknown = user;",
        "const main = (): void => { const numbers: i32[] = [1]; const held: unknown = numbers; };",
        "const describe = (value: unknown): void => {};\nconst main = (): void => { describe([1, 2]); };",
        "function declared(value: unknown): void {}\nconst main = (): void => { declared({ a: 1 }); };",
        "const main = (): void => { const local = (value: unknown): void => {}; const numbers: i32[] = [4]; local(numbers); };",
        "const collect = (...values: unknown[]): void => {};\nconst main = (): void => { collect(1, [2]); };",
    ] {
        let codes = error_codes(source);
        assert!(
            codes.contains(&"E0794"),
            "expected E0794 for {source:?}, got {codes:?}"
        );
    }
}

#[test]
fn rejects_typeof_comparison_with_a_category_typeof_never_produces() {
    for category in ["number", "String", "bool", "User", "undefined"] {
        let source = format!(
            "const main = (): void => {{ const value: unknown = 1; if (typeof value === \"{category}\") {{ console.log(1); }} }};"
        );
        assert_eq!(error_codes(&source), ["E0795"], "category {category:?}");
    }
    let reversed = error_codes(
        "const main = (): void => { const value: string | i32 = 1; if (\"number\" !== typeof value) { console.log(1); } };",
    );
    assert_eq!(reversed, ["E0795"]);
    let valid = error_codes(
        "const main = (): void => { const value: string | i32 = 1; if (typeof value === \"object\") { console.log(1); } };",
    );
    assert!(valid.is_empty(), "got {valid:?}");
}

#[test]
fn native_unknown_parameters_still_accept_composite_values() {
    let source =
        "import { expect } from \"tyrus:test\";\nconst main = (): void => { expect([1, 2]); };";
    let tokens = tyrus_core::lexer::lex(source).unwrap();
    let program = Parser::new(tokens).parse_program().unwrap();
    let imports = [("expect".to_string(), tyrus_core::test::expect_type())]
        .into_iter()
        .collect();
    let codes = TypeChecker::new()
        .check_program_with_imports(&program, &imports)
        .into_iter()
        .map(|diagnostic| diagnostic.code)
        .collect::<Vec<_>>();
    assert!(!codes.contains(&"E0794"), "got {codes:?}");
}

fn archive_diagnostics(body: &str) -> Vec<(String, String)> {
    archive_source_diagnostics(&format!(
        "import {{ ArchiveEntry, ArchiveError, PackOptions, list, pack }} from \"tyrus:archive\";\nconst main = async (): Promise<void> => {{\n    try {{\n        {body}\n    }} catch (error: ArchiveError) {{\n        printError(error.code);\n    }}\n}};\n"
    ))
}

fn archive_source_diagnostics(source: &str) -> Vec<(String, String)> {
    let mut types = tyrus_core::collections::HashMap::default();
    let mut interfaces = tyrus_core::collections::HashMap::default();
    let mut targets = tyrus_core::collections::HashMap::default();
    let throws = tyrus_core::collections::HashMap::default();
    for name in [
        "pack",
        "list",
        "ArchiveEntry",
        "ArchiveError",
        "PackOptions",
        "ArchiveSummary",
    ] {
        let (ty, fields, _) = tyrus_core::module_loader::native_export_info("tyrus:archive", name)
            .expect("archive export metadata");
        types.insert(name.to_string(), ty);
        if let Some(fields) = fields {
            interfaces.insert(name.to_string(), fields);
        }
        if let Some(target) = tyrus_core::module_loader::native_export_target("tyrus:archive", name)
        {
            targets.insert(name.to_string(), target);
        }
    }
    let analysis = tyrus_core::analysis::analyze_with_context(
        source,
        tyrus_core::analysis::AnalyzeContext {
            import_types: Some(&types),
            import_targets: Some(&targets),
            import_interfaces: Some(&interfaces),
            import_throws: Some(&throws),
            ..tyrus_core::analysis::AnalyzeContext::default()
        },
    );
    analysis
        .diagnostics
        .into_iter()
        .filter(|diagnostic| diagnostic.code.starts_with('E'))
        .map(|diagnostic| {
            (
                diagnostic.code.clone(),
                source[diagnostic.range.clone()].to_string(),
            )
        })
        .collect()
}

#[test]
fn archive_pack_rejects_a_literal_destination_without_a_known_lowercase_extension() {
    for destination in [
        "\"./dist/app.jar\"",
        "\"./dist/App.ZIP\"",
        "\"./dist/release\"",
    ] {
        let diagnostics = archive_diagnostics(&format!("await pack(\"./src\", {destination});"));
        assert_eq!(
            diagnostics,
            vec![("E3311".to_string(), destination.to_string())],
            "{destination}"
        );
    }
    for destination in ["./a.zip", "./a.tar", "./a.tar.gz", "./a.tgz"] {
        let diagnostics =
            archive_diagnostics(&format!("await pack(\"./src\", \"{destination}\");"));
        assert!(diagnostics.is_empty(), "{destination}: {diagnostics:?}");
    }
}

#[test]
fn archive_pack_rejects_a_literal_level_the_format_cannot_use() {
    assert_eq!(
        archive_diagnostics("await pack(\"./src\", \"./a.zip\", { level: 10 });"),
        vec![("E3312".to_string(), "10".to_string())]
    );
    assert_eq!(
        archive_diagnostics("await pack(\"./src\", \"./a.tgz\", { level: -1 });"),
        vec![("E3312".to_string(), "-1".to_string())]
    );
    assert_eq!(
        archive_diagnostics("await pack(\"./src\", \"./a.tar\", { level: 1 });"),
        vec![("E3312".to_string(), "1".to_string())]
    );
    assert!(archive_diagnostics("await pack(\"./src\", \"./a.zip\", { level: 0 });").is_empty());
    assert!(archive_diagnostics("await pack(\"./src\", \"./a.tgz\", { level: 9 });").is_empty());
}

#[test]
fn archive_entries_cross_an_await_without_importing_their_kind_enum() {
    // `ArchiveEntry.kind` is a payload-free enum; holding the listing across
    // the next `await` must not require importing `ArchiveEntryKind`.
    let diagnostics = archive_source_diagnostics(
        "import { ArchiveEntry, ArchiveError, ArchiveSummary, list, pack } from \"tyrus:archive\";\nasync function inspect(): Promise<Result<void, ArchiveError>> {\n    const entries: ArchiveEntry[] = await list(\"./a.zip\")?;\n    await pack(\"./src\", \"./b.zip\")?;\n    print(entries.length);\n    return Ok();\n}\n",
    );
    assert!(diagnostics.is_empty(), "{diagnostics:?}");
}

#[test]
fn an_unimported_native_options_interface_names_the_missing_import_at_the_argument() {
    let source = "import { ArchiveError, list } from \"tyrus:archive\";\nasync function inspect(): Promise<Result<void, ArchiveError>> {\n    await list(\"./a.zip\", { maxEntries: 10 })?;\n    return Ok();\n}\n";
    let mut types = tyrus_core::collections::HashMap::default();
    let mut interfaces = tyrus_core::collections::HashMap::default();
    let mut targets = tyrus_core::collections::HashMap::default();
    for name in ["list", "ArchiveError"] {
        let (ty, fields, _) = tyrus_core::module_loader::native_export_info("tyrus:archive", name)
            .expect("archive export metadata");
        types.insert(name.to_string(), ty);
        if let Some(fields) = fields {
            interfaces.insert(name.to_string(), fields);
        }
        if let Some(target) = tyrus_core::module_loader::native_export_target("tyrus:archive", name)
        {
            targets.insert(name.to_string(), target);
        }
    }
    let throws = tyrus_core::collections::HashMap::default();
    let analysis = tyrus_core::analysis::analyze_with_context(
        source,
        tyrus_core::analysis::AnalyzeContext {
            import_types: Some(&types),
            import_targets: Some(&targets),
            import_interfaces: Some(&interfaces),
            import_throws: Some(&throws),
            ..tyrus_core::analysis::AnalyzeContext::default()
        },
    );
    let errors: Vec<_> = analysis
        .diagnostics
        .iter()
        .filter(|diagnostic| diagnostic.code.starts_with('E'))
        .collect();
    assert_eq!(errors.len(), 1, "{errors:#?}");
    assert_eq!(errors[0].code, "E0412");
    assert_eq!(
        errors[0].message,
        "cannot find type `UnpackOptions` in this scope"
    );
    assert_eq!(&source[errors[0].range.clone()], "{ maxEntries: 10 }");
    assert_eq!(
        errors[0].help,
        vec!["argument 2 of `list` expects `UnpackOptions`, which is exported by `tyrus:archive`; add `import { UnpackOptions } from \"tyrus:archive\";`".to_string()]
    );
}

#[test]
fn an_async_local_from_a_native_call_is_rejected_with_a_working_rewrite() {
    let source = "import { CpuInfo, cpus } from \"tyrus:os\";\nconst later = async (): Promise<void> => {};\nasync function run(): Promise<void> {\n    const cores: CpuInfo[] = cpus();\n    await later();\n    print(cores.length);\n}\n";
    let mut types = tyrus_core::collections::HashMap::default();
    let mut interfaces = tyrus_core::collections::HashMap::default();
    let mut targets = tyrus_core::collections::HashMap::default();
    for name in ["cpus", "CpuInfo"] {
        let (ty, fields, _) = tyrus_core::module_loader::native_export_info("tyrus:os", name)
            .expect("os export metadata");
        types.insert(name.to_string(), ty);
        if let Some(fields) = fields {
            interfaces.insert(name.to_string(), fields);
        }
        if let Some(target) = tyrus_core::module_loader::native_export_target("tyrus:os", name) {
            targets.insert(name.to_string(), target);
        }
    }
    let analysis = tyrus_core::analysis::analyze_with_context(
        source,
        tyrus_core::analysis::AnalyzeContext {
            import_types: Some(&types),
            import_targets: Some(&targets),
            import_interfaces: Some(&interfaces),
            ..tyrus_core::analysis::AnalyzeContext::default()
        },
    );
    let error = analysis
        .diagnostics
        .iter()
        .find(|diagnostic| diagnostic.code == "E3088")
        .unwrap_or_else(|| panic!("expected E3088: {:#?}", analysis.diagnostics));
    assert_eq!(&source[error.range.clone()], "cores");
    assert_eq!(
        error.help.first().map(String::as_str),
        Some(
            "return the `cpus(...)` result from a module-level function, such as `const load = (): CpuInfo[] => ...;`, and initialize `cores` with `load()`"
        )
    );
}

fn analyzed_errors(source: &str) -> Vec<tyrus_core::analysis::AnalysisDiagnostic> {
    tyrus_core::analysis::analyze(source)
        .diagnostics
        .into_iter()
        .filter(|diagnostic| diagnostic.code.starts_with('E'))
        .collect()
}

#[test]
fn a_try_on_a_promise_of_a_result_suggests_awaiting_first() {
    let source = "const load = async (): Promise<Result<i32, string>> => {\n    return Ok(1);\n};\n\nconst next = async (): Promise<Result<i32, string>> => {\n    const value = await (load()?);\n    return Ok(value);\n};\n\nconst main = async (): Promise<void> => {\n    const result = await next();\n};\n";
    let errors = analyzed_errors(source);
    let error = errors
        .iter()
        .find(|error| error.code == "E3076")
        .unwrap_or_else(|| panic!("expected E3076: {errors:#?}"));
    assert_eq!(&source[error.range.clone()], "load");
    assert_eq!(
        error.help,
        vec!["await the promise before propagating its result: `await load(...)?` awaits first, then applies `?` to the `Result`".to_string()]
    );
}

#[test]
fn an_awaited_value_the_frame_cannot_hold_is_reported_at_the_await() {
    let source = "const make = async (): Promise<Set<string>> => {\n    return Set<string>();\n};\n\nconst consume = (value: Set<string>): void => {\n    print(\"used\");\n};\n\nconst main = async (): Promise<void> => {\n    consume(await make());\n    consume(await make());\n};\n";
    let errors = analyzed_errors(source);
    let frame_errors: Vec<_> = errors
        .iter()
        .filter(|error| error.code == "E3088")
        .collect();
    assert_eq!(frame_errors.len(), 2, "{errors:#?}");
    for error in frame_errors {
        assert_eq!(
            error.message,
            "the result of this `await` has type `Set<string>`, which requires owned frame storage that is not implemented yet"
        );
        assert_eq!(&source[error.range.clone()], "await make()");
        assert!(!error.message.contains('#'), "{}", error.message);
    }
}

#[test]
fn an_async_local_whose_type_did_not_resolve_is_not_blamed_on_the_frame() {
    let source = "const load = async (): Promise<i32> => {\n    return 1;\n};\n\nconst next = async (): Promise<Result<i32, string>> => {\n    const value = (await load())?;\n    await load();\n    return Ok(1);\n};\n\nconst main = async (): Promise<void> => {\n    const result = await next();\n};\n";
    let codes: Vec<String> = analyzed_errors(source)
        .into_iter()
        .map(|error| error.code)
        .collect();
    assert!(codes.contains(&"E3076".to_string()), "{codes:?}");
    assert!(!codes.contains(&"E3088".to_string()), "{codes:?}");
}
