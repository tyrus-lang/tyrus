use super::*;
use logos::Logos;
use tyrus_core::borrow_checker::BorrowChecker;
use tyrus_core::lexer::Token;
use tyrus_core::parser::Parser;

fn dprintf_symbol() -> &'static str {
    "tyrus_dprintf"
}

fn write_captured_stdout(bytes: &[u8]) {
    let mut offset = 0;
    while offset < bytes.len() {
        #[cfg(windows)]
        let written = unsafe {
            libc::write(
                1,
                bytes[offset..].as_ptr().cast(),
                (bytes.len() - offset)
                    .try_into()
                    .expect("test output fits in u32"),
            )
        };
        #[cfg(not(windows))]
        let written =
            unsafe { libc::write(1, bytes[offset..].as_ptr().cast(), bytes.len() - offset) };
        assert!(written > 0, "stdout write failed: {written}");
        offset += written as usize;
    }
}

#[path = "codegen/array_tests/array_tests.rs"]
mod array_tests;
#[path = "codegen/async_tests/async_tests.rs"]
mod async_tests;
#[path = "codegen/fs_tests/fs_tests.rs"]
mod fs_tests;
#[path = "codegen/http_tests/http_tests.rs"]
mod http_tests;
#[path = "codegen/simd_tests/simd_tests.rs"]
mod simd_tests;
#[path = "codegen/string_tests/string_tests.rs"]
mod string_tests;

#[test]
fn emits_custom_validator_calls_for_interface_fields() {
    let source = r#"
        type Validator = { isValid: true } | { isValid: false; message: string };

        validator function startsWithUser(value: string): Validator {
            if (!value.startsWith("user_")) {
                return { isValid: false, message: "Username must start with user_" };
            }
            return { isValid: true };
        }

        validator function atLeast(value: u32, constraint: u32): Validator {
            if (value < constraint) {
                return { isValid: false, message: "Value is below the constraint" };
            }
            return { isValid: true };
        }

        interface Account {
            @startsWithUser()
            username: string;
            @atLeast(5)
            age: u32;
        }

        const getUsername = (): string => "guest user";

        const main = (): void => {
            try {
                const username: string = getUsername();
                const account: Account = { username, age: 4 };
                print(account.username);
            } catch (error: ValidationError) {
                print(error.issues.length);
                print(error.issues[0].target);
                print(error.issues[0].validator);
                print(error.issues[0].message);
            }
        };
    "#;
    let ir = compile_to_ir(source, "custom-validator-field");
    assert!(ir.contains("call { i8, ["));
    assert!(ir.contains("@startsWithUser"));
    assert!(ir.contains("@atLeast"));
}

fn compile_error(source: &str) -> String {
    let tokens = tyrus_core::lexer::lex(source).unwrap();
    let program = Parser::new(tokens).parse_program().unwrap();
    let output = std::env::temp_dir().join(format!("tyrus-codegen-error-{}", std::process::id()));
    compile(&program, "codegen_error", output.to_str().unwrap(), false).unwrap_err()
}

fn compile_to_ir(source: &str, label: &str) -> String {
    let program = Parser::from_source(source)
        .unwrap()
        .parse_program()
        .unwrap();
    let output = std::env::temp_dir().join(format!("tyrus-{label}-{}", std::process::id()));
    compile(&program, label, output.to_str().unwrap(), false).unwrap();
    let ir = std::fs::read_to_string(output.with_extension("ll")).unwrap();
    let _ = std::fs::remove_file(output.with_extension("ll"));
    let _ = std::fs::remove_file(&output);
    ir
}

fn compile_module_to_ir(source: &str, label: &str) -> String {
    let root = std::env::temp_dir().join(format!("tyrus-{label}-source-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).unwrap();
    std::fs::write(root.join("main.tyr"), source).unwrap();
    let program = tyrus_core::module_loader::load_program(&root.join("main.tyr"), None).unwrap();
    let output = std::env::temp_dir().join(format!("tyrus-{label}-{}", std::process::id()));
    compile(&program, label, output.to_str().unwrap(), false).unwrap();
    let ir = std::fs::read_to_string(output.with_extension("ll")).unwrap();
    let _ = std::fs::remove_file(output.with_extension("ll"));
    let _ = std::fs::remove_file(&output);
    let _ = std::fs::remove_dir_all(root);
    ir
}

#[test]
fn emits_automatic_copy_and_structural_equality_for_data_interfaces() {
    let ir = compile_module_to_ir(
        r#"
            interface User {
                name: string;
                age: i32;
            }
            function main(): void {
                const user: User = { name: "Ada", age: 30 };
                const copied: User = user.copy();
                console.log(user.equals(copied));
            }
        "#,
        "automatic_data_methods",
    );
    assert!(
        ir.contains("async.payload.string.clone"),
        "copy must clone owned string fields:\n{ir}"
    );
    assert!(
        ir.contains("validator.unique_eq_field"),
        "equals must compare every stored field:\n{ir}"
    );
}

#[test]
fn emits_automatic_equality_for_optional_fields() {
    let ir = compile_to_ir(
        r#"
            interface User { name: string; nickname?: string; }
            function main(): void {
                const first: User = { name: "Ada" };
                const second: User = { name: "Ada" };
                console.log(first.equals(second));
            }
        "#,
        "automatic_equality_optional",
    );
    assert!(
        ir.contains("optional.eq.both_present"),
        "equals must compare present optional payloads:\n{ir}"
    );
}

#[test]
fn emits_automatic_debug_for_data_interfaces() {
    let ir = compile_to_ir(
        r#"
            interface User {
                name: string;
                age: i32;
            }
            function main(): void {
                const user: User = { name: "Ada", age: 30 };
                console.log(user.toString());
            }
        "#,
        "automatic_debug",
    );
    assert!(
        ir.contains("automatic.debug"),
        "toString must join field names and field values:\n{ir}"
    );
    assert!(
        ir.contains("tyrus_string_escape_debug"),
        "toString must escape string fields:\n{ir}"
    );
}

#[test]
fn emits_automatic_debug_for_decimal_fields() {
    let ir = compile_to_ir(
        r#"
            interface Balance { amount: decimal; }
            function main(): void {
                const balance: Balance = { amount: 12.50d };
                console.log(balance.toString());
            }
        "#,
        "automatic_debug_decimal",
    );
    assert!(
        ir.contains("tyrus_decimal_to_string"),
        "toString must format decimal fields through the runtime:\n{ir}"
    );
}

#[test]
fn emits_automatic_equality_for_decimal_fields() {
    let ir = compile_to_ir(
        r#"
            interface Balance { amount: decimal; }
            function main(): void {
                const first: Balance = { amount: 12.50d };
                const second: Balance = { amount: 12.5d };
                console.log(first.equals(second));
            }
        "#,
        "automatic_equality_decimal",
    );
    assert!(
        ir.contains("tyrus_decimal_compare"),
        "equals must compare decimal fields by numeric value:\n{ir}"
    );
}

#[test]
fn emits_automatic_json_for_data_interfaces() {
    let ir = compile_to_ir(
        r#"
            interface User { name: string; age: i32; active: boolean; }
            function main(): void {
                const user: User = { name: "Ada", age: 30, active: true };
                console.log(user.toJson());
            }
        "#,
        "automatic_json",
    );
    assert!(
        ir.contains("tyrus_json_builder_finish"),
        "toJson must finish a JSON builder:\n{ir}"
    );
}

#[test]
fn emits_pretty_automatic_json_for_data_interfaces() {
    let ir = compile_to_ir(
        r#"
            interface User { name: string; }
            function main(): void {
                const user: User = { name: "Ada" };
                console.log(user.toJson({ pretty: true }));
            }
        "#,
        "automatic_json_pretty",
    );
    assert!(
        ir.contains("tyrus_json_builder_finish_pretty"),
        "toJson({{ pretty: true }}) must finish a pretty JSON builder:\n{ir}"
    );
}

#[test]
fn emits_automatic_json_decoding_for_data_interfaces() {
    let ir = compile_to_ir(
        r#"
            interface User { name: string; age: i32; active: boolean; }
            function main(): void {
                const result = User.fromJson("{\\\"name\\\":\\\"Ada\\\",\\\"age\\\":37,\\\"active\\\":true}");
                if (result) {
                    const user: User = result;
                    console.log(user.name);
                }
            }
        "#,
        "automatic_json_decode",
    );
    assert!(
        ir.contains("tyrus_json_document_parse"),
        "fromJson must parse a JSON document:\n{ir}"
    );
    assert!(
        ir.contains("ERR_JSON_MISSING_FIELD"),
        "fromJson must preserve a stable missing-field error code:\n{ir}"
    );
}

#[test]
fn emits_automatic_json_for_optional_fields() {
    let ir = compile_to_ir(
        r#"
            interface User { name: string; nickname?: string; }
            function main(): void {
                const user: User = { name: "Ada" };
                const text = user.toJson();
                const decoded = User.fromJson(text);
                if (decoded) {
                    const restored: User = decoded;
                    console.log(restored.name);
                }
            }
        "#,
        "automatic_json_optional",
    );
    assert!(
        ir.contains("automatic.json.optional"),
        "optional fields must be omitted when empty and restored as absent:\n{ir}"
    );
}

#[test]
fn a_decimal_slot_carries_the_alignment_the_runtime_writes_through() {
    let ir = compile_to_ir(
        r#"
            interface Priced { amount: decimal; }
            function main(): void {
                const decoded = Priced.fromJson("{\"amount\": 1.5}");
                console.log(1);
            }
        "#,
        "decimal_slot_alignment",
    );
    assert!(
        ir.contains("target datalayout"),
        "the module must carry a data layout, or LLVM invents its own:\n{ir}"
    );
    assert!(
        !ir.contains("alloca { i128, i32 }, align 8"),
        "a decimal slot must not be 8-aligned:\n{ir}"
    );
}

#[test]
fn does_not_wrap_a_decoded_result_in_a_second_result() {
    let ir = compile_to_ir(
        r#"
            interface UserSnapshot { name: string; count: i32; }
            function main(): void {
                const restored = UserSnapshot.fromJson("{}");
                console.log(1);
            }
        "#,
        "automatic_json_no_double_wrap",
    );
    assert!(
        !ir.contains("%result.value = alloca"),
        "the decoded result was wrapped in a second Result:\n{ir}"
    );
}

#[test]
fn emits_automatic_json_decoding_for_nested_data_interfaces() {
    let ir = compile_to_ir(
        r#"
            interface Address { city: string; }
            interface User { address: Address; }
            function main(): void {
                const decoded = User.fromJson("{\\\"address\\\":{\\\"city\\\":\\\"Ada\\\"}}");
                if (decoded) { const user: User = decoded; console.log(user.address.city); }
            }
        "#,
        "automatic_json_decode_nested",
    );
    assert!(
        ir.contains("tyrus_json_document_get_raw"),
        "fromJson must recursively decode nested data interfaces:\n{ir}"
    );
}

#[test]
fn emits_automatic_debug_for_nested_structural_objects() {
    let ir = compile_to_ir(
        r#"
            interface User {
                address: { city: string; zip: i32; };
            }
            function main(): void {
                const user: User = { address: { city: "São Paulo", zip: 1000 } };
                console.log(user.toString());
            }
        "#,
        "automatic_debug_structural",
    );
    assert!(
        ir.matches("automatic.debug.field").count() >= 3,
        "toString must visit outer and nested object fields:\n{ir}"
    );
}

#[test]
fn emits_automatic_debug_for_optional_fields() {
    let ir = compile_to_ir(
        r#"
            interface User {
                name: string;
                email?: string;
            }
            function main(): void {
                const user: User = { name: "Ada" };
                console.log(user.toString());
            }
        "#,
        "automatic_debug_optional",
    );
    assert!(
        ir.contains("automatic.debug.optional.is_present"),
        "toString must branch on optional fields:\n{ir}"
    );
}

#[test]
fn emits_automatic_debug_for_union_fields() {
    let ir = compile_to_ir(
        r#"
            interface Event {
                detail: string | i32;
            }
            function main(): void {
                const event: Event = { detail: "ready" };
                console.log(event.toString());
            }
        "#,
        "automatic_debug_union",
    );
    assert!(
        ir.contains("automatic.debug.union.is.0"),
        "toString must dispatch on the active union variant:\n{ir}"
    );
}

#[test]
fn emits_automatic_debug_for_tuple_fields() {
    let ir = compile_to_ir(
        r#"
            interface Point {
                coordinate: [i32, i32];
            }
            function main(): void {
                const point: Point = { coordinate: [10, 20] };
                console.log(point.toString());
            }
        "#,
        "automatic_debug_tuple",
    );
    assert!(
        ir.contains("automatic.debug.tuple.element"),
        "toString must visit every tuple element:\n{ir}"
    );
}

#[test]
fn declaration_only_program_emits_a_no_op_main() {
    let source = r#"
        function runGraphExamples(): void {
            console.log("graph examples");
        }
    "#;
    let output = std::env::temp_dir().join(format!(
        "tyrus-codegen-declaration-only-{}",
        std::process::id()
    ));
    let program = Parser::from_source(source)
        .unwrap()
        .parse_program()
        .unwrap();

    compile(
        &program,
        "declaration_only",
        output.to_str().unwrap(),
        false,
    )
    .unwrap();

    let ir = std::fs::read_to_string(output.with_extension("ll")).unwrap();
    assert!(ir.contains("define i32 @main()"), "{ir}");
    assert!(ir.contains("ret i32 0"), "{ir}");
    std::fs::remove_file(output.with_extension("ll")).unwrap();
}

#[test]
fn emits_special_float_constants_and_is_nan_as_an_unordered_comparison() {
    let ir = compile_to_ir(
        "function main(): void { const positive: f64 = Infinity; const negative: f32 = -Infinity; const invalid: f128 = NaN; console.log(isNaN(positive), isNaN(negative), isNaN(invalid), NaN === NaN, NaN !== NaN); }",
        "special_float_literals",
    );
    assert!(
        ir.contains("fcmp uno"),
        "expected isNaN lowering, got:\n{ir}"
    );
    assert!(
        ir.contains("0x7FF0000000000000") || ir.contains("store double +inf"),
        "expected f64 infinity, got:\n{ir}"
    );
}

#[test]
fn emits_net_ip_address_parser_with_optional_owned_object_result() {
    let root = std::env::temp_dir().join(format!(
        "tyrus-codegen-net-ip-address-source-{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).unwrap();
    std::fs::write(
        root.join("main.tyr"),
        r#"
            import { IpAddress, parseIpAddress } from "tyrus:net";
            import { SocketAddress, parseSocketAddress } from "tyrus:net";
            function main(): void {
                const value: IpAddress? = parseIpAddress("127.0.0.1");
                if (value) {
                    console.log(value.address);
                }
                const socket: SocketAddress? = parseSocketAddress("127.0.0.1:8080");
                if (socket) {
                    console.log(socket.ip.address, socket.port);
                }
            }
        "#,
    )
    .unwrap();
    let program = tyrus_core::module_loader::load_program(&root.join("main.tyr"), None).unwrap();
    let output = std::env::temp_dir().join(format!(
        "tyrus-codegen-net-ip-address-{}",
        std::process::id()
    ));
    compile(&program, "net_ip_address", output.to_str().unwrap(), false).unwrap();
    let ir = std::fs::read_to_string(output.with_extension("ll")).unwrap();
    assert!(ir.contains("@tyrus_net_parse_ip_address"), "{ir}");
    assert!(ir.contains("@tyrus_net_parse_socket_address"), "{ir}");
    let _ = std::fs::remove_file(output.with_extension("ll"));
    let _ = std::fs::remove_file(output);
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn emits_net_tcp_stream_connect_read_write_and_close() {
    let root = std::env::temp_dir().join(format!(
        "tyrus-codegen-net-tcp-source-{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).unwrap();
    std::fs::write(
        root.join("main.tyr"),
        r#"
            import { NetError, TcpStream, tcpConnect } from "tyrus:net";
            async function exchange(): Promise<Result<void, NetError>> {
                const stream: TcpStream = await tcpConnect("127.0.0.1:1", 10)?;
                const sent: u64 = await stream.write([1, 2, 3], 10)?;
                const received: u8[] = await stream.read(3, 10)?;
                console.log(sent, received.length);
                await stream.shutdown();
                console.log(stream.localAddress(), stream.peerAddress());
                stream.close();
                return Ok();
            }
        "#,
    )
    .unwrap();
    let program = tyrus_core::module_loader::load_program(&root.join("main.tyr"), None).unwrap();
    let output = std::env::temp_dir().join(format!("tyrus-codegen-net-tcp-{}", std::process::id()));
    compile(&program, "net_tcp", output.to_str().unwrap(), false).unwrap();
    let ir = std::fs::read_to_string(output.with_extension("ll")).unwrap();
    assert!(ir.contains("@tyrus_net_tcp_connect"), "{ir}");
    assert!(ir.contains("@tyrus_net_tcp_stream_read"), "{ir}");
    assert!(ir.contains("@tyrus_net_tcp_stream_write"), "{ir}");
    assert!(ir.contains("@tyrus_net_tcp_stream_shutdown"), "{ir}");
    assert!(ir.contains("@tyrus_net_tcp_stream_local_address"), "{ir}");
    assert!(ir.contains("@tyrus_net_tcp_stream_peer_address"), "{ir}");
    assert!(ir.contains("@tyrus_net_tcp_stream_close"), "{ir}");
    let _ = std::fs::remove_file(output.with_extension("ll"));
    let _ = std::fs::remove_file(output);
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn emits_net_tcp_listener_accept_and_close() {
    let root = std::env::temp_dir().join(format!(
        "tyrus-codegen-net-listener-source-{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).unwrap();
    std::fs::write(
        root.join("main.tyr"),
        r#"
            import { NetError, TcpListener, TcpStream, tcpListen } from "tyrus:net";
            async function acceptOne(): Promise<Result<void, NetError>> {
                const listener: TcpListener = await tcpListen("127.0.0.1:0", 10)?;
                const stream: TcpStream = await listener.accept(10)?;
                console.log(listener.localAddress());
                listener.close();
                stream.close();
                return Ok();
            }
        "#,
    )
    .unwrap();
    let program = tyrus_core::module_loader::load_program(&root.join("main.tyr"), None).unwrap();
    let output =
        std::env::temp_dir().join(format!("tyrus-codegen-net-listener-{}", std::process::id()));
    compile(
        &program,
        "net_tcp_listener",
        output.to_str().unwrap(),
        false,
    )
    .unwrap();
    let ir = std::fs::read_to_string(output.with_extension("ll")).unwrap();
    assert!(ir.contains("@tyrus_net_tcp_listen"), "{ir}");
    assert!(ir.contains("@tyrus_net_tcp_listener_accept"), "{ir}");
    assert!(ir.contains("@tyrus_net_tcp_listener_local_address"), "{ir}");
    assert!(ir.contains("@tyrus_net_tcp_listener_close"), "{ir}");
    let _ = std::fs::remove_file(output.with_extension("ll"));
    let _ = std::fs::remove_file(output);
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn emits_net_udp_and_capability_gated_probe_calls() {
    let root = std::env::temp_dir().join(format!(
        "tyrus-codegen-net-udp-source-{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).unwrap();
    std::fs::write(
        root.join("main.tyr"),
        r#"
            import { NetError, UdpPacket, UdpSocket, udpBind, ping, traceroute, supportsIcmp } from "tyrus:net";
            async function probe(): Promise<void> {
                const socket: UdpSocket = await udpBind("127.0.0.1:0") onfail {
                    printError(socket.code);
                    return;
                };
                const sendResult: Result<u64, NetError> = await socket.sendTo([1, 2, 3], "127.0.0.1:9", 10);
                match (sendResult) {
                    Ok(sent) => { console.log(sent); }
                    Err(error) => { printError(error.code); }
                }
                const receiveResult: Result<UdpPacket, NetError> = await socket.receiveFrom(32, 10);
                match (receiveResult) {
                    Ok(packet) => { console.log(packet.data.length, packet.address); }
                    Err(error) => { printError(error.code); }
                }
                await socket.setBroadcast(true);
                await socket.setTtl(1);
                console.log(socket.localAddress(), supportsIcmp());
                socket.close();
                await ping("127.0.0.1", 10);
                await traceroute("127.0.0.1", 10);
            }
        "#,
    )
    .unwrap();
    let program = tyrus_core::module_loader::load_program(&root.join("main.tyr"), None).unwrap();
    let output = std::env::temp_dir().join(format!("tyrus-codegen-net-udp-{}", std::process::id()));
    compile(&program, "net_udp", output.to_str().unwrap(), false).unwrap();
    let ir = std::fs::read_to_string(output.with_extension("ll")).unwrap();
    for symbol in [
        "@tyrus_net_udp_bind",
        "@tyrus_net_udp_send_to",
        "@tyrus_net_udp_receive_from",
        "@tyrus_net_udp_set_broadcast",
        "@tyrus_net_udp_set_ttl",
        "@tyrus_net_udp_local_address",
        "@tyrus_net_udp_close",
        "@tyrus_net_supports_icmp",
        "@tyrus_net_ping",
        "@tyrus_net_traceroute",
    ] {
        assert!(ir.contains(symbol), "missing {symbol} in {ir}");
    }
    let _ = std::fs::remove_file(output.with_extension("ll"));
    let _ = std::fs::remove_file(output);
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn emits_behavior_methods_and_shorthand_bindings() {
    let source = r#"
        interface User { age: u32; }
        interface UserMethods at User {
            isAdult(user: User): boolean { return user.age >= 18; }
        }
        const ageText = (user: User): string => user.age.toString();
        interface UserText at User { ageText; }
        const main = (): void => {
            const user: User = { age: 21 };
            console.log(user.isAdult(), user.ageText());
        };
    "#;
    let root = std::env::temp_dir().join(format!(
        "tyrus-codegen-plain-date-source-{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).unwrap();
    std::fs::write(root.join("main.tyr"), source).unwrap();
    let program = tyrus_core::module_loader::load_program(&root.join("main.tyr"), None).unwrap();
    let output =
        std::env::temp_dir().join(format!("tyrus-codegen-behavior-{}", std::process::id()));
    compile(
        &program,
        "behavior_interfaces",
        output.to_str().unwrap(),
        false,
    )
    .unwrap();
    let ir = std::fs::read_to_string(output.with_extension("ll")).unwrap();
    assert!(ir.contains("__tyrus_at___tyrus_module_0_UserMethods_isAdult_0"));
    assert!(ir.contains("@__tyrus_module_0_ageText"));
}

#[test]
fn folds_constant_decimal_addition_without_a_runtime_call() {
    let source = r#"
        const main = (): void => {
            const value: decimal = 0.1d + 0.2d;
            console.log(value);
        };
    "#;
    let output = std::env::temp_dir().join(format!("tyrus-codegen-decimal-{}", std::process::id()));
    let program = Parser::from_source(source)
        .unwrap()
        .parse_program()
        .unwrap();
    compile(
        &program,
        "decimal_addition",
        output.to_str().unwrap(),
        false,
    )
    .unwrap();
    let ir = std::fs::read_to_string(output.with_extension("ll")).unwrap();
    assert!(!ir.contains("tyrus_decimal_add"));
    assert!(ir.contains("i128 3, i32 -1"));
    assert!(ir.contains("tyrus_decimal_print"));
    let _ = std::fs::remove_file(output.with_extension("ll"));
    let _ = std::fs::remove_file(output);
}

#[test]
fn comptime_calls_are_materialized_and_runtime_uses_keep_the_function() {
    let source = r#"
        comptime function double(value: i32): i32 { return value * 2; }
        const main = (): void => {
            console.log(comptime(double(21)));
            console.log(double(21));
        };
    "#;
    let root = std::env::temp_dir().join(format!(
        "tyrus-codegen-comptime-runtime-{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).unwrap();
    std::fs::write(root.join("main.tyr"), source).unwrap();
    let program = tyrus_core::module_loader::load_program(&root.join("main.tyr"), None).unwrap();
    let output = root.join("comptime_runtime");
    compile(
        &program,
        "comptime_runtime",
        output.to_str().unwrap(),
        false,
    )
    .unwrap();
    let ir = std::fs::read_to_string(output.with_extension("ll")).unwrap();
    assert_eq!(ir.matches("@__tyrus_module_0_double(").count(), 2, "{ir}");
    assert!(ir.contains("i32 42"), "{ir}");
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn structured_comptime_result_reaches_codegen_as_an_ordinary_literal() {
    let source = r#"
        comptime function build(): i32[] { return [20, 22]; }
        const main = (): void => {
            const values: i32[] = comptime(build());
            console.log(values[0] + values[1]);
        };
    "#;
    let root = std::env::temp_dir().join(format!(
        "tyrus-codegen-comptime-structured-{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).unwrap();
    std::fs::write(root.join("main.tyr"), source).unwrap();
    let program = tyrus_core::module_loader::load_program(&root.join("main.tyr"), None).unwrap();
    let output = root.join("comptime_structured");
    compile(
        &program,
        "comptime_structured",
        output.to_str().unwrap(),
        false,
    )
    .unwrap();
    let ir = std::fs::read_to_string(output.with_extension("ll")).unwrap();
    assert_eq!(ir.matches("@__tyrus_module_0_build(").count(), 1, "{ir}");
    assert!(ir.contains("i32 20") && ir.contains("i32 22"), "{ir}");
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn folds_nested_constant_decimal_arithmetic_with_precedence_and_parentheses() {
    let source = r#"
        const main = (): void => {
            const value: decimal = (0.1d + 0.2d) * 10d - 1d;
            console.log(value);
        };
    "#;
    let output =
        std::env::temp_dir().join(format!("tyrus-codegen-decimal-fold-{}", std::process::id()));
    let program = Parser::from_source(source)
        .unwrap()
        .parse_program()
        .unwrap();
    compile(
        &program,
        "decimal_constant_fold",
        output.to_str().unwrap(),
        false,
    )
    .unwrap();
    let ir = std::fs::read_to_string(output.with_extension("ll")).unwrap();
    assert!(!ir.contains("tyrus_decimal_add"));
    assert!(!ir.contains("tyrus_decimal_mul"));
    assert!(!ir.contains("tyrus_decimal_sub"));
    assert!(ir.contains("i128 2, i32 0"));
    let _ = std::fs::remove_file(output.with_extension("ll"));
    let _ = std::fs::remove_file(output);
}

#[test]
fn keeps_non_constant_decimal_arithmetic_in_the_runtime() {
    let source = r#"
        const sum = (left: decimal, right: decimal): decimal => {
            return left + right;
        };
        const main = (): void => {
            console.log(sum(0.1d, 0.2d));
        };
    "#;
    let output = std::env::temp_dir().join(format!(
        "tyrus-codegen-decimal-runtime-{}",
        std::process::id()
    ));
    let program = Parser::from_source(source)
        .unwrap()
        .parse_program()
        .unwrap();
    compile(
        &program,
        "decimal_runtime_arithmetic",
        output.to_str().unwrap(),
        false,
    )
    .unwrap();
    let ir = std::fs::read_to_string(output.with_extension("ll")).unwrap();
    assert!(ir.contains("tyrus_decimal_add"));
    let _ = std::fs::remove_file(output.with_extension("ll"));
    let _ = std::fs::remove_file(output);
}

#[test]
fn emits_decimal_min_and_max_constants() {
    let source = r#"
        const main = (): void => {
            const min: decimal = decimal.MIN;
            const max: decimal = decimal.MAX;
            console.log(min, max);
        };
    "#;
    let output = std::env::temp_dir().join(format!(
        "tyrus-codegen-decimal-bounds-{}",
        std::process::id()
    ));
    let program = Parser::from_source(source)
        .unwrap()
        .parse_program()
        .unwrap();
    compile(&program, "decimal_bounds", output.to_str().unwrap(), false).unwrap();
    let ir = std::fs::read_to_string(output.with_extension("ll")).unwrap();
    assert!(ir.contains("9999999999999999999999999999999999"));
    assert!(ir.contains("tyrus_decimal_print"));
    let _ = std::fs::remove_file(output.with_extension("ll"));
    let _ = std::fs::remove_file(output);
}

#[test]
fn converts_integers_to_decimal_without_a_float_intermediate() {
    let source = r#"
        const main = (): void => {
            const signed: decimal = 12i32 as decimal;
            const unsigned: decimal = 34u64 as decimal;
            console.log(signed, unsigned);
        };
    "#;
    let output =
        std::env::temp_dir().join(format!("tyrus-codegen-decimal-cast-{}", std::process::id()));
    let program = Parser::from_source(source)
        .unwrap()
        .parse_program()
        .unwrap();
    compile(&program, "decimal_cast", output.to_str().unwrap(), false).unwrap();
    let ir = std::fs::read_to_string(output.with_extension("ll")).unwrap();
    assert!(ir.contains("store { i128, i32 } { i128 12, i32 0 }"));
    assert!(ir.contains("tyrus_decimal_print"));
    assert!(!ir.contains("sitofp"));
    assert!(!ir.contains("uitofp"));
    let _ = std::fs::remove_file(output.with_extension("ll"));
    let _ = std::fs::remove_file(output);
}

#[test]
fn converts_integral_decimal_to_an_integer_without_float_parsing() {
    let source = r#"
        const main = (): void => {
            const value: decimal = 12.0d;
            const integer: i32 = value as i32;
            console.log(integer);
        };
    "#;
    let output = std::env::temp_dir().join(format!(
        "tyrus-codegen-decimal-to-int-{}",
        std::process::id()
    ));
    let program = Parser::from_source(source)
        .unwrap()
        .parse_program()
        .unwrap();
    compile(&program, "decimal_to_int", output.to_str().unwrap(), false).unwrap();
    let ir = std::fs::read_to_string(output.with_extension("ll")).unwrap();
    assert!(ir.contains("tyrus_decimal_to_i128"));
    assert!(!ir.contains("fptosi"));
    let _ = std::fs::remove_file(output.with_extension("ll"));
    let _ = std::fs::remove_file(output);
}

#[test]
fn parses_decimal_text_to_an_optional_decimal_without_float_parsing() {
    let source = r#"
        const main = (): void => {
            const value: decimal? = parseDecimal("19.3");
            console.log(value ?? 0d);
        };
    "#;
    let output = std::env::temp_dir().join(format!(
        "tyrus-codegen-parse-decimal-{}",
        std::process::id()
    ));
    let program = Parser::from_source(source)
        .unwrap()
        .parse_program()
        .unwrap();
    compile(&program, "parse_decimal", output.to_str().unwrap(), false).unwrap();
    let ir = std::fs::read_to_string(output.with_extension("ll")).unwrap();
    assert!(ir.contains("tyrus_parse_decimal"));
    assert!(ir.contains("tyrus_decimal_print"));
    assert!(!ir.contains("tyrus_parse_f64"));
    let _ = std::fs::remove_file(output.with_extension("ll"));
    let _ = std::fs::remove_file(output);
}

#[test]
fn rejects_decimal_literals_outside_the_finite_range() {
    let source = r#"
        const main = (): void => {
            const value: decimal = 1e6145d;
            console.log(value);
        };
    "#;
    let output = std::env::temp_dir().join(format!(
        "tyrus-codegen-decimal-overflow-{}",
        std::process::id()
    ));
    let program = Parser::from_source(source)
        .unwrap()
        .parse_program()
        .unwrap();
    let error = compile(
        &program,
        "decimal_overflow",
        output.to_str().unwrap(),
        false,
    )
    .unwrap_err();
    assert!(error.contains("decimal literal is outside the decimal range"));
    let _ = std::fs::remove_file(output.with_extension("ll"));
    let _ = std::fs::remove_file(output);
}

#[test]
fn normalizes_trailing_zeroes_before_validating_decimal_literal_precision() {
    let source = r#"
        const main = (): void => {
            const value: decimal = 1.0000000000000000000000000000000000000000d;
            console.log(value);
        };
    "#;
    let output = std::env::temp_dir().join(format!(
        "tyrus-codegen-decimal-trailing-zeroes-{}",
        std::process::id()
    ));
    let program = Parser::from_source(source)
        .unwrap()
        .parse_program()
        .unwrap();
    compile(
        &program,
        "decimal_trailing_zeroes",
        output.to_str().unwrap(),
        false,
    )
    .unwrap();
    let ir = std::fs::read_to_string(output.with_extension("ll")).unwrap();
    assert!(ir.contains("i128 1"));
    let _ = std::fs::remove_file(output.with_extension("ll"));
    let _ = std::fs::remove_file(output);
}

#[test]
fn folds_constant_decimal_division_without_a_runtime_call() {
    let source = r#"
        const main = (): void => {
            const value: decimal = 1d / 3d;
            console.log(value);
        };
    "#;
    let output =
        std::env::temp_dir().join(format!("tyrus-codegen-decimal-div-{}", std::process::id()));
    let program = Parser::from_source(source)
        .unwrap()
        .parse_program()
        .unwrap();
    compile(
        &program,
        "decimal_division",
        output.to_str().unwrap(),
        false,
    )
    .unwrap();
    let ir = std::fs::read_to_string(output.with_extension("ll")).unwrap();
    assert!(!ir.contains("tyrus_decimal_div"));
    assert!(ir.contains("tyrus_decimal_print"));
    let _ = std::fs::remove_file(output.with_extension("ll"));
    let _ = std::fs::remove_file(output);
}

#[test]
fn folds_constant_decimal_power_without_a_runtime_call() {
    let source = r#"
        const main = (): void => {
            const value: decimal = 2.5d ** 2d;
            console.log(value);
        };
    "#;
    let output =
        std::env::temp_dir().join(format!("tyrus-codegen-decimal-pow-{}", std::process::id()));
    let program = Parser::from_source(source)
        .unwrap()
        .parse_program()
        .unwrap();
    compile(&program, "decimal_power", output.to_str().unwrap(), false).unwrap();
    let ir = std::fs::read_to_string(output.with_extension("ll")).unwrap();
    assert!(!ir.contains("tyrus_decimal_pow"));
    assert!(ir.contains("tyrus_decimal_print"));
    let _ = std::fs::remove_file(output.with_extension("ll"));
    let _ = std::fs::remove_file(output);
}

#[test]
fn folds_constant_decimal_comparisons_without_a_runtime_call() {
    let source = r#"
        const main = (): void => {
            console.log(1.0d === 1.00d, 0.1d < 0.2d);
        };
    "#;
    let output =
        std::env::temp_dir().join(format!("tyrus-codegen-decimal-cmp-{}", std::process::id()));
    let program = Parser::from_source(source)
        .unwrap()
        .parse_program()
        .unwrap();
    compile(
        &program,
        "decimal_comparison",
        output.to_str().unwrap(),
        false,
    )
    .unwrap();
    let ir = std::fs::read_to_string(output.with_extension("ll")).unwrap();
    assert!(!ir.contains("tyrus_decimal_compare"));
    let _ = std::fs::remove_file(output.with_extension("ll"));
    let _ = std::fs::remove_file(output);
}

#[test]
fn types_extension_method_calls_so_boolean_results_print_as_booleans() {
    let source = r#"
        interface Account { active: boolean; owner: string; }
        const ownerText = (account: Account): string => account.owner;
        interface AccountText at Account { ownerText; }
        interface AccountRules at Account {
            isActive(account: Account): boolean { return account.active; }
        }
        const main = (): void => {
            const account: Account = { active: true, owner: "Ada" };
            console.log(account.isActive());
            console.log(account.ownerText());
        };
    "#;
    let root = std::env::temp_dir().join(format!(
        "tyrus-codegen-ext-bool-source-{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).unwrap();
    std::fs::write(root.join("main.tyr"), source).unwrap();
    let program = tyrus_core::module_loader::load_program(&root.join("main.tyr"), None).unwrap();
    let output =
        std::env::temp_dir().join(format!("tyrus-codegen-ext-bool-{}", std::process::id()));
    compile(&program, "ext_bool", output.to_str().unwrap(), false).unwrap();
    let ir = std::fs::read_to_string(output.with_extension("ll")).unwrap();
    assert!(
        ir.contains("select i1") && ir.contains("@bool.true"),
        "boolean extension method result is not printed through the boolean renderer"
    );
    assert!(ir.contains("@ownerText") || ir.contains("ownerText"));
    let _ = std::fs::remove_file(output.with_extension("ll"));
    let _ = std::fs::remove_file(output);
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn emits_shorthand_bindings_declared_in_an_imported_module() {
    let root = std::env::temp_dir().join(format!(
        "tyrus-codegen-module-binding-source-{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).unwrap();
    std::fs::write(
        root.join("users.tyr"),
        r#"
        export interface User { age: u32; }
        const ageText = (user: User): string => user.age.toString();
        interface UserText at User { ageText; }
        export const describe = (): void => {
            const user: User = { age: 21 };
            console.log(user.ageText());
        };
        "#,
    )
    .unwrap();
    std::fs::write(
        root.join("main.tyr"),
        r#"
        import { describe } from "./users";
        const main = (): void => { describe(); };
        "#,
    )
    .unwrap();
    let program = tyrus_core::module_loader::load_program(&root.join("main.tyr"), None).unwrap();
    let output = std::env::temp_dir().join(format!(
        "tyrus-codegen-module-binding-{}",
        std::process::id()
    ));
    compile(&program, "module_binding", output.to_str().unwrap(), false).unwrap();
    let ir = std::fs::read_to_string(output.with_extension("ll")).unwrap();
    assert!(
        ir.contains("__tyrus_module_") && ir.contains("ageText"),
        "module-declared shorthand binding did not reach codegen"
    );
    let _ = std::fs::remove_file(output.with_extension("ll"));
    let _ = std::fs::remove_file(output);
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn rejects_ambiguous_behavior_dispatch_even_without_a_checker_pass() {
    let error = compile_error(
        r#"
        interface Account { age: u32; }
        interface FirstRules at Account {
            label(account: Account): string { return account.age.toString(); }
        }
        interface SecondRules at Account {
            label(account: Account): string { return "account"; }
        }
        const main = (): void => {
            const account: Account = { age: 21 };
            console.log(account.label());
        };
        "#,
    );
    assert!(error.contains("error[E0034]"), "got: {error}");
    assert!(error.contains("`FirstRules`"), "got: {error}");
    assert!(error.contains("`SecondRules`"), "got: {error}");
}

#[test]
fn emits_inherited_behavior_methods_for_every_effective_target() {
    let source = r#"
        interface Account { age: u32; }
        interface NumericRules at i32, u32 {
            isPositive(value): boolean { return value > 0; }
        }
        interface AccountRules at Account {
            isAdult(account: Account): boolean { return account.age >= 18; }
        }
        interface PremiumAccountRules extends AccountRules {
            hasPriority(account: Account): boolean { return account.isAdult(); }
        }
        const main = (): void => {
            const account: Account = { age: 21 };
            const signed: i32 = 1;
            const unsigned: u32 = 1;
            console.log(account.isAdult(), account.hasPriority());
            console.log(signed.isPositive(), unsigned.isPositive());
        };
    "#;
    let root = std::env::temp_dir().join(format!(
        "tyrus-codegen-inherited-behavior-source-{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).unwrap();
    std::fs::write(root.join("main.tyr"), source).unwrap();
    let program = tyrus_core::module_loader::load_program(&root.join("main.tyr"), None).unwrap();
    let output = std::env::temp_dir().join(format!(
        "tyrus-codegen-inherited-behavior-{}",
        std::process::id()
    ));
    compile(
        &program,
        "inherited_behavior_interfaces",
        output.to_str().unwrap(),
        false,
    )
    .unwrap();
    let ir = std::fs::read_to_string(output.with_extension("ll")).unwrap();
    assert!(ir.contains("__tyrus_at___tyrus_module_0_AccountRules_isAdult_0"));
    assert!(ir.contains("__tyrus_at___tyrus_module_0_PremiumAccountRules_hasPriority_0"));
    assert!(ir.contains("__tyrus_at___tyrus_module_0_NumericRules_isPositive_0"));
    assert!(ir.contains("__tyrus_at___tyrus_module_0_NumericRules_isPositive_1"));
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn emits_plain_date_temporal_calls() {
    let source = r#"
        import { Duration, PlainDate, PlainTime, TemporalError, plainDateAddDays, plainDateAddDuration, plainDateAddMonths, plainDateAddYears, plainDateDayOfWeek, plainDateDayOfYear, plainDateDaysUntil, plainDateFrom, plainDateToString, plainDateEquals, plainTimeAddDuration, plainTimeAddNanoseconds, plainTimeAddSeconds, plainTimeEquals, plainTimeFrom, plainTimeNanosecondsUntil, plainTimeSecondsUntil, plainTimeToString, durationFromFields, durationTotal } from "tyrus:temporal";
        const main = (): Result<void, TemporalError> => {
                const date: PlainDate = plainDateFrom("2026-07-09")?;
                console.log(plainDateToString(date)?);
                console.log(plainDateEquals(date, date));
                console.log(plainDateToString(plainDateAddDays(date, 1)?)?);
                console.log(plainDateDayOfWeek(date)?);
                console.log(plainDateDayOfYear(date)?);
                console.log(plainDateToString(plainDateAddMonths(date, 1)?)?);
                console.log(plainDateToString(plainDateAddYears(date, 1)?)?);
                console.log(plainDateDaysUntil(date, plainDateAddDays(date, 1)?)?);
                const time: PlainTime = plainTimeFrom("14:30:15.500")?;
                console.log(plainTimeToString(time)?);
                console.log(plainTimeEquals(time, time));
                const later: PlainTime = plainTimeAddSeconds(time, 60)?;
                console.log(plainTimeSecondsUntil(time, later)?);
                const shiftedTime: PlainTime = plainTimeAddNanoseconds(time, 750_000_000)?;
                console.log(plainTimeNanosecondsUntil(time, shiftedTime)?);
                const duration: Duration = durationFromFields({ years: 0, months: 0, weeks: 0, days: 0, hours: 1, minutes: 30, seconds: 0, milliseconds: 0, microseconds: 0, nanoseconds: 0 });
                console.log(durationTotal(duration, "minutes")?);
                console.log(plainDateToString(plainDateAddDuration(date, duration)?)?);
                console.log(plainTimeToString(plainTimeAddDuration(time, duration)?)?);
                return Ok();
        };
    "#;
    let root = std::env::temp_dir().join(format!(
        "tyrus-codegen-plain-date-source-{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).unwrap();
    std::fs::write(root.join("main.tyr"), source).unwrap();
    let program = tyrus_core::module_loader::load_program(&root.join("main.tyr"), None).unwrap();
    let output =
        std::env::temp_dir().join(format!("tyrus-codegen-plain-date-{}", std::process::id()));
    compile(
        &program,
        "plain_date_temporal",
        output.to_str().unwrap(),
        false,
    )
    .unwrap();
    let ir = std::fs::read_to_string(output.with_extension("ll")).unwrap();
    assert!(ir.contains("tyrus_temporal_plain_date_from"));
    assert!(ir.contains("tyrus_temporal_plain_date_to_string"));
    assert!(ir.contains("tyrus_temporal_plain_date_equals"));
    assert!(ir.contains("tyrus_temporal_plain_date_add_days"));
    assert!(ir.contains("tyrus_temporal_plain_date_day_of_week"));
    assert!(ir.contains("tyrus_temporal_plain_date_day_of_year"));
    assert!(ir.contains("tyrus_temporal_plain_date_add_months"));
    assert!(ir.contains("tyrus_temporal_plain_date_add_years"));
    assert!(ir.contains("tyrus_temporal_plain_date_days_until"));
    assert!(ir.contains("tyrus_temporal_plain_time_from"));
    assert!(ir.contains("tyrus_temporal_plain_time_to_string"));
    assert!(ir.contains("tyrus_temporal_plain_time_equals"));
    assert!(ir.contains("tyrus_temporal_plain_time_add_seconds"));
    assert!(ir.contains("tyrus_temporal_plain_time_seconds_until"));
    assert!(ir.contains("tyrus_temporal_plain_time_add_nanoseconds"));
    assert!(ir.contains("tyrus_temporal_plain_time_nanoseconds_until"));
    assert!(ir.contains("tyrus_temporal_duration_from_fields"));
    assert!(ir.contains("tyrus_temporal_duration_total"));
    assert!(ir.contains("tyrus_temporal_plain_date_add_duration"));
    assert!(ir.contains("tyrus_temporal_plain_time_add_duration"));
    assert!(ir.contains("declare void @tyrus_temporal_plain_date_from_result({ ptr, i64 }, ptr)"));
    assert!(ir.contains("declare void @tyrus_temporal_plain_time_from_result({ ptr, i64 }, ptr)"));
    assert!(ir.contains("declare void @tyrus_temporal_duration_from_fields(ptr, ptr)"));
    assert!(
        ir.contains("declare void @tyrus_temporal_duration_total_result(ptr, { ptr, i64 }, ptr)")
    );
    assert!(
        ir.contains("declare void @tyrus_temporal_plain_date_add_duration_result(ptr, ptr, ptr)")
    );
    assert!(
        ir.contains("declare void @tyrus_temporal_plain_time_add_duration_result(ptr, ptr, ptr)")
    );
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn emits_crypto_abi_for_bytes_aes_and_argon2_migration_checks() {
    let source = r#"        import { CryptoError, argon2, cipher, digest, hmac, rand, timingSafeEqual } from "tyrus:crypto";



        import { EncodingError, utf8 } from "tyrus:encoding";
function cryptoBytes(result: Result<u8[], CryptoError>): u8[] {
    match (result) {
        Ok(value) => { return value; }
        Err(error) => { printError(error.code); return []; }
    }
}
        function main(): Result<void, CryptoError> {
            try {
                const bytes: u8[] = [0, 1, 2, 0, 255];
                const key: u8[] = [
                    0, 1, 2, 3, 4, 5, 6, 7,
                    8, 9, 10, 11, 12, 13, 14, 15,
                    16, 17, 18, 19, 20, 21, 22, 23,
                    24, 25, 26, 27, 28, 29, 30, 31
                ];
                const encoded: u8[] = utf8.encode("abc");
                const decodedResult: Result<string, EncodingError> = utf8.decode(encoded);
                match (decodedResult) {
                    Ok(decoded) => { console.log(decoded); }
                    Err(encodingError) => { console.error(encodingError.code); }
                }
                const textHash: u8[] = cryptoBytes(digest.sha256("abc"));
                const binaryHash: u8[] = cryptoBytes(digest.sha256(bytes));
                const hash384: u8[] = cryptoBytes(digest.sha384(bytes));
                const hash512: u8[] = cryptoBytes(digest.sha512(bytes));
                const legacyHash: u8[] = cryptoBytes(digest.md5(bytes));
                const mac: u8[] = cryptoBytes(hmac.sha256(bytes, key));
                const mac384: u8[] = cryptoBytes(hmac.sha384(bytes, key));
                const mac512: u8[] = cryptoBytes(hmac.sha512(bytes, key));
                const stream: DigestStream = digest.create("sha256")?;
                stream.update(bytes)?;
                const streamed: u8[] = stream.digest()?;
                stream.reset()?;
                stream.dispose();
                const encryptedResult: Result<u8[], CryptoError> = cipher.aes.encrypt(bytes, key);
                match (encryptedResult) {
                    Ok(encrypted) => {
                        const decryptedResult: Result<u8[], CryptoError> = cipher.aes.decryptBytes(encrypted, key);
                        match (decryptedResult) {
                            Ok(decrypted) => { timingSafeEqual(bytes, decrypted); }
                            Err(_) => { const ignoredDecryptError: u8 = 0; }
                        }
                    }
                    Err(_) => { const ignoredEncryptError: u8 = 0; }
                }
                const equalText: boolean = timingSafeEqual("same", "same");
            const randomBytesResult: Result<u8[], CryptoError> = rand.randomBytes(16);
            match (randomBytesResult) {
                Ok(_) => { const ignoredBytes: u8 = 0; }
                Err(_) => { const ignoredByteError: u8 = 0; }
            }
            const randomIntResult: Result<i64, CryptoError> = rand.randomInt(-10, 10);
            match (randomIntResult) {
                Ok(_) => { const ignoredInt: u8 = 0; }
                Err(_) => { const ignoredIntError: u8 = 0; }
            }
                const randomFloat: f64 = rand.randomFloat();
                const uuid4: string = rand.uuidv4();
                const uuid7: string = rand.uuidv7();
                const ulidResult: Result<string, CryptoError> = rand.ulid();
                match (ulidResult) {
                    Ok(_) => { const ignoredUlid: u8 = 0; }
                    Err(_) => { const ignoredUlidError: u8 = 0; }
                }
                const rehashResult: Result<boolean, CryptoError> = argon2.needsRehash("$argon2id$v=19$m=65536,t=3,p=4$c29tZXNhbHQxMjM0NTY3OA$AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA");
                match (rehashResult) {
                    Ok(migrate) => { const needsMigration: boolean = migrate; }
                    Err(_) => { const ignoredRehashError: u8 = 0; }
                }
            } catch (error: Error) {
                console.error(error.message);
            }
            return Ok();
        }
        async function passwordFlow(): Promise<Result<void, CryptoError>> {
            const pepper: u8[] = [1, 2, 3, 4];
            const hash: string = await argon2.hash("password", {
                memoryCost: 65536,
                parallelism: 1,
                secret: pepper,
                timeCost: 3,
                type: "argon2id",
            })?;
            const valid: boolean = await argon2.verify(hash, "password", { secret: pepper })?;
            console.log(valid);
            return Ok();
        }
    "#;
    let root = std::env::temp_dir().join(format!("tyrus-crypto-source-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).unwrap();
    std::fs::write(root.join("main.tyr"), source).unwrap();
    let program = tyrus_core::module_loader::load_program(&root.join("main.tyr"), None).unwrap();
    let output = std::env::temp_dir().join(format!("tyrus-crypto-abi-{}", std::process::id()));
    compile(&program, "crypto_abi", output.to_str().unwrap(), false).unwrap();
    let ir = std::fs::read_to_string(output.with_extension("ll")).unwrap();
    assert!(
        ir.matches("call void @tyrus_crypto_digest_stream_free(")
            .count()
            >= 3,
        "stream cleanup must cover explicit disposal and Result propagation exits:\n{ir}"
    );
    for symbol in [
        "tyrus_encoding_utf8_encode",
        "tyrus_encoding_utf8_decode",
        "tyrus_crypto_digest_sha256_result",
        "tyrus_crypto_digest_sha384_result",
        "tyrus_crypto_digest_sha512_result",
        "tyrus_crypto_digest_md5_result",
        "tyrus_crypto_hmac_sha256_result",
        "tyrus_crypto_hmac_sha384_result",
        "tyrus_crypto_hmac_sha512_result",
        "tyrus_crypto_digest_create_result",
        "tyrus_crypto_digest_stream_update_result",
        "tyrus_crypto_digest_stream_digest_result",
        "tyrus_crypto_digest_stream_reset_result",
        "tyrus_crypto_digest_stream_free",
        "tyrus_crypto_rand_random_bytes",
        "tyrus_crypto_rand_random_int",
        "tyrus_crypto_rand_random_float",
        "tyrus_crypto_rand_uuidv4",
        "tyrus_crypto_rand_uuidv7",
        "tyrus_crypto_rand_ulid",
        "tyrus_crypto_cipher_aes_encrypt_result",
        "tyrus_crypto_cipher_aes_decrypt_bytes_result",
        "tyrus_crypto_timing_safe_equal",
        "tyrus_crypto_argon2_needs_rehash",
        "tyrus_crypto_argon2_hash",
        "tyrus_crypto_argon2_verify",
    ] {
        assert!(ir.contains(symbol), "missing `{symbol}` in generated IR");
    }
    assert!(
        !ir.contains("crypto.string.length"),
        "hashing text should read the byte length from the value:\n{ir}"
    );
    assert!(ir.contains("crypto.digest.result"));
    assert!(ir.contains("crypto.cipher.encrypt.result"));
    assert!(ir.contains("crypto.argon2.hash.options.memoryCost"));
    assert!(ir.contains("crypto.argon2.hash.options.secret"));
    assert!(ir.contains("crypto.argon2.verify.options.secret"));
    assert!(ir.contains("async.payload.drop.copy.array.data"));
    assert!(ir.contains("call void @free("));
    let _ = std::fs::remove_file(output.with_extension("ll"));
    let _ = std::fs::remove_file(output);
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn jwt_sign_claim_serialization_failure_is_returned_as_result() {
    let source = r#"
        import { CryptoError, jwt } from "tyrus:crypto";
        interface Claims { sub: string; }
        const createToken = (): Result<string, CryptoError> => {
            const claims: Claims = { sub: "user-42" };
            return jwt.sign<Claims>(claims, "0123456789abcdef0123456789abcdef");
        };
        const main = (): void => {
            match (createToken()) {
                Ok(_) => {}
                Err(error) => { printError(error.code); }
            }
        };
    "#;
    let ir = compile_module_to_ir(source, "jwt-sign-result");
    assert!(ir.contains("crypto.jwt.sign.done"));
    assert!(ir.contains("call void @tyrus_crypto_take_error_result"));
    assert!(ir.contains("call void @tyrus_crypto_jwt_sign_result"));
    assert!(
        !ir.contains("call void @tyrus_throw"),
        "JWT claim serialization failures must populate the Result instead of throwing"
    );
}

#[test]
fn encoding_hex_emits_runtime_symbols_and_owns_buffers() {
    let source = r#"
        import { hex } from "tyrus:encoding";
        function main(): void {
            const text: string = hex.encode([0xde, 0xad, 0xbe, 0xef]);
            const bytes: u8[] = hex.decode(text);
            console.log(text.length + bytes.length);
        }
    "#;
    let ir = compile_to_ir(source, "hex-encoding");
    assert!(ir.contains("tyrus_encoding_hex_encode"), "{ir}");
    assert!(ir.contains("tyrus_encoding_hex_decode"), "{ir}");
    assert!(
        ir.matches("call void @free(").count() >= 2,
        "hex encode string and decode bytes must release their runtime buffers:\n{ir}"
    );
}

#[test]
fn encoding_base_codecs_emit_runtime_symbols_and_own_buffers() {
    let source = r#"
        import { base64, base64Url, base32 } from "tyrus:encoding";
        function main(): void {
            const b: u8[] = [0xde, 0xad, 0xbe, 0xef];
            const a: string = base64.encode(b);
            const c: u8[] = base64Url.decode(a);
            const d: string = base32.encode(c);
            console.log(a.length + c.length + d.length);
        }
    "#;
    let ir = compile_to_ir(source, "base-encoding");
    assert!(ir.contains("tyrus_encoding_base64_encode"), "{ir}");
    assert!(ir.contains("tyrus_encoding_base64url_decode"), "{ir}");
    assert!(ir.contains("tyrus_encoding_base32_encode"), "{ir}");
    assert!(
        ir.matches("call void @free(").count() >= 1,
        "decoded bytes must release their runtime buffer:\n{ir}"
    );
    assert!(
        ir.matches("call void @tyrus_string_release_buffer(")
            .count()
            >= 2,
        "encoded base strings must release their runtime buffers:\n{ir}"
    );
}

#[test]
fn encoding_radix_emits_runtime_symbols_and_guards_errors() {
    let source = r#"
        import { EncodingError, radix } from "tyrus:encoding";
        function main(): void {
            const hex: string = radix.format(255, 16);
            const parsed: Result<i64, EncodingError> = radix.parseI64(hex, 16);
            const unsigned: Result<u64, EncodingError> = radix.parseU64(hex, 16);
            console.log(hex.length);
            match (parsed) {
                Ok(value) => { console.log(value); }
                Err(error) => { console.log(error.code); }
            }
            match (unsigned) {
                Ok(value) => { console.log(value); }
                Err(error) => { console.log(error.code); }
            }
        }
    "#;
    let ir = compile_module_to_ir(source, "radix-encoding");
    assert!(ir.contains("tyrus_encoding_radix_format"), "{ir}");
    assert!(ir.contains("tyrus_encoding_radix_parse_i64_result"), "{ir}");
    assert!(ir.contains("tyrus_encoding_radix_parse_u64_result"), "{ir}");
    assert!(!ir.contains("tyrus_encoding_take_error"), "{ir}");
    assert!(!ir.contains("tyrus_encoding_clear_error"), "{ir}");
    assert!(
        ir.matches("call void @tyrus_string_release_buffer(")
            .count()
            >= 1,
        "the formatted radix string must release its runtime buffer:\n{ir}"
    );
}

#[test]
fn emits_memory_drop_for_native_collections() {
    let source = r#"
        function main(): void {
            let entries: Map<string, i32> = Map<string, i32>();
            entries.set("key", 1);
            __tyrus_memory_drop(entries);
        }
    "#;
    let ir = compile_to_ir(source, "memory-drop-native-collection");
    assert!(ir.contains("tyrus_map_free"), "{ir}");
}

#[test]
fn emits_memory_swap_and_replace_for_copy_bindings() {
    let source = r#"
        function main(): void {
            let left: i32 = 4;
            let right: i32 = 9;
            let first: string = "first".slice(0);
            let second: string = "second".slice(0);
            __tyrus_memory_swap(left, right);
            __tyrus_memory_swap(first, second);
            let source: string = "source".slice(0);
            let destination: string = "destination".slice(0);
            const previous: string = __tyrus_memory_replace(destination, source);
            print(previous, destination);
            const previous: i32 = __tyrus_memory_replace(left, 12);
            print(left, right, previous);
        }
    "#;
    let ir = compile_to_ir(source, "memory-swap-replace");
    assert!(ir.contains("memory.swap.left"), "{ir}");
    assert!(ir.contains("memory.swap.right"), "{ir}");
    assert!(ir.contains("memory.replace.old"), "{ir}");
    assert!(ir.contains("memory.replace.source.owned"), "{ir}");
}

#[test]
fn rejects_memory_swap_of_the_same_binding() {
    let error = compile_error(
        r#"
            function main(): void {
                let value: i32 = 4;
                __tyrus_memory_swap(value, value);
            }
        "#,
    );
    assert!(
        error.contains("two distinct mutable local bindings"),
        "{error}"
    );
}

#[test]
fn evaluates_memory_layout_queries_at_compile_time() {
    let source = r#"
        function main(): void {
            const size: usize = __tyrus_memory_size_of<i32>();
            const alignment: usize = __tyrus_memory_align_of<i32>();
            print(size, alignment);
        }
    "#;
    let ir = compile_to_ir(source, "memory-layout-query");
    assert!(!ir.contains("tyrus_memory_size_of"), "{ir}");
    assert!(!ir.contains("tyrus_memory_align_of"), "{ir}");
    assert!(ir.contains("i64 4"), "{ir}");
}

#[test]
fn emits_utf16_encoding_abi_for_aliased_namespaces() {
    let source = r#"
        import { EncodingError, utf16 as windowsText } from "tyrus:encoding";
        function main(): void {
            const units: u16[] = windowsText.encode("A😀");
            const littleEndian: u8[] = windowsText.encodeLE("A😀");
            const bigEndian: u8[] = windowsText.encodeBE("A😀");
            const decoded: Result<string, EncodingError> = windowsText.decode(units);
            const decodedLittleEndian: Result<string, EncodingError> = windowsText.decodeLE(littleEndian);
            const decodedBigEndian: Result<string, EncodingError> = windowsText.decodeBE(bigEndian);
            match (decoded) {
                Ok(text) => { console.log(text); }
                Err(error) => { console.error(error.code); }
            }
            match (decodedLittleEndian) {
                Ok(text) => { console.log(text); }
                Err(error) => { console.error(error.code); }
            }
            match (decodedBigEndian) {
                Ok(text) => { console.log(text); }
                Err(error) => { console.error(error.code); }
            }
        }
    "#;
    let root = std::env::temp_dir().join(format!("tyrus-encoding-source-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).unwrap();
    std::fs::write(root.join("main.tyr"), source).unwrap();
    let program = tyrus_core::module_loader::load_program(&root.join("main.tyr"), None).unwrap();
    let output = std::env::temp_dir().join(format!("tyrus-encoding-abi-{}", std::process::id()));
    compile(&program, "encoding_abi", output.to_str().unwrap(), false).unwrap();
    let ir = std::fs::read_to_string(output.with_extension("ll")).unwrap();
    for symbol in [
        "tyrus_encoding_utf16_encode",
        "tyrus_encoding_utf16_encode_le",
        "tyrus_encoding_utf16_encode_be",
        "tyrus_encoding_utf16_decode_result",
        "tyrus_encoding_utf16_decode_le_result",
        "tyrus_encoding_utf16_decode_be_result",
    ] {
        assert!(
            ir.contains(symbol),
            "missing `{symbol}` in generated IR:\n{ir}"
        );
    }
    let _ = std::fs::remove_dir_all(root);
    let _ = std::fs::remove_file(output.with_extension("ll"));
}

#[test]
fn emits_jwt_options_jwks_and_catchable_error_abi() {
    let source = r#"
        import { CryptoError, jwt } from "tyrus:crypto";


function cryptoString(result: Result<string, CryptoError>): string {
    match (result) {
        Ok(value) => { return value; }
        Err(error) => { printError(error.code); return ""; }
    }
}

        interface SessionClaims {
            sub: string;
            amount: decimal;
        }

        function tokenFlow(): void {
            try {
                const secret: string = "0123456789abcdef0123456789abcdef";
                const payload: SessionClaims = { sub: "user-42", amount: 0.1d };
                const token: string = cryptoString(jwt.sign(payload, secret, {
                    algorithm: "HS384",
                    audience: "tyrus-api",
                    expiresIn: "2h",
                    issuer: "https://issuer.example",
                    keyId: "active-key",
                    subject: "user-42",
                }));
                const claimsResult: Result<SessionClaims, CryptoError> = jwt.verify<SessionClaims>(token, secret, {
                    algorithms: ["HS384"],
                    audience: "tyrus-api",
                    clockTolerance: 5,
                    issuer: "https://issuer.example",
                    maxAge: "2h",
                    subject: "user-42",
                });
                match (claimsResult) {
                    Ok(claims) => { console.log(claims); }
                    Err(error) => { console.log(error.code); }
                }
                const decoded: Result<SessionClaims, CryptoError> = jwt.decodeUnverified<SessionClaims>(token);
                match (decoded) {
                    Ok(claims) => { console.log(claims.sub); }
                    Err(error) => { console.log(error.code); }
                }
                const jwksClaims: Result<SessionClaims, CryptoError> = jwt.verifyWithJwks<SessionClaims>(token, "{}", { algorithms: ["RS256"] });
                match (jwksClaims) {
                    Ok(claims) => { console.log(claims.sub); }
                    Err(error) => { console.log(error.code); }
                }
                try {
                    match (jwt.verify<SessionClaims>(token, secret)) {
                        Ok(claims) => { console.log(claims); }
                        Err(error) => { console.log(error.code); }
                    }
                } catch (error: Error) {
                    console.log(error.message);
                }
            } catch (error: Error) {
                console.error(error.message);
            }
        }

        function main(): void {
            tokenFlow();
        }

        async function loadKeys(): Promise<void> {
            const keys: Result<string, CryptoError> = await jwt.fetchJwks("https://issuer.example/.well-known/jwks.json", { forceRefresh: true });
            match (keys) {
                Ok(document) => { console.log(document); }
                Err(error) => { console.error(error.code); }
            }
        }
    "#;
    let root = std::env::temp_dir().join(format!("tyrus-jwt-source-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).unwrap();
    std::fs::write(root.join("main.tyr"), source).unwrap();
    let program = tyrus_core::module_loader::load_program(&root.join("main.tyr"), None).unwrap();
    let output = std::env::temp_dir().join(format!("tyrus-jwt-abi-{}", std::process::id()));
    compile(&program, "jwt_abi", output.to_str().unwrap(), false).unwrap();
    let ir = std::fs::read_to_string(output.with_extension("ll")).unwrap();
    for symbol in [
        "tyrus_crypto_jwt_sign_result",
        "tyrus_crypto_jwt_verify_result",
        "tyrus_crypto_jwt_verify_with_jwks_result",
        "tyrus_crypto_jwt_decode_unverified_result",
        "tyrus_crypto_take_error_result",
        "tyrus_crypto_jwt_fetch_jwks",
        "tyrus_crypto_clear_error",
        "tyrus_crypto_take_error",
        "tyrus_json_builder_add_decimal",
        "tyrus_json_document_get_decimal",
    ] {
        assert!(ir.contains(symbol), "missing `{symbol}` in generated IR");
    }
    assert!(ir.contains("crypto.jwt.sign.options"));
    assert!(ir.contains("crypto.jwt.verify.options"));
    assert!(ir.contains("crypto.jwt.sign.options.algorithm"));
    assert!(ir.contains("crypto.options.array.element"));
    let _ = std::fs::remove_file(output.with_extension("ll"));
    let _ = std::fs::remove_file(output);
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn releases_top_level_crypto_results_before_synthetic_main_returns() {
    let source = r#"        import { CryptoError, digest } from "tyrus:crypto";
function cryptoBytes(result: Result<u8[], CryptoError>): u8[] {
    match (result) {
        Ok(value) => { return value; }
        Err(error) => { printError(error.code); return []; }
    }
}



        const bytes: u8[] = [1, 2, 3];
        const hash: u8[] = cryptoBytes(digest.sha256(bytes));
    "#;
    let root =
        std::env::temp_dir().join(format!("tyrus-crypto-global-source-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).unwrap();
    std::fs::write(root.join("main.tyr"), source).unwrap();
    let program = tyrus_core::module_loader::load_program(&root.join("main.tyr"), None).unwrap();
    let output = std::env::temp_dir().join(format!("tyrus-crypto-global-{}", std::process::id()));
    compile(&program, "crypto_global", output.to_str().unwrap(), false).unwrap();
    let ir = std::fs::read_to_string(output.with_extension("ll")).unwrap();
    assert!(ir.contains("async.payload.drop.copy.array.data"));
    assert!(ir.contains("call void @free("));
    let _ = std::fs::remove_file(output.with_extension("ll"));
    let _ = std::fs::remove_file(output);
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn automatically_drops_digest_streams_and_disarms_explicit_dispose() {
    let source = r#"
        import { CryptoError, digest } from "tyrus:crypto";

        function automatic(): Result<void, CryptoError> {
            const stream: DigestStream = digest.create("sha256")?;
            stream.update("data")?;
            return Ok();
        }
        function explicit(): Result<void, CryptoError> {
            const stream: DigestStream = digest.create("sha256")?;
            stream.dispose();
            return Ok();
        }
    "#;
    let root =
        std::env::temp_dir().join(format!("tyrus-digest-stream-source-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).unwrap();
    std::fs::write(root.join("main.tyr"), source).unwrap();
    let program = tyrus_core::module_loader::load_program(&root.join("main.tyr"), None).unwrap();
    let output =
        std::env::temp_dir().join(format!("tyrus-digest-stream-drop-{}", std::process::id()));
    compile(
        &program,
        "digest_stream_drop",
        output.to_str().unwrap(),
        false,
    )
    .unwrap();
    let ir = std::fs::read_to_string(output.with_extension("ll")).unwrap();
    assert!(ir.matches("@tyrus_crypto_digest_stream_free").count() >= 3);
    assert!(ir.contains("%native.cleanup = load ptr"));
    assert!(ir.contains("store ptr null, ptr %stream"));
    let _ = std::fs::remove_file(output.with_extension("ll"));
    let _ = std::fs::remove_file(output);
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn infers_void_return_type_for_a_concise_arrow_body_that_logs() {
    for method in ["log", "error", "warn"] {
        let source = format!(
            r#"
            const hello = () => console.{method}("hi");

            function main() {{
                hello();
            }}
        "#
        );
        let tokens = tyrus_core::lexer::lex(&source).unwrap();
        let program = Parser::new(tokens).parse_program().unwrap();
        let output = std::env::temp_dir().join(format!(
            "tyrus-arrow-console-{method}-{}",
            std::process::id()
        ));
        compile(&program, "arrow_console", output.to_str().unwrap(), false)
            .unwrap_or_else(|error| panic!("console.{method} case must compile: {error}"));
        let _ = std::fs::remove_file(output.with_extension("ll"));
        let _ = std::fs::remove_file(output);
    }
}

#[test]
fn chained_comparison_evaluates_a_shared_middle_term_exactly_once() {
    let source = r#"
        function obterValor(): i32 {
            return 25;
        }

        function main() {
            const a: i32 = 10;
            const b: i32 = 50;
            console.log(a < obterValor() <= b);
        }
    "#;
    let tokens = tyrus_core::lexer::lex(source).unwrap();
    let program = Parser::new(tokens).parse_program().unwrap();
    let output = std::env::temp_dir().join(format!(
        "tyrus-chained-comparison-single-eval-{}",
        std::process::id()
    ));
    compile(
        &program,
        "chained_comparison_single_eval",
        output.to_str().unwrap(),
        false,
    )
    .unwrap();
    let ir = std::fs::read_to_string(output.with_extension("ll")).unwrap();
    let call_count = ir.matches("call i32 @obterValor(").count();
    assert_eq!(
        call_count, 1,
        "expected exactly one call to obterValor:\n{ir}"
    );
    let _ = std::fs::remove_file(output.with_extension("ll"));
    let _ = std::fs::remove_file(output);
}

#[test]
fn chained_comparison_short_circuits_with_conditional_branches() {
    let source = r#"
        function main() {
            const a: i32 = 10;
            const b: i32 = 20;
            const c: i32 = 30;
            console.log(a < b < c);
        }
    "#;
    let tokens = tyrus_core::lexer::lex(source).unwrap();
    let program = Parser::new(tokens).parse_program().unwrap();
    let output = std::env::temp_dir().join(format!(
        "tyrus-chained-comparison-short-circuit-{}",
        std::process::id()
    ));
    compile(
        &program,
        "chained_comparison_short_circuit",
        output.to_str().unwrap(),
        false,
    )
    .unwrap();
    let ir = std::fs::read_to_string(output.with_extension("ll")).unwrap();
    assert!(
        ir.contains("chaincmp.next"),
        "expected a chaincmp.next block:\n{ir}"
    );
    assert!(
        ir.contains("chaincmp.merge"),
        "expected a chaincmp.merge block:\n{ir}"
    );
    assert!(ir.contains("br i1"), "expected a conditional branch:\n{ir}");
    let _ = std::fs::remove_file(output.with_extension("ll"));
    let _ = std::fs::remove_file(output);
}

#[test]
fn template_format_specifiers_call_the_precision_and_padded_runtime_helpers() {
    let source = r#"
        function main() {
            const pi: f64 = 3.141592;
            const id: i32 = 42;
            console.log(`Valor: ${pi:.2f} | Codigo: ${id:04d}`);
        }
    "#;
    let tokens = tyrus_core::lexer::lex(source).unwrap();
    let program = Parser::new(tokens).parse_program().unwrap();
    let output = std::env::temp_dir().join(format!(
        "tyrus-template-format-specifiers-{}",
        std::process::id()
    ));
    compile(
        &program,
        "template_format_specifiers",
        output.to_str().unwrap(),
        false,
    )
    .unwrap();
    let ir = std::fs::read_to_string(output.with_extension("ll")).unwrap();
    assert!(
        ir.contains("call { ptr, i64 } @tyrus_format_f64_precision("),
        "expected a call to tyrus_format_f64_precision:\n{ir}"
    );
    assert!(
        ir.contains("call { ptr, i64 } @tyrus_format_i128_padded("),
        "expected a call to tyrus_format_i128_padded:\n{ir}"
    );
    let _ = std::fs::remove_file(output.with_extension("ll"));
    let _ = std::fs::remove_file(output);
}

#[test]
fn range_in_emits_a_bounds_check_with_no_allocation() {
    let source = r#"
        function main() {
            const status: i32 = 204;
            console.log(status in 200..299);
        }
    "#;
    let tokens = tyrus_core::lexer::lex(source).unwrap();
    let program = Parser::new(tokens).parse_program().unwrap();
    let output = std::env::temp_dir().join(format!("tyrus-in-range-{}", std::process::id()));
    compile(&program, "in_range", output.to_str().unwrap(), false).unwrap();
    let ir = std::fs::read_to_string(output.with_extension("ll")).unwrap();
    assert!(
        ir.contains("range.ge") && ir.contains("range.end_ok"),
        "expected the range bounds-check instructions:\n{ir}"
    );
    assert!(
        !ir.contains("@malloc("),
        "a Range membership check must not allocate:\n{ir}"
    );
    let _ = std::fs::remove_file(output.with_extension("ll"));
    let _ = std::fs::remove_file(output);
}

#[test]
fn range_contains_method_reuses_the_same_bounds_check_as_in() {
    let source = r#"
        function main() {
            const r = 0..10;
            console.log(r.contains(5));
        }
    "#;
    let tokens = tyrus_core::lexer::lex(source).unwrap();
    let program = Parser::new(tokens).parse_program().unwrap();
    let output = std::env::temp_dir().join(format!("tyrus-range-contains-{}", std::process::id()));
    compile(&program, "range_contains", output.to_str().unwrap(), false).unwrap();
    let ir = std::fs::read_to_string(output.with_extension("ll")).unwrap();
    assert!(
        ir.contains("range.ge") && ir.contains("range.end_ok"),
        "expected the shared range bounds-check instructions:\n{ir}"
    );
    let _ = std::fs::remove_file(output.with_extension("ll"));
    let _ = std::fs::remove_file(output);
}

#[test]
fn not_in_negates_the_membership_check_like_a_plain_not() {
    let source = r#"
        function main() {
            const nums: i32[] = [1, 2, 3];
            console.log(5 not in nums);
        }
    "#;
    let tokens = tyrus_core::lexer::lex(source).unwrap();
    let program = Parser::new(tokens).parse_program().unwrap();
    let output = std::env::temp_dir().join(format!("tyrus-not-in-{}", std::process::id()));
    compile(&program, "not_in", output.to_str().unwrap(), false).unwrap();
    let ir = std::fs::read_to_string(output.with_extension("ll")).unwrap();
    assert!(
        ir.contains("call i64 @tyrus_array_index_of("),
        "expected `not in` to still dispatch through the array membership check:\n{ir}"
    );
    assert!(
        ir.contains("= xor i1"),
        "expected the shared boolean-negation codegen (same as a plain `!`):\n{ir}"
    );
    let _ = std::fs::remove_file(output.with_extension("ll"));
    let _ = std::fs::remove_file(output);
}

#[test]
fn structural_in_over_an_object_compares_field_names_by_length_and_bytes() {
    let source = r#"
        interface Point {
            x: i32,
            y: i32,
        }
        function main() {
            const p: Point = { x: 1, y: 2 };
            console.log("x" in p);
        }
    "#;
    let tokens = tyrus_core::lexer::lex(source).unwrap();
    let program = Parser::new(tokens).parse_program().unwrap();
    let output = std::env::temp_dir().join(format!("tyrus-in-object-{}", std::process::id()));
    compile(&program, "in_object", output.to_str().unwrap(), false).unwrap();
    let ir = std::fs::read_to_string(output.with_extension("ll")).unwrap();
    assert!(
        ir.contains("call i32 @memcmp("),
        "expected the structural field-name comparison:\n{ir}"
    );
    assert!(
        !ir.contains("@strcmp"),
        "`strcmp` walks to a terminator the value no longer needs:\n{ir}"
    );
    let _ = std::fs::remove_file(output.with_extension("ll"));
    let _ = std::fs::remove_file(output);
}

#[test]
fn closed_range_match_pattern_emits_signed_bounds_check() {
    let source = r#"
        function main() {
            const score: i32 = 85;
            match (score) {
                90..100 => { console.log("A"); }
                80..90  => { console.log("B"); }
                _       => { console.log("other"); }
            }
        }
    "#;
    let tokens = tyrus_core::lexer::lex(source).unwrap();
    let program = Parser::new(tokens).parse_program().unwrap();
    let output =
        std::env::temp_dir().join(format!("tyrus-match-range-closed-{}", std::process::id()));
    compile(
        &program,
        "match_range_closed",
        output.to_str().unwrap(),
        false,
    )
    .unwrap();
    let ir = std::fs::read_to_string(output.with_extension("ll")).unwrap();
    assert!(
        ir.contains("icmp sge") && ir.contains("icmp slt"),
        "expected a signed `>=` and `<` bounds check for `80..90`:\n{ir}"
    );
    let _ = std::fs::remove_file(output.with_extension("ll"));
    let _ = std::fs::remove_file(output);
}

#[test]
fn compact_match_arms_codegen_as_returns() {
    let source = r#"
        function classify(score: i32): string {
            match (score) {
                90..=100 => "A";
                _ => "F";
            }
        }
    "#;
    let program = Parser::from_source(source)
        .unwrap()
        .parse_program()
        .unwrap();
    let output = std::env::temp_dir().join(format!("tyrus-compact-match-{}", std::process::id()));
    compile(&program, "compact_match", output.to_str().unwrap(), false).unwrap();
    assert!(output.with_extension("ll").exists());
    let _ = std::fs::remove_file(output.with_extension("ll"));
    let _ = std::fs::remove_file(output);
}

#[test]
fn compact_match_arm_in_void_entry_point_returns_process_success() {
    let source = r#"
        const main = (value: boolean): void => {
            match (value) {
                true => { print("true"); }
                false => print("false");
            }
        };
    "#;
    let program = Parser::from_source(source)
        .unwrap()
        .parse_program()
        .unwrap();
    let output =
        std::env::temp_dir().join(format!("tyrus-compact-main-match-{}", std::process::id()));
    compile(
        &program,
        "compact_main_match",
        output.to_str().unwrap(),
        false,
    )
    .unwrap();
    assert!(output.with_extension("ll").exists());
    let _ = std::fs::remove_file(output.with_extension("ll"));
    let _ = std::fs::remove_file(output);
}

#[test]
fn optional_none_match_pattern_tests_the_box_and_unboxes_the_binding() {
    let source = r#"
        function main() {
            const absent: i32? = None;
            const present: i32? = 42;
            match (absent) {
                None => { console.log("absent"); }
                value => { console.log(value); }
            }
            match (present) {
                None => { console.log("missing"); }
                value => { console.log(value); }
            }
        }
    "#;
    let program = Parser::from_source(source)
        .unwrap()
        .parse_program()
        .unwrap();
    let output =
        std::env::temp_dir().join(format!("tyrus-match-optional-none-{}", std::process::id()));
    compile(
        &program,
        "match_optional_none",
        output.to_str().unwrap(),
        false,
    )
    .unwrap();
    let ir = std::fs::read_to_string(output.with_extension("ll")).unwrap();
    assert!(
        ir.contains("match.none.0"),
        "expected optional None check:\n{ir}"
    );
    assert!(
        ir.contains("icmp eq ptr"),
        "expected pointer presence comparison:\n{ir}"
    );
    assert!(
        ir.contains("match.binding.value.payload"),
        "expected payload unboxing for the binding:\n{ir}"
    );
    let _ = std::fs::remove_file(output.with_extension("ll"));
    let _ = std::fs::remove_file(output);
}

#[test]
fn inclusive_range_match_pattern_emits_a_le_upper_bound() {
    let source = r#"
        function main() {
            const score: i32 = 95;
            match (score) {
                90..=100 => { console.log("A"); }
                _ => { console.log("other"); }
            }
        }
    "#;
    let tokens = tyrus_core::lexer::lex(source).unwrap();
    let program = Parser::new(tokens).parse_program().unwrap();
    let output = std::env::temp_dir().join(format!(
        "tyrus-match-range-inclusive-{}",
        std::process::id()
    ));
    compile(
        &program,
        "match_range_inclusive",
        output.to_str().unwrap(),
        false,
    )
    .unwrap();
    let ir = std::fs::read_to_string(output.with_extension("ll")).unwrap();
    assert!(
        ir.contains("icmp sle"),
        "expected an inclusive `<=` upper bound for `90..=100`:\n{ir}"
    );
    let _ = std::fs::remove_file(output.with_extension("ll"));
    let _ = std::fs::remove_file(output);
}

#[test]
fn open_ended_range_match_patterns_emit_exactly_one_bound_check() {
    let source = r#"
        function main() {
            const age: u8 = 70;
            match (age) {
                0..18 => { console.log("minor"); }
                65..  => { console.log("senior"); }
                _     => { console.log("adult"); }
            }
        }
    "#;
    let tokens = tyrus_core::lexer::lex(source).unwrap();
    let program = Parser::new(tokens).parse_program().unwrap();
    let output =
        std::env::temp_dir().join(format!("tyrus-match-range-open-{}", std::process::id()));
    compile(
        &program,
        "match_range_open",
        output.to_str().unwrap(),
        false,
    )
    .unwrap();
    let ir = std::fs::read_to_string(output.with_extension("ll")).unwrap();
    assert!(
        ir.contains("icmp uge") && ir.contains("icmp ult"),
        "expected unsigned bounds checks for a `u8` subject:\n{ir}"
    );
    let _ = std::fs::remove_file(output.with_extension("ll"));
    let _ = std::fs::remove_file(output);
}

#[test]
fn slice_via_index_syntax_emits_the_same_slice_call_as_calling_slice_by_hand() {
    let via_index = r#"
        function main() {
            const nums: i32[] = [10, 20, 30, 40, 50];
            const middle: i32[] = nums[1..4];
            console.log(middle);
        }
    "#;
    let via_method = r#"
        function main() {
            const nums: i32[] = [10, 20, 30, 40, 50];
            const middle: i32[] = nums.slice(1, 4);
            console.log(middle);
        }
    "#;
    let slice_call_line = |source: &str, tag: &str| -> String {
        let tokens = tyrus_core::lexer::lex(source).unwrap();
        let program = Parser::new(tokens).parse_program().unwrap();
        let output = std::env::temp_dir().join(format!("tyrus-slice-{tag}-{}", std::process::id()));
        compile(&program, "slice_ir", output.to_str().unwrap(), false).unwrap();
        let ir = std::fs::read_to_string(output.with_extension("ll")).unwrap();
        let _ = std::fs::remove_file(output.with_extension("ll"));
        let _ = std::fs::remove_file(output);
        ir.lines()
            .find(|line| line.contains("call i32 @tyrus_array_slice("))
            .unwrap_or_else(|| panic!("expected a tyrus_array_slice call:\n{ir}"))
            .to_string()
    };
    let index_call = slice_call_line(via_index, "index");
    let method_call = slice_call_line(via_method, "method");
    assert_eq!(
        index_call, method_call,
        "arr[1..4] must emit the exact same tyrus_array_slice call as arr.slice(1, 4)"
    );
}

#[test]
fn split_at_emits_two_slice_calls_and_a_runtime_bounds_check() {
    let source = r#"
        function main() {
            const nums: i32[] = [1, 2, 3, 4, 5];
            let index: usize = 2;
            index = index + 1;
            const parts: [i32[], i32[]] = nums.splitAt(index);
            console.log(parts[0]);
            console.log(parts[1]);
        }
    "#;
    let tokens = tyrus_core::lexer::lex(source).unwrap();
    let program = Parser::new(tokens).parse_program().unwrap();
    let output = std::env::temp_dir().join(format!("tyrus-split-at-{}", std::process::id()));
    compile(&program, "split_at_ir", output.to_str().unwrap(), false).unwrap();
    let ir = std::fs::read_to_string(output.with_extension("ll")).unwrap();
    let slice_calls = ir.matches("call i32 @tyrus_array_slice(").count();
    assert!(
        slice_calls >= 2,
        "expected two tyrus_array_slice calls (one per half), found {slice_calls}:\n{ir}"
    );
    assert!(
        ir.contains("splitat_too_large"),
        "expected a dedicated splitAt bounds check block:\n{ir}"
    );
    let _ = std::fs::remove_file(output.with_extension("ll"));
    let _ = std::fs::remove_file(output);
}

#[test]
fn split_at_registers_its_tuple_result_as_owned() {
    let source = r#"
        function main() {
            const words: string[] = ["a", "b", "c"];
            console.log(words.splitAt(1));
        }
    "#;
    let tokens = tyrus_core::lexer::lex(source).unwrap();
    let program = Parser::new(tokens).parse_program().unwrap();
    let output = std::env::temp_dir().join(format!("tyrus-split-at-owned-{}", std::process::id()));
    compile(
        &program,
        "split_at_owned_ir",
        output.to_str().unwrap(),
        false,
    )
    .unwrap();
    let ir = std::fs::read_to_string(output.with_extension("ll")).unwrap();
    assert!(
        ir.contains("call void @free("),
        "expected the discarded splitAt result to be released:\n{ir}"
    );
    let _ = std::fs::remove_file(output.with_extension("ll"));
    let _ = std::fs::remove_file(output);
}

#[test]
fn binding_a_chained_allocating_method_call_frees_the_original_before_the_clone() {
    let source = r#"
        function main() {
            const words: string[] = ["a", "b", "c"];
            const nested = words.splitAt(1)[1].splitAt(1);
            console.log(nested[0]);
        }
    "#;
    let tokens = tyrus_core::lexer::lex(source).unwrap();
    let program = Parser::new(tokens).parse_program().unwrap();
    let output =
        std::env::temp_dir().join(format!("tyrus-chained-split-at-{}", std::process::id()));
    compile(
        &program,
        "chained_split_at_ir",
        output.to_str().unwrap(),
        false,
    )
    .unwrap();
    let ir = std::fs::read_to_string(output.with_extension("ll")).unwrap();
    let drops = ir
        .matches("call void @tyrus_array_drop_string_contents(")
        .count();
    assert!(
        drops >= 6,
        "expected three full tuple releases (6 field drops): the inner \
         call's left-over entry, the freed original, and the bound clone; \
         found {drops}:\n{ir}"
    );
    let _ = std::fs::remove_file(output.with_extension("ll"));
    let _ = std::fs::remove_file(output);
}

#[test]
fn destructuring_a_direct_calls_owned_tuple_does_not_leave_a_dangling_pending_result() {
    let source = r#"
        function pair(): [i32, string] {
            return [42, "erro"];
        }

        function main() {
            const [code, message] = pair();
            console.log(code);
            helper();
        }

        function helper(): void {
            console.log("hi");
        }
    "#;
    let tokens = tyrus_core::lexer::lex(source).unwrap();
    let program = Parser::new(tokens).parse_program().unwrap();
    let output = std::env::temp_dir().join(format!(
        "tyrus-tuple-destructure-dangling-pending-{}",
        std::process::id()
    ));
    compile(
        &program,
        "tuple_destructure_dangling_pending",
        output.to_str().unwrap(),
        false,
    )
    .unwrap();
    let ir = std::fs::read_to_string(output.with_extension("ll")).unwrap();
    assert!(
        ir.contains("@tyrus_string_share"),
        "expected the destructured string element to get its own reference:\n{ir}"
    );
    let _ = std::fs::remove_file(output.with_extension("ll"));
    let _ = std::fs::remove_file(output);
}

#[test]
fn object_literal_registers_a_boxed_optional_field_as_owned() {
    let source = r#"
        interface Widget {
            name: string;
            label?: string;
        }

        function main() {
            const w: Widget = { name: "Neo", label: "value" };
            console.log(w.name);
        }
    "#;
    let tokens = tyrus_core::lexer::lex(source).unwrap();
    let program = Parser::new(tokens).parse_program().unwrap();
    let output = std::env::temp_dir().join(format!(
        "tyrus-object-literal-boxed-optional-{}",
        std::process::id()
    ));
    compile(
        &program,
        "object_literal_boxed_optional",
        output.to_str().unwrap(),
        false,
    )
    .unwrap();
    let ir = std::fs::read_to_string(output.with_extension("ll")).unwrap();
    assert!(
        ir.contains("native.cleanup.field"),
        "expected the object's Fields release plan to extract and release the boxed field:\n{ir}"
    );
    let _ = std::fs::remove_file(output.with_extension("ll"));
    let _ = std::fs::remove_file(output);
}

#[test]
fn an_object_field_read_from_a_binding_is_cloned_rather_than_aliased() {
    // Storing a binding into an object field copies it, and the copy used to
    // be shallow: it carried the very pointers the binding had allocated and
    // still released at its own scope exit. Whoever freed the object walked
    // its type and freed them a second time, a double free rather than a leak,
    // reached by any `const a: T = { ... }` with an optional field stored into
    // another literal.
    //
    // The language settles that this is a copy, not a move: the borrow checker
    // refuses `const b: T = a;` and asks for `&a`, yet accepts `a` inside a
    // literal and leaves the binding usable afterwards. A copy of an owned
    // value has to be a deep one, so the field takes the same clone `return`
    // takes when it hands an owned value to a caller.
    let source = r#"
        interface Options {
            path?: string;
            secure?: boolean;
        }

        interface Cookie {
            name: string;
            options?: Options;
        }

        function main() {
            const options: Options = { path: "/", secure: true };
            const cookies: Cookie[] = [{ name: "theme", options: options }];
            console.log(cookies.length);
        }
    "#;
    let tokens = tyrus_core::lexer::lex(source).unwrap();
    let program = Parser::new(tokens).parse_program().unwrap();
    let output = std::env::temp_dir().join(format!(
        "tyrus-object-field-borrowed-place-{}",
        std::process::id()
    ));
    compile(
        &program,
        "object_field_borrowed_place",
        output.to_str().unwrap(),
        false,
    )
    .unwrap();
    let ir = std::fs::read_to_string(output.with_extension("ll")).unwrap();
    assert!(
        ir.contains("object.field.owned"),
        "expected the field read from a binding to be cloned:\n{ir}"
    );
    assert!(
        ir.contains("tyrus_string_share"),
        "expected the clone to reach the string nested in the optional field:\n{ir}"
    );
    let _ = std::fs::remove_file(output.with_extension("ll"));
    let _ = std::fs::remove_file(output);
}

#[test]
fn synchronous_owned_field_assignment_promotes_the_root_binding() {
    let source = r#"
        interface Record {
            label: string;
        }

        function main() {
            let row: Record = { label: "edge" };
            row.label = "written";
            console.log(row.label);
        }
    "#;
    let tokens = tyrus_core::lexer::lex(source).unwrap();
    let program = Parser::new(tokens).parse_program().unwrap();
    let output = std::env::temp_dir().join(format!(
        "tyrus-synchronous-owned-field-assignment-{}",
        std::process::id()
    ));
    compile(
        &program,
        "synchronous_owned_field_assignment",
        output.to_str().unwrap(),
        false,
    )
    .unwrap();
    let ir = std::fs::read_to_string(output.with_extension("ll")).unwrap();
    assert_eq!(
        ir.matches("call void @tyrus_string_release_buffer(")
            .count(),
        2,
        "expected releases for the replaced field and its owned replacement at scope exit:\n{ir}"
    );
    let _ = std::fs::remove_file(output.with_extension("ll"));
    let _ = std::fs::remove_file(output);
}

#[test]
fn object_update_registers_a_boxed_optional_field_as_owned() {
    let source = r#"
        interface Widget {
            name: string;
            label?: string;
        }

        function main() {
            const base: Widget = { name: "Ada" };
            const updated: Widget = { ...base, label: "value" };
            console.log(updated.name);
        }
    "#;
    let tokens = tyrus_core::lexer::lex(source).unwrap();
    let program = Parser::new(tokens).parse_program().unwrap();
    let output = std::env::temp_dir().join(format!(
        "tyrus-object-update-boxed-optional-{}",
        std::process::id()
    ));
    compile(
        &program,
        "object_update_boxed_optional",
        output.to_str().unwrap(),
        false,
    )
    .unwrap();
    let ir = std::fs::read_to_string(output.with_extension("ll")).unwrap();
    assert!(
        ir.contains("native.cleanup.field"),
        "expected the update's own Fields release plan to extract and release the boxed field:\n{ir}"
    );
    let _ = std::fs::remove_file(output.with_extension("ll"));
    let _ = std::fs::remove_file(output);
}

#[test]
fn object_update_with_an_already_allocating_optional_override_leaves_the_general_fallback_to_claim_it()
 {
    // `{ ...base, label: "value".toUpperCase() }`: the override's own value
    // is already a pending owned result (the raw string `.toUpperCase()`
    // registered) *before* it gets boxed into `label`'s `string?` storage.
    //
    // `ReleasePlan::Box` (introduced closing the `optionals/examples.tyr`
    // dominance crash, an object literal with an owned optional field,
    // boxed for a `return` inside one arm of an `if` whose other arm bare-
    // returns, left an orphaned pending entry nothing downstream could ever
    // match by identity again) changed *how* this case gets claimed: the
    // generic `(_, Optional(inner))` boxing arm now takes the raw string's
    // own pending entry and re-registers it against the box itself before
    // this update's own field processing ever sees it, so `boxed_here`'s
    // gate (`pending_owned_native_results.is_empty()`) now finds the box's
    // *own* new entry already there and correctly folds it into this
    // update's `Fields` plan directly, the same way every other override
    // is claimed. The recursive, expression-shape-driven fallback this test
    // originally guarded (`emit_async_cleanup_source_expression`'s
    // `Type::Named`/`Type::Object` arm reaching an `ObjectUpdate` override)
    // is no longer what frees this specific shape, confirmed clean with
    // valgrind reporting zero heap blocks in use at exit across 200
    // iterations before and after this change, so this now asserts the more
    // direct path fires instead: the field is released through `Fields`,
    // recursing into `ReleasePlan::Box`'s own release block.
    let source = r#"
        interface Widget {
            name: string;
            label?: string;
        }

        function main() {
            const base: Widget = { name: "Ada" };
            const updated: Widget = { ...base, label: "value".toUpperCase() };
            console.log(updated.name);
        }
    "#;
    let tokens = tyrus_core::lexer::lex(source).unwrap();
    let program = Parser::new(tokens).parse_program().unwrap();
    let output = std::env::temp_dir().join(format!(
        "tyrus-object-update-boxed-allocating-{}",
        std::process::id()
    ));
    compile(
        &program,
        "object_update_boxed_allocating",
        output.to_str().unwrap(),
        false,
    )
    .unwrap();
    let ir = std::fs::read_to_string(output.with_extension("ll")).unwrap();
    assert!(
        ir.contains("native.cleanup.field") && ir.contains("native.cleanup.box.inner"),
        "expected the override to be released through Fields, recursing into ReleasePlan::Box:\n{ir}"
    );
    let _ = std::fs::remove_file(output.with_extension("ll"));
    let _ = std::fs::remove_file(output);
}

#[test]
fn replaces_repeated_owned_await_bindings_before_the_next_loop_iteration() {
    let source = r#"
        async function source(value: string): Promise<string> {
            return value + "!";
        }

        async function main(): Promise<void> {
            for (let index: i32 = 0; index < 2; index += 1) {
                const value: string = await source("value");
                console.log(value);
            }
        }
    "#;
    let tokens = tyrus_core::lexer::lex(source).unwrap();
    let program = Parser::new(tokens).parse_program().unwrap();
    let output = std::env::temp_dir().join(format!(
        "tyrus-repeated-owned-await-binding-{}",
        std::process::id()
    ));
    compile(
        &program,
        "repeated_owned_await_binding",
        output.to_str().unwrap(),
        false,
    )
    .unwrap();
    let ir = std::fs::read_to_string(output.with_extension("ll")).unwrap();
    assert!(ir.contains("async.local.initialized"));
    assert!(ir.contains("async.local.drop"));
    assert!(ir.contains("async.payload.drop.string"));
    assert!(ir.contains("@tyrus_string_release_buffer"));
    let _ = std::fs::remove_file(output.with_extension("ll"));
    let _ = std::fs::remove_file(output);
}

#[test]
fn emits_ordered_aggregate_error_binding_for_promise_any() {
    let source = r#"
        async function main(): Promise<void> {
            try {
                const first: Error = { message: "first", code: "ERR_FIRST" };
                const second: Error = { message: "second", code: "ERR_SECOND" };
                await Promise.any([Promise.reject(first), Promise.reject(second)]);
            } catch (error: AggregateError) {
                console.log(error.message);
                console.log(error.errors.length);
            }
        }
    "#;
    let tokens = tyrus_core::lexer::lex(source).unwrap();
    let program = Parser::new(tokens).parse_program().unwrap();
    let output = std::env::temp_dir().join(format!("tyrus-aggregate-error-{}", std::process::id()));
    compile(&program, "aggregate_error", output.to_str().unwrap(), false).unwrap();
    let ir = std::fs::read_to_string(output.with_extension("ll")).unwrap();
    assert!(ir.contains("tyrus_promise_any"));
    assert!(ir.contains("tyrus_promise_clone_aggregate_error"));
    assert!(ir.contains("tyrus_drop_async_aggregate_error"));
    let _ = std::fs::remove_file(output.with_extension("ll"));
    let _ = std::fs::remove_file(output);
}

#[test]
fn persists_and_drops_recursive_owned_locals_across_awaits() {
    let source = r#"
        interface User {
            name: string;
        }

        async function pause(): Promise<void> {}

        async function main(): Promise<void> {
            const users: User[] = [{ name: "Ada" }, { name: "Grace" }];
            await pause();
            console.log(users.length);
        }
    "#;
    let tokens = tyrus_core::lexer::lex(source).unwrap();
    let program = Parser::new(tokens).parse_program().unwrap();
    let output =
        std::env::temp_dir().join(format!("tyrus-async-owned-local-{}", std::process::id()));
    compile(
        &program,
        "async_owned_local",
        output.to_str().unwrap(),
        false,
    )
    .unwrap();
    let ir = std::fs::read_to_string(output.with_extension("ll")).unwrap();
    assert!(ir.contains("async.frame.local.users"));
    assert!(ir.contains("tyrus_array_clone_owned"));
    assert!(ir.contains("tyrus_array_drop_owned_contents"));
    let _ = std::fs::remove_file(output.with_extension("ll"));
    let _ = std::fs::remove_file(output);
}

#[test]
fn lowers_copy_awaits_inside_initializers_arguments_and_returns() {
    let source = r#"
        async function value(): Promise<i32> {
            return 20;
        }

        function add(left: i32, right: i32): i32 {
            return left + right;
        }

        async function total(): Promise<i32> {
            const doubled: i32 = (await value()) * 2;
            console.log(add(doubled, await value()));
            return add(await value(), 2);
        }
    "#;
    let tokens = tyrus_core::lexer::lex(source).unwrap();
    let program = Parser::new(tokens).parse_program().unwrap();
    let output = std::env::temp_dir().join(format!(
        "tyrus-async-expression-await-{}",
        std::process::id()
    ));
    compile(
        &program,
        "async_expression_await",
        output.to_str().unwrap(),
        false,
    )
    .unwrap();
    let ir = std::fs::read_to_string(output.with_extension("ll")).unwrap();
    assert!(ir.contains("async.frame.await.value"));
    assert!(ir.contains("async.continuation.value"));
    assert!(ir.contains("async.state.3"));
    let _ = std::fs::remove_file(output.with_extension("ll"));
    let _ = std::fs::remove_file(output);
}

#[test]
fn lowers_multiple_copy_awaits_in_one_linear_expression() {
    let source = r#"
        async function value(input: i32): Promise<i32> {
            return input;
        }

        async function nested(input: i32): Promise<i32> {
            return input;
        }

        async function main(): Promise<void> {
            const total: i32 = (await value(20)) + (await value(22));
            const result: i32 = await nested(await value(total));
            console.log(result);
        }
    "#;
    let tokens = tyrus_core::lexer::lex(source).unwrap();
    let program = Parser::new(tokens).parse_program().unwrap();
    let output = std::env::temp_dir().join(format!(
        "tyrus-async-multiple-expression-awaits-{}",
        std::process::id()
    ));
    compile(
        &program,
        "async_multiple_expression_awaits",
        output.to_str().unwrap(),
        false,
    )
    .unwrap();
    let ir = std::fs::read_to_string(output.with_extension("ll")).unwrap();
    assert!(ir.contains("async.state.4"));
    assert!(ir.matches("async.frame.await.value").count() >= 4);
    let _ = std::fs::remove_file(output.with_extension("ll"));
    let _ = std::fs::remove_file(output);
}

#[test]
fn preserves_short_circuit_and_ternary_guards_around_await() {
    let source = r#"
        async function flag(): Promise<boolean> { return true; }
        async function value(): Promise<i32> { return 42; }
        async function fallback(input?: i32): Promise<i32> {
            return input ?? await value();
        }

        async function main(): Promise<void> {
            const andValue: boolean = false && await flag();
            const orValue: boolean = true || await flag();
            const thenValue: i32 = true ? await value() : 0;
            const elseValue: i32 = true ? 0 : await value();
            const fallbackValue: i32 = await fallback();
            const presentValue: i32 = await fallback(7);
            console.log(andValue, orValue, thenValue, elseValue, fallbackValue, presentValue);
        }
    "#;
    let tokens = tyrus_core::lexer::lex(source).unwrap();
    let program = Parser::new(tokens).parse_program().unwrap();
    let output = std::env::temp_dir().join(format!(
        "tyrus-async-expression-guards-{}",
        std::process::id()
    ));
    compile(
        &program,
        "async_expression_guards",
        output.to_str().unwrap(),
        false,
    )
    .unwrap();
    let ir = std::fs::read_to_string(output.with_extension("ll")).unwrap();
    assert!(ir.contains("async.expression.guard.schedule"));
    assert!(ir.contains("async.expression.guard.bypass"));
    assert!(ir.contains("async.expression.guard.after"));
    let _ = std::fs::remove_file(output.with_extension("ll"));
    let _ = std::fs::remove_file(output);
}

#[test]
fn lowers_repeated_awaits_in_a_while_condition_with_break_and_continue_targets() {
    let source = r#"
        async function belowLimit(value: i32): Promise<boolean> {
            return value < 3;
        }

        async function main(): Promise<void> {
            let count: i32 = 0;
            while (await belowLimit(count)) {
                count += 1;
                if (count === 1) {
                    continue;
                }
                if (count > 10) {
                    break;
                }
            }
            console.log(count);
        }
    "#;
    let tokens = tyrus_core::lexer::lex(source).unwrap();
    let program = Parser::new(tokens).parse_program().unwrap();
    let output =
        std::env::temp_dir().join(format!("tyrus-async-while-await-{}", std::process::id()));
    compile(
        &program,
        "async_while_await",
        output.to_str().unwrap(),
        false,
    )
    .unwrap();
    let ir = std::fs::read_to_string(output.with_extension("ll")).unwrap();
    assert!(ir.contains("async.while.body"));
    assert!(ir.contains("async.while.schedule"));
    assert!(ir.contains("async.while.after"));
    assert!(ir.contains("call ptr @belowLimit"));
    let _ = std::fs::remove_file(output.with_extension("ll"));
    let _ = std::fs::remove_file(output);
}

#[test]
fn lowers_repeated_awaits_in_a_do_while_condition_after_the_first_body() {
    let source = r#"
        async function belowLimit(value: i32): Promise<boolean> {
            return value < 3;
        }

        async function main(): Promise<void> {
            let count: i32 = 0;
            do {
                count += 1;
                if (count === 1) {
                    continue;
                }
                if (count > 10) {
                    break;
                }
            } while (await belowLimit(count));
            console.log(count);
        }
    "#;
    let tokens = tyrus_core::lexer::lex(source).unwrap();
    let program = Parser::new(tokens).parse_program().unwrap();
    let output =
        std::env::temp_dir().join(format!("tyrus-async-do-while-await-{}", std::process::id()));
    compile(
        &program,
        "async_do_while_await",
        output.to_str().unwrap(),
        false,
    )
    .unwrap();
    let ir = std::fs::read_to_string(output.with_extension("ll")).unwrap();
    assert!(ir.contains("async.do_while.body"));
    assert!(ir.contains("async.do_while.schedule"));
    assert!(ir.contains("async.do_while.after"));
    assert!(ir.contains("call ptr @belowLimit"));
    let _ = std::fs::remove_file(output.with_extension("ll"));
    let _ = std::fs::remove_file(output);
}

#[test]
fn lowers_repeated_awaits_in_a_for_condition_with_ordered_update() {
    let source = r#"
        async function belowLimit(value: i32): Promise<boolean> {
            return value < 3;
        }

        async function main(): Promise<void> {
            let total: i32 = 0;
            for (let index: i32 = 0; await belowLimit(index); index += 1) {
                if (index === 0) {
                    continue;
                }
                total += index;
                if (total > 10) {
                    break;
                }
            }
            console.log(total);
        }
    "#;
    let tokens = tyrus_core::lexer::lex(source).unwrap();
    let program = Parser::new(tokens).parse_program().unwrap();
    let output = std::env::temp_dir().join(format!("tyrus-async-for-await-{}", std::process::id()));
    compile(&program, "async_for_await", output.to_str().unwrap(), false).unwrap();
    let ir = std::fs::read_to_string(output.with_extension("ll")).unwrap();
    assert!(ir.contains("async.for.body"));
    assert!(ir.contains("async.for.update"));
    assert!(ir.contains("async.for.schedule"));
    assert!(ir.contains("async.for.after"));
    assert!(ir.contains("async.frame.local.index"));
    let _ = std::fs::remove_file(output.with_extension("ll"));
    let _ = std::fs::remove_file(output);
}

#[test]
fn lowers_a_direct_await_inside_the_selected_if_branch() {
    let source = r#"
        async function pause(): Promise<void> {}
        async function label(): Promise<string> { return "ready"; }

        async function main(): Promise<void> {
            const enabled: boolean = true;
            if (enabled) {
                const nested: string = "nested".toUpperCase();
                console.log("before");
                await pause();
                console.log(nested);
                const resumedLabel: string = await label();
                console.log(resumedLabel);
                console.log("after");
            } else {
                console.log("disabled");
            }
            console.log("joined");
        }
    "#;
    let tokens = tyrus_core::lexer::lex(source).unwrap();
    let program = Parser::new(tokens).parse_program().unwrap();
    let output = std::env::temp_dir().join(format!(
        "tyrus-async-if-branch-await-{}",
        std::process::id()
    ));
    compile(
        &program,
        "async_if_branch_await",
        output.to_str().unwrap(),
        false,
    )
    .unwrap();
    let ir = std::fs::read_to_string(output.with_extension("ll")).unwrap();
    assert!(ir.contains("async.if.selected"));
    assert!(ir.contains("async.if.alternative"));
    assert!(ir.contains("async.if.after"));
    assert!(ir.contains("async.frame.local.nested"));
    assert!(ir.contains("async.frame.local.resumedLabel"));
    let _ = std::fs::remove_file(output.with_extension("ll"));
    let _ = std::fs::remove_file(output);
}

#[test]
fn lowers_multiple_awaits_inside_the_selected_if_branch() {
    let source = r#"
        async function pause(): Promise<void> {}

        async function main(): Promise<void> {
            const enabled: boolean = true;
            if (enabled) {
                console.log("before");
                await pause();
                console.log("between");
                await pause();
                console.log("after");
            } else {
                console.log("disabled");
            }
            console.log("joined");
        }
    "#;
    let tokens = tyrus_core::lexer::lex(source).unwrap();
    let program = Parser::new(tokens).parse_program().unwrap();
    let output = std::env::temp_dir().join(format!(
        "tyrus-async-if-branch-multiple-awaits-{}",
        std::process::id()
    ));
    compile(
        &program,
        "async_if_branch_multiple_awaits",
        output.to_str().unwrap(),
        false,
    )
    .unwrap();
    let ir = std::fs::read_to_string(output.with_extension("ll")).unwrap();
    assert!(ir.contains("i32 2"));
    assert!(ir.contains("async.if.after"));
    let _ = std::fs::remove_file(output.with_extension("ll"));
    let _ = std::fs::remove_file(output);
}

#[test]
fn lowers_a_repeated_await_inside_a_while_body() {
    let source = r#"
        async function pause(): Promise<void> {}
        async function step(): Promise<i32> { return 1; }

        async function main(): Promise<void> {
            let count: i32 = 0;
            while (count < 4) {
                const iterationLabel: string = "loop".toUpperCase();
                await pause();
                const increment: i32 = await step();
                count += increment;
                await pause();
                console.log(iterationLabel);
                if (count === 2) {
                    continue;
                }
                if (count === 3) {
                    break;
                }
            }
            console.log(count);
        }
    "#;
    let tokens = tyrus_core::lexer::lex(source).unwrap();
    let program = Parser::new(tokens).parse_program().unwrap();
    let output = std::env::temp_dir().join(format!(
        "tyrus-async-while-body-await-{}",
        std::process::id()
    ));
    compile(
        &program,
        "async_while_body_await",
        output.to_str().unwrap(),
        false,
    )
    .unwrap();
    let ir = std::fs::read_to_string(output.with_extension("ll")).unwrap();
    assert!(ir.contains("async.while_body.condition"));
    assert!(ir.contains("async.while_body.selected"));
    assert!(ir.contains("async.while_body.after"));
    assert!(ir.contains("async.local.previous.string"));
    assert!(ir.contains("async.frame.local.increment"));
    let _ = std::fs::remove_file(output.with_extension("ll"));
    let _ = std::fs::remove_file(output);
}

#[test]
fn lowers_a_repeated_await_inside_a_do_while_body() {
    let source = r#"
        async function pause(): Promise<void> {}

        async function main(): Promise<void> {
            let count: i32 = 0;
            do {
                await pause();
                count += 1;
                await pause();
            } while (count < 3);
            console.log(count);
        }
    "#;
    let tokens = tyrus_core::lexer::lex(source).unwrap();
    let program = Parser::new(tokens).parse_program().unwrap();
    let output = std::env::temp_dir().join(format!(
        "tyrus-async-do-while-body-await-{}",
        std::process::id()
    ));
    compile(
        &program,
        "async_do_while_body_await",
        output.to_str().unwrap(),
        false,
    )
    .unwrap();
    let ir = std::fs::read_to_string(output.with_extension("ll")).unwrap();
    assert!(ir.contains("async.do_while_body.selected"));
    assert!(ir.contains("async.do_while_body.condition"));
    assert!(ir.contains("async.do_while_body.after"));
    let _ = std::fs::remove_file(output.with_extension("ll"));
    let _ = std::fs::remove_file(output);
}

#[test]
fn lowers_a_repeated_await_inside_a_for_body() {
    let source = r#"
        async function pause(): Promise<void> {}

        async function main(): Promise<void> {
            let total: i32 = 0;
            for (let index: i32 = 0; index < 4; index += 1) {
                await pause();
                if (index === 1) {
                    continue;
                }
                await pause();
                total += index;
                if (total > 2) {
                    break;
                }
            }
            console.log(total);
        }
    "#;
    let tokens = tyrus_core::lexer::lex(source).unwrap();
    let program = Parser::new(tokens).parse_program().unwrap();
    let output =
        std::env::temp_dir().join(format!("tyrus-async-for-body-await-{}", std::process::id()));
    compile(
        &program,
        "async_for_body_await",
        output.to_str().unwrap(),
        false,
    )
    .unwrap();
    let ir = std::fs::read_to_string(output.with_extension("ll")).unwrap();
    assert!(ir.contains("async.for_body.selected"));
    assert!(ir.contains("async.for_body.update"));
    assert!(ir.contains("async.for_body.condition"));
    assert!(ir.contains("async.for_body.after"));
    let _ = std::fs::remove_file(output.with_extension("ll"));
    let _ = std::fs::remove_file(output);
}

#[test]
fn lowers_a_repeated_await_inside_a_for_of_body() {
    let source = r#"
        async function pause(): Promise<void> {}

        async function main(): Promise<void> {
            const values: i32[] = [1, 2, 3];
            let total: i32 = 0;
            for (const value of values) {
                await pause();
                total += value;
                await pause();
            }
            console.log(total);
        }
    "#;
    let tokens = tyrus_core::lexer::lex(source).unwrap();
    let program = Parser::new(tokens).parse_program().unwrap();
    let output = std::env::temp_dir().join(format!(
        "tyrus-async-for-of-body-await-{}",
        std::process::id()
    ));
    compile(
        &program,
        "async_for_of_body_await",
        output.to_str().unwrap(),
        false,
    )
    .unwrap();
    let ir = std::fs::read_to_string(output.with_extension("ll")).unwrap();
    assert!(ir.contains("async.for_of.condition"));
    assert!(ir.contains("async.for_of.selected"));
    assert!(ir.contains("async.for_of.update"));
    assert!(ir.contains("async.for_of.after"));
    let _ = std::fs::remove_file(output.with_extension("ll"));
    let _ = std::fs::remove_file(output);
}

#[test]
fn emits_the_complete_native_promise_library_surface() {
    let source = r#"
        function capabilityPromise(): Promise<i32> {
            const capability = Promise.withResolvers<i32>();
            capability.resolve(42);
            return capability.promise;
        }

        async function main(): Promise<void> {
            await Promise.resolve(Promise.resolve(42));
            await Promise.resolve().then((): void => {});
            await Promise.all<i32>([Promise.resolve(20), Promise.resolve(22)]).then((values: i32[]): void => {
                console.log(values.length);
            });
            await Promise.race<i32>([Promise.resolve(42), Promise.resolve(7)]);
            await Promise.any<i32>([Promise.reject<i32>({ message: "skip", code: "ERR_SKIP" }), Promise.resolve(42)]);
            await Promise.allSettled<i32>([Promise.resolve(42), Promise.reject<i32>({ message: "expected", code: "ERR_EXPECTED" })]).then((results): void => {
                console.log(results.length);
            });
            await Promise.resolve(21).then((value: i32): i32 => value * 2);
            await Promise.resolve(42).then((value: i32): Promise<i32> => Promise.resolve(value));
            await Promise.reject<i32>({ message: "boom", code: "ERR_BOOM" }).catch((error: Error): i32 => 42);
            await Promise.resolve(42).finally((): void => {});
            await capabilityPromise();
        }
    "#;
    let tokens = tyrus_core::lexer::lex(source).unwrap();
    let program = Parser::new(tokens).parse_program().unwrap();
    let output = std::env::temp_dir().join(format!("tyrus-promise-library-{}", std::process::id()));
    compile(&program, "promise_library", output.to_str().unwrap(), false).unwrap();
    let ir = std::fs::read_to_string(output.with_extension("ll")).unwrap();
    for symbol in [
        "tyrus_promise_resolve",
        "tyrus_promise_reject",
        "tyrus_promise_all_typed",
        "tyrus_promise_race",
        "tyrus_promise_all_settled_typed",
        "tyrus_promise_any",
        "tyrus_promise_with_resolvers",
        "tyrus_promise_then",
        "tyrus_promise_catch",
        "tyrus_promise_finally",
    ] {
        assert!(ir.contains(symbol), "missing `{symbol}` in generated IR");
    }
    assert!(ir.contains("__tyrus_drop_settled_array_"));
    assert!(ir.contains("promise.resolve.flatten"));
    let _ = std::fs::remove_file(output.with_extension("ll"));
    let _ = std::fs::remove_file(output);
}

#[test]
fn emits_nonblocking_tasks_sleep_as_an_awaitable_promise() {
    let source = r#"
        async function main(): Promise<void> {
            await __tyrus_task_sleepAsync(1);
        }
    "#;
    let tokens = tyrus_core::lexer::lex(source).unwrap();
    let program = Parser::new(tokens).parse_program().unwrap();
    let output = std::env::temp_dir().join(format!("tyrus-tasks-sleep-{}", std::process::id()));
    compile(&program, "tasks_sleep", output.to_str().unwrap(), false).unwrap();
    let ir = std::fs::read_to_string(output.with_extension("ll")).unwrap();
    assert!(ir.contains("call ptr @tyrus_task_sleep_async(i64 1)"));
    assert!(ir.contains("tyrus_async_task_await"));
    let _ = std::fs::remove_file(output.with_extension("ll"));
    let _ = std::fs::remove_file(output);
}

#[test]
fn emits_public_cooperative_task_cancellation() {
    let source = r#"
        async function main(): Promise<void> {
            const operation: Promise<void> = __tyrus_task_sleepAsync(1000);
            const cancelled: boolean = __tyrus_task_cancel(operation, "Cancelled by caller");
            console.log(cancelled);
            try {
                await operation;
            } catch (error: Error) {
                console.log(error.message);
            }
        }
    "#;
    let tokens = tyrus_core::lexer::lex(source).unwrap();
    let program = Parser::new(tokens).parse_program().unwrap();
    let output = std::env::temp_dir().join(format!("tyrus-tasks-cancel-{}", std::process::id()));
    compile(&program, "tasks_cancel", output.to_str().unwrap(), false).unwrap();
    let ir = std::fs::read_to_string(output.with_extension("ll")).unwrap();
    assert!(ir.contains("call i32 @tyrus_task_cancel"));
    assert!(ir.contains("async.error.message.clone"));
    let _ = std::fs::remove_file(output.with_extension("ll"));
    let _ = std::fs::remove_file(output);
}

#[test]
fn stores_and_releases_owned_string_local_from_native_crypto_call_in_async_fn() {
    let source = r#"
        import { CryptoError, jwt } from "tyrus:crypto";


function cryptoString(result: Result<string, CryptoError>): string {
    match (result) {
        Ok(value) => { return value; }
        Err(error) => { printError(error.code); return ""; }
    }
}

        interface SessionClaims {
            sub: string;
        }

        async function load(): Promise<void> {}

        async function main(): Promise<void> {
            await load();
            try {
                const secret: string = "0123456789abcdef0123456789abcdef";
                const payload: SessionClaims = { sub: "user-42" };
                const token: string = cryptoString(jwt.sign(payload, secret, {}));
                console.log(token);
            } catch (error: Error) {
                console.error(error.message);
            }
        }
    "#;
    let root = std::env::temp_dir().join(format!(
        "tyrus-jwt-async-frame-source-{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).unwrap();
    std::fs::write(root.join("main.tyr"), source).unwrap();
    let program = tyrus_core::module_loader::load_program(&root.join("main.tyr"), None).unwrap();
    let output = std::env::temp_dir().join(format!("tyrus-jwt-async-frame-{}", std::process::id()));
    compile(&program, "jwt_async_frame", output.to_str().unwrap(), false).unwrap();
    let ir = std::fs::read_to_string(output.with_extension("ll")).unwrap();
    assert!(ir.contains("tyrus_crypto_jwt_sign_result"));
    assert!(ir.contains("async.frame.local.token"));
    assert!(
        ir.contains("@tyrus_string_release_buffer"),
        "an async frame local's string must be released through its ownership bit"
    );
    let _ = std::fs::remove_file(output.with_extension("ll"));
    let _ = std::fs::remove_file(output);
    let _ = std::fs::remove_dir_all(root);
}

// An aggregate is *cloned* into the async frame (`strdup` per string field),
// not moved like a plain string local, so the source must be released right
// after the clone. `jwt.verify` left no pending ownership registration to
// identity-match, inside an async function `record_owned_native_result` is a
// deliberate no-op, and no shape predicate recognized it, so both claim
// strings from `tyrus_json_document_get_string` leaked (confirmed under
// Valgrind: 22 bytes in 2 blocks, gone after the fix).
#[test]
fn releases_cloned_claims_source_from_native_crypto_call_in_async_fn() {
    let source = r#"
        import { CryptoError, jwt } from "tyrus:crypto";


function cryptoString(result: Result<string, CryptoError>): string {
    match (result) {
        Ok(value) => { return value; }
        Err(error) => { printError(error.code); return ""; }
    }
}

        interface SessionClaims {
            role: string;
            sub: string;
        }

        async function load(): Promise<void> {}

        async function main(): Promise<void> {
            await load();
            try {
                const secret: string = "0123456789abcdef0123456789abcdef";
                const payload: SessionClaims = { role: "administrator", sub: "user-42" };
                const token: string = cryptoString(jwt.sign(payload, secret, {}));
                const claimsResult: Result<SessionClaims, CryptoError> = jwt.verify<SessionClaims>(token, secret, {});
                match (claimsResult) {
                    Ok(claims) => { console.log(claims.sub); }
                    Err(error) => { console.log(error.code); }
                }
            } catch (error: Error) {
                console.error(error.message);
            }
        }
    "#;
    let root = std::env::temp_dir().join(format!(
        "tyrus-jwt-claims-clone-source-{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).unwrap();
    std::fs::write(root.join("main.tyr"), source).unwrap();
    let program = tyrus_core::module_loader::load_program(&root.join("main.tyr"), None).unwrap();
    let output =
        std::env::temp_dir().join(format!("tyrus-jwt-claims-clone-{}", std::process::id()));
    compile(
        &program,
        "jwt_claims_clone",
        output.to_str().unwrap(),
        false,
    )
    .unwrap();
    let ir = std::fs::read_to_string(output.with_extension("ll")).unwrap();
    assert!(ir.contains("tyrus_crypto_jwt_verify_result"));
    assert!(ir.contains("async.payload.string.clone"));
    assert!(
        ir.contains("native.cleanup.owned"),
        "cloned claims source is never released; its string fields leak"
    );
    let _ = std::fs::remove_file(output.with_extension("ll"));
    let _ = std::fs::remove_file(output);
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn stores_and_mutates_copy_object_locals_across_awaits() {
    let source = r#"
        interface Cursor {
            row: i32;
            column: i32;
            visible: boolean;
        }
        async function load(): Promise<void> {}
        async function main(): Promise<void> {
            let cursor: Cursor = { row: 1, column: 2, visible: true };
            await load();
            cursor.row = 3;
            cursor = { row: 4, column: 5, visible: false };
            await load();
            console.log(cursor.row);
        }
    "#;
    let tokens = tyrus_core::lexer::lex(source).unwrap();
    let program = Parser::new(tokens).parse_program().unwrap();
    let output = std::env::temp_dir().join(format!(
        "tyrus-async-mutable-copy-object-local-{}",
        std::process::id()
    ));
    compile(
        &program,
        "async_mutable_copy_object_local",
        output.to_str().unwrap(),
        false,
    )
    .unwrap();
    let ir = std::fs::read_to_string(output.with_extension("ll")).unwrap();
    assert!(ir.contains("async.frame.local.cursor"));
    assert!(ir.contains("%row = getelementptr"));
    let _ = std::fs::remove_file(output.with_extension("ll"));
    let _ = std::fs::remove_file(output);
}

#[test]
fn deeply_replaces_owned_object_locals_across_awaits() {
    let source = r#"
        interface Snapshot {
            count: i32;
            label: string;
            tags: string[];
            note?: string;
        }
        async function load(): Promise<void> {}
        async function main(): Promise<void> {
            let snapshot: Snapshot = { count: 1, label: "first", tags: ["static"] };
            await load();
            snapshot.count = 2;
            snapshot = {
                count: 3,
                label: "second".toUpperCase(),
                tags: "owned,array".split(","),
                note: "present".toUpperCase(),
            };
            await load();
            console.log(snapshot.label);
            console.log(snapshot.tags[0]);
            console.log(snapshot.note ?? "missing");
        }
    "#;
    let tokens = tyrus_core::lexer::lex(source).unwrap();
    let program = Parser::new(tokens).parse_program().unwrap();
    let output = std::env::temp_dir().join(format!(
        "tyrus-async-mutable-owned-object-local-{}",
        std::process::id()
    ));
    compile(
        &program,
        "async_mutable_owned_object_local",
        output.to_str().unwrap(),
        false,
    )
    .unwrap();
    let ir = std::fs::read_to_string(output.with_extension("ll")).unwrap();
    assert!(ir.contains("async.frame.local.snapshot"));
    assert!(ir.contains("async.drop.snapshot.slot"));
    assert!(ir.contains("async.payload.label.slot"));
    assert!(ir.contains("@tyrus_array_clone_strings"));
    let _ = std::fs::remove_file(output.with_extension("ll"));
    let _ = std::fs::remove_file(output);
}

#[test]
fn accepts_native_promise_bindings_during_codegen() {
    let source = "const pending: Promise<i32> = Promise.resolve(1);";
    let tokens = tyrus_core::lexer::lex(source).unwrap();
    let program = Parser::new(tokens).parse_program().unwrap();
    let output = std::env::temp_dir().join(format!("tyrus-promise-binding-{}", std::process::id()));
    compile(&program, "promise_binding", output.to_str().unwrap(), false).unwrap();
    let ir = std::fs::read_to_string(output.with_extension("ll")).unwrap();
    assert!(ir.contains("tyrus_promise_resolve"));
    let _ = std::fs::remove_file(output.with_extension("ll"));
    let _ = std::fs::remove_file(output);
}

#[test]
fn rejects_await_expression_before_llvm_codegen() {
    let error = compile_error(
        r#"
            const main = (): void => {
                const value = await ready;
            };
        "#,
    );
    assert!(error.contains("await codegen is not implemented yet"));
}

#[test]
fn emits_context_typed_numeric_literals() {
    let source = r#"
        const calculate = (big: i64, small: f32): void => {
            const next: i64 = big + 2;
            const ratio: f32 = small + 2.5;
            console.log(next);
            console.log(ratio);
        };
    "#;
    let tokens = tyrus_core::lexer::lex(source).unwrap();
    let program = Parser::new(tokens).parse_program().unwrap();
    let output = std::env::temp_dir().join(format!("tyrus-typed-numeric-{}", std::process::id()));
    compile(&program, "typed_numeric", output.to_str().unwrap(), false).unwrap();
    let ir_path = output.with_extension("ll");
    let ir = std::fs::read_to_string(&ir_path).unwrap();
    assert!(ir.contains("add i64"));
    assert!(ir.contains("fadd float"));
    std::fs::remove_file(ir_path).unwrap();
}

#[test]
fn emits_reassoc_only_for_explicit_algebraic_float_methods() {
    let ir = compile_to_ir(
        r#"
            const calculate = (a: f64, b: f64, c: f32, d: f32): void => {
                const sum: f64 = a.algebraicAdd(b);
                const difference: f64 = a.algebraicSub(b);
                const product: f32 = c.algebraicMul(d);
                const quotient: f64 = a.algebraicDiv(b);
                const remainder: f64 = a.algebraicRem(b);
                const nan: f64 = (0.0 / 0.0).algebraicAdd(a);
                const infinity: f64 = (1.0 / 0.0).algebraicSub(b);
                const signedZero: f64 = (-0.0).algebraicRem(1.0);
                const strict: f64 = a + b;
                console.log(sum, difference, product, quotient, remainder, nan, infinity, signedZero, strict);
            };
        "#,
        "algebraic_float_methods",
    );
    assert!(ir.contains("fadd reassoc double"), "{ir}");
    assert!(ir.contains("fsub reassoc double"), "{ir}");
    assert!(ir.contains("fmul reassoc float"), "{ir}");
    assert!(ir.contains("fdiv reassoc double"), "{ir}");
    assert!(ir.contains("frem reassoc double"), "{ir}");
    assert!(ir.contains("fadd double"), "{ir}");
    assert!(!ir.contains("fast fadd"), "{ir}");
}

#[test]
fn omits_absent_optional_fields_when_printing_a_struct() {
    let source = r#"
        interface Account {
            id: i32;
            nickname?: string;
        }
        const account: Account = { id: 1 };
        console.log(account);
    "#;
    let tokens = tyrus_core::lexer::lex(source).unwrap();
    let program = Parser::new(tokens).parse_program().unwrap();
    let output = std::env::temp_dir().join(format!("tyrus-opt-print-{}", std::process::id()));
    compile(&program, "opt_print", output.to_str().unwrap(), false).unwrap();
    let ir_path = output.with_extension("ll");
    let ir = std::fs::read_to_string(&ir_path).unwrap();
    assert!(ir.contains("obj.field.print"));
    assert!(ir.contains("obj.printed"));
    std::fs::remove_file(ir_path).unwrap();
}

#[test]
fn console_log_prints_multiple_arguments_on_one_line() {
    let source = r#"
        interface User {
            id: i32;
            name: string;
            active: boolean;
        }

        const main = (): void => {
            const user = { id: 1, name: "Ada", active: true };
            console.log("user", user);
        };
    "#;
    let tokens = tyrus_core::lexer::lex(source).unwrap();
    let program = Parser::new(tokens).parse_program().unwrap();
    let output = std::env::temp_dir().join(format!("tyrus-console-log-{}", std::process::id()));
    compile(&program, "console_log", output.to_str().unwrap(), false).unwrap();

    let ir_path = output.with_extension("ll");
    let ir = std::fs::read_to_string(&ir_path).unwrap();
    assert!(
        ir.contains("print.arg.space"),
        "expected a separator between console.log arguments"
    );
    assert!(
        ir.contains("obj.open"),
        "expected anonymous object printing"
    );
    assert!(ir.contains("id: "), "expected first object field label");
    assert!(ir.contains("name: "), "expected string object field label");
    assert!(ir.contains("active: "), "expected bool object field label");
    std::fs::remove_file(ir_path).unwrap();
}

#[test]
fn console_error_and_warn_write_to_stderr() {
    let source = r#"
        interface Item { group: i32; value: i32; }
        const main = (): void => {
            console.log("out");
            console.error("err", 1);
            console.warn("warn", true);
        };
    "#;
    let tokens = tyrus_core::lexer::lex(source).unwrap();
    let program = Parser::new(tokens).parse_program().unwrap();
    let output = std::env::temp_dir().join(format!("tyrus-console-stderr-{}", std::process::id()));
    compile(&program, "console_stderr", output.to_str().unwrap(), false).unwrap();

    let ir_path = output.with_extension("ll");
    let ir = std::fs::read_to_string(&ir_path).unwrap();
    assert!(ir.contains("declare i32 @printf(ptr, ...)"));
    assert!(ir.contains(&format!("declare i32 @{}(i32, ptr, ...)", dprintf_symbol())));
    assert!(
        ir.contains(&format!(
            "call i32 (i32, ptr, ...) @{}(i32 2",
            dprintf_symbol()
        )),
        "expected console.error/console.warn output to target stderr"
    );
    std::fs::remove_file(ir_path).unwrap();
}

#[test]
fn console_warn_and_error_wrap_the_whole_line_in_one_color_check_each() {
    let source = r#"
        const main = (): void => {
            console.warn("code", 42, "failed");
            console.error("boom");
        };
    "#;
    let tokens = tyrus_core::lexer::lex(source).unwrap();
    let program = Parser::new(tokens).parse_program().unwrap();
    let output = std::env::temp_dir().join(format!("tyrus-console-color-{}", std::process::id()));
    compile(&program, "console_color", output.to_str().unwrap(), false).unwrap();

    let ir_path = output.with_extension("ll");
    let ir = std::fs::read_to_string(&ir_path).unwrap();
    assert_eq!(
        ir.matches("tyrus_console_stderr_color_enabled").count(),
        3,
        "expected exactly one color-enabled check per console.warn/console.error call"
    );
    assert_eq!(
        ir.matches("tyrus_stderr_lock_begin").count(),
        3,
        "expected exactly one stderr lock acquired per console.warn/console.error call, not per argument"
    );
    assert_eq!(
        ir.matches("tyrus_stderr_lock_end").count(),
        3,
        "expected exactly one stderr lock released per console.warn/console.error call, not per argument"
    );
    assert!(
        ir.contains("\\1B[33m"),
        "expected console.warn to use yellow"
    );
    assert!(ir.contains("\\1B[31m"), "expected console.error to use red");
    std::fs::remove_file(ir_path).unwrap();
}

#[test]
fn console_log_print_and_print_raw_never_check_color() {
    let source = r#"
        const main = (): void => {
            console.log("out");
            print("also out");
            printRaw("raw\n");
        };
    "#;
    let tokens = tyrus_core::lexer::lex(source).unwrap();
    let program = Parser::new(tokens).parse_program().unwrap();
    let output =
        std::env::temp_dir().join(format!("tyrus-console-no-color-{}", std::process::id()));
    compile(
        &program,
        "console_no_color",
        output.to_str().unwrap(),
        false,
    )
    .unwrap();

    let ir_path = output.with_extension("ll");
    let ir = std::fs::read_to_string(&ir_path).unwrap();
    assert!(
        !ir.contains("tyrus_console_stderr_color_enabled"),
        "console.log/print/printRaw must never check or apply color"
    );
    std::fs::remove_file(ir_path).unwrap();
}

#[test]
fn print_error_and_print_warn_write_to_stderr_like_console_error_and_warn() {
    let source = r#"
        const main = (): void => {
            printError("boom", 1);
            printWarn("careful", true);
        };
    "#;
    let tokens = tyrus_core::lexer::lex(source).unwrap();
    let program = Parser::new(tokens).parse_program().unwrap();
    let output =
        std::env::temp_dir().join(format!("tyrus-print-error-warn-{}", std::process::id()));
    compile(
        &program,
        "print_error_warn",
        output.to_str().unwrap(),
        false,
    )
    .unwrap();

    let ir_path = output.with_extension("ll");
    let ir = std::fs::read_to_string(&ir_path).unwrap();
    assert!(
        ir.contains(&format!("declare i32 @{}(i32, ptr, ...)", dprintf_symbol())),
        "printError/printWarn must target stderr through dprintf like console.error/warn"
    );
    assert!(
        ir.contains(&format!(
            "call i32 (i32, ptr, ...) @{}(i32 2",
            dprintf_symbol()
        )),
        "printError/printWarn must call dprintf on fd 2 (stderr)"
    );
    assert_eq!(
        ir.matches("tyrus_console_stderr_color_enabled").count(),
        3,
        "expected exactly one color-enabled check per printError/printWarn call"
    );
    assert_eq!(
        ir.matches("tyrus_stderr_lock_begin").count(),
        3,
        "expected exactly one stderr lock acquired per printError/printWarn call, not per argument"
    );
    assert_eq!(
        ir.matches("tyrus_stderr_lock_end").count(),
        3,
        "expected exactly one stderr lock released per printError/printWarn call, not per argument"
    );
    assert!(
        ir.contains("\\1B[31m"),
        "expected printError to use red like console.error"
    );
    assert!(
        ir.contains("\\1B[33m"),
        "expected printWarn to use yellow like console.warn"
    );
    std::fs::remove_file(ir_path).unwrap();
}

#[test]
fn print_and_console_log_emit_the_same_shape_of_ir() {
    for (callee, source) in [
        (
            "print",
            r#"const main = (): void => { print("a", 1, true); };"#,
        ),
        (
            "console.log",
            r#"const main = (): void => { console.log("a", 1, true); };"#,
        ),
    ] {
        let tokens = tyrus_core::lexer::lex(source).unwrap();
        let program = Parser::new(tokens).parse_program().unwrap();
        let output = std::env::temp_dir().join(format!(
            "tyrus-print-alias-{}-{}",
            callee.replace('.', "_"),
            std::process::id()
        ));
        compile(&program, "print_alias", output.to_str().unwrap(), false)
            .unwrap_or_else(|error| panic!("`{callee}` case must compile: {error}"));
        let ir_path = output.with_extension("ll");
        let ir = std::fs::read_to_string(&ir_path).unwrap();
        assert!(
            ir.contains("print.arg.space"),
            "`{callee}` expected a separator between arguments"
        );
        assert!(
            ir.contains("print.newline"),
            "`{callee}` expected a trailing newline"
        );
        std::fs::remove_file(ir_path).unwrap();
    }
}

#[test]
fn print_raw_omits_the_trailing_newline() {
    let source = r#"const main = (): void => { printRaw("no newline"); };"#;
    let tokens = tyrus_core::lexer::lex(source).unwrap();
    let program = Parser::new(tokens).parse_program().unwrap();
    let output = std::env::temp_dir().join(format!("tyrus-print-raw-{}", std::process::id()));
    compile(&program, "print_raw", output.to_str().unwrap(), false).unwrap();
    let ir_path = output.with_extension("ll");
    let ir = std::fs::read_to_string(&ir_path).unwrap();
    assert!(ir.contains("no newline"));
    assert!(
        !ir.contains("print.newline"),
        "printRaw must not emit a trailing newline literal"
    );
    std::fs::remove_file(ir_path).unwrap();
}

#[test]
fn only_print_raw_and_print_cr_flush_stdout() {
    let source = r#"
        const main = (): void => {
            print("a");
            printRaw("b");
            printCR("c");
        };
    "#;
    let tokens = tyrus_core::lexer::lex(source).unwrap();
    let program = Parser::new(tokens).parse_program().unwrap();
    let output = std::env::temp_dir().join(format!("tyrus-print-flush-{}", std::process::id()));
    compile(&program, "print_flush", output.to_str().unwrap(), false).unwrap();
    let ir_path = output.with_extension("ll");
    let ir = std::fs::read_to_string(&ir_path).unwrap();
    assert_eq!(
        ir.matches("call i32 @fflush(ptr null)").count(),
        2,
        "expected exactly one fflush for printRaw and one for printCR, none for print"
    );
    std::fs::remove_file(ir_path).unwrap();
}

#[test]
fn print_cr_clears_the_line_only_when_stdout_is_a_terminal() {
    let source = r#"const main = (): void => { printCR("=====>"); printCR("=>"); };"#;
    let tokens = tyrus_core::lexer::lex(source).unwrap();
    let program = Parser::new(tokens).parse_program().unwrap();
    let output = std::env::temp_dir().join(format!("tyrus-print-cr-{}", std::process::id()));
    compile(&program, "print_cr", output.to_str().unwrap(), false).unwrap();
    let ir_path = output.with_extension("ll");
    let ir = std::fs::read_to_string(&ir_path).unwrap();
    assert_eq!(
        ir.matches("tyrus_stdout_is_terminal").count(),
        3,
        "expected exactly one is-terminal check per printCR call"
    );
    assert!(
        ir.contains(r#"c"\0D\1B[K\00""#),
        "expected the clear-line sequence to be `\\r\\x1B[K` as a single static string"
    );
    assert!(
        !ir.contains("print.newline"),
        "printCR must not append `\\n`"
    );
    std::fs::remove_file(ir_path).unwrap();
}

#[test]
fn a_non_literal_argument_still_flushes_the_literal_run_around_it() {
    let source = r#"
        const main = (): void => {
            const x: i32 = 1;
            print("a", x, "b");
        };
    "#;
    let tokens = tyrus_core::lexer::lex(source).unwrap();
    let program = Parser::new(tokens).parse_program().unwrap();
    let output =
        std::env::temp_dir().join(format!("tyrus-print-fold-split-{}", std::process::id()));
    compile(
        &program,
        "print_fold_split",
        output.to_str().unwrap(),
        false,
    )
    .unwrap();
    let ir_path = output.with_extension("ll");
    let ir = std::fs::read_to_string(&ir_path).unwrap();
    assert!(
        ir.contains(r#"c"a\00""#),
        "expected the leading \"a\" run on its own"
    );
    assert!(
        ir.contains(r#"c" b\00""#),
        "expected the trailing run to fold its leading separator space into \" b\""
    );
    assert!(
        ir.contains("print.arg.space"),
        "expected the separator before the non-literal argument `x` to stay its own call"
    );
    std::fs::remove_file(ir_path).unwrap();
}

#[test]
fn print_writes_to_stdout_and_print_raw_shares_the_same_printf_declaration() {
    let source = r#"
        const main = (): void => {
            print("out");
            printRaw("also-out");
        };
    "#;
    let tokens = tyrus_core::lexer::lex(source).unwrap();
    let program = Parser::new(tokens).parse_program().unwrap();
    let output = std::env::temp_dir().join(format!("tyrus-print-stdout-{}", std::process::id()));
    compile(&program, "print_stdout", output.to_str().unwrap(), false).unwrap();
    let ir_path = output.with_extension("ll");
    let ir = std::fs::read_to_string(&ir_path).unwrap();
    assert!(ir.contains("declare i32 @printf(ptr, ...)"));
    assert!(
        !ir.contains(&format!("@{}", dprintf_symbol())),
        "print/printRaw must never route through stderr's dprintf"
    );
    std::fs::remove_file(ir_path).unwrap();
}

#[test]
fn print_and_console_log_each_take_the_stdout_lock_exactly_once() {
    let source = r#"
        const main = (): void => {
            print("a", 1, "b");
            console.log("c", 2, "d");
        };
    "#;
    let tokens = tyrus_core::lexer::lex(source).unwrap();
    let program = Parser::new(tokens).parse_program().unwrap();
    let output = std::env::temp_dir().join(format!("tyrus-print-lock-{}", std::process::id()));
    compile(&program, "print_lock", output.to_str().unwrap(), false).unwrap();
    let ir_path = output.with_extension("ll");
    let ir = std::fs::read_to_string(&ir_path).unwrap();
    assert_eq!(
        ir.matches("tyrus_stdout_lock_begin").count(),
        3,
        "expected exactly one stdout lock acquired per call, not per argument"
    );
    assert_eq!(
        ir.matches("tyrus_stdout_lock_end").count(),
        3,
        "expected exactly one stdout lock released per call, not per argument"
    );
    std::fs::remove_file(ir_path).unwrap();
}

#[test]
fn a_union_typed_argument_falls_back_to_the_unlocked_print_path() {
    let source = r#"
        function show(value: string | i32): void {
            print("value:", value);
        }
        const main = (): void => { show(42); };
    "#;
    let tokens = tyrus_core::lexer::lex(source).unwrap();
    let program = Parser::new(tokens).parse_program().unwrap();
    let output =
        std::env::temp_dir().join(format!("tyrus-print-union-nolock-{}", std::process::id()));
    compile(
        &program,
        "print_union_nolock",
        output.to_str().unwrap(),
        false,
    )
    .unwrap();
    let ir_path = output.with_extension("ll");
    let ir = std::fs::read_to_string(&ir_path).unwrap();
    assert!(
        !ir.contains("tyrus_stdout_lock_begin"),
        "a union-typed argument must keep the unlocked fallback path"
    );
    std::fs::remove_file(ir_path).unwrap();
}

#[test]
fn a_throwing_argument_is_evaluated_before_the_stdout_lock_is_taken() {
    let source = r#"
        function risky(): i32 throws string {
            throw "boom";
        }
        const main = (): void => {
            try {
                print("a", risky(), "b");
            } catch (e: string) {
                console.log("caught:", e);
            }
        };
    "#;
    let tokens = tyrus_core::lexer::lex(source).unwrap();
    let program = Parser::new(tokens).parse_program().unwrap();
    let output =
        std::env::temp_dir().join(format!("tyrus-print-throw-order-{}", std::process::id()));
    compile(
        &program,
        "print_throw_order",
        output.to_str().unwrap(),
        false,
    )
    .unwrap();
    let ir_path = output.with_extension("ll");
    let ir = std::fs::read_to_string(&ir_path).unwrap();
    let risky_call = ir
        .find("call i32 @risky()")
        .expect("expected a call to `risky`");
    let lock_begin = ir
        .find("call void @tyrus_stdout_lock_begin()")
        .expect("expected a stdout lock acquisition");
    assert!(
        risky_call < lock_begin,
        "`risky()` must be called before the stdout lock is taken, so a throw \
         mid-argument never leaves the lock held"
    );
    std::fs::remove_file(ir_path).unwrap();
}

#[test]
fn a_user_defined_print_never_reaches_codegen() {
    let source = r#"
        function print(label: string): void {
            console.log("shadowed:", label);
        }
        const main = (): void => { print("hello"); };
    "#;
    let tokens = tyrus_core::lexer::lex(source).unwrap();
    let error = Parser::new(tokens)
        .parse_program()
        .expect_err("redeclaring `print` must be rejected before codegen runs");
    assert_eq!(error.code, Some("E0003"));
}

#[test]
fn emits_full_numeric_type_set_constants_and_conversions() {
    let source = r#"
        const main = (): void => {
            const signed: i128 = i128.MAX;
            // `isize`/`usize` are as wide as a pointer on the target, so they
            // expose no `MIN`/`MAX`; only their values take part here.
            const pointerSigned: isize = -1;
            const signedMinLiteral: i128 = -170_141_183_460_469_231_731_687_303_715_884_105_728;
            const inferredHuge = 3_000_000_000_000_000_000_000;
            const explicitHuge: i128 = inferredHuge;
            const unsigned: u128 = u128.MAX;
            const pointerUnsigned: usize = 4_294_967_296;
            const unsignedFromLiteral: u128 = 170_141_183_460_469_231_731_687_303_715_884_105_728;
            const smallUnsigned: u32 = 42;
            const unsignedSum: u64 = (smallUnsigned as u64) + 1;
            const wideFloat: f128 = unsignedSum as f128;
            const narrowed: u32 = wideFloat as u32;
            const signedNarrow: i64 = signed as i64;
            let values: i32[] = [1, 2];
            const count: usize = values.length;
            const pushed: usize = values.push(3);
            const first: i32 = values[0 as usize];
            console.log(signed);
            console.log(pointerSigned);
            console.log(signedMinLiteral);
            console.log(explicitHuge);
            console.log(unsigned);
            console.log(pointerUnsigned);
            console.log(unsignedFromLiteral);
            console.log(wideFloat);
            console.log(narrowed);
            console.log(signedNarrow);
            console.log(count);
            console.log(pushed);
            console.log(first);
        };
    "#;
    let tokens = tyrus_core::lexer::lex(source).unwrap();
    let program = Parser::new(tokens).parse_program().unwrap();
    let mut checker = BorrowChecker::new();
    assert_eq!(checker.check_program(&program), Vec::<String>::new());

    let output = std::env::temp_dir().join(format!("tyrus-full-numeric-{}", std::process::id()));
    compile(&program, "full_numeric", output.to_str().unwrap(), false).unwrap();
    let ir_path = output.with_extension("ll");
    let ir = std::fs::read_to_string(&ir_path).unwrap();
    assert!(ir.contains("i128"));
    assert!(ir.contains("3000000000000000000000"));
    assert!(ir.contains("uitofp"));
    assert!(ir.contains("fptoui"));
    assert!(ir.contains("add i64"));
    assert!(ir.contains("__tyrus_print_i128"));
    assert!(ir.contains("__tyrus_print_u128"));
    assert!(ir.contains("__tyrus_print_f128"));
    std::fs::remove_file(ir_path).unwrap();
}

#[test]
fn emits_narrow_integer_types_at_their_own_width() {
    let source = r#"
        const main = (): void => {
            const tiny: i8 = -128;
            const small: i16 = -30_000;
            const byte: u8 = 255;
            const word: u16 = 65_535;
            console.log(tiny);
            console.log(small);
            console.log(byte);
            console.log(word);
        };
    "#;
    let tokens = tyrus_core::lexer::lex(source).unwrap();
    let program = Parser::new(tokens).parse_program().unwrap();
    let mut checker = BorrowChecker::new();
    assert_eq!(checker.check_program(&program), Vec::<String>::new());

    let output = std::env::temp_dir().join(format!("tyrus-narrow-int-{}", std::process::id()));
    compile(&program, "narrow_int", output.to_str().unwrap(), false).unwrap();
    let ir_path = output.with_extension("ll");
    let ir = std::fs::read_to_string(&ir_path).unwrap();
    assert!(ir.contains("alloca i8"), "expected an 8-bit slot");
    assert!(ir.contains("alloca i16"), "expected a 16-bit slot");
    assert!(ir.contains("sext i8"), "expected i8 printed with its sign");
    assert!(
        ir.contains("sext i16"),
        "expected i16 printed with its sign"
    );
    assert!(ir.contains("zext i8"), "expected u8 printed without a sign");
    assert!(
        ir.contains("zext i16"),
        "expected u16 printed without a sign"
    );
    std::fs::remove_file(ir_path).unwrap();
}

#[test]
fn emits_narrow_and_unsigned_maximums_without_a_bogus_range_check() {
    let source = r#"
        const main = (): void => {
            const byte: u8 = u8.MAX;
            const word: u16 = u16.MAX;
            const wide: u32 = u32.MAX;
            const low: i8 = i8.MIN;
            const high: i16 = i16.MAX;
            console.log(byte);
            console.log(word);
            console.log(wide);
            console.log(low);
            console.log(high);
        };
    "#;
    let tokens = tyrus_core::lexer::lex(source).unwrap();
    let program = Parser::new(tokens).parse_program().unwrap();
    let output = std::env::temp_dir().join(format!("tyrus-narrow-max-{}", std::process::id()));
    compile(&program, "narrow_max", output.to_str().unwrap(), false).unwrap();
    let ir_path = output.with_extension("ll");
    let ir = std::fs::read_to_string(&ir_path).unwrap();
    assert!(
        !ir.contains("cast.range.error"),
        "a constant assigned to its own type must not be range-checked"
    );
    std::fs::remove_file(ir_path).unwrap();
}

#[test]
fn emits_enum_members_as_native_constants() {
    let source = r#"
        enum Status { Success, Error, Pending }
        enum HttpStatus { Ok = 200, NotFound = 404 }
        enum Role { Admin = "admin", User = "user" }
        enum Flag { Enabled = true, Disabled = false }
        const main = (): void => {
            const s: Status = Status.Pending;
            const code: HttpStatus = HttpStatus.NotFound;
            const role: Role = Role.Admin;
            const flag: Flag = Flag.Enabled;
            console.log(s as i32);
            console.log(code as i32);
            console.log(role as string);
            console.log(flag as boolean);
        };
    "#;
    let tokens = Token::lexer(source).filter_map(Result::ok).collect();
    let program = Parser::new(tokens).parse_program().unwrap();
    let mut checker = BorrowChecker::new();
    assert_eq!(checker.check_program(&program), Vec::<String>::new());

    let output = std::env::temp_dir().join(format!("tyrus-enum-{}", std::process::id()));
    compile(&program, "enum_lowering", output.to_str().unwrap(), false).unwrap();
    let ir_path = output.with_extension("ll");
    let ir = std::fs::read_to_string(&ir_path).unwrap();
    assert!(ir.contains("i32 2"));
    assert!(ir.contains("404"));
    assert!(ir.contains("admin"));
    std::fs::remove_file(ir_path).unwrap();
}

#[test]
fn emits_tuple_print_and_index_assignment() {
    let source = r#"
        const main = (): void => {
            let pair: [i32, string] = [1, "one"];
            console.log(pair);
            pair[0] = 5;
            console.log(pair[0]);
            const nested: [[i32, i32], boolean] = [[3, 4], true];
            console.log(nested);
        };
    "#;
    let tokens = Token::lexer(source).filter_map(Result::ok).collect();
    let program = Parser::new(tokens).parse_program().unwrap();
    let mut checker = BorrowChecker::new();
    assert_eq!(checker.check_program(&program), Vec::<String>::new());

    let output = std::env::temp_dir().join(format!("tyrus-tuple-ops-{}", std::process::id()));
    compile(&program, "tuple_ops", output.to_str().unwrap(), false).unwrap();
    let ir_path = output.with_extension("ll");
    let ir = std::fs::read_to_string(&ir_path).unwrap();
    assert!(ir.contains("tuple_index_target"));
    std::fs::remove_file(ir_path).unwrap();
}

#[test]
fn emits_runtime_calls_for_dynamic_map() {
    let source = r#"
        const main = (): void => {
            let scores: Record<string, i32> = { alice: 10 };
            scores["bob"] = 20;
            console.log(scores.size);
            const alice: i32? = scores["alice"];
            console.log(scores.has("bob"));
            console.log(scores.delete("bob"));
            const names: string[] = scores.keys();
            console.log(names.length);
            if (alice) {
                console.log(alice);
            }
        };
    "#;
    let tokens = Token::lexer(source).filter_map(Result::ok).collect();
    let program = Parser::new(tokens).parse_program().unwrap();
    let mut checker = BorrowChecker::new();
    assert_eq!(checker.check_program(&program), Vec::<String>::new());

    let output = std::env::temp_dir().join(format!("tyrus-map-ops-{}", std::process::id()));
    compile(&program, "map_ops", output.to_str().unwrap(), false).unwrap();
    let ir_path = output.with_extension("ll");
    let ir = std::fs::read_to_string(&ir_path).unwrap();
    for symbol in [
        "tyrus_map_new",
        "tyrus_map_insert",
        "tyrus_map_get",
        "tyrus_map_size",
        "tyrus_map_has",
        "tyrus_map_delete",
        "tyrus_map_keys",
    ] {
        assert!(ir.contains(symbol), "expected `{symbol}` in IR");
    }
    std::fs::remove_file(ir_path).unwrap();
}

#[test]
fn emits_lazy_map_get_or_insert_control_flow() {
    let source = r#"
        const fallback = (): i32 => {
            console.log("fallback");
            return 42;
        };
        const main = (): void => {
            let values: Map<string, i32> = Map<string, i32>();
            const first = values.getOrInsert("answer", fallback());
            const second = values.getOrInsert("answer", fallback());
            console.log(first, second);
        };
    "#;
    let tokens = Token::lexer(source).filter_map(Result::ok).collect();
    let program = Parser::new(tokens).parse_program().unwrap();
    let mut checker = BorrowChecker::new();
    assert_eq!(checker.check_program(&program), Vec::<String>::new());

    let output =
        std::env::temp_dir().join(format!("tyrus-map-get-or-insert-{}", std::process::id()));
    compile(
        &program,
        "map_get_or_insert",
        output.to_str().unwrap(),
        false,
    )
    .unwrap();
    let ir_path = output.with_extension("ll");
    let ir = std::fs::read_to_string(&ir_path).unwrap();
    assert!(ir.contains("tyrus_map_has"));
    assert!(ir.contains("map.get_or_insert.hit"));
    assert!(ir.contains("map.get_or_insert.miss"));
    std::fs::remove_file(ir_path).unwrap();
}

#[test]
fn emits_conditional_throw_for_assertions() {
    let source = r#"
        const main = (): void => {
            assert(1 + 1 === 2);
            assert(2 > 1, "two beats one");
            assertEqual(2 + 3, 5);
            assertEqual("a", "a");
        };
    "#;
    let tokens = Token::lexer(source).filter_map(Result::ok).collect();
    let program = Parser::new(tokens).parse_program().unwrap();
    let mut checker = BorrowChecker::new();
    assert_eq!(checker.check_program(&program), Vec::<String>::new());

    let output = std::env::temp_dir().join(format!("tyrus-assert-{}", std::process::id()));
    compile(&program, "assert_test", output.to_str().unwrap(), false).unwrap();
    let ir_path = output.with_extension("ll");
    let ir = std::fs::read_to_string(&ir_path).unwrap();
    assert!(
        ir.contains("assert.fail"),
        "assert must branch to a fail block"
    );
    assert!(
        ir.contains("@longjmp"),
        "a failing assert throws via longjmp"
    );
    std::fs::remove_file(ir_path).unwrap();
}

#[test]
fn escaping_closure_clones_owned_captures_and_generates_a_destructor() {
    // A closure capturing an owned value used to store the captured pointer in
    // its environment, so a closure outliving the capture read freed memory.
    // The environment must instead clone the capture (`strdup` for a string)
    // and expose a destructor that releases the clone, mirroring the async
    // closure path. Both halves are asserted here because dropping either one
    // reintroduces a use-after-free or a leak.
    let source = r#"
        const makeCounter = (label: string): () => usize => {
            const held: string = label.toUpperCase();
            return (): usize => {
                return held.length;
            };
        };
        const main = (): void => {
            const counter = makeCounter("ab");
            console.log(counter());
        };
    "#;
    let tokens = Token::lexer(source).filter_map(Result::ok).collect();
    let program = Parser::new(tokens).parse_program().unwrap();
    let mut checker = BorrowChecker::new();
    assert_eq!(checker.check_program(&program), Vec::<String>::new());

    let output = std::env::temp_dir().join(format!("tyrus-closure-drop-{}", std::process::id()));
    compile(&program, "closure_drop", output.to_str().unwrap(), false).unwrap();
    let ir_path = output.with_extension("ll");
    let ir = std::fs::read_to_string(&ir_path).unwrap();
    assert!(
        ir.contains("@tyrus_string_share"),
        "an owned capture must be cloned into the environment, not aliased"
    );
    assert!(
        ir.contains("_environment_drop"),
        "an escaping environment must have a generated destructor"
    );
    std::fs::remove_file(ir_path).unwrap();
}

#[test]
fn reference_counts_a_returned_closure_bound_to_a_name() {
    let owned = r#"
        const makeCounter = (label: string): () => usize => {
            const held: string = label.toUpperCase();
            return (): usize => { return held.length; };
        };
        const main = (): void => {
            const f = makeCounter("ab");
            console.log(f());
        };
    "#;
    let aliased = r#"
        const makeCounter = (label: string): () => usize => {
            const held: string = label.toUpperCase();
            return (): usize => { return held.length; };
        };
        const main = (): void => {
            const f = makeCounter("ab");
            const g = f;
            console.log(g());
        };
    "#;
    let emit = |source: &str, name: &str| {
        let tokens = Token::lexer(source).filter_map(Result::ok).collect();
        let program = Parser::new(tokens).parse_program().unwrap();
        let output = std::env::temp_dir().join(format!("tyrus-{name}-{}", std::process::id()));
        compile(&program, name, output.to_str().unwrap(), false).unwrap();
        let ir_path = output.with_extension("ll");
        let ir = std::fs::read_to_string(&ir_path).unwrap();
        std::fs::remove_file(ir_path).unwrap();
        ir
    };
    let owned_ir = emit(owned, "closure_owned");
    assert!(
        owned_ir.contains("tyrus_closure_release"),
        "an owned closure binding must release its reference at scope exit"
    );
    assert!(
        !owned_ir.contains("tyrus_closure_retain"),
        "a binding straight from a call must not add a reference"
    );
    assert!(
        emit(aliased, "closure_aliased").contains("tyrus_closure_retain"),
        "aliasing a closure must add a reference for the new owner"
    );
}

#[test]
fn emits_runtime_calls_for_set_and_map_collections() {
    let source = r#"
        const main = (): void => {
            let s: Set<i32> = Set<i32>();
            s.add(1);
            console.log(s.has(1));
            console.log(s.size);
            for (const v of s) {
                console.log(v);
            }
            let m: Map<i32, string> = Map<i32, string>();
            m.set(7, "seven");
            console.log(m.get(7) ?? "none");
            const entries: [i32, string][] = m.entries();
            console.log(entries.length);
            for (const e of m) {
                console.log(e[0]);
            }
        };
    "#;
    let tokens = Token::lexer(source).filter_map(Result::ok).collect();
    let program = Parser::new(tokens).parse_program().unwrap();
    let mut checker = BorrowChecker::new();
    assert_eq!(checker.check_program(&program), Vec::<String>::new());

    let output = std::env::temp_dir().join(format!("tyrus-collections-{}", std::process::id()));
    compile(&program, "collections", output.to_str().unwrap(), false).unwrap();
    let ir_path = output.with_extension("ll");
    let ir = std::fs::read_to_string(&ir_path).unwrap();
    for symbol in [
        "tyrus_set_new",
        "tyrus_set_add",
        "tyrus_set_has",
        "tyrus_set_values",
        "tyrus_map_new",
        "tyrus_map_insert",
        "tyrus_map_get",
        "tyrus_map_keys",
        "tyrus_map_values",
    ] {
        assert!(ir.contains(symbol), "expected `{symbol}` in IR");
    }
    std::fs::remove_file(ir_path).unwrap();
}

#[test]
fn emits_const_assertions_as_zero_cost_values() {
    let source = r#"
        const main = (): void => {
            const status: "ready" = "ready" as const;
            const values: readonly i32[] = [1, 2, 3] as const;
            console.log(status);
            console.log(values[0]);
        };
    "#;
    let tokens = Token::lexer(source).filter_map(Result::ok).collect();
    let program = Parser::new(tokens).parse_program().unwrap();
    let mut checker = BorrowChecker::new();
    assert_eq!(checker.check_program(&program), Vec::<String>::new());

    let output = std::env::temp_dir().join(format!("tyrus-const-assert-{}", std::process::id()));
    compile(&program, "const_assert", output.to_str().unwrap(), false).unwrap();
    let ir_path = output.with_extension("ll");
    let ir = std::fs::read_to_string(&ir_path).unwrap();
    assert!(ir.contains("ready"));
    assert!(!ir.contains("__tyrus_print_const"));
    std::fs::remove_file(ir_path).unwrap();
}

#[test]
fn emits_tagged_union_discriminant_narrowing() {
    let source = r#"
        type Shape =
            | { kind: "circle"; radius: f64 }
            | { kind: "square"; size: f64 }
            | { kind: "rect"; width: f64; height: f64 };

        const printShape = (shape: Shape): void => {
            if (shape.kind === "circle") {
                console.log(shape.radius);
            } else if (shape.kind === "square") {
                console.log(shape.size);
            } else {
                console.log(shape.width);
                console.log(shape.height);
            }
        };

        const main = (): void => {
            const shape: Shape = { kind: "rect", width: 10.0, height: 5.0 };
            printShape(shape);
        };
    "#;
    let tokens = Token::lexer(source).filter_map(Result::ok).collect();
    let program = Parser::new(tokens).parse_program().unwrap();
    let mut checker = BorrowChecker::new();
    assert_eq!(checker.check_program(&program), Vec::<String>::new());

    let output = std::env::temp_dir().join(format!(
        "tyrus-tagged-union-narrowing-{}",
        std::process::id()
    ));
    compile(
        &program,
        "tagged_union_narrowing",
        output.to_str().unwrap(),
        false,
    )
    .unwrap();
    let ir_path = output.with_extension("ll");
    let ir = std::fs::read_to_string(&ir_path).unwrap();
    assert!(ir.contains("union.tag.matches"));
    assert!(ir.contains("union.payload"));
    assert!(ir.contains("{ i8, [4 x i64] }"));
    std::fs::remove_file(ir_path).unwrap();
}

#[test]
fn emits_equality_for_a_non_union_object_field_without_narrowing() {
    let source = r#"
        interface RequestMetadata {
            cookies: string;
        }

        const main = (): void => {
            const request: RequestMetadata = { cookies: "{}" };
            if (request.cookies === "{}") {
                console.log("no cookies");
            }
        };
    "#;
    let tokens = Token::lexer(source).filter_map(Result::ok).collect();
    let program = Parser::new(tokens).parse_program().unwrap();
    let mut checker = BorrowChecker::new();
    assert_eq!(checker.check_program(&program), Vec::<String>::new());

    let output = std::env::temp_dir().join(format!(
        "tyrus-non-union-field-equality-{}",
        std::process::id()
    ));
    compile(
        &program,
        "non_union_field_equality",
        output.to_str().unwrap(),
        false,
    )
    .unwrap();
    let ir_path = output.with_extension("ll");
    let ir = std::fs::read_to_string(&ir_path).unwrap();
    assert!(ir.contains("no cookies"));
    assert!(!ir.contains("union.tag.matches"));
    std::fs::remove_file(ir_path).unwrap();
}

#[test]
fn emits_object_literal_return_as_tagged_union() {
    let source = r#"
        type Result =
            | { tag: "success"; data: string }
            | { tag: "error"; code: i32; message: string };

        const createExample = (): Result => {
            return { tag: "error", code: 404, message: "Not Found" };
        };

        const main = (): void => {
            const result: Result = createExample();
            console.log(result);
        };
    "#;
    let tokens = Token::lexer(source).filter_map(Result::ok).collect();
    let program = Parser::new(tokens).parse_program().unwrap();
    let output =
        std::env::temp_dir().join(format!("tyrus-union-object-return-{}", std::process::id()));
    compile(
        &program,
        "union_object_return",
        output.to_str().unwrap(),
        false,
    )
    .unwrap();
    let ir_path = output.with_extension("ll");
    let ir = std::fs::read_to_string(&ir_path).unwrap();
    assert!(ir.contains("union.value"));
    assert!(ir.contains("union.payload"));
    std::fs::remove_file(ir_path).unwrap();
}

#[test]
fn emits_result_return_propagation_and_match() {
    let source = r#"
        function load(): Result<i32, string> { return Ok(7); }
        function useResult(): Result<i32, string> {
            const value = load()?;
            return Ok(value);
        }
        function inspect(result: Result<i32, string>): Result<i32, string> {
            if (result) {
                return Ok(result);
            } else {
                return Err(result);
            }
        }
        const main = (): void => {
            const result = useResult();
            match (result) {
                Ok(value) => { console.log(value); }
                Err(error) => { console.log(error); }
            }
        };
    "#;
    let program = Parser::from_source(source)
        .unwrap()
        .parse_program()
        .unwrap();
    let output = std::env::temp_dir().join(format!("tyrus-result-codegen-{}", std::process::id()));
    compile(&program, "result_codegen", output.to_str().unwrap(), false).unwrap();
    let ir = std::fs::read_to_string(output.with_extension("ll")).unwrap();
    assert!(ir.contains("result.success"));
    assert!(ir.contains("result.tag"));
    std::fs::remove_file(output.with_extension("ll")).unwrap();
}

#[test]
fn emits_non_copy_result_payload_storage() {
    let source = r#"
        function load(): Result<string, string> { return Ok("ready"); }
        const main = (): void => {
            const result = load();
            match (result) {
                Ok(value) => { console.log(value); }
                Err(error) => { console.log(error); }
            }
        };
    "#;
    let program = Parser::from_source(source)
        .unwrap()
        .parse_program()
        .unwrap();
    let output = std::env::temp_dir().join(format!(
        "tyrus-result-non-copy-codegen-{}",
        std::process::id()
    ));
    compile(
        &program,
        "result_non_copy_codegen",
        output.to_str().unwrap(),
        false,
    )
    .unwrap();
    let ir_path = output.with_extension("ll");
    let ir = std::fs::read_to_string(&ir_path).unwrap();
    assert!(ir.contains("result.tag"));
    std::fs::remove_file(ir_path).unwrap();
}

#[test]
fn result_with_owned_payload_uses_union_clone_and_drop_glue() {
    let ir = compile_to_ir(
        r#"
            function load(): Result<i32, string> {
                return Err("failed".toUpperCase());
            }

            function main(): void {
                match (load()) {
                    Ok(value) => { console.log(value); }
                    Err(error) => { console.log(error); }
                }
            }
        "#,
        "result_owned_payload_glue",
    );
    assert!(
        ir.contains("async.union.clone.variant"),
        "Result cloning must walk its active tagged-union payload:\n{ir}"
    );
    assert!(
        ir.contains("@tyrus_string_share"),
        "Result cloning must retain an owned string payload:\n{ir}"
    );
    assert!(
        ir.contains("drop.union.variant"),
        "Result cleanup must drop its active tagged-union payload:\n{ir}"
    );
}

#[test]
fn emits_transparent_primitive_borrows() {
    let source = r#"
        const increment = (value: &i32): i32 => value + 1;
        const main = (): void => {
            const value: i32 = 41;
            console.log(increment(&value));
        };
    "#;
    let tokens = Token::lexer(source).filter_map(Result::ok).collect();
    let program = Parser::new(tokens).parse_program().unwrap();
    let output = std::env::temp_dir().join(format!("tyrus-borrow-{}", std::process::id()));
    compile(&program, "borrow", output.to_str().unwrap(), false).unwrap();
    let ir_path = output.with_extension("ll");
    let ir = std::fs::read_to_string(&ir_path).unwrap();
    assert!(ir.contains("define i32 @increment(i32"));
    std::fs::remove_file(ir_path).unwrap();
}

#[test]
fn emits_optional_object_fields_and_object_updates() {
    let source = r#"
        interface User {
            name: string;
            age: i32;
            nickname?: string;
        }

        const main = (): void => {
            const user: User = { name: "Ada", age: 36 };
            const renamed: User = { ...user, name: "Grace" };
            const withNickname: User = { ...renamed, nickname: "Amazing Grace" };
            console.log(withNickname.name);
        };
    "#;
    let tokens = Token::lexer(source).filter_map(Result::ok).collect();
    let program = Parser::new(tokens).parse_program().unwrap();
    let mut checker = BorrowChecker::new();
    assert_eq!(checker.check_program(&program), Vec::<String>::new());

    let output = std::env::temp_dir().join(format!("tyrus-object-test-{}", std::process::id()));
    compile(&program, "object_test", output.to_str().unwrap(), false).unwrap();
    let ir_path = output.with_extension("ll");
    let ir = std::fs::read_to_string(&ir_path).unwrap();
    assert!(ir.contains("insertvalue"));
    assert!(ir.contains("Amazing Grace"));
    std::fs::remove_file(ir_path).unwrap();
}

#[test]
fn emits_static_object_pick_and_omit_projections() {
    let ir = compile_to_ir(
        r#"
            interface User {
                name: string;
                age: i32;
                nickname?: string;
            }

            function main(): void {
                const user: User = { name: "Ada", age: 36 };
                const picked = user.pick(["name", "nickname"]);
                const omitted = user.omit(["age"]);
                console.log(picked.name, omitted.name);
            }
        "#,
        "object_pick_omit_projection",
    );
    assert!(
        ir.contains("object.projection.field"),
        "missing projection loads:\n{ir}"
    );
    assert!(
        ir.contains("object.projection.field.slot"),
        "missing projection stores:\n{ir}"
    );
    assert!(
        !ir.contains("object.projection.key"),
        "static projection must not materialize keys at runtime:\n{ir}"
    );
}

#[test]
fn emits_readonly_non_copy_function_arguments_without_consuming_callers() {
    let source = r#"
        interface User {
            name: string;
        }

        const greet = (name: string): void => {
            console.log(name);
        };

        const showUser = (user: User): void => {
            console.log(user.name);
        };

        const main = (): void => {
            const name: string = "Ada";
            greet(name);
            console.log(name);

            const user: User = { name: "Grace" };
            showUser(user);
            console.log(user.name);
        };
    "#;
    let tokens = Token::lexer(source).filter_map(Result::ok).collect();
    let program = Parser::new(tokens).parse_program().unwrap();
    let mut checker = BorrowChecker::new();
    assert_eq!(checker.check_program(&program), Vec::<String>::new());

    let output = std::env::temp_dir().join(format!("tyrus-readonly-call-{}", std::process::id()));
    compile(&program, "readonly_call", output.to_str().unwrap(), false).unwrap();
    let ir_path = output.with_extension("ll");
    let ir = std::fs::read_to_string(&ir_path).unwrap();
    assert!(ir.contains("call void @greet"));
    assert!(ir.contains("call void @showUser"));
    std::fs::remove_file(ir_path).unwrap();
}

#[test]
fn emits_for_of_over_a_tuple_without_allocating() {
    let source = r#"
        const main = (): void => {
            const point: [i32, i32, i32] = [10, 20, 30];
            for (const value of point) {
                console.log(value);
            }
        };
    "#;
    let tokens = Token::lexer(source).filter_map(Result::ok).collect();
    let program = Parser::new(tokens).parse_program().unwrap();

    let output = std::env::temp_dir().join(format!("tyrus-tuple-forof-{}", std::process::id()));
    compile(&program, "tuple_forof", output.to_str().unwrap(), false).unwrap();
    let ir_path = output.with_extension("ll");
    let ir = std::fs::read_to_string(&ir_path).unwrap();
    assert!(ir.contains("forof_tuple"));
    assert!(ir.contains("forof.cond"));
    std::fs::remove_file(ir_path).unwrap();
}

#[test]
fn emits_in_place_field_mutation() {
    let source = r#"
        interface Line {
            length: i64;
        }

        interface Point {
            x: i64;
            y: i64;
        }

        const main = (): void => {
            let p: Point = { x: 1, y: 2 };
            p.x = 10;
            p.y += 5;
            console.log(p.x);

            let points: Point[] = [{ x: 0, y: 0 }];
            points[0].x = 99;
            console.log(points[0].x);
        };
    "#;
    let tokens = Token::lexer(source).filter_map(Result::ok).collect();
    let program = Parser::new(tokens).parse_program().unwrap();

    let output = std::env::temp_dir().join(format!("tyrus-field-assign-{}", std::process::id()));
    compile(&program, "field_assign", output.to_str().unwrap(), false).unwrap();
    let ir_path = output.with_extension("ll");
    let ir = std::fs::read_to_string(&ir_path).unwrap();
    assert!(ir.contains("getelementptr"));
    assert!(ir.contains("store"));
    std::fs::remove_file(ir_path).unwrap();
}

#[test]
fn emits_typescript_style_tuples() {
    let source = r#"
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
    "#;
    let tokens = Token::lexer(source).filter_map(Result::ok).collect();
    let program = Parser::new(tokens).parse_program().unwrap();
    let mut checker = BorrowChecker::new();
    assert_eq!(checker.check_program(&program), Vec::<String>::new());

    let output = std::env::temp_dir().join(format!("tyrus-tuple-{}", std::process::id()));
    compile(&program, "tuple_test", output.to_str().unwrap(), false).unwrap();
    let ir_path = output.with_extension("ll");
    let ir = std::fs::read_to_string(&ir_path).unwrap();
    assert!(ir.contains("define double @lengthSquared({ double, double }"));
    assert!(ir.contains("extractvalue { double, double }"));
    std::fs::remove_file(ir_path).unwrap();
}

#[test]
fn emits_indirect_call_for_arrow_function_value() {
    let source = r#"
        const main = (): void => {
            const factor: i32 = 2;
            const double: (i32) => i32 = (value: i32): i32 => value * factor;
            const result: i32 = double(21);
            console.log(result);
        };
    "#;
    let tokens = Token::lexer(source).filter_map(Result::ok).collect();
    let program = Parser::new(tokens).parse_program().unwrap();
    let mut checker = BorrowChecker::new();
    assert_eq!(checker.check_program(&program), Vec::<String>::new());

    let output = std::env::temp_dir().join(format!("tyrus-arrow-test-{}", std::process::id()));
    compile(&program, "arrow_test", output.to_str().unwrap(), false).unwrap();
    let ir_path = output.with_extension("ll");
    let ir = std::fs::read_to_string(&ir_path).unwrap();
    assert!(ir.contains("__tyrus_arrow_0"));
    assert!(ir.contains("closure_environment"));
    assert!(ir.contains("call i32 %"));
    std::fs::remove_file(ir_path).unwrap();
}

#[test]
fn monomorphizes_explicit_and_inferred_generic_calls() {
    let source = r#"
        const identity = <T>(value: T): T => value;

        const main = (): void => {
            const count: i32 = identity<i32>(42);
            const message: string = identity("tyrus");
            console.log(count);
            console.log(message);
        };
    "#;
    let tokens = Token::lexer(source).filter_map(Result::ok).collect();
    let program = Parser::new(tokens).parse_program().unwrap();
    let mut checker = BorrowChecker::new();
    assert_eq!(checker.check_program(&program), Vec::<String>::new());

    let output = std::env::temp_dir().join(format!("tyrus-generics-test-{}", std::process::id()));
    compile(&program, "generics_test", output.to_str().unwrap(), false).unwrap();
    let ir_path = output.with_extension("ll");
    let ir = std::fs::read_to_string(&ir_path).unwrap();
    assert!(ir.contains("@identity.__tyrus_generic.i32"));
    assert!(ir.contains("@identity.__tyrus_generic.string"));
    assert_eq!(
        ir.matches("define i32 @identity.__tyrus_generic.i32")
            .count(),
        1
    );
    assert_eq!(
        ir.matches("define { ptr, i64 } @identity.__tyrus_generic.string")
            .count(),
        1
    );
    std::fs::remove_file(ir_path).unwrap();
}

#[test]
fn emits_modulo_and_exponentiation_for_integers_and_floats() {
    let source = r#"
        const integerModulo = (left: i32, right: i32): i32 => left % right;
        const integerPower = (base: i32, exponent: i32): i32 => base ** exponent;
        const floatModulo = (left: f64, right: f64): f64 => left % right;
        const floatPower = (base: f64, exponent: f64): f64 => base ** exponent;
    "#;
    let tokens = Token::lexer(source).filter_map(Result::ok).collect();
    let program = Parser::new(tokens).parse_program().unwrap();
    let mut checker = BorrowChecker::new();
    assert_eq!(checker.check_program(&program), Vec::<String>::new());

    let output = std::env::temp_dir().join(format!("tyrus-operators-test-{}", std::process::id()));
    compile(&program, "operators_test", output.to_str().unwrap(), false).unwrap();
    let ir_path = output.with_extension("ll");
    let ir = std::fs::read_to_string(&ir_path).unwrap();
    assert!(ir.contains("srem i32"));
    assert!(ir.contains("frem double"));
    assert!(ir.contains("@__tyrus_pow_i32"));
    assert!(ir.contains("@llvm.pow.f64"));
    std::fs::remove_file(ir_path).unwrap();
}

#[test]
fn emits_modulo_and_power_assignments() {
    let source = r#"
        const main = (): void => {
            let value: i32 = 10;
            value %= 4;
            value **= 3;
            console.log(value);
        };
    "#;
    let tokens = Token::lexer(source).filter_map(Result::ok).collect();
    let program = Parser::new(tokens).parse_program().unwrap();
    let mut checker = BorrowChecker::new();
    assert_eq!(checker.check_program(&program), Vec::<String>::new());

    let output = std::env::temp_dir().join(format!(
        "tyrus-compound-operators-test-{}",
        std::process::id()
    ));
    compile(
        &program,
        "compound_operators_test",
        output.to_str().unwrap(),
        false,
    )
    .unwrap();
    let ir_path = output.with_extension("ll");
    let ir = std::fs::read_to_string(&ir_path).unwrap();
    assert!(ir.contains("@__tyrus_pow_i32"));
    std::fs::remove_file(ir_path).unwrap();
}

#[test]
fn emits_short_circuit_logical_assignments() {
    let source = r#"
        const main = (): void => {
            let enabled: boolean = true;
            enabled &&= false;
            enabled ||= true;
            let name: string? = "Ada";
            name ??= "guest";
            console.log(enabled);
            console.log(name ?? "missing");
        };
    "#;
    let tokens = Token::lexer(source).filter_map(Result::ok).collect();
    let program = Parser::new(tokens).parse_program().unwrap();
    let mut checker = BorrowChecker::new();
    assert_eq!(checker.check_program(&program), Vec::<String>::new());

    let output = std::env::temp_dir().join(format!(
        "tyrus-logical-assignments-test-{}",
        std::process::id()
    ));
    compile(
        &program,
        "logical_assignments_test",
        output.to_str().unwrap(),
        false,
    )
    .unwrap();
    let ir_path = output.with_extension("ll");
    let ir = std::fs::read_to_string(&ir_path).unwrap();
    assert!(ir.contains("logical.assign"));
    assert!(ir.contains("logical.continue"));
    assert!(ir.contains("is_null"));
    std::fs::remove_file(ir_path).unwrap();
}

#[test]
fn emits_lazy_logical_and_nullish_expressions() {
    let source = r#"
        const evaluateBoolean = (): boolean => true;
        const evaluateString = (): string => "fallback";
        const printMissing = (missing?: string): void => {
            console.log(missing ?? evaluateString());
        };
        const main = (): void => {
            const disabled: boolean = false;
            const enabled: boolean = true;
            console.log(disabled && evaluateBoolean());
            console.log(enabled || evaluateBoolean());
            printMissing();
        };
    "#;
    let tokens = Token::lexer(source).filter_map(Result::ok).collect();
    let program = Parser::new(tokens).parse_program().unwrap();
    let output = std::env::temp_dir().join(format!("tyrus-lazy-operators-{}", std::process::id()));
    compile(&program, "lazy_operators", output.to_str().unwrap(), false).unwrap();
    let ir_path = output.with_extension("ll");
    let ir = std::fs::read_to_string(&ir_path).unwrap();

    assert!(ir.matches("logical.right").count() >= 2);
    assert!(ir.matches("logical.merge").count() >= 2);
    assert!(ir.contains("coalesce.fallback:"));
    assert!(ir.contains("coalesce.present:"));
    assert!(ir.contains("coalesce.merge:"));
    std::fs::remove_file(ir_path).unwrap();
}

#[test]
fn runtime_safety_aborts_route_through_runtime_panic() {
    let source = r#"
        const getZero = (): i32 => 0;
        const main = (): void => {
            console.log(10 / getZero());
        };
    "#;
    let tokens = Token::lexer(source).filter_map(Result::ok).collect();
    let program = Parser::new(tokens).parse_program().unwrap();
    let output = std::env::temp_dir().join(format!("tyrus-runtime-panic-{}", std::process::id()));
    compile(&program, "runtime_panic", output.to_str().unwrap(), false).unwrap();
    let ir_path = output.with_extension("ll");
    let ir = std::fs::read_to_string(&ir_path).unwrap();
    assert!(
        ir.contains("call void @tyrus_panic"),
        "safety abort should call the runtime panic helper, got:\n{ir}"
    );
    assert!(ir.contains("tyrus runtime error: integer division by zero"));
    std::fs::remove_file(ir_path).unwrap();
}

#[test]
fn lowers_optional_unwrap_ast_to_runtime_guard() {
    let source = r#"
        const show = (value?: string): void => {
            console.log(value!);
        };
        const main = (): void => {
            show();
        };
    "#;
    let tokens = Token::lexer(source).filter_map(Result::ok).collect();
    let program = Parser::new(tokens).parse_program().unwrap();
    let output = std::env::temp_dir().join(format!("tyrus-guarded-unwrap-{}", std::process::id()));
    compile(&program, "guarded_unwrap", output.to_str().unwrap(), false).unwrap();
    let ir_path = output.with_extension("ll");
    let ir = std::fs::read_to_string(&ir_path).unwrap();

    assert!(ir.contains("unwrap.failure:"));
    assert!(ir.contains("unwrap.present:"));
    assert!(ir.contains("call void @tyrus_panic"));
    assert!(ir.contains("tyrus runtime error: attempted to unwrap null"));
    std::fs::remove_file(ir_path).unwrap();
}

#[test]
fn emits_lazy_optional_method_calls() {
    let source = r#"
        const show = (values?: i32[]): void => {
            const contains: boolean? = values?.includes(2);
            console.log(contains ?? false);
        };
        const main = (): void => {
            show();
        };
    "#;
    let tokens = Token::lexer(source).filter_map(Result::ok).collect();
    let program = Parser::new(tokens).parse_program().unwrap();
    let output = std::env::temp_dir().join(format!("tyrus-optional-method-{}", std::process::id()));
    compile(&program, "optional_method", output.to_str().unwrap(), false).unwrap();
    let ir_path = output.with_extension("ll");
    let ir = std::fs::read_to_string(&ir_path).unwrap();

    assert!(ir.contains("optional.absent:"));
    assert!(ir.contains("optional.present:"));
    assert!(ir.contains("optional.merge:"));
    std::fs::remove_file(ir_path).unwrap();
}

#[test]
fn emits_a_three_way_read_for_an_optional_request_field() {
    let source = r#"
        enum HttpStatus { Created = 201 }
        interface CreateBody {
            name: string;
            nickname?: string;
            age?: i32;
        }
        interface CreatedResponse {
            name: string;
        }
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
    "#;
    let tokens = Token::lexer(source).filter_map(Result::ok).collect();
    let program = Parser::new(tokens).parse_program().unwrap();
    let output = std::env::temp_dir().join(format!(
        "tyrus-http-optional-field-test-{}",
        std::process::id()
    ));
    compile(
        &program,
        "http_optional_field_test",
        output.to_str().unwrap(),
        false,
    )
    .unwrap();
    let ir_path = output.with_extension("ll");
    let ir = std::fs::read_to_string(&ir_path).unwrap();
    assert!(ir.contains("http.json.optional.present"));
    assert!(ir.contains("http.json.optional.absent"));
    assert!(ir.contains("http.json.optional.invalid"));
    assert!(ir.contains("@tyrus_json_document_get_i32"));
    std::fs::remove_file(ir_path).unwrap();
}

#[test]
fn emits_zip_as_a_length_guarded_loop_with_a_recoverable_throw() {
    let ir = compile_to_ir(
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
        "zip_length_guard",
    );

    for marker in [
        "zip.same_length",
        "zip.mismatch",
        "zip.proceed",
        "array.zip.condition",
        "zip.error.code",
        "zip.error.message",
    ] {
        assert!(ir.contains(marker), "missing LLVM marker `{marker}`");
    }
}

#[test]
fn releases_each_owned_map_callback_result_after_cloning_it_into_the_output() {
    let ir = compile_to_ir(
        r#"
            interface Entry {
                label: string;
            }

            const main = (): void => {
                const entries: Entry[] = [{ label: "value" }];
                console.log(entries.map((entry): string => entry.label));
            };
        "#,
        "temporary_map_callback_result_release",
    );

    let map_body = ir
        .split("array.map.body:")
        .nth(1)
        .and_then(|block| block.split("br label %array.map.condition").next())
        .expect("map code generation must emit its callback body");
    let push = map_body
        .find("call i64 @tyrus_array_push_owned_string")
        .expect("map code generation must append the cloned callback result");
    assert!(
        map_body[push..].contains("call void @tyrus_string_release_buffer("),
        "the owned callback result must be released after its clone is appended\n{map_body}"
    );
}

#[test]
fn preserves_the_environment_of_a_stored_callback() {
    let source = r#"
        const main = (): void => {
            const factor: i32 = 3;
            const scale: (i32) => i32 = value => value * factor;
            const values: i32[] = [1, 2];
            const first: i32[] = values.map(scale);
            const second: i32[] = values.map(scale);
            console.log(first[0]);
            console.log(second[1]);
        };
    "#;
    let tokens = Token::lexer(source).filter_map(Result::ok).collect();
    let program = Parser::new(tokens).parse_program().unwrap();
    let output = std::env::temp_dir().join(format!("tyrus-stored-callback-{}", std::process::id()));
    compile(&program, "stored_callback", output.to_str().unwrap(), false).unwrap();
    let ir_path = output.with_extension("ll");
    let ir = std::fs::read_to_string(&ir_path).unwrap();
    assert!(ir.contains("closure_environment"));
    assert!(
        ir.contains("tyrus_closure_release"),
        "a stored closure must keep its environment through every call and release its reference at scope exit"
    );
    std::fs::remove_file(ir_path).unwrap();
}

#[test]
fn non_escaping_callback_uses_stack_environment_without_malloc() {
    let source = r#"
        const main = (): void => {
            const factor: i32 = 2;
            const numbers: i32[] = [1, 2, 3];
            const doubled: i32[] = numbers.map((n: i32): i32 => {
                return n * factor;
            });
        };
    "#;
    let tokens = Token::lexer(source).filter_map(Result::ok).collect();
    let program = Parser::new(tokens).parse_program().unwrap();
    let mut checker = BorrowChecker::new();
    assert_eq!(checker.check_program(&program), Vec::<String>::new());
    let output = std::env::temp_dir().join(format!("tyrus-non-escaping-{}", std::process::id()));
    compile(&program, "non_escaping", output.to_str().unwrap(), false).unwrap();
    let ir_path = output.with_extension("ll");
    let ir = std::fs::read_to_string(&ir_path).unwrap();
    assert!(
        !ir.contains("closure_environment"),
        "non-escaping callback must not use the heap closure environment label"
    );
    assert!(
        ir.contains("closure_env_stack"),
        "non-escaping callback must use a stack-allocated environment"
    );
    std::fs::remove_file(ir_path).unwrap();
}

#[test]
fn a_confined_closure_captures_by_reference() {
    let source = r#"
        function main(): void {
            let counter = 0;
            const bump = (): i32 => {
                counter = counter + 1;
                return counter;
            };
            console.log(bump());
        }
    "#;
    let ir = compile_to_ir(source, "confined-closure");
    assert!(ir.contains("closure_env_stack"));
    assert!(!ir.contains("call ptr @malloc"));
    assert!(!ir.contains("tyrus_closure_release"));
    assert!(ir.contains("alloca { ptr }"));
    assert!(ir.contains("capture_ptr"));
}

#[test]
fn a_confined_closure_environment_is_never_released() {
    let source = r#"
        function main(): void {
            let counter = 0;
            const bump = (): void => {
                counter = counter + 1;
            };
            bump();
            console.log(counter);
        }
    "#;
    let ir = compile_to_ir(source, "confined-closure-release");
    assert!(ir.contains("closure_env_stack"));
    assert!(!ir.contains("tyrus_closure_release"));
    assert!(!ir.contains("tyrus_closure_retain"));
}

#[test]
fn escaping_closure_heap_allocates_non_copy_capture() {
    let source = r#"
        function make(): () => void {
            const label: string = "hello";
            const f: () => void = (): void => {
                console.log(label);
            };
            return f;
        }
        const main = (): void => {
            make()();
        };
    "#;
    let tokens = Token::lexer(source).filter_map(Result::ok).collect();
    let program = Parser::new(tokens).parse_program().unwrap();
    let output =
        std::env::temp_dir().join(format!("tyrus-escaping-capture-{}", std::process::id()));
    compile(
        &program,
        "escaping_capture",
        output.to_str().unwrap(),
        false,
    )
    .unwrap();
    let ir_path = output.with_extension("ll");
    let ir = std::fs::read_to_string(&ir_path).unwrap();
    assert!(
        ir.contains("call ptr @malloc"),
        "escaping closure must heap-allocate its environment"
    );
    assert!(
        ir.contains("closure_environment"),
        "escaping closure must label its heap environment"
    );
    std::fs::remove_file(ir_path).unwrap();
}

#[test]
fn emits_else_if_chain() {
    let source = r#"
        const main = (): void => {
            const score: i64 = 75;
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
        };
    "#;
    let tokens = Token::lexer(source).filter_map(Result::ok).collect();
    let program = Parser::new(tokens).parse_program().unwrap();
    let mut checker = BorrowChecker::new();
    assert_eq!(checker.check_program(&program), Vec::<String>::new());

    let output = std::env::temp_dir().join(format!("tyrus-else-if-{}", std::process::id()));
    compile(&program, "else_if", output.to_str().unwrap(), false).unwrap();
    let ir = std::fs::read_to_string(output.with_extension("ll")).unwrap();
    assert!(
        ir.contains("br label"),
        "expected at least one branch in LLVM IR"
    );
    std::fs::remove_file(output.with_extension("ll")).unwrap();
}

#[test]
fn emits_unary_minus() {
    let source = r#"
        const main = (): void => {
            const val: i64 = 42;
            const result: i64 = -val;
            console.log(result);
        };
    "#;
    let tokens = Token::lexer(source).filter_map(Result::ok).collect();
    let program = Parser::new(tokens).parse_program().unwrap();
    let mut checker = BorrowChecker::new();
    assert_eq!(checker.check_program(&program), Vec::<String>::new());

    let output = std::env::temp_dir().join(format!("tyrus-unary-minus-{}", std::process::id()));
    compile(&program, "unary_minus", output.to_str().unwrap(), false).unwrap();
    let ir = std::fs::read_to_string(output.with_extension("ll")).unwrap();
    assert!(
        ir.contains("sub i64 0"),
        "expected i64 negation via `sub i64 0, %val`"
    );
    std::fs::remove_file(output.with_extension("ll")).unwrap();
}

#[test]
fn emits_div_by_zero_check() {
    let source = r#"
        const main = (): void => {
            const x: i32 = 10 / 0;
            const y: i32 = 10 % 0;
        };
    "#;
    let tokens = Token::lexer(source).filter_map(Result::ok).collect();
    let program = Parser::new(tokens).parse_program().unwrap();
    let output = std::env::temp_dir().join(format!("tyrus-div-zero-{}", std::process::id()));
    compile(&program, "div_zero", output.to_str().unwrap(), false).unwrap();
    let ir = std::fs::read_to_string(output.with_extension("ll")).unwrap();
    assert!(
        ir.contains("integer division by zero"),
        "expected div-by-zero check for `/`"
    );
    assert!(
        ir.contains("integer remainder by zero"),
        "expected div-by-zero check for `%`"
    );
    std::fs::remove_file(output.with_extension("ll")).unwrap();
}

#[test]
fn emits_runtime_validator_check_for_decorated_object_fields() {
    let source = r#"
        interface ProductFilter {
            @min(5)
            pageSize: i32;
        }

        const main = (): void => {
            const size: i32 = 3;
            const filter: ProductFilter = {
                pageSize: size,
            };
            console.log(filter);
        };
    "#;
    let tokens = Token::lexer(source).filter_map(Result::ok).collect();
    let program = Parser::new(tokens).parse_program().unwrap();
    let output = std::env::temp_dir().join(format!("tyrus-validator-field-{}", std::process::id()));
    compile(&program, "validator_field", output.to_str().unwrap(), false).unwrap();
    let ir = std::fs::read_to_string(output.with_extension("ll")).unwrap();
    assert!(ir.contains("validator.failure"));
    assert!(ir.contains("call void @tyrus_validation_issue_add"));
    assert!(ir.contains("c\"pageSize\""));
    assert!(ir.contains(
        "tyrus validation error: `@min(5)` failed for `pageSize`: value must be greater than or equal to 5"
    ));
    std::fs::remove_file(output.with_extension("ll")).unwrap();
}

#[test]
fn emits_named_validator_call_for_decorated_object_updates() {
    let source = r#"
        interface ProductFilter {
            @min(5)
            pageSize: i32;
        }

        const main = (): void => {
            const original: ProductFilter = { pageSize: 10 };
            const nextSize: i32 = 20;
            const updated: ProductFilter = { ...original, pageSize: nextSize };
            console.log(updated.pageSize);
        };
    "#;
    let tokens = Token::lexer(source).filter_map(Result::ok).collect();
    let program = Parser::new(tokens).parse_program().unwrap();
    let output = std::env::temp_dir().join(format!(
        "tyrus-validator-object-update-{}",
        std::process::id()
    ));
    compile(
        &program,
        "validator_object_update",
        output.to_str().unwrap(),
        false,
    )
    .unwrap();
    let ir = std::fs::read_to_string(output.with_extension("ll")).unwrap();
    assert!(
        ir.matches("call void @tyrus_validation_issues_begin")
            .count()
            >= 2
    );
    assert!(ir.matches("call void @tyrus_validation_issue_add").count() >= 2);
    std::fs::remove_file(output.with_extension("ll")).unwrap();
}

#[test]
fn emits_runtime_validator_check_for_decorated_parameters() {
    let source = r#"
        const divide = (a: i32, @notEqual(0) b: i32): i32 => {
            return a / b;
        };

        const main = (): void => {
            const divisor: i32 = 0;
            console.log(divide(10, divisor));
        };
    "#;
    let tokens = Token::lexer(source).filter_map(Result::ok).collect();
    let program = Parser::new(tokens).parse_program().unwrap();
    let output = std::env::temp_dir().join(format!("tyrus-validator-param-{}", std::process::id()));
    compile(&program, "validator_param", output.to_str().unwrap(), false).unwrap();
    let ir = std::fs::read_to_string(output.with_extension("ll")).unwrap();
    assert!(ir.contains("define void @tyrus_validate_param_divide_b_i32"));
    assert!(ir.contains("call void @tyrus_validate_param_divide_b_i32"));
    assert!(ir.contains("validator.failure"));
    assert!(ir.contains(
        "tyrus validation error: `@notEqual(0)` failed for `b`: value must be different from 0"
    ));
    assert!(!ir.contains("tyrus runtime error: integer division by zero"));
    std::fs::remove_file(output.with_extension("ll")).unwrap();
}

#[test]
fn emits_normalized_runtime_validator_checks() {
    let source = r#"
        interface ProductFilter {
            @min(1) @min(5) pageSize: i32;
        }

        const main = (): void => {
            const filter: ProductFilter = { pageSize: 7 };
            console.log(filter.pageSize);
        };
    "#;
    let tokens = Token::lexer(source).filter_map(Result::ok).collect();
    let program = Parser::new(tokens).parse_program().unwrap();
    let output =
        std::env::temp_dir().join(format!("tyrus-validator-normalized-{}", std::process::id()));
    compile(
        &program,
        "validator_normalized",
        output.to_str().unwrap(),
        false,
    )
    .unwrap();
    let ir = std::fs::read_to_string(output.with_extension("ll")).unwrap();
    assert!(!ir.contains("`@min(1)` failed for `pageSize`"));
    assert!(ir.contains("`@min(5)` failed for `pageSize`"));
    std::fs::remove_file(output.with_extension("ll")).unwrap();
}

#[test]
fn a_top_level_declaration_is_carried_into_main() {
    for (name, source) in [
        (
            "global_validated_sync_main",
            r#"
            interface Counter { @min(1) total: i32; }
            const counter: Counter = { total: 5 };
            const main = (): void => { console.log(counter.total); };
        "#,
        ),
        (
            "global_validated_async_main",
            r#"
            interface Counter { @min(1) total: i32; }
            const counter: Counter = { total: 5 };
            async function main(): Promise<void> { console.log(counter.total); }
        "#,
        ),
        (
            "global_plain_async_main",
            r#"
            const total: i32 = 5;
            async function main(): Promise<void> { console.log(total); }
        "#,
        ),
    ] {
        let tokens = Token::lexer(source).filter_map(Result::ok).collect();
        let program = Parser::new(tokens).parse_program().unwrap();
        let output = std::env::temp_dir().join(format!("tyrus-{name}-{}", std::process::id()));
        compile(&program, name, output.to_str().unwrap(), false)
            .unwrap_or_else(|error| panic!("`{name}` did not compile: {error}"));
        std::fs::remove_file(output.with_extension("ll")).unwrap();
    }
}

#[test]
fn emits_runtime_validator_check_for_port_decorator() {
    let source = r#"
        const bind = (@port() p: i32): void => {
            console.log(p);
        };

        const main = (): void => {
            const requested: i32 = 70000;
            bind(requested);
        };
    "#;
    let tokens = Token::lexer(source).filter_map(Result::ok).collect();
    let program = Parser::new(tokens).parse_program().unwrap();
    let output = std::env::temp_dir().join(format!("tyrus-validator-port-{}", std::process::id()));
    compile(&program, "validator_port", output.to_str().unwrap(), false).unwrap();
    let ir = std::fs::read_to_string(output.with_extension("ll")).unwrap();
    assert!(ir.contains("validator.port"));
    assert!(ir.contains(
        "tyrus validation error: `@port()` failed for `p`: value must be a registered port between 1024 and 49151"
    ));
    std::fs::remove_file(output.with_extension("ll")).unwrap();
}

#[test]
fn emits_runtime_validator_check_for_extended_numeric_decorators() {
    let source = r#"
        interface NumericInput {
            @equal(42)
            answer: i32;

            @equal(true)
            active: boolean;

            @notEqual(false)
            enabled: boolean;

            @powerOfTwo()
            capacity: u64;

            @safeInteger()
            ratio: f64;
        }

        const main = (): void => {
            const answer: i32 = 42;
            const active: boolean = true;
            const enabled: boolean = true;
            const capacity: u64 = 64;
            const ratio: f64 = 42.0;
            const input: NumericInput = {
                answer: answer,
                active: active,
                enabled: enabled,
                capacity: capacity,
                ratio: ratio,
            };
            console.log(input.answer);
        };
    "#;
    let tokens = Token::lexer(source).filter_map(Result::ok).collect();
    let program = Parser::new(tokens).parse_program().unwrap();
    let output = std::env::temp_dir().join(format!(
        "tyrus-validator-extended-numeric-{}",
        std::process::id()
    ));
    compile(
        &program,
        "validator_extended_numeric",
        output.to_str().unwrap(),
        false,
    )
    .unwrap();
    let ir = std::fs::read_to_string(output.with_extension("ll")).unwrap();
    assert!(ir.contains("validator.equal"));
    assert!(ir.contains("validator.bool"));
    assert!(ir.contains("validator.power_of_two"));
    assert!(ir.contains("validator.safe_integer"));
    assert!(ir.contains(
        "tyrus validation error: `@equal(42)` failed for `answer`: value must be equal to 42"
    ));
    assert!(ir.contains(
        "tyrus validation error: `@notEqual(false)` failed for `enabled`: value must be different from false"
    ));
    assert!(ir.contains(
        "tyrus validation error: `@powerOfTwo()` failed for `capacity`: value must be a power of two"
    ));
    assert!(ir.contains(
        "tyrus validation error: `@safeInteger()` failed for `ratio`: value must be a safe integer"
    ));
    std::fs::remove_file(output.with_extension("ll")).unwrap();
}

#[test]
fn validator_failure_throws_structured_validation_error() {
    let source = r#"
        const check = (@min(5) n: i32): void => {
            console.log(n);
        };
    "#;
    let tokens = Token::lexer(source).filter_map(Result::ok).collect();
    let program = Parser::new(tokens).parse_program().unwrap();
    let output = std::env::temp_dir().join(format!("tyrus-validator-throw-{}", std::process::id()));
    compile(&program, "validator_throw", output.to_str().unwrap(), false).unwrap();
    let ir = std::fs::read_to_string(output.with_extension("ll")).unwrap();
    assert!(
        ir.contains("validator.failure"),
        "expected a validator failure block"
    );
    assert!(
        ir.contains("validation.error.issues") && ir.contains("throw.longjmp"),
        "validator failure should throw a structured ValidationError, got:\n{ir}"
    );
    assert!(ir.contains("tyrus_validation_issue_add"));
    assert!(ir.contains("tyrus_validation_issues_count"));
    assert!(!ir.contains("call void @tyrus_panic_validation"));
    let failure_section = ir.split("validator.failure").nth(1).unwrap_or("");
    let next_block = failure_section
        .split("\n\n")
        .next()
        .unwrap_or(failure_section);
    assert!(
        !next_block.contains("@puts"),
        "validator failure must not print via puts (stdout)"
    );
    std::fs::remove_file(output.with_extension("ll")).unwrap();
}

#[test]
fn caught_validation_error_releases_aggregated_issues() {
    let source = r#"
        const check = (@min(5) n: i32): void => { console.log(n); };
        const read = (): i32 => 1;
        const main = (): void => {
            try { check(read()); }
            catch (error: ValidationError) { console.log(error.issues.length); return; }
        };
    "#;
    let tokens = Token::lexer(source).filter_map(Result::ok).collect();
    let program = Parser::new(tokens).parse_program().unwrap();
    let output =
        std::env::temp_dir().join(format!("tyrus-validator-cleanup-{}", std::process::id()));
    compile(
        &program,
        "validator_cleanup",
        output.to_str().unwrap(),
        false,
    )
    .unwrap();
    let ir = std::fs::read_to_string(output.with_extension("ll")).unwrap();
    assert!(ir.contains("call void @tyrus_validation_issues_free"));
    std::fs::remove_file(output.with_extension("ll")).unwrap();
}

#[test]
fn validation_failure_releases_owned_temporaries_before_longjmp() {
    let source = r#"
        interface Input { @minLength(3) name: string; }
        const read = (): string => "x";
        const check = (@minLength(3) name: string): void => { console.log(name); };
        const main = (): void => {
            try { const input: Input = { name: read() }; console.log(input.name); }
            catch (_error: ValidationError) {}
            try { check(read()); }
            catch (_error: ValidationError) {}
        };
    "#;
    let tokens = Token::lexer(source).filter_map(Result::ok).collect();
    let program = Parser::new(tokens).parse_program().unwrap();
    let output = std::env::temp_dir().join(format!(
        "tyrus-validator-failure-cleanup-{}",
        std::process::id()
    ));
    compile(
        &program,
        "validator_failure_cleanup",
        output.to_str().unwrap(),
        false,
    )
    .unwrap();
    let ir = std::fs::read_to_string(output.with_extension("ll")).unwrap();
    let failure_blocks = ir
        .split("\n\n")
        .filter(|block| block.starts_with("validation.collection.failure"))
        .collect::<Vec<_>>();
    assert!(
        failure_blocks.iter().any(|block| {
            block.contains("call void @free(")
                || block.contains("call void @tyrus_string_release_buffer(")
        }) && ir
            .matches("call void @tyrus_string_release_buffer(")
            .count()
            >= 4,
        "object and parameter validation failures must release their owned temporaries:\n{ir}"
    );
    std::fs::remove_file(output.with_extension("ll")).unwrap();
}

#[test]
fn optional_argument_box_is_released_when_validation_rejects_before_the_call() {
    let ir = compile_to_ir(
        r#"
            const read = (): i32 => 1;
            const check = (@min(5) value?: i32): void => { console.log(value ?? 0); };
            const main = (): void => {
                try { check(read()); }
                catch (_error: ValidationError) {}
            };
        "#,
        "optional-argument-validation-cleanup",
    );
    let failure_blocks = ir
        .split("\n\n")
        .filter(|block| block.starts_with("validation.collection.failure"))
        .collect::<Vec<_>>();
    assert!(
        failure_blocks
            .iter()
            .any(|block| block.contains("call void @free(")),
        "pre-call validation failure must release the caller-owned optional box:\n{ir}"
    );
}

#[test]
fn returning_validation_issues_clones_before_releasing_the_catch_binding() {
    let source = r#"
        const check = (@min(5) n: i32): void => { console.log(n); };
        const collect = (): readonly ValidationIssue[] => {
            try { check(1); }
            catch (error: ValidationError) { return error.issues; }
            return [];
        };
    "#;
    let tokens = Token::lexer(source).filter_map(Result::ok).collect();
    let program = Parser::new(tokens).parse_program().unwrap();
    let output = std::env::temp_dir().join(format!(
        "tyrus-validator-return-cleanup-{}",
        std::process::id()
    ));
    compile(
        &program,
        "validator_return_cleanup",
        output.to_str().unwrap(),
        false,
    )
    .unwrap();
    let ir = std::fs::read_to_string(output.with_extension("ll")).unwrap();
    let clone_at = ir
        .find("call i32 @tyrus_array_clone_owned")
        .expect("the returned issue array should be deeply cloned");
    let free_at = ir
        .find("call void @tyrus_validation_issues_free")
        .expect("the original caught issue array should be released");
    assert!(
        clone_at < free_at,
        "the return clone must complete before catch cleanup releases its source:\n{ir}"
    );
    std::fs::remove_file(output.with_extension("ll")).unwrap();
}

#[test]
fn validation_error_cleanup_covers_return_break_and_continue() {
    let source = r#"
        const check = (@min(5) n: i32): void => { console.log(n); };
        const read = (): i32 => 1;
        const returnFromCatch = (): void => {
            try { check(read()); }
            catch (error: ValidationError) { console.log(error.message); return; }
        };
        const breakFromCatch = (): void => {
            while (true) {
                try { check(read()); }
                catch (error: ValidationError) { console.log(error.message); break; }
            }
        };
        const continueFromCatch = (): void => {
            let running: boolean = true;
            while (running) {
                running = false;
                try { check(read()); }
                catch (error: ValidationError) { console.log(error.message); continue; }
            }
        };
    "#;
    let tokens = Token::lexer(source).filter_map(Result::ok).collect();
    let program = Parser::new(tokens).parse_program().unwrap();
    let output = std::env::temp_dir().join(format!(
        "tyrus-validator-cleanup-flow-{}",
        std::process::id()
    ));
    compile(
        &program,
        "validator_cleanup_flow",
        output.to_str().unwrap(),
        false,
    )
    .unwrap();
    let ir = std::fs::read_to_string(output.with_extension("ll")).unwrap();
    assert!(
        ir.matches("call void @tyrus_validation_issues_free")
            .count()
            >= 3
    );
    std::fs::remove_file(output.with_extension("ll")).unwrap();
}

#[test]
fn rethrow_transfers_validation_issue_ownership_to_outer_catch() {
    let source = r#"
        const check = (@min(5) n: i32): void => { console.log(n); };
        const read = (): i32 => 1;
        const main = (): void => {
            try {
                try { check(read()); }
                catch (error: ValidationError) { throw error; }
            } catch (error: ValidationError) {
                console.log(error.issues.length);
            }
        };
    "#;
    let tokens = Token::lexer(source).filter_map(Result::ok).collect();
    let program = Parser::new(tokens).parse_program().unwrap();
    let output =
        std::env::temp_dir().join(format!("tyrus-validator-rethrow-{}", std::process::id()));
    compile(
        &program,
        "validator_rethrow",
        output.to_str().unwrap(),
        false,
    )
    .unwrap();
    let ir = std::fs::read_to_string(output.with_extension("ll")).unwrap();
    assert_eq!(
        ir.matches("call void @tyrus_validation_issues_free")
            .count(),
        1
    );
    std::fs::remove_file(output.with_extension("ll")).unwrap();
}

#[test]
fn emits_runtime_validator_check_for_regex_decorator() {
    let source = r#"
        interface RouteInput {
            @regex(/^[a-z0-9-]+$/i)
            slug: string;
        }

        const main = (): void => {
            const slug: string = "release-2026";
            const input: RouteInput = {
                slug: slug,
            };
            console.log(input.slug);
        };
    "#;
    let tokens = tyrus_core::lexer::lex(source).unwrap();
    let program = Parser::new(tokens).parse_program().unwrap();
    let output = std::env::temp_dir().join(format!("tyrus-validator-regex-{}", std::process::id()));
    compile(&program, "validator_regex", output.to_str().unwrap(), false).unwrap();
    let ir = std::fs::read_to_string(output.with_extension("ll")).unwrap();
    assert!(ir.contains("tyrus_string_matches_regex"));
    assert!(ir.contains("validator.regex"));
    assert!(ir.contains("(?i)^[a-z0-9-]+$"));
    assert!(ir.contains(
        "tyrus validation error: `@regex(/^[a-z0-9-]+$/i)` failed for `slug`: value must match /^[a-z0-9-]+$/i"
    ));
    std::fs::remove_file(output.with_extension("ll")).unwrap();
}

#[test]
fn emits_runtime_validator_check_for_membership_decorators() {
    let source = r#"
        enum Role: string {
            Admin = "admin",
            User = "user",
        }

        const allowedRoles: Role[] = [Role.Admin, Role.User];

        interface AccessInput {
            @in(allowedRoles)
            role: Role;

            @notIn([0, -1])
            status: i32;

            @in([true])
            active: boolean;
        }

        const main = (): void => {
            const role: Role = Role.Admin;
            const status: i32 = 1;
            const active: boolean = true;
            const input: AccessInput = {
                role: role,
                status: status,
                active: active,
            };
            console.log(input.role);
        };
    "#;
    let tokens = Token::lexer(source).filter_map(Result::ok).collect();
    let program = Parser::new(tokens).parse_program().unwrap();
    let output =
        std::env::temp_dir().join(format!("tyrus-validator-membership-{}", std::process::id()));
    compile(
        &program,
        "validator_membership",
        output.to_str().unwrap(),
        false,
    )
    .unwrap();
    let ir = std::fs::read_to_string(output.with_extension("ll")).unwrap();
    assert!(ir.contains("validator.in_string"));
    assert!(ir.contains("validator.in_integer"));
    assert!(ir.contains("validator.in_bool"));
    assert!(ir.contains(
        "tyrus validation error: `@in([\\22admin\\22, \\22user\\22])` failed for `role`: value must be one of [\\22admin\\22, \\22user\\22]"
    ));
    assert!(ir.contains(
        "tyrus validation error: `@notIn([0, -1])` failed for `status`: value must not be one of [0, -1]"
    ));
    std::fs::remove_file(output.with_extension("ll")).unwrap();
}

#[test]
fn emits_runtime_validator_check_for_optional_presence_decorators() {
    let source = r#"
        interface SessionInput {
            @notNull()
            token: string?;
        }

        const main = (): void => {
            const token: string? = "abc";
            const input: SessionInput = {
                token: token,
            };
            console.log(input.token);
        };
    "#;
    let tokens = Token::lexer(source).filter_map(Result::ok).collect();
    let program = Parser::new(tokens).parse_program().unwrap();
    let output =
        std::env::temp_dir().join(format!("tyrus-validator-presence-{}", std::process::id()));
    compile(
        &program,
        "validator_presence",
        output.to_str().unwrap(),
        false,
    )
    .unwrap();
    let ir = std::fs::read_to_string(output.with_extension("ll")).unwrap();
    assert!(ir.contains("validator.required"));
    assert!(ir.contains(
        "tyrus validation error: `@notNull()` failed for `token`: value must not be null"
    ));
    std::fs::remove_file(output.with_extension("ll")).unwrap();
}

#[test]
fn emits_overflow_check_for_add_in_debug() {
    let source = r#"
        const main = (): void => {
            const x: i32 = 2147483647 + 1;
        };
    "#;
    let tokens = Token::lexer(source).filter_map(Result::ok).collect();
    let program = Parser::new(tokens).parse_program().unwrap();
    let output = std::env::temp_dir().join(format!("tyrus-overflow-add-{}", std::process::id()));
    compile(&program, "overflow_add", output.to_str().unwrap(), false).unwrap();
    let ir = std::fs::read_to_string(output.with_extension("ll")).unwrap();
    assert!(
        ir.contains("overflow.error"),
        "expected overflow check for `+` in debug mode"
    );
    std::fs::remove_file(output.with_extension("ll")).unwrap();
}

#[test]
fn emits_overflow_check_for_sub_in_debug() {
    let source = r#"
        const main = (): void => {
            const x: i32 = -2147483648 - 1;
        };
    "#;
    let tokens = Token::lexer(source).filter_map(Result::ok).collect();
    let program = Parser::new(tokens).parse_program().unwrap();
    let output = std::env::temp_dir().join(format!("tyrus-overflow-sub-{}", std::process::id()));
    compile(&program, "overflow_sub", output.to_str().unwrap(), false).unwrap();
    let ir = std::fs::read_to_string(output.with_extension("ll")).unwrap();
    assert!(
        ir.contains("overflow.error"),
        "expected overflow check for `-` in debug mode"
    );
    std::fs::remove_file(output.with_extension("ll")).unwrap();
}

#[test]
fn emits_narrowing_cast_range_check_in_debug() {
    let source = r#"
        const take_i32 = (x: i32): void => {};
        const main = (): void => {
            const x: i64 = 1000;
            take_i32(x);
        };
    "#;
    let tokens = Token::lexer(source).filter_map(Result::ok).collect();
    let program = Parser::new(tokens).parse_program().unwrap();
    let output = std::env::temp_dir().join(format!("tyrus-narrow-cast-{}", std::process::id()));
    compile(&program, "narrow_cast", output.to_str().unwrap(), false).unwrap();
    let ir = std::fs::read_to_string(output.with_extension("ll")).unwrap();
    assert!(
        ir.contains("cast.range.error"),
        "expected range check for narrowing cast in debug mode"
    );
    std::fs::remove_file(output.with_extension("ll")).unwrap();
}

#[test]
fn keeps_overflow_and_range_checks_in_release() {
    let source = r#"
        const main = (): void => {
            const x: i32 = 2147483647 + 1;
            const y: u32 = u64.MAX as u32;
        };
    "#;
    let tokens = Token::lexer(source).filter_map(Result::ok).collect();
    let program = Parser::new(tokens).parse_program().unwrap();
    let output =
        std::env::temp_dir().join(format!("tyrus-overflow-release-{}", std::process::id()));
    compile(&program, "overflow_release", output.to_str().unwrap(), true).unwrap();
    let ir = std::fs::read_to_string(output.with_extension("ll")).unwrap();
    assert!(
        ir.contains("overflow.error"),
        "release must keep the signed overflow check"
    );
    assert!(
        ir.contains("cast.range.error"),
        "release must keep the narrowing cast range check"
    );
    std::fs::remove_file(output.with_extension("ll")).unwrap();
}

#[test]
fn emits_destructured_object_parameter() {
    let source = r#"
        interface Point { x: i32; y: i32; }

        const getX = ({x}: Point): i32 => x;
        const main = (): void => {
            console.log(getX({ x: 42, y: 0 }));
        };
    "#;
    let tokens: Vec<Token> = Token::lexer(source).filter_map(Result::ok).collect();
    let program = Parser::new(tokens).parse_program().unwrap();
    let output = std::env::temp_dir().join(format!("tyrus-destructure-obj-{}", std::process::id()));
    compile(&program, "destructure_obj", output.to_str().unwrap(), false).unwrap();
    let ir = std::fs::read_to_string(output.with_extension("ll")).unwrap();
    assert!(ir.contains("getX"));
    std::fs::remove_file(output.with_extension("ll")).unwrap();
}

#[test]
fn emits_tagged_union_struct_type() {
    let source = r#"
        const main = (): void => {
            const val: string | i32 = 42;
            console.log(val);
        };
    "#;
    let tokens: Vec<Token> = Token::lexer(source).filter_map(Result::ok).collect();
    let program = Parser::new(tokens).parse_program().unwrap();
    let output = std::env::temp_dir().join(format!("tyrus-union-struct-{}", std::process::id()));
    compile(&program, "union_example", output.to_str().unwrap(), false).unwrap();
    let ir = std::fs::read_to_string(output.with_extension("ll")).unwrap();
    assert!(ir.contains("{ i8, [2 x i64] }"));
    std::fs::remove_file(output.with_extension("ll")).unwrap();
}

#[test]
fn emits_runtime_typeof_on_union_var() {
    let source = r#"
        const main = (): void => {
            const val: string | i32 = 42;
            const t: string = typeof val;
            console.log(t);
        };
    "#;
    let tokens: Vec<Token> = Token::lexer(source).filter_map(Result::ok).collect();
    let program = Parser::new(tokens).parse_program().unwrap();
    let output = std::env::temp_dir().join(format!("tyrus-union-typeof-{}", std::process::id()));
    compile(&program, "union_typeof", output.to_str().unwrap(), false).unwrap();
    let ir = std::fs::read_to_string(output.with_extension("ll")).unwrap();
    assert!(ir.contains(r#"c"i32""#));
    std::fs::remove_file(output.with_extension("ll")).unwrap();
}

#[test]
fn emits_runtime_narrowing_with_typeof() {
    let source = r#"
        const main = (): void => {
            const val: string | i32 = "hello";
            if (typeof val === "string") {
                console.log("str", val);
            } else if (typeof val === "i32") {
                console.log("num", val);
            }
        };
    "#;
    let tokens: Vec<Token> = Token::lexer(source).filter_map(Result::ok).collect();
    let program = Parser::new(tokens).parse_program().unwrap();
    let output = std::env::temp_dir().join(format!("tyrus-union-narrow-{}", std::process::id()));
    compile(&program, "union_narrow", output.to_str().unwrap(), false).unwrap();
    let ir = std::fs::read_to_string(output.with_extension("ll")).unwrap();
    assert!(ir.contains("call i32 @memcmp"));
    assert!(ir.contains("then:"));
    assert!(ir.contains("merge:"));
    std::fs::remove_file(output.with_extension("ll")).unwrap();
}

#[test]
fn emits_wide_numeric_union_payload_without_truncating_storage() {
    let source = r#"
        const main = (): void => {
            const val: string | u128 = u128.MAX;
            if (typeof val === "u128") {
                console.log(val);
            }
        };
    "#;
    let tokens: Vec<Token> = Token::lexer(source).filter_map(Result::ok).collect();
    let program = Parser::new(tokens).parse_program().unwrap();
    let output = std::env::temp_dir().join(format!("tyrus-union-u128-{}", std::process::id()));
    compile(&program, "union_u128", output.to_str().unwrap(), false).unwrap();
    let ir = std::fs::read_to_string(output.with_extension("ll")).unwrap();
    assert!(ir.contains("{ i8, [1 x i128] }"));
    assert!(ir.contains(r#"c"u128""#));
    std::fs::remove_file(output.with_extension("ll")).unwrap();
}

#[test]
fn persists_a_copy_union_binding_across_another_await() {
    let source = r#"
        async function source(): Promise<boolean | i32> { return 42; }
        async function pause(): Promise<void> {}
        async function main(): Promise<void> {
            const result: boolean | i32 = await source();
            await pause();
            if (typeof result === "boolean") {
                console.log(result);
            } else {
                console.log(result + 1);
            }
        }
    "#;
    let tokens = tyrus_core::lexer::lex(source).unwrap();
    let program = Parser::new(tokens).parse_program().unwrap();
    let output = std::env::temp_dir().join(format!(
        "tyrus-async-copy-union-local-{}",
        std::process::id()
    ));
    compile(
        &program,
        "async_copy_union_local",
        output.to_str().unwrap(),
        false,
    )
    .unwrap();
    let ir = std::fs::read_to_string(output.with_extension("ll")).unwrap();
    assert!(ir.contains("async.frame.local.result"));
    assert!(ir.contains("async.state.2"));
    let _ = std::fs::remove_file(output.with_extension("ll"));
    let _ = std::fs::remove_file(output);
}

#[test]
fn try_catch_emits_setjmp_and_context() {
    let source = r#"
        const main = (): void => {
            try {
                throw "something went wrong";
            } catch (e: string) {
                console.log(e);
            }
        };
    "#;
    let tokens: Vec<Token> = Token::lexer(source).filter_map(Result::ok).collect();
    let program = Parser::new(tokens).parse_program().unwrap();
    let output = std::env::temp_dir().join(format!("tyrus-try-catch-{}", std::process::id()));
    compile(&program, "try_catch", output.to_str().unwrap(), false).unwrap();
    let ir = std::fs::read_to_string(output.with_extension("ll")).unwrap();
    assert!(ir.contains("_setjmp"), "expected _setjmp call in IR");
    assert!(
        ir.contains("tyrus.catch.top"),
        "expected TLS catch context in IR"
    );
    assert!(ir.contains("try.catch.body"), "expected catch body block");
    std::fs::remove_file(output.with_extension("ll")).unwrap();
}

#[test]
fn try_catch_finally_emits_after_block() {
    let source = r#"
        const main = (): void => {
            try {
                throw "err";
            } catch (e: string) {
                console.log(e);
            } finally {
                console.log("cleanup");
            }
        };
    "#;
    let tokens: Vec<Token> = Token::lexer(source).filter_map(Result::ok).collect();
    let program = Parser::new(tokens).parse_program().unwrap();
    let output = std::env::temp_dir().join(format!("tyrus-try-finally-{}", std::process::id()));
    compile(&program, "try_finally", output.to_str().unwrap(), false).unwrap();
    let ir = std::fs::read_to_string(output.with_extension("ll")).unwrap();
    assert!(ir.contains("try.after"), "expected after block in IR");
    assert!(ir.contains("try.merge"), "expected merge block in IR");
    std::fs::remove_file(output.with_extension("ll")).unwrap();
}

#[test]
fn finally_runs_for_early_return_and_rethrow_paths() {
    let source = r#"
        const finishEarly = (): i32 => {
            try {
                return 42;
            } catch (error: string) {
                return 0;
            } finally {
                console.log("return cleanup");
            }
        };

        const rethrow = (): void => {
            try {
                throw "inner";
            } catch (error: string) {
                throw error;
            } finally {
                console.log("throw cleanup");
            }
        };
    "#;
    let tokens: Vec<Token> = Token::lexer(source).filter_map(Result::ok).collect();
    let program = Parser::new(tokens).parse_program().unwrap();
    let output = std::env::temp_dir().join(format!(
        "tyrus-try-finally-control-flow-{}",
        std::process::id()
    ));
    compile(
        &program,
        "try_finally_control_flow",
        output.to_str().unwrap(),
        false,
    )
    .unwrap();
    let ir = std::fs::read_to_string(output.with_extension("ll")).unwrap();
    assert!(ir.contains("return cleanup"));
    assert!(ir.contains("throw cleanup"));
    assert!(ir.contains("throw.longjmp"));
    std::fs::remove_file(output.with_extension("ll")).unwrap();
}

#[test]
fn user_main_with_arithmetic_and_for_of_emits_valid_ir() {
    let source = r#"
        const main = (): void => {
            let total: i32 = 5;
            total = total + 10;
            console.log(total);
            const items: i32[] = [1, 2, 3];
            for (const n of items) {
                console.log(n);
            }
        };
    "#;
    let tokens = Token::lexer(source).filter_map(Result::ok).collect();
    let program = Parser::new(tokens).parse_program().unwrap();
    let output = std::env::temp_dir().join(format!("tyrus-user-main-{}", std::process::id()));
    compile(&program, "user_main", output.to_str().unwrap(), false).unwrap();
    let _ = std::fs::remove_file(output.with_extension("ll"));
}

#[test]
fn explicit_main_call_is_rejected_cleanly() {
    let source = r#"
        const main = (): void => {
            console.log(1);
        };
        main();
    "#;
    let tokens = Token::lexer(source).filter_map(Result::ok).collect();
    let program = Parser::new(tokens).parse_program().unwrap();
    let output = std::env::temp_dir().join(format!("tyrus-main-call-{}", std::process::id()));
    let error = compile(&program, "main_call", output.to_str().unwrap(), false)
        .expect_err("calling an explicit `main` should be rejected");
    assert!(
        error.contains("runs automatically"),
        "expected a guiding diagnostic, got: {error}"
    );
    let _ = std::fs::remove_file(output.with_extension("ll"));
}

fn env_schema_for_ir(env_source: &str) -> tyrus_core::env::EnvSchema {
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
}

fn compile_with_env_to_ir(name: &str, env_source: &str, source: &str) -> String {
    let schema = env_schema_for_ir(env_source);
    let tokens = tyrus_core::lexer::lex(source).unwrap();
    let program = Parser::new(tokens).parse_program().unwrap();
    let output = std::env::temp_dir().join(format!("tyrus-{name}-{}", std::process::id()));
    compile_with_env(
        &program,
        name,
        output.to_str().unwrap(),
        false,
        Some(&schema),
    )
    .unwrap();
    let ir_path = output.with_extension("ll");
    let ir = std::fs::read_to_string(&ir_path).unwrap();
    std::fs::remove_file(ir_path).unwrap();
    ir
}

#[test]
fn env_startup_runs_before_user_code_in_generated_main() {
    let ir = compile_with_env_to_ir(
        "env_startup",
        "@type(u32)\n@min(1024)\nPORT=3000\nTYRUS_ENV=development\n",
        r#"
            const port: u32 = process.env.PORT;
            console.log(port);
        "#,
    );
    let init = ir.find("call void @tyrus_env_init").expect("env init call");
    let require = ir
        .find("call void @tyrus_env_require")
        .expect("env require call");
    let user_read = ir
        .find("call i64 @tyrus_env_get_u64")
        .expect("typed getter");
    assert!(init < require);
    assert!(require < user_read);
    assert!(ir.contains("tyrus_panic_validation"));
    assert_eq!(ir.matches("call void @tyrus_env_require").count(), 2);
}

#[test]
fn env_startup_runs_inside_an_explicit_main() {
    let ir = compile_with_env_to_ir(
        "env_explicit_main",
        "@type(boolean)\nDEBUG=true\n",
        r#"
            const main = (): void => {
                const debug: boolean = process.env.DEBUG;
                console.log(debug);
            };
        "#,
    );
    assert!(ir.contains("call void @tyrus_env_init"));
    assert!(ir.contains("call void @tyrus_env_require"));
    assert!(ir.contains("call i1 @tyrus_env_get_bool"));
}

#[test]
fn sealed_env_startup_uses_binary_relative_runtime_loader() {
    let schema = env_schema_for_ir("@type(u32)\nPORT=3000\n");
    let tokens = tyrus_core::lexer::lex(
        r#"
            const port: u32 = process.env.PORT;
            console.log(port);
        "#,
    )
    .unwrap();
    let program = Parser::new(tokens).parse_program().unwrap();
    let output = std::env::temp_dir().join(format!("tyrus-env-sealed-{}", std::process::id()));
    compile_with_env_options(
        &program,
        "sealed-app",
        output.to_str().unwrap(),
        false,
        Some(&schema),
        Some(&SealedEnvMetadata {
            project_name: "sealed-app".to_string(),
            fingerprint: "abc123".to_string(),
        }),
    )
    .unwrap();
    let ir_path = output.with_extension("ll");
    let ir = std::fs::read_to_string(&ir_path).unwrap();
    std::fs::remove_file(ir_path).unwrap();
    assert!(ir.contains("call void @tyrus_env_init_sealed"));
    assert!(ir.contains("sealed-app"));
    assert!(ir.contains("abc123"));
    assert!(!ir.contains("call void @tyrus_env_init("));
}

#[test]
fn env_destructuring_reads_each_variable_through_getters() {
    let ir = compile_with_env_to_ir(
        "env_destructure",
        "@type(u32)\nPORT=3000\nHOST=localhost\n",
        r#"
            const { PORT, HOST } = process.env;
            console.log(HOST);
            console.log(PORT);
        "#,
    );
    assert!(ir.contains("call i64 @tyrus_env_get_u64"));
    assert!(ir.contains("call { ptr, i64 } @tyrus_env_get_str"));
}

#[test]
fn pipeline_desugars_to_nested_calls() {
    let source = r#"
        const double = (n: i64): i64 => n * 2;
        const main = (): void => {
            const value: i64 = 5 |> double |> double;
            console.log(value);
        };
    "#;
    let tokens = tyrus_core::lexer::lex(source).unwrap();
    let program = Parser::new(tokens).parse_program().unwrap();
    let output = std::env::temp_dir().join(format!("tyrus-pipe-{}", std::process::id()));
    compile(&program, "pipe", output.to_str().unwrap(), false).unwrap();
    let ir = std::fs::read_to_string(output.with_extension("ll")).unwrap();
    assert_eq!(ir.matches("call i64 @double").count(), 2);
    std::fs::remove_file(output.with_extension("ll")).unwrap();
}

#[test]
fn topic_pipeline_lowers_to_explicit_topic_expression() {
    let source = r#"
        const add = (a: i64, b: i64): i64 => a + b;
        const main = (): void => {
            const value: i64 = 5 |> add(10, %) |> add(%, 2);
            console.log(value);
        };
    "#;
    let tokens = tyrus_core::lexer::lex(source).unwrap();
    let program = Parser::new(tokens).parse_program().unwrap();
    let output = std::env::temp_dir().join(format!("tyrus-topic-pipe-{}", std::process::id()));
    compile(&program, "topic_pipe", output.to_str().unwrap(), false).unwrap();
    let ir = std::fs::read_to_string(output.with_extension("ll")).unwrap();
    assert_eq!(ir.matches("call i64 @add").count(), 2);
    std::fs::remove_file(output.with_extension("ll")).unwrap();
}

#[test]
fn cli_prompts_lower_to_runtime_calls() {
    let source = r#"
        const main = (): void => {
            const name: string = __tyrus_cli_input({ message: "Name?", default: "app" });
            const secret: string = __tyrus_cli_password({ message: "Token?", confirm: true });
            const git: boolean = __tyrus_cli_confirm({ message: "Init git?", default: true });
            const target: string = __tyrus_cli_select({ message: "Target?", choices: ["script", "cli"] });
            const features: string[] = __tyrus_cli_multiselect({ message: "Features?", choices: ["a", "b"] });
            const ratio: f64 = __tyrus_cli_number({ message: "Ratio?", min: 1.0, max: 65535.0 });
            const port: i64 = __tyrus_cli_integer({ message: "Port?", default: 3000, min: 1, max: 65535 });
            console.log(name);
            console.log(secret);
            console.log(target);
            console.log(git);
            console.log(features[0]);
            console.log(ratio > 0.0);
            console.log(port);
        };
    "#;
    let tokens = tyrus_core::lexer::lex(source).unwrap();
    let program = Parser::new(tokens).parse_program().unwrap();
    let output = std::env::temp_dir().join(format!("tyrus-cli-{}", std::process::id()));
    compile(&program, "cli", output.to_str().unwrap(), false).unwrap();
    let ir = std::fs::read_to_string(output.with_extension("ll")).unwrap();
    assert!(ir.contains("call { ptr, i64 } @__tyrus_cli_input"));
    assert!(ir.contains("call { ptr, i64 } @__tyrus_cli_password"));
    assert!(ir.contains("call i1 @__tyrus_cli_confirm"));
    assert!(ir.contains("call { ptr, i64 } @__tyrus_cli_select"));
    // multiselect returns via a void call taking an out-parameter.
    assert!(ir.contains("call void @__tyrus_cli_multiselect"));
    assert!(ir.contains("call double @__tyrus_cli_number"));
    assert!(ir.contains("call i64 @__tyrus_cli_integer"));
    std::fs::remove_file(output.with_extension("ll")).unwrap();
}

#[test]
fn style_text_lowers_to_tyrus_style_with_compile_time_ansi() {
    let source = r#"
        import { styleText } from "tyrus:utils";
        const main = (): void => {
            console.log(styleText(["bold", "red"], "hi"));
            console.log(styleText.green("ok"));
        };
    "#;
    let tokens = tyrus_core::lexer::lex(source).unwrap();
    let program = Parser::new(tokens).parse_program().unwrap();
    let output = std::env::temp_dir().join(format!("tyrus-styletext-{}", std::process::id()));
    compile(&program, "styletext", output.to_str().unwrap(), false).unwrap();
    let ir = std::fs::read_to_string(output.with_extension("ll")).unwrap();
    assert_eq!(ir.matches("call { ptr, i64 } @tyrus_style").count(), 2);
    assert!(ir.contains("1B") || ir.contains("\\1B") || ir.contains("[1m"));
    std::fs::remove_file(output.with_extension("ll")).unwrap();
}

#[test]
fn figlet_lowers_to_runtime_call_with_compile_time_font() {
    let source = r#"
        import { figlet } from "tyrus:utils";
        const main = (): void => {
            console.log(figlet("HELLO"));
            console.log(figlet("HELLO", "slant"));
        };
    "#;
    let tokens = tyrus_core::lexer::lex(source).unwrap();
    let program = Parser::new(tokens).parse_program().unwrap();
    let output = std::env::temp_dir().join(format!("tyrus-figlet-{}", std::process::id()));
    compile(&program, "figlet", output.to_str().unwrap(), false).unwrap();
    let ir = std::fs::read_to_string(output.with_extension("ll")).unwrap();
    assert_eq!(ir.matches("call { ptr, i64 } @tyrus_figlet").count(), 2);
    assert!(ir.contains("standard"));
    assert!(ir.contains("slant"));
    std::fs::remove_file(output.with_extension("ll")).unwrap();
}

#[test]
fn lowers_optional_field_access_to_a_null_guarded_read() {
    let source = r#"
        const name: string? = "abc";
        const size: usize? = name?.length;
        console.log(size ?? 0);
    "#;
    let tokens = tyrus_core::lexer::lex(source).unwrap();
    let program = Parser::new(tokens).parse_program().unwrap();
    let output = std::env::temp_dir().join(format!("tyrus-opt-field-{}", std::process::id()));
    compile(&program, "opt_field", output.to_str().unwrap(), false).unwrap();
    let ir = std::fs::read_to_string(output.with_extension("ll")).unwrap();
    assert!(ir.contains("optional.field.is_null"));
    assert!(ir.contains("optional.field.absent"));
    assert!(ir.contains("optional.field.present"));
    std::fs::remove_file(output.with_extension("ll")).unwrap();
}

#[test]
fn terminates_the_outer_merge_block_of_a_top_level_else_if() {
    let source = r#"
        const value: i32 = 5;
        if (value > 10) {
            console.log("high");
        } else if (value > 3) {
            console.log("medium");
        } else {
            console.log("low");
        }
        console.log("done");
    "#;
    let tokens = tyrus_core::lexer::lex(source).unwrap();
    let program = Parser::new(tokens).parse_program().unwrap();
    let output = std::env::temp_dir().join(format!("tyrus-else-if-{}", std::process::id()));
    compile(&program, "else_if", output.to_str().unwrap(), false).unwrap();
    let ir = std::fs::read_to_string(output.with_extension("ll")).unwrap();
    assert!(ir.contains("ret i32 0"));
    std::fs::remove_file(output.with_extension("ll")).unwrap();
}

#[test]
fn infers_an_omitted_return_type_from_the_function_body() {
    let source = r#"
        interface Point {
            x: i32;
        }
        function add(left: i32, right: i32) {
            return left + right;
        }
        function sign(value: i32) {
            if (value > 0) {
                return 1;
            }
            return -1;
        }
        function announce(message: string) {
            console.log(message);
        }
        function widen(value: i32) {
            return annotated(value);
        }
        function annotated(value: i32): i64 {
            return 7;
        }
        function origin() {
            const point: Point = { x: 7 };
            return point;
        }
        console.log(add(2, 3));
        console.log(sign(5));
        announce("ok");
        console.log(widen(1));
        console.log(origin().x);
    "#;
    let tokens = tyrus_core::lexer::lex(source).unwrap();
    let program = Parser::new(tokens).parse_program().unwrap();
    let output = std::env::temp_dir().join(format!("tyrus-inferred-ret-{}", std::process::id()));
    compile(&program, "inferred_ret", output.to_str().unwrap(), false).unwrap();
    let ir = std::fs::read_to_string(output.with_extension("ll")).unwrap();
    assert!(ir.contains("define i32 @add"));
    assert!(ir.contains("define i32 @sign"));
    assert!(ir.contains("define void @announce"));
    assert!(ir.contains("define i64 @widen"));
    assert!(ir.contains("define %Point @origin") || ir.contains("@origin"));
    std::fs::remove_file(output.with_extension("ll")).unwrap();
}

#[test]
fn infers_a_return_type_that_depends_on_a_later_inferred_function() {
    let source = r#"
        function outer(value: i32) {
            return inner(value);
        }
        function inner(value: i32) {
            return value * 2;
        }
    "#;
    let tokens = tyrus_core::lexer::lex(source).unwrap();
    let program = Parser::new(tokens).parse_program().unwrap();
    let output = std::env::temp_dir().join(format!("tyrus-chained-ret-{}", std::process::id()));
    compile(&program, "chained_ret", output.to_str().unwrap(), false).unwrap();
    let ir = std::fs::read_to_string(output.with_extension("ll")).unwrap();
    assert!(ir.contains("define i32 @outer"));
    assert!(ir.contains("define i32 @inner"));
    std::fs::remove_file(output.with_extension("ll")).unwrap();
}

#[test]
fn lowers_a_never_returning_function_to_a_void_signature() {
    let source = r#"
        function fail(message: string): never {
            throw message;
        }
        console.log("ok");
    "#;
    let tokens = tyrus_core::lexer::lex(source).unwrap();
    let program = Parser::new(tokens).parse_program().unwrap();
    let output = std::env::temp_dir().join(format!("tyrus-never-fn-{}", std::process::id()));
    compile(&program, "never_fn", output.to_str().unwrap(), false).unwrap();
    let ir = std::fs::read_to_string(output.with_extension("ll")).unwrap();
    assert!(ir.contains("define void @fail"));
    std::fs::remove_file(output.with_extension("ll")).unwrap();
}

#[test]
fn switch_coerces_case_literals_to_the_subject_width() {
    let source = r#"
        const value: i64 = 2;
        switch (value) {
            case 1: { console.log("one"); }
            case 2: { console.log("two"); }
            default: { console.log("other"); }
        }
    "#;
    let tokens = tyrus_core::lexer::lex(source).unwrap();
    let program = Parser::new(tokens).parse_program().unwrap();
    let output = std::env::temp_dir().join(format!("tyrus-switch-i64-{}", std::process::id()));
    compile(&program, "switch_i64", output.to_str().unwrap(), false).unwrap();
    let ir = std::fs::read_to_string(output.with_extension("ll")).unwrap();
    assert!(ir.contains("switch i64"));
    assert!(ir.contains("i64 1, label"));
    assert!(ir.contains("i64 2, label"));
    assert!(!ir.contains("i32 1, label"));
    assert!(!ir.contains("i32 2, label"));
    std::fs::remove_file(output.with_extension("ll")).unwrap();
}

#[test]
fn a_switch_over_constant_labels_becomes_a_jump_table() {
    let source = r#"
        function classify(n: i32): i32 {
            switch (n) {
                case 0: return 10;
                case 1: return 20;
                case 2: return 30;
                default: return 0;
            }
        }
        console.log(classify(1));
    "#;
    let ir = compile_to_ir(source, "switch-jump-table");
    assert!(ir.contains("switch i32"));
    assert!(!ir.contains("switch.cmp"));
    assert!(ir.contains("i32 0, label"));
    assert!(ir.contains("i32 1, label"));
    assert!(ir.contains("i32 2, label"));
}

#[test]
fn a_switch_over_enum_members_becomes_a_jump_table() {
    let source = r#"
        enum Color { Red, Green, Blue }
        function name(c: Color): i32 {
            switch (c) {
                case Color.Red: return 1;
                case Color.Green: return 2;
                case Color.Blue: return 3;
            }
            return 0;
        }
        console.log(name(Color.Green));
    "#;
    let ir = compile_to_ir(source, "switch-jump-table-enum");
    assert!(ir.contains("switch i32"));
    assert!(!ir.contains("switch.cmp"));
}

#[test]
fn a_switch_with_a_duplicate_label_stays_on_the_comparison_chain() {
    let source = r#"
        function classify(n: i32): i32 {
            switch (n) {
                case 1: return 10;
                case 1: return 20;
                default: return 0;
            }
        }
        console.log(classify(1));
    "#;
    let ir = compile_to_ir(source, "switch-duplicate-label");
    assert!(!ir.contains("switch i32"));
    assert!(ir.contains("switch.cmp"));
}

#[test]
fn a_ternary_over_scalar_operands_becomes_a_select() {
    let source = r#"
        function pick(a: i32, b: i32): i32 {
            return a > b ? a : b;
        }
        console.log(pick(7, 4));
    "#;
    let ir = compile_to_ir(source, "ternary-select");
    assert!(ir.contains("tern.select"));
    assert!(!ir.contains("tern.then"));
    assert!(!ir.contains("tern.phi"));
}

#[test]
fn a_ternary_widens_a_bare_literal_arm_to_the_other_arms_width() {
    let source = r#"
        function clamp(n: i64): i64 {
            return n > 3 ? 0 : n;
        }
        console.log(clamp(7));
    "#;
    let ir = compile_to_ir(source, "ternary-literal-width");
    assert!(ir.contains("i64"));
    assert!(!ir.contains("phi i32"));
    assert!(ir.contains("tern.select"));
}

#[test]
fn a_ternary_whose_arm_can_abort_keeps_its_branches() {
    let source = r#"
        function guarded(a: i32, b: i32): i32 {
            return b === 0 ? 0 : a * b;
        }
        console.log(guarded(6, 7));
    "#;
    let ir = compile_to_ir(source, "ternary-guarded");
    assert!(ir.contains("tern.then"));
    assert!(ir.contains("tern.phi"));
}

#[test]
fn lowers_multiple_direct_awaits_inside_switch_to_resumable_cases() {
    let source = r#"
        async function main(): Promise<void> {
            if (true) {
                switch (await Promise.resolve<string>("selected")) {
                    case await Promise.resolve<string>("selected"): {
                        await Promise.resolve<void>();
                        await Promise.resolve<void>();
                        break;
                    }
                    default: {
                        await Promise.resolve<void>();
                        break;
                    }
                }
            }
            console.log("done");
        }
    "#;
    let tokens = tyrus_core::lexer::lex(source).unwrap();
    let program = Parser::new(tokens).parse_program().unwrap();
    let output = std::env::temp_dir().join(format!("tyrus-async-switch-{}", std::process::id()));
    compile(&program, "async_switch", output.to_str().unwrap(), false).unwrap();
    let ir = std::fs::read_to_string(output.with_extension("ll")).unwrap();
    assert!(ir.contains("async.switch.case.0"));
    assert!(ir.contains("async.switch.end"));
    assert!(ir.contains("async.frame.local.__tyrus_async_switch_subject_"));
    assert!(ir.contains("async.switch.awaited.selected"));
    assert!(ir.contains("async.switch.context.if.join"));
    assert!(ir.contains("async.await.ready"));
    std::fs::remove_file(output.with_extension("ll")).unwrap();
}

#[test]
fn produces_empty_optionals_as_null_pointers() {
    let source = r#"
        function findValue(found: boolean): i32? {
            if (found) {
                return 7;
            }
            return;
        }
        let pending: i32?;
        console.log(pending ?? 0);
        console.log(findValue(false) ?? -1);
    "#;
    let tokens = tyrus_core::lexer::lex(source).unwrap();
    let program = Parser::new(tokens).parse_program().unwrap();
    let output = std::env::temp_dir().join(format!("tyrus-empty-opt-{}", std::process::id()));
    compile(&program, "empty_opt", output.to_str().unwrap(), false).unwrap();
    let ir = std::fs::read_to_string(output.with_extension("ll")).unwrap();
    assert!(ir.contains("ret ptr null"));
    std::fs::remove_file(output.with_extension("ll")).unwrap();
}

#[test]
fn lowers_explicit_none_returns_and_initializers_to_empty_optionals() {
    let source = r#"
        function findValue(): i32? {
            return None;
        }
        const absent: i32? = None;
        console.log(absent ?? 0);
        console.log(findValue() ?? -1);
    "#;
    let tokens = tyrus_core::lexer::lex(source).unwrap();
    let program = Parser::new(tokens).parse_program().unwrap();
    let output = std::env::temp_dir().join(format!("tyrus-explicit-none-{}", std::process::id()));
    compile(&program, "explicit_none", output.to_str().unwrap(), false).unwrap();
    let ir = std::fs::read_to_string(output.with_extension("ll")).unwrap();
    assert!(
        ir.contains("@absent = global ptr null")
            && ir.contains("store ptr null, ptr @absent")
            && ir.contains("store ptr null, ptr %owned.return"),
        "explicit `None` was not lowered directly to an empty optional pointer:\n{ir}"
    );
    std::fs::remove_file(output.with_extension("ll")).unwrap();
    let _ = std::fs::remove_file(output);
}

#[test]
fn materializes_a_named_function_passed_as_an_argument() {
    // Passing a named function as a value used to abort codegen (`not in codegen
    // scope`). It now becomes a closure `{ thunk, null }` whose thunk adapts the
    // environment-first closure ABI to the plain function signature.
    let source = r#"
        function apply(f: (i32) => i32, v: i32): i32 {
            return f(v);
        }
        const double = (x: i32): i32 => x * 2;
        console.log(apply(double, 21));
    "#;
    let tokens = tyrus_core::lexer::lex(source).unwrap();
    let program = Parser::new(tokens).parse_program().unwrap();
    let output = std::env::temp_dir().join(format!("tyrus-fnvalue-{}", std::process::id()));
    compile(&program, "fn_value", output.to_str().unwrap(), false).unwrap();
    let ir = std::fs::read_to_string(output.with_extension("ll")).unwrap();
    assert!(ir.contains("@__tyrus_fnval_double"));
    std::fs::remove_file(output.with_extension("ll")).unwrap();
}

#[test]
fn lowers_optional_conditions_and_logical_statements() {
    let source = r#"
        function effect(): void {
            console.log("ran");
        }
        function run(name: string?): void {
            const label: string = name ? name : "anon";
            console.log(label);
            name && effect();
        }
        run("hi");
    "#;
    let tokens = tyrus_core::lexer::lex(source).unwrap();
    let program = Parser::new(tokens).parse_program().unwrap();
    let output = std::env::temp_dir().join(format!("tyrus-optcond-{}", std::process::id()));
    compile(&program, "opt_cond", output.to_str().unwrap(), false).unwrap();
    let ir = std::fs::read_to_string(output.with_extension("ll")).unwrap();
    assert!(ir.contains("optional.is_present"));
    assert!(ir.contains("logical.stmt.right"));
    std::fs::remove_file(output.with_extension("ll")).unwrap();
}

#[test]
fn lowers_for_of_with_destructuring() {
    let source = r#"
        const pairs: [i32, i32][] = [[1, 2], [3, 4]];
        for (const [a, b] of pairs) {
            console.log(a + b);
        }
    "#;
    let tokens = tyrus_core::lexer::lex(source).unwrap();
    let program = Parser::new(tokens).parse_program().unwrap();
    let output = std::env::temp_dir().join(format!("tyrus-forof-destr-{}", std::process::id()));
    compile(&program, "forof_destr", output.to_str().unwrap(), false).unwrap();
    let ir = std::fs::read_to_string(output.with_extension("ll")).unwrap();
    assert!(ir.contains("forof"));
    std::fs::remove_file(output.with_extension("ll")).unwrap();
}

#[test]
fn parse_builtins_use_the_runtime_entry_point_for_the_annotated_width() {
    let source = r#"
        const main = (): void => {
            const port: i32? = parseInt("8080");
            const count: u32? = parseInt("7");
            const size: usize? = parseInt("9");
            const wide: i64? = parseInt("9999999999");
            const ratio: f32? = parseFloat("3.5");
            const whole: f32? = Number("2.5");
            const fallback = parseInt("42");
            console.log(port ?? -1);
            console.log(count ?? 0);
            console.log(size ?? 0);
            console.log(wide ?? -1);
            console.log(ratio ?? 0.0);
            console.log(whole ?? 0.0);
            console.log(fallback ?? -1);
        };
    "#;
    let tokens = Token::lexer(source).filter_map(Result::ok).collect();
    let program = Parser::new(tokens).parse_program().unwrap();
    let mut checker = BorrowChecker::new();
    assert_eq!(checker.check_program(&program), Vec::<String>::new());

    let output = std::env::temp_dir().join(format!("tyrus-parse-width-{}", std::process::id()));
    compile(&program, "parse_width", output.to_str().unwrap(), false).unwrap();
    let ir_path = output.with_extension("ll");
    let ir = std::fs::read_to_string(&ir_path).unwrap();
    for symbol in [
        "tyrus_parse_i32",
        "tyrus_parse_u32",
        "tyrus_parse_usize",
        "tyrus_parse_i64",
        "tyrus_parse_f32_leading",
        "tyrus_parse_f32_whole",
    ] {
        assert!(ir.contains(symbol), "missing `{symbol}` in IR");
    }
    let _ = std::fs::remove_file(&ir_path);
}

#[test]
fn a_plain_cast_checks_the_bounds_of_both_types() {
    let source = r#"
        const main = (): void => {
            const negative: i32 = -5;
            console.log(negative as u32);
        };
    "#;
    let tokens = Token::lexer(source).filter_map(Result::ok).collect();
    let program = Parser::new(tokens).parse_program().unwrap();
    let output = std::env::temp_dir().join(format!("tyrus-sign-cast-{}", std::process::id()));
    compile(&program, "sign_cast", output.to_str().unwrap(), false).unwrap();
    let ir_path = output.with_extension("ll");
    let ir = std::fs::read_to_string(&ir_path).unwrap();
    assert!(
        ir.contains("cast.range.error"),
        "a same-width cast that changes signedness must be checked"
    );
    let _ = std::fs::remove_file(&ir_path);

    let source = r#"
        const main = (): void => {
            const small: i32 = 5;
            console.log(small as i64);
        };
    "#;
    let tokens = Token::lexer(source).filter_map(Result::ok).collect();
    let program = Parser::new(tokens).parse_program().unwrap();
    let output = std::env::temp_dir().join(format!("tyrus-widen-cast-{}", std::process::id()));
    compile(&program, "widen_cast", output.to_str().unwrap(), false).unwrap();
    let ir_path = output.with_extension("ll");
    let ir = std::fs::read_to_string(&ir_path).unwrap();
    assert!(
        !ir.contains("cast.range.error"),
        "a cast that always fits must not pay for a check"
    );
    let _ = std::fs::remove_file(&ir_path);
}

#[test]
fn an_optional_cast_range_checks_instead_of_truncating() {
    let source = r#"
        const main = (): void => {
            const narrowed = parseInt("9999999999") as i32?;
            const unsigned = parseInt("-5") as u32?;
            const single = parseFloat("3.5") as f32?;
            const truncated = parseFloat("2.9") as i32?;
            const wide: i64 = 9999999999;
            const scalar = wide as i32?;
            console.log(narrowed ?? -1);
            console.log(unsigned ?? 0);
            console.log(single ?? -1.0);
            console.log(truncated ?? -1);
            console.log(scalar ?? -1);
        };
    "#;
    let tokens = Token::lexer(source).filter_map(Result::ok).collect();
    let program = Parser::new(tokens).parse_program().unwrap();
    let mut checker = BorrowChecker::new();
    assert_eq!(checker.check_program(&program), Vec::<String>::new());

    let output = std::env::temp_dir().join(format!("tyrus-optional-cast-{}", std::process::id()));
    compile(&program, "optional_cast", output.to_str().unwrap(), false).unwrap();
    let ir_path = output.with_extension("ll");
    let ir = std::fs::read_to_string(&ir_path).unwrap();
    for symbol in [
        "cast.present",
        "cast.absent",
        "cast.payload",
        "cast.fits",
        "cast.empty",
        "fptosi",
    ] {
        assert!(ir.contains(symbol), "missing `{symbol}` in IR");
    }
    assert!(
        !ir.contains("cast.range.error"),
        "an optional cast must not abort on an out-of-range value"
    );
    let _ = std::fs::remove_file(&ir_path);
}

#[test]
fn validators_on_an_optional_target_guard_on_presence() {
    let source = r#"
        interface Profile {
            @email()
            mail?: string;

            @min(18)
            age?: i32;
        }

        const readMail = (): string => "a@b.com";
        const readAge = (): i32 => 30;

        const acceptAge = (@min(18) age?: i32): void => {
            console.log(age ?? -1);
        };

        const main = (): void => {
            try {
                const profile: Profile = { mail: readMail(), age: readAge() };
                console.log(profile.age ?? -1);
                acceptAge(readAge());
                acceptAge();
            } catch (error: ValidationError) {
                console.log(error.issues.length);
            }
        };
    "#;
    let tokens = Token::lexer(source).filter_map(Result::ok).collect();
    let program = Parser::new(tokens).parse_program().unwrap();
    let mut checker = BorrowChecker::new();
    assert_eq!(checker.check_program(&program), Vec::<String>::new());

    let output =
        std::env::temp_dir().join(format!("tyrus-optional-validator-{}", std::process::id()));
    compile(
        &program,
        "optional_validator",
        output.to_str().unwrap(),
        false,
    )
    .unwrap();
    let ir = std::fs::read_to_string(output.with_extension("ll")).unwrap();
    assert!(ir.contains("validator.present"), "missing presence guard");
    assert!(ir.contains("validator.absent"), "missing absent branch");
    assert!(ir.contains("validator.payload"), "missing payload load");
    let _ = std::fs::remove_file(output.with_extension("ll"));
}

#[test]
fn lowers_await_of_a_promise_held_in_a_local() {
    let source = r#"
        async function load(): Promise<i32> {
            return 21;
        }

        async function main(): Promise<void> {
            const pending = load();
            const value = await pending;
            console.log("value:", value);
        }
    "#;
    let tokens = tyrus_core::lexer::lex(source).unwrap();
    let program = Parser::new(tokens).parse_program().unwrap();
    let output = std::env::temp_dir().join(format!("tyrus-async-deferred-{}", std::process::id()));
    compile(&program, "async_deferred", output.to_str().unwrap(), false).unwrap();
    let ir = std::fs::read_to_string(output.with_extension("ll")).unwrap();
    assert!(ir.contains("tyrus_async_task_await"));
    assert!(ir.contains("async.await.binding.payload"));
    let _ = std::fs::remove_file(output.with_extension("ll"));
    let _ = std::fs::remove_file(output);
}

#[test]
fn boxes_an_awaited_payload_into_an_optional_binding() {
    let source = r#"
        async function load(): Promise<string> {
            return "value";
        }

        async function main(): Promise<void> {
            const text: string? = await load();
            console.log("text:", text ?? "none");
        }
    "#;
    let tokens = tyrus_core::lexer::lex(source).unwrap();
    let program = Parser::new(tokens).parse_program().unwrap();
    let output = std::env::temp_dir().join(format!("tyrus-async-optional-{}", std::process::id()));
    compile(&program, "async_optional", output.to_str().unwrap(), false).unwrap();
    let ir = std::fs::read_to_string(output.with_extension("ll")).unwrap();
    assert!(
        ir.contains("optional_storage"),
        "awaited payload was stored without boxing it into the optional"
    );
    let _ = std::fs::remove_file(output.with_extension("ll"));
    let _ = std::fs::remove_file(output);
}

#[test]
fn infers_the_payload_of_promise_all_from_its_arguments() {
    let source = r#"
        async function load(value: i32): Promise<i32> {
            return value * 2;
        }

        async function main(): Promise<void> {
            const values = await Promise.all([load(1), load(2)]);
            console.log("second:", values[1]);
        }
    "#;
    let tokens = tyrus_core::lexer::lex(source).unwrap();
    let program = Parser::new(tokens).parse_program().unwrap();
    let output =
        std::env::temp_dir().join(format!("tyrus-async-promise-all-{}", std::process::id()));
    compile(
        &program,
        "async_promise_all",
        output.to_str().unwrap(),
        false,
    )
    .unwrap();
    let _ = std::fs::remove_file(output.with_extension("ll"));
    let _ = std::fs::remove_file(output);
}

#[test]
fn joins_the_try_continuation_when_the_catch_returns() {
    let source = r#"
        async function verify(): Promise<void> {
            try {
                const failure: Error = { message: "failure", code: "ERR_FAILURE" };
                await Promise.reject(failure);
            } catch (error: Error) {
                console.log(error.message);
                return;
            } finally {
                console.log("cleanup");
            }
        }

        async function main(): Promise<void> {
            await verify();
        }
    "#;
    let tokens = tyrus_core::lexer::lex(source).unwrap();
    let program = Parser::new(tokens).parse_program().unwrap();
    let output =
        std::env::temp_dir().join(format!("tyrus-async-try-return-{}", std::process::id()));
    compile(
        &program,
        "async_try_return",
        output.to_str().unwrap(),
        false,
    )
    .unwrap();
    let _ = std::fs::remove_file(output.with_extension("ll"));
    let _ = std::fs::remove_file(output);
}

#[test]
fn lowers_an_await_binding_inside_a_try_block() {
    let source = r#"
        async function load(value: i32): Promise<i32> {
            return value * 2;
        }

        async function main(): Promise<void> {
            let total: i32 = 0;
            try {
                const doubled = await load(21);
                total = await load(doubled);
                console.log("total:", total);
            } catch (error: Error) {
                console.log("failed:", error.message);
            } finally {
                console.log("done");
            }
        }
    "#;
    let tokens = tyrus_core::lexer::lex(source).unwrap();
    let program = Parser::new(tokens).parse_program().unwrap();
    let output =
        std::env::temp_dir().join(format!("tyrus-async-try-binding-{}", std::process::id()));
    compile(
        &program,
        "async_try_binding",
        output.to_str().unwrap(),
        false,
    )
    .unwrap();
    let ir = std::fs::read_to_string(output.with_extension("ll")).unwrap();
    assert_eq!(ir.matches("call void @tyrus_async_task_await").count(), 2);
    assert!(ir.contains("async.try.recovered"));
    let _ = std::fs::remove_file(output.with_extension("ll"));
    let _ = std::fs::remove_file(output);
}

#[test]
fn lowers_awaits_in_a_catch_when_the_try_does_not_suspend() {
    let source = r#"
        function fail(): void throws Error {
            const error: Error = { message: "boom", code: "ERR_BOOM" };
            throw error;
        }

        async function message(): Promise<string> {
            return "recovered";
        }

        async function run(): Promise<void> {
            try {
                fail();
            } catch (error: Error) {
                const recovered: string = await message();
                console.log(error.message, recovered);
                await Promise.resolve();
            } finally {
                console.log("cleanup");
            }
            console.log("after");
        }

        async function main(): Promise<void> {
            await run();
        }
    "#;
    let tokens = tyrus_core::lexer::lex(source).unwrap();
    let program = Parser::new(tokens).parse_program().unwrap();
    let output =
        std::env::temp_dir().join(format!("tyrus-async-catch-only-{}", std::process::id()));
    compile(
        &program,
        "async_catch_only",
        output.to_str().unwrap(),
        false,
    )
    .unwrap();
    let ir = std::fs::read_to_string(output.with_extension("ll")).unwrap();
    assert!(ir.contains("async.try.setjmp"));
    assert!(ir.contains("async.try.catch.entry"));
    assert!(ir.contains("async.catch.0.state.0"));
    assert!(ir.contains("async.catch.0.state.1"));
    assert!(ir.contains("async.try.recovered"));
    let _ = std::fs::remove_file(output.with_extension("ll"));
    let _ = std::fs::remove_file(output);
}

#[test]
fn lowers_awaits_in_catch_after_a_suspending_if_try_body() {
    let source = r#"
        async function step(shouldFail: boolean): Promise<void> throws Error {
            if (shouldFail) {
                const error: Error = { message: "boom", code: "ERR_BOOM" };
                throw error;
            }
        }

        async function recover(): Promise<string> {
            return "recovered";
        }

        async function run(flag: boolean): Promise<void> {
            try {
                if (flag) {
                    await step(true);
                } else {
                    await step(false);
                }
            } catch (error: Error) {
                const message: string = await recover();
                console.log(error.message, message);
                await Promise.resolve();
            } finally {
                console.log("cleanup");
            }
        }
    "#;
    let tokens = tyrus_core::lexer::lex(source).unwrap();
    let program = Parser::new(tokens).parse_program().unwrap();
    let output = std::env::temp_dir().join(format!("tyrus-async-if-catch-{}", std::process::id()));
    compile(&program, "async_if_catch", output.to_str().unwrap(), false).unwrap();
    let ir = std::fs::read_to_string(output.with_extension("ll")).unwrap();
    assert!(ir.contains("async.if.after"));
    assert!(ir.contains("async.catch.0.state.0"));
    assert!(ir.contains("async.catch.0.state.1"));
    assert!(ir.contains("async.catch.fulfilled"));
    let _ = std::fs::remove_file(output.with_extension("ll"));
    let _ = std::fs::remove_file(output);
}

#[test]
fn lowers_await_inside_both_if_branches_within_a_try_body() {
    let source = r#"
        function step(shouldFail: boolean): Promise<void> {
            return shouldFail ? Promise.reject<void>({ message: "boom", code: "ERR_BOOM" }) : Promise.resolve();
        }

        async function run(shouldFail: boolean): Promise<void> {
            try {
                if (shouldFail) {
                    await step(true);
                } else {
                    await step(false);
                }
            } catch (error: Error) {
                console.log(error.message);
            } finally {
                console.log("cleanup");
            }
        }

        async function main(): Promise<void> {
            await run(false);
            await run(true);
        }
    "#;
    let tokens = tyrus_core::lexer::lex(source).unwrap();
    let program = Parser::new(tokens).parse_program().unwrap();
    let output = std::env::temp_dir().join(format!("tyrus-async-try-if-{}", std::process::id()));
    compile(&program, "async_try_if", output.to_str().unwrap(), false).unwrap();
    let ir = std::fs::read_to_string(output.with_extension("ll")).unwrap();
    assert!(ir.contains("async.if.selected"));
    assert!(ir.contains("async.if.after"));
    assert!(ir.contains("async.await.fulfilled"));
    assert!(ir.contains("async.await.failed"));
    let _ = std::fs::remove_file(output.with_extension("ll"));
    let _ = std::fs::remove_file(output);
}

#[test]
fn lowers_await_in_both_branches_of_an_if() {
    let source = r#"
        async function step(value: i32): Promise<i32> {
            return value;
        }

        async function run(flag: boolean): Promise<void> {
            if (flag) {
                console.log("then");
                const left: i32 = await step(7);
                await Promise.resolve();
                console.log(left);
            } else {
                console.log("else");
                const right: i32 = await step(9);
                await Promise.resolve();
                console.log(right);
            }
            console.log("after");
        }

        async function main(): Promise<void> {
            await run(true);
            await run(false);
        }
    "#;
    let tokens = tyrus_core::lexer::lex(source).unwrap();
    let program = Parser::new(tokens).parse_program().unwrap();
    let output = std::env::temp_dir().join(format!("tyrus-async-if-both-{}", std::process::id()));
    compile(&program, "async_if_both", output.to_str().unwrap(), false).unwrap();
    let ir = std::fs::read_to_string(output.with_extension("ll")).unwrap();
    assert!(ir.contains("async.if.selected"));
    assert!(ir.contains("async.if.alternative"));
    assert!(ir.contains("async.if.after"));
    assert!(ir.matches("call void @tyrus_async_task_await").count() >= 4);
    let _ = std::fs::remove_file(output.with_extension("ll"));
    let _ = std::fs::remove_file(output);
}

#[test]
fn lowers_await_in_an_if_condition() {
    let source = r#"
        async function enabled(): Promise<boolean> {
            return true;
        }

        async function value(): Promise<i32> {
            return 7;
        }

        async function run(): Promise<void> {
            if (await enabled()) {
                console.log("enabled");
            } else {
                console.log("disabled");
            }
            if ((await value()) > 0) {
                console.log("positive");
            }
        }

        async function main(): Promise<void> {
            await run();
        }
    "#;
    let tokens = tyrus_core::lexer::lex(source).unwrap();
    let program = Parser::new(tokens).parse_program().unwrap();
    let output =
        std::env::temp_dir().join(format!("tyrus-async-if-condition-{}", std::process::id()));
    compile(
        &program,
        "async_if_condition",
        output.to_str().unwrap(),
        false,
    )
    .unwrap();
    let ir = std::fs::read_to_string(output.with_extension("ll")).unwrap();
    assert!(ir.contains("async.condition.payload"));
    assert!(ir.contains("async.if.selected"));
    assert!(ir.contains("async.if.alternative"));
    assert!(ir.contains("async.if.after"));
    let _ = std::fs::remove_file(output.with_extension("ll"));
    let _ = std::fs::remove_file(output);
}

#[test]
fn lowers_a_suspending_if_nested_in_a_while_with_loop_transfers() {
    let source = r#"
        async function pause(): Promise<void> {}
        async function main(): Promise<void> {
            let index: i32 = 0;
            let total: i32 = 0;
            while (index < 6) {
                index += 1;
                if (index % 2 === 0) {
                    await pause();
                    if (index === 2) { continue; }
                }
                total += index;
                if (index === 5) { break; }
            }
            console.log(total);
        }
    "#;
    let tokens = tyrus_core::lexer::lex(source).unwrap();
    let program = Parser::new(tokens).parse_program().unwrap();
    let output = std::env::temp_dir().join(format!(
        "tyrus-async-nested-while-if-{}",
        std::process::id()
    ));
    compile(
        &program,
        "async_nested_while_if",
        output.to_str().unwrap(),
        false,
    )
    .unwrap();
    let ir = std::fs::read_to_string(output.with_extension("ll")).unwrap();
    assert!(ir.contains("async.control.condition"));
    assert!(ir.contains("async.control.after"));
    assert!(ir.contains("async.if.selected"));
    let _ = std::fs::remove_file(output.with_extension("ll"));
    let _ = std::fs::remove_file(output);
}

#[test]
fn lowers_await_through_recursively_nested_control_flow() {
    let source = r#"
        async function pause(): Promise<void> {}
        async function main(): Promise<void> {
            let outer: i32 = 0;
            while (outer < 2) {
                outer += 1;
                let inner: i32 = 0;
                while (inner < 2) {
                    inner += 1;
                    if (outer + inner > 0) {
                        await pause();
                    }
                }
            }
            console.log(outer);
        }
        async function both(flag: boolean): Promise<void> {
            while (flag) {
                if (flag) { await pause(); break; } else { await pause(); break; }
            }
        }
        async function switchInTry(): Promise<void> {
            let index: i32 = 0;
            while (index < 2) {
                index += 1;
                try {
                    switch (index) {
                        case 1: { await pause(); }
                        default: { await pause(); }
                    }
                } catch (error: Error) {} finally { console.log("cleanup"); }
            }
        }
    "#;
    let tokens = tyrus_core::lexer::lex(source).unwrap();
    let program = Parser::new(tokens).parse_program().unwrap();
    let output = std::env::temp_dir().join(format!(
        "tyrus-async-recursive-control-{}",
        std::process::id()
    ));
    compile(
        &program,
        "async_recursive_control",
        output.to_str().unwrap(),
        false,
    )
    .unwrap();
    let ir = std::fs::read_to_string(output.with_extension("ll")).unwrap();
    assert!(ir.contains("async.recursive.while.condition"));
    assert!(ir.contains("async.recursive.if.selected"));
    assert!(ir.contains("async.recursive.try.after"));
    assert!(ir.contains("async.switch.case.0"));
    let _ = std::fs::remove_file(output.with_extension("ll"));
    let _ = std::fs::remove_file(output);
}

#[test]
fn lowers_suspending_finally_through_recursive_control_flow() {
    let source = r#"
        async function pause(): Promise<void> {}
        async function run(): Promise<i32> {
            let index: i32 = 0;
            while (index < 1) {
                index += 1;
                try {
                    await pause();
                    return 7;
                } catch (error: Error) {
                    await pause();
                } finally {
                    await pause();
                }
            }
            return -1;
        }
    "#;
    let tokens = tyrus_core::lexer::lex(source).unwrap();
    let program = Parser::new(tokens).parse_program().unwrap();
    let output = std::env::temp_dir().join(format!(
        "tyrus-async-recursive-finally-{}",
        std::process::id()
    ));
    compile(
        &program,
        "async_recursive_finally",
        output.to_str().unwrap(),
        false,
    )
    .unwrap();
    let ir = std::fs::read_to_string(output.with_extension("ll")).unwrap();
    assert!(ir.contains("async.recursive.finally.entry"));
    assert!(ir.contains("async.switch.finally.completion.return"));
    assert!(ir.contains("async.finally.state"));
    let _ = std::fs::remove_file(output.with_extension("ll"));
    let _ = std::fs::remove_file(output);
}

#[test]
fn an_uncaught_throw_prints_only_the_message_it_was_given() {
    let source = r#"
        function boom(): i32 throws string {
            throw "sync failure";
        }

        function main(): void {
            boom();
        }
    "#;
    let tokens = tyrus_core::lexer::lex(source).unwrap();
    let program = Parser::new(tokens).parse_program().unwrap();
    let output = std::env::temp_dir().join(format!("tyrus-uncaught-{}", std::process::id()));
    compile(&program, "uncaught_throw", output.to_str().unwrap(), false).unwrap();
    let ir = std::fs::read_to_string(output.with_extension("ll")).unwrap();
    assert!(
        ir.contains("uncaught exception: %.*s"),
        "the abort line must bound the message by its recorded length"
    );
    assert!(
        !ir.contains("uncaught exception: %s"),
        "the unbounded format would read past the buffer"
    );
    let _ = std::fs::remove_file(output.with_extension("ll"));
    let _ = std::fs::remove_file(output);
}

#[test]
fn lowers_an_await_nested_in_a_loop_condition() {
    let source = r#"
        async function next(value: i32): Promise<i32> {
            return value + 1;
        }

        async function main(): Promise<void> {
            let index: i32 = 0;
            while ((await next(index)) < 4) {
                index = index + 1;
            }
            console.log("index:", index);
        }
    "#;
    let tokens = tyrus_core::lexer::lex(source).unwrap();
    let program = Parser::new(tokens).parse_program().unwrap();
    let output =
        std::env::temp_dir().join(format!("tyrus-async-loop-condition-{}", std::process::id()));
    compile(
        &program,
        "async_loop_condition",
        output.to_str().unwrap(),
        false,
    )
    .unwrap();
    let ir = std::fs::read_to_string(output.with_extension("ll")).unwrap();
    assert!(ir.contains("async.condition.payload"));
    assert!(ir.contains("async.while.body"));
    let _ = std::fs::remove_file(output.with_extension("ll"));
    let _ = std::fs::remove_file(output);
}

#[test]
fn partition_sorted_by_and_avg_by_results_are_released_at_scope_exit() {
    let ir = compile_to_ir(
        r#"
            interface Point { x: i32; y: i32; }
            function main() {
                const values: i32[] = [1, 2, 3, 4];
                const halves: [i32[], i32[]] = values.partition(value % 2 === 0);
                const points: Point[] = [{ x: 2, y: 4 }, { x: 1, y: 3 }];
                const sorted: Point[] = points.sortedBy("x");
                const average: f64? = points.avgBy("y");
                console.log(halves[0].length, sorted[0].x, average ?? 0.0);
            }
        "#,
        "functional-array-owned-results",
    );
    let free_calls = ir.matches("call void @free(").count();
    assert!(
        free_calls >= 6,
        "expected input arrays, both partition arrays, sortedBy, and avgBy to be released, found {free_calls} frees:\n{ir}"
    );
}

#[test]
fn returning_an_object_literal_with_an_owned_optional_field_from_one_if_arm_compiles() {
    // Regression test for a compiler panic, not a leak: `return { id: 1,
    // nickname: "ada" };` inside one arm of an `if` whose other arm is a
    // bare `return;` (function returns `Account?`) failed with "invalid
    // LLVM IR: Instruction does not dominate all uses", the object
    // literal's own `Fields`-plan pending registration stayed on the
    // *pre-box* struct value, a different SSA register than the box pointer
    // everything downstream (including the *other* `if` arm's own
    // end-of-function sweep) actually held, so it was never forgotten by
    // identity and was eventually swept from a sibling branch that does not
    // dominate where it was defined.
    //
    // Closed with `ReleasePlan::Box`: the generic `(_, Optional(inner))`
    // boxing arm now takes the raw value's own pending plan (if any) and
    // re-registers it against the box itself, so the existing identity-based
    // forget/transfer machinery can always find it afterward, regardless of
    // which branch's cleanup eventually runs.
    let source = r#"
        interface Account {
            id: i32;
            nickname?: string;
        }

        function findAccount(found: boolean): Account? {
            if (found) {
                return { id: 1, nickname: "ada" };
            }

            return;
        }

        function main() {
            const present: Account? = findAccount(true);
            console.log(present?.id ?? -1);
            const absent: Account? = findAccount(false);
            console.log(absent?.id ?? -1);
        }
    "#;
    let tokens = Token::lexer(source).filter_map(Result::ok).collect();
    let program = Parser::new(tokens).parse_program().unwrap();
    let output = std::env::temp_dir().join(format!(
        "tyrus-optional-object-literal-if-return-{}",
        std::process::id()
    ));
    compile(
        &program,
        "optional_object_literal_if_return",
        output.to_str().unwrap(),
        false,
    )
    .unwrap();
    let _ = std::fs::remove_file(output.with_extension("ll"));
    let _ = std::fs::remove_file(output);
}

#[test]
fn optional_chaining_into_an_already_optional_field_does_not_double_box() {
    let source = r#"
        interface Account {
            id: i32;
            nickname?: string;
        }

        function main() {
            const account: Account? = { id: 1, nickname: "ada" };
            console.log(account?.nickname ?? "none");
            console.log("assert:", "ada");
            const noNickname: Account? = { id: 2 };
            console.log(noNickname?.nickname ?? "none");
            console.log("assert:", "none");
        }
    "#;
    let tokens = tyrus_core::lexer::lex(source).unwrap();
    let program = Parser::new(tokens).parse_program().unwrap();
    let output = std::env::temp_dir().join(format!(
        "tyrus-optional-field-no-double-box-{}",
        std::process::id()
    ));
    compile(
        &program,
        "optional_field_no_double_box",
        output.to_str().unwrap(),
        false,
    )
    .unwrap();
    let _ = std::fs::remove_file(output.with_extension("ll"));
    let _ = std::fs::remove_file(output);
}

fn ir_fn_body<'a>(ir: &'a str, name: &str) -> &'a str {
    let start = ir
        .find(&format!("define i32 @{name}("))
        .unwrap_or_else(|| panic!("function `{name}` not found in IR"));
    let body_start = ir[start..].find('{').map(|i| start + i).unwrap();
    let next_define = ir[body_start + 1..].find("define ");
    let end = next_define.map_or(ir.len(), |i| body_start + 1 + i);
    &ir[body_start..end]
}

#[test]
fn helper_functions_do_not_free_module_globals() {
    let ir = compile_with_env_to_ir(
        "repl_global_owned_helpers",
        "",
        r#"
            let name = "Thomas";
            let completeName = `${name}`;

            function helper(): i32 {
                console.log(name);
                return 0;
            }
        "#,
    );
    let helper = ir_fn_body(&ir, "helper");
    assert!(
        !helper.contains("@tyrus_string_release_buffer"),
        "helper must not release module globals it does not own:\n{helper}"
    );
    assert!(
        helper.contains("console.log") || helper.contains("printf"),
        "helper should still print"
    );
    let main = ir_fn_body(&ir, "main");
    assert!(
        main.contains("@tyrus_string_release_buffer"),
        "main still owns module globals"
    );
}

#[test]
fn emits_f128_literals_as_fp128_values() {
    let source = r#"
        const main = (): void => {
            const tenth: f128 = 0.1;
            console.log(tenth);
        };
    "#;
    let tokens = tyrus_core::lexer::lex(source).unwrap();
    let program = Parser::new(tokens).parse_program().unwrap();
    let output = std::env::temp_dir().join(format!("tyrus-f128-literal-{}", std::process::id()));
    compile(&program, "f128_literal", output.to_str().unwrap(), false).unwrap();
    assert!(
        std::fs::read_to_string(output.with_extension("ll"))
            .unwrap()
            .contains("store fp128 1.000000e-01"),
        "f128 literal was not emitted as an fp128 value"
    );
    let _ = std::fs::remove_file(output.with_extension("ll"));
    let _ = std::fs::remove_file(output);
}

/// A literal's lengths are computed at compile time and land in the constant, so
/// `"héllo".length` has nothing left to count at run time. Both are checked on
/// text where they differ: five characters in six bytes.
///
/// The constant is one word holding both, the byte length low, the character
/// count high, because that is the form the C ABI passes a sixteen-byte struct
/// in. `(5 << 32) | 6` is `21474836486`.
#[test]
fn a_string_literal_carries_both_lengths_as_a_constant() {
    let context = Context::create();
    let codegen = Codegen::new(&context, "string_literal", false, CompileTarget::Native);

    let value = codegen.const_string("héllo");

    assert_eq!(value.get_type(), codegen.string_type());
    let text = value.to_string();
    assert!(
        text.contains("21474836486"),
        "the literal's lengths were not folded into the constant: {text}"
    );
}

#[test]
fn a_concatenation_chain_allocates_once() {
    let source = r#"
        const main = (): void => {
            const a: string = "alpha".toUpperCase();
            const joined: string = a + "-" + a + "-" + a + "-" + a;
            console.log(joined);
        };
    "#;
    let tokens = tyrus_core::lexer::lex(source).unwrap();
    let program = Parser::new(tokens).parse_program().unwrap();
    let output = std::env::temp_dir().join(format!("tyrus-concat-chain-{}", std::process::id()));
    compile(&program, "concat_chain", output.to_str().unwrap(), false).unwrap();
    let ir = std::fs::read_to_string(output.with_extension("ll")).unwrap();

    let main = ir
        .split("define i32 @main(")
        .nth(1)
        .expect("the entry function");
    let main = main.split("\n}\n").next().expect("the function body");
    let allocations = main.matches("@tyrus_string_alloc").count();
    assert_eq!(
        allocations, 1,
        "a six-operator chain must allocate once, found {allocations} allocations:\n{main}"
    );
    let copies = main.matches("@llvm.memcpy").count();
    assert!(
        copies <= 7,
        "each operand is copied once into the single buffer, found {copies} copies"
    );

    let _ = std::fs::remove_file(output.with_extension("ll"));
    let _ = std::fs::remove_file(output);
}

#[test]
fn a_two_operand_concatenation_is_unchanged() {
    let source = r#"
        const main = (): void => {
            const a: string = "alpha".toUpperCase();
            const joined: string = a + "-";
            console.log(joined);
        };
    "#;
    let tokens = tyrus_core::lexer::lex(source).unwrap();
    let program = Parser::new(tokens).parse_program().unwrap();
    let output = std::env::temp_dir().join(format!("tyrus-concat-pair-{}", std::process::id()));
    compile(&program, "concat_pair", output.to_str().unwrap(), false).unwrap();
    let ir = std::fs::read_to_string(output.with_extension("ll")).unwrap();

    let main = ir
        .split("define i32 @main(")
        .nth(1)
        .expect("the entry function");
    let main = main.split("\n}\n").next().expect("the function body");
    assert_eq!(main.matches("@tyrus_string_alloc").count(), 1);

    let _ = std::fs::remove_file(output.with_extension("ll"));
    let _ = std::fs::remove_file(output);
}

#[test]
fn no_stack_slot_is_allocated_inside_a_loop() {
    let source = r#"
        interface Point { x: i32; y: i32; }

        const main = (): void => {
            let total: i64 = 0;
            for (let i: i32 = 0; i < 10; i += 1) {
                let numbers: i32[] = [i, i + 1];
                numbers.push(i);
                const joined: string = "a" + i.toString();
                const shouted: string = i.toString().toUpperCase();
                const pair: [i32, i32] = [i, i + 1];
                const point: Point = { x: i, y: i };
                let names: string[] = ["x"];
                const maybe: i32? = i;
                total += numbers[0] as i64;
                total += joined.length as i64;
                total += shouted.length as i64;
                total += pair[0] as i64;
                total += point.x as i64;
                total += names.join(",").length as i64;
                total += (maybe ?? 0) as i64;
            }
            console.log(total);
        };
    "#;
    let root = std::env::temp_dir().join(format!("tyrus-loop-alloca-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).unwrap();
    std::fs::write(root.join("main.tyr"), source).unwrap();
    let program = tyrus_core::module_loader::load_program(&root.join("main.tyr"), None).unwrap();
    let output = std::env::temp_dir().join(format!("tyrus-loop-alloca-out-{}", std::process::id()));
    compile(&program, "loop_alloca", output.to_str().unwrap(), false).unwrap();
    let ir = std::fs::read_to_string(output.with_extension("ll")).unwrap();

    let main = ir
        .split("define i32 @main(")
        .nth(1)
        .expect("the entry function");
    let main = main.split("\n}\n").next().expect("the function body");
    let mut offenders = Vec::new();
    let mut label = "entry";
    for line in main.lines() {
        let trimmed = line.trim();
        if let Some(name) = trimmed.strip_suffix(':') {
            label = name;
        } else if let Some(name) = trimmed.split_once(':').map(|(name, _)| name)
            && trimmed.contains("; preds =")
        {
            label = name;
        }
        if trimmed.contains(" = alloca ") && label != "entry" {
            offenders.push(format!("{label}: {trimmed}"));
        }
    }
    assert!(
        offenders.is_empty(),
        "every stack slot must be allocated in the entry block, found:\n{}",
        offenders.join("\n")
    );
}

#[test]
fn concurrent_stdout_captures_do_not_steal_each_others_output() {
    let threads = (0..8)
        .map(|id| {
            std::thread::spawn(move || {
                let marker = format!("capture-{id}\n");
                let captured = crate::core::capture_stdout(|| {
                    std::thread::sleep(std::time::Duration::from_millis(20));
                    write_captured_stdout(marker.as_bytes());
                })
                .expect("capture must succeed");
                assert!(
                    captured.contains(&marker),
                    "thread {id} lost its own output, got {captured:?}"
                );
                for other in 0..8 {
                    if other == id {
                        continue;
                    }
                    let stolen = format!("capture-{other}\n");
                    assert!(
                        !captured.contains(&stolen),
                        "thread {id} captured thread {other}'s output"
                    );
                }
            })
        })
        .collect::<Vec<_>>();
    for thread in threads {
        thread.join().expect("capture thread must not panic");
    }
}

#[test]
fn capture_excludes_what_another_thread_prints_through_rust_stdout() {
    use std::io::Write;
    use std::sync::Arc;
    use std::sync::atomic::{AtomicBool, Ordering};

    let stop = Arc::new(AtomicBool::new(false));
    let started = Arc::new(AtomicBool::new(false));
    let printer = {
        let stop = Arc::clone(&stop);
        let started = Arc::clone(&started);
        std::thread::spawn(move || {
            while !stop.load(Ordering::Relaxed) {
                let handle = std::io::stdout();
                let mut locked = handle.lock();
                let _ = locked.write_all(b"intruder\n");
                let _ = locked.flush();
                drop(locked);
                started.store(true, Ordering::Relaxed);
                std::thread::sleep(std::time::Duration::from_millis(1));
            }
        })
    };
    while !started.load(Ordering::Relaxed) {
        std::hint::spin_loop();
    }

    let captured = crate::core::capture_stdout(|| {
        unsafe {
            libc::write(1, c"payload\n".as_ptr().cast(), 8);
        }
        std::thread::sleep(std::time::Duration::from_millis(30));
    })
    .expect("capture must succeed");

    stop.store(true, Ordering::Relaxed);
    printer.join().expect("printing thread must not panic");

    assert_eq!(
        captured, "payload\n",
        "the capture picked up another thread's stdout"
    );
}

#[test]
fn capture_redirects_rust_stdout_writes() {
    use std::io::Write;

    let captured = crate::core::capture_stdout(|| {
        let mut stdout = std::io::stdout().lock();
        stdout
            .write_all(b"rust-captured-output\n")
            .expect("Rust stdout write must succeed");
        stdout.flush().expect("Rust stdout flush must succeed");
    })
    .expect("capture must redirect Rust stdout");

    assert_eq!(captured, "rust-captured-output\n");
}

#[test]
fn capture_flushes_pending_c_stdio_before_redirecting_stdout() {
    const CHILD_MODE: &str = "TYRUS_CAPTURE_STDOUT_TEST_CHILD";
    if std::env::var_os(CHILD_MODE).is_some() {
        unsafe { libc::printf(c"tyrus-pre-capture-c-stdio".as_ptr()) };
        let captured = crate::core::capture_stdout(|| unsafe {
            libc::printf(c"captured-c-stdio\n".as_ptr());
            libc::write(1, c"captured-output\n".as_ptr().cast(), 16);
        })
        .expect("child capture must succeed");
        assert_eq!(captured, "captured-c-stdio\ncaptured-output\n");
        return;
    }
    let output = std::process::Command::new(std::env::current_exe().expect("test executable"))
        .args([
            "--exact",
            "capture_flushes_pending_c_stdio_before_redirecting_stdout",
            "--nocapture",
        ])
        .env(CHILD_MODE, "1")
        .output()
        .expect("capture test child must start");
    assert!(
        output.status.success(),
        "child capture failed: {}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

#[cfg(windows)]
#[test]
fn capture_redirects_win32_stdout_handles() {
    use windows_sys::Win32::Storage::FileSystem::WriteFile;
    use windows_sys::Win32::System::Console::{GetStdHandle, STD_OUTPUT_HANDLE};

    let message = b"win32-captured-output\n";
    let write_stdout = || unsafe {
        let handle = GetStdHandle(STD_OUTPUT_HANDLE);
        assert!(
            !handle.is_null() && handle as isize != -1,
            "Win32 stdout handle must be valid"
        );
        let mut written = 0;
        let result = WriteFile(
            handle,
            message.as_ptr().cast(),
            message.len() as u32,
            &mut written,
            std::ptr::null_mut(),
        );
        assert_ne!(result, 0, "Win32 stdout write failed");
        assert_eq!(written as usize, message.len());
    };
    let captured =
        crate::core::capture_stdout(write_stdout).expect("capture must redirect Win32 stdout");
    let restored = crate::core::capture_stdout(write_stdout)
        .expect("capture must restore the Win32 stdout handle");

    assert_eq!(captured, "win32-captured-output\n");
    assert_eq!(restored, "win32-captured-output\n");
}

fn emitted_ir(name: &str, source: &str) -> String {
    let output = std::env::temp_dir().join(format!("tyrus-codegen-{name}-{}", std::process::id()));
    let program = Parser::from_source(source)
        .unwrap()
        .parse_program()
        .unwrap();
    compile(&program, name, output.to_str().unwrap(), false).unwrap();
    let ir = std::fs::read_to_string(output.with_extension("ll")).unwrap();
    let _ = std::fs::remove_file(output.with_extension("ll"));
    ir
}

#[test]
fn a_counting_loop_emits_no_bounds_check_for_its_own_indexing() {
    let ir = emitted_ir(
        "bce_counting_loop",
        r#"
        function sweep(values: i32[]): i32 {
            let sum = 0;
            for (let i: usize = 0; i < values.length; i += 1) {
                sum += values[i];
            }
            return sum;
        }

        function main(): void {
            const values: i32[] = [1, 2, 3];
            console.log(sweep(values));
        }
        "#,
    );
    assert!(
        !ir.contains("array.bounds.error"),
        "the proved access kept its bounds check:\n{ir}"
    );
}

#[test]
fn a_constant_index_into_a_readonly_const_table_emits_no_bounds_check() {
    let ir = emitted_ir(
        "bce_constant_index",
        r#"
        readonly const TABLE: i32[] = [10, 20, 30, 40];

        function main(): void {
            console.log(TABLE[2]);
        }
        "#,
    );
    assert!(
        !ir.contains("array.bounds.error"),
        "a literal index into a fixed table kept its check:\n{ir}"
    );
}

#[test]
fn a_counting_loop_over_a_const_global_emits_no_bounds_check() {
    let ir = emitted_ir(
        "bce_const_global_loop",
        r#"
        const TABLE: i32[] = [10, 20, 30, 40];

        function main(): void {
            let sum = 0;
            for (let i: usize = 0; i < TABLE.length; i += 1) {
                sum += TABLE[i];
            }
            console.log(sum);
        }
        "#,
    );
    assert!(
        !ir.contains("array.bounds.error"),
        "a loop over a fixed global kept its check:\n{ir}"
    );
}

#[test]
fn an_unproved_index_keeps_its_bounds_check() {
    let ir = emitted_ir(
        "bce_unproved_index",
        r#"
        function pick(values: i32[], slot: usize): i32 {
            return values[slot];
        }

        function sweep(values: i32[]): i32 {
            let sum = 0;
            for (let i: usize = 0; i < values.length; i += 1) {
                sum += values[i];
            }
            return sum;
        }

        function main(): void {
            const values: i32[] = [1, 2, 3];
            console.log(sweep(values) + pick(values, 0));
        }
        "#,
    );
    assert!(
        ir.contains("array.bounds.error"),
        "an index nothing proves must keep its check:\n{ir}"
    );
    assert_eq!(
        ir.matches("array.bounds.valid:").count(),
        1,
        "expected exactly the unproved access to keep a check:\n{ir}"
    );
}

#[test]
fn a_counting_loop_emits_no_bounds_check_when_it_writes_elements() {
    let ir = emitted_ir(
        "bce_element_write",
        r#"
        function double(): i32 {
            let values: i32[] = [1, 2, 3];
            for (let i: usize = 0; i < values.length; i += 1) {
                values[i] = values[i] * 2;
            }
            return values[0];
        }

        function main(): void {
            console.log(double());
        }
        "#,
    );
    assert!(
        ir.contains("array.bounds.error"),
        "the `values[0]` after the loop is not proved and must keep its check:\n{ir}"
    );
    assert_eq!(
        ir.matches("array.bounds.valid:").count(),
        1,
        "both accesses inside the loop must lose their check:\n{ir}"
    );
}

#[test]
fn a_move_hands_the_release_to_the_destination_binding() {
    // `let xs = src;` copies the array header and shares the buffer, so exactly
    // one of the two names may release it. The release used to stay with the
    // source, which is invisible until one of them reallocates: `push` frees the
    // old buffer, `xs` gets a new one, and scope exit then frees `src`'s stale
    // pointer. ASan reported that as a double free, and the checker's `E0382`
    // guarantees `src` is never read again, so the destination is the only
    // sound survivor.
    //
    // `tests/leaks/clean/array-moved-into-a-mutable-binding.tyr` asserts the
    // same thing by measurement, across `i32[]`, `string[]` and a global source.
    let ir = compile_to_ir(
        r#"
        const main = (): void => {
            const src: i32[] = [2, 3, 5];
            let xs: i32[] = src;
            xs.push(9);
            console.log(xs.length);
        };
        "#,
        "array-move-release-transfer",
    );
    assert!(
        ir.contains("drop.copy.array = load { ptr, i64, i64 }, ptr %xs"),
        "expected the destination binding to own the release:\n{ir}"
    );
    assert!(
        !ir.contains("drop.copy.array = load { ptr, i64, i64 }, ptr %src"),
        "expected the moved-from source to release nothing:\n{ir}"
    );
}

#[test]
fn a_global_const_table_needs_no_run_time_construction() {
    // A global `const` array used to be built by the top of `main`: a `malloc`,
    // one store per element, and a `free` at the end. None of it is necessary
    // for a value that is constant, deeply immutable and known now, the buffer
    // becomes a constant global the loader maps read-only, and the header a
    // constant pointing at it.
    //
    // The gate is `escape_analysis::analyze_constant_global_arrays`, which
    // reuses the containment question: what survives it is indexing and reading
    // `.length`, neither of which can move or grow the buffer. A `.rodata`
    // pointer reaching `free` or `realloc` is a fault, not a leak, so the
    // analysis errs toward refusing.
    let ir = compile_to_ir(
        r#"
        const PRIMES: i32[] = [2, 3, 5, 7];

        const main = (): void => {
            console.log(PRIMES[2]);
            console.log(PRIMES.length);
        };
        "#,
        "constant-global-array-promotion",
    );
    assert!(
        ir.contains("private unnamed_addr constant [4 x i32] [i32 2, i32 3, i32 5, i32 7]"),
        "expected the table itself to be a constant global:\n{ir}"
    );
    assert!(
        ir.contains(".data, i64 4, i64 4 }"),
        "expected a constant header pointing at the table:\n{ir}"
    );
    let main_body = ir
        .split("define i32 @main()")
        .nth(1)
        .expect("a main definition");
    let main_body = main_body.split("\n}").next().expect("a closing brace");
    assert!(
        !main_body.contains("@malloc"),
        "expected no run-time construction of the table:\n{main_body}"
    );
    assert!(
        !main_body.contains("@free"),
        "expected nothing to release a table that was never allocated:\n{main_body}"
    );
}

#[test]
fn no_scratch_slot_is_reserved_inside_a_loop() {
    // LLVM only reuses a stack slot when its `alloca` is in the entry block. An
    // `alloca` emitted where the caller happens to stand reserves fresh space on
    // every pass, so a long-running loop walks off the end of the stack: a loop
    // calling `split` crashed at 400,000 iterations and survived 200,000, with a
    // flat heap the whole way, and AddressSanitizer named it `stack-overflow`.
    //
    // The fix is `entry_alloca`, which already existed for exactly this; the
    // sites had drifted back to `build_alloca`. Only the reservation moves, a
    // store that resets the slot stays where it was, so each iteration still
    // starts from a cleared value.
    //
    // This asserts the property rather than the sites, because there are still
    // hundreds of `build_alloca` calls across code generation and any of them
    // can reintroduce it. Every construct below was one that failed.
    let ir = compile_to_ir(
        r#"
        interface Point { x: i32; y: i32; }

        const main = (): void => {
            let acc: usize = 0;
            for (let round: i32 = 0; round < 4; round += 1) {
                const label: string = `item-${round}`;
                acc += label.split("-").length + label.chars().length;
                acc += label.toUpperCase().length + label.trim().length;
                acc += label.replace("i", "y").length;
                const numbers: i32[] = [5, 3, 1, 4];
                acc += numbers.map((x: i32): i32 => x + 1).length;
                acc += numbers.filter((x: i32): boolean => x > 1).length;
                acc += numbers.reduce((a: i32, b: i32): i32 => a + b, 0) as usize;
                acc += numbers.toSorted().length;
                acc += numbers.toReversed().length;
                acc += numbers.slice(1).length;
                acc += numbers.concat([9]).length;
                acc += label.split("-").join(",").length;
                acc += numbers.indexOf(3) as usize;
                const point: Point = { x: round, y: round };
                acc += (point.x + point.y) as usize;
                const maybe: string? = "x";
                acc += (maybe ?? "").length;
                acc += numbers.some((x: i32): boolean => x > 3) ? 1 : 0;
                acc += numbers.findIndex((x: i32): boolean => x > 3) as usize;
            }
            console.log(acc);
        };
        "#,
        "no-scratch-slot-inside-a-loop",
    );
    let mut offenders = Vec::new();
    for definition in ir.split("\ndefine ").skip(1) {
        let name = definition
            .split('(')
            .next()
            .unwrap_or("")
            .split_whitespace()
            .last()
            .unwrap_or("<unnamed>")
            .to_string();
        let body = definition.split("\n}").next().unwrap_or("");
        let mut block = "entry".to_string();
        for line in body.lines() {
            let trimmed = line.trim_end();
            if let Some(label) = trimmed.strip_suffix(':') {
                if !label.contains(char::is_whitespace) {
                    block = label.to_string();
                }
            } else if let Some(label) = trimmed.split_once(':').map(|(head, _)| head)
                && !label.is_empty()
                && !label.starts_with(' ')
                && trimmed.contains("; preds =")
                && !label.contains(char::is_whitespace)
            {
                block = label.to_string();
            }
            if line.contains(" = alloca ") && block != "entry" {
                offenders.push(format!("{name} [{block}] {}", line.trim()));
            }
        }
    }
    assert!(
        offenders.is_empty(),
        "these stack slots are reserved outside the entry block, so a loop \
         around them grows the stack every iteration:\n{}",
        offenders.join("\n")
    );
}

#[test]
fn the_described_runtime_tables_do_not_overlap() {
    let tables: [(&str, &[&str]); 3] = [
        ("pure-total", crate::core::PURE_TOTAL_RUNTIME_FNS),
        ("reading-total", crate::core::READING_TOTAL_RUNTIME_FNS),
        ("pure-aborting", crate::core::PURE_ABORTING_RUNTIME_FNS),
    ];
    for (index, (left_name, left)) in tables.iter().enumerate() {
        for (right_name, right) in tables.iter().skip(index + 1) {
            let overlap: Vec<_> = left.iter().filter(|name| right.contains(name)).collect();
            assert!(
                overlap.is_empty(),
                "described by both {left_name} and {right_name}: {overlap:?}"
            );
        }
    }
}

/// Every name in the two tables is a claim about a body in `tyrus-runtime`. A
/// renamed or deleted helper would leave the claim behind, pointing at nothing:
/// harmless for the rename (the attribute is simply never emitted) but a lie
/// waiting to be re-attached if the name is ever reused for something impure.
///
/// This reads the runtime sources rather than linking against them, because the
/// question is about the *name* the code generator writes into the IR, which is
/// exactly what a `#[no_mangle] extern "C"` definition publishes.
#[test]
fn every_described_runtime_helper_is_defined_in_the_runtime_crate() {
    let runtime_src = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../tyrus-runtime/src")
        .canonicalize()
        .expect("runtime sources");
    fn collect(directory: &std::path::Path, sources: &mut String) {
        for entry in std::fs::read_dir(directory).expect("read runtime src") {
            let path = entry.expect("dir entry").path();
            if path.is_dir() {
                collect(&path, sources);
            } else if path.extension().is_some_and(|ext| ext == "rs") {
                sources.push_str(&std::fs::read_to_string(&path).expect("read source"));
            }
        }
    }
    let mut sources = String::new();
    collect(&runtime_src, &mut sources);
    let missing: Vec<_> = crate::core::PURE_TOTAL_RUNTIME_FNS
        .iter()
        .chain(crate::core::READING_TOTAL_RUNTIME_FNS)
        .chain(crate::core::PURE_ABORTING_RUNTIME_FNS)
        .filter(|name| !sources.contains(&format!("fn {name}(")))
        .collect();
    assert!(
        missing.is_empty(),
        "described but not defined in tyrus-runtime: {missing:?}"
    );
}

/// `splice` with an insertion argument builds a temporary array for the inserted
/// elements, and the runtime clones what it inserts, so that temporary is still
/// owned by the caller afterwards.
///
/// Inside an async function the same literal is also registered as a pending
/// owned native result, and that cleanup releases every element and frees the
/// buffer. Both used to run: the generic one then walked a buffer the splice
/// branch had already freed and called `release` on whatever the allocator had
/// left there. The corpus reported it as corruption rather than a leak, so the
/// fast backend never saw it; valgrind and AddressSanitizer both landed on
/// `TyrusString::release`.
#[test]
fn an_async_splice_cleans_its_inserted_temporary_exactly_once() {
    let ir = emitted_ir(
        "async_splice_single_cleanup",
        r#"
        async function exercise(): Promise<usize> {
            let mutated: string[] = ["charlie", "bravo", "alpha"];
            const removed: string[] = mutated.splice(1, 1, "delta");
            return removed[0].length;
        }

        const main = async (): Promise<void> => {
            const n: usize = await exercise();
            console.log(n);
        };
        "#,
    );
    let block = ir
        .split("array.splice.ok:")
        .nth(1)
        .unwrap_or_else(|| panic!("no `array.splice.ok` block in:\n{ir}"))
        .split("\n\n")
        .next()
        .unwrap()
        .to_string();
    assert!(
        !block.contains("@free(ptr %array_splice_inserted_data)"),
        "the splice branch must not free the inserted buffer itself; the generic \
         cleanup owns it: {block}"
    );
    assert_eq!(
        block.matches("@tyrus_array_drop_string_contents").count(),
        1,
        "the inserted temporary must be dropped exactly once, by the generic \
         pending-owned-native-result cleanup: {block}"
    );
}

/// An array literal element named by a binding is not a temporary the literal
/// can claim: the binding's own cleanup still releases it. Storing the value
/// as-is gave one allocation two owners, and they disagreed the moment the
/// array outlived the binding, `return [v]` had the callee release `v` at
/// scope end and the caller release the same element when it dropped the array
/// it received, which valgrind reported as an invalid read inside
/// `TyrusString::release`.
///
/// Taking a reference of its own is what the counted buffer is for.
#[test]
fn an_array_literal_takes_its_own_reference_to_a_named_string_element() {
    let ir = emitted_ir(
        "array_literal_shares_named_element",
        r#"
        function makeArr(): string[] {
            const v: string = "b".toUpperCase();
            return [v];
        }

        function main(): void {
            const arr: string[] = makeArr();
            console.log(arr.length);
        }
        "#,
    );
    let body = ir
        .split("\ndefine ")
        .find(|body| {
            body.lines()
                .next()
                .is_some_and(|line| line.contains("makeArr"))
        })
        .unwrap_or_else(|| panic!("no `makeArr` in:\n{ir}"));
    assert!(
        body.contains("@tyrus_string_share"),
        "the literal must take its own reference to `v`: {body}"
    );
}

#[test]
fn a_union_of_literal_types_carries_no_payload() {
    let ir = emitted_ir(
        "union_literal_payload_free",
        r#"
        type Status = "active" | "inactive" | "pending";

        interface Row {
            id: i32;
            status: Status;
        }

        function main(): void {
            const row: Row = { id: 1, status: "pending" };
            console.log(row.status);
        }
        "#,
    );
    assert!(
        ir.contains("{ i8, [0 x i64] }"),
        "expected a payload-free union:\n{ir}"
    );
    assert!(
        !ir.contains("{ i8, [2 x i64] }"),
        "the string payload should be gone:\n{ir}"
    );
}

#[test]
fn a_union_with_a_non_literal_variant_keeps_its_payload() {
    let ir = emitted_ir(
        "union_mixed_payload",
        r#"
        type Mixed = i32 | "none";

        function main(): void {
            const value: Mixed = 42;
            console.log(value);
        }
        "#,
    );
    assert!(
        ir.contains("{ i8, [1 x i64] }") || ir.contains("{ i8, [2 x i64] }"),
        "a union with a real variant must keep a payload:\n{ir}"
    );
}

#[test]
fn matching_a_union_compares_its_tag() {
    let ir = emitted_ir(
        "union_match_tag",
        r#"
        type Status = "active" | "inactive" | "pending";

        function code(status: Status): i32 {
            match (status) {
                "active" => {
                    return 1;
                }
                "inactive" => {
                    return 2;
                }
                _ => {
                    return 3;
                }
            }
        }

        function main(): void {
            console.log(code("inactive"));
        }
        "#,
    );
    assert!(
        ir.contains("match.union.tag"),
        "expected a tag comparison for the union subject:\n{ir}"
    );
    assert!(
        !ir.contains("call i32 @memcmp"),
        "matching a literal union must not compare text:\n{ir}"
    );
}
#[test]
fn a_capture_larger_than_the_pipe_buffer_completes() {
    let chunk = "x".repeat(1024) + "\n";
    let captured = crate::core::capture_stdout(|| {
        for _ in 0..128 {
            write_captured_stdout(chunk.as_bytes());
        }
    })
    .expect("capture must succeed");
    assert!(
        captured.len() >= 128 * 1025,
        "expected the whole write back, got {} bytes",
        captured.len()
    );
}

#[test]
fn a_panic_inside_a_capture_still_restores_stdout() {
    let panicked = std::panic::catch_unwind(|| {
        let _ = crate::core::capture_stdout(|| panic!("inside the capture"));
    });
    assert!(panicked.is_err(), "the panic must propagate");

    let after = crate::core::capture_stdout(|| {
        let text = "still here\n";
        write_captured_stdout(text.as_bytes());
    })
    .expect("capture must succeed");
    assert!(
        after.contains("still here"),
        "stdout was not restored, got {after:?}"
    );
}

#[test]
fn a_declared_interface_is_stored_widest_field_first() {
    let source = r#"
        interface Padded {
            flag: boolean;
            id: i64;
            small: u8;
            amount: f64;
            ok: boolean;
            count: i32;
        }

        function main(): void {
            const row: Padded = {
                flag: true,
                id: 1,
                small: 2,
                amount: 0.5,
                ok: false,
                count: 3,
            };
            console.log(row.id, row.count, row.small);
        }
    "#;
    let ir = compile_to_ir(source, "struct-storage-order");
    assert!(
        ir.contains("{ i64, double, i32, i1, i8, i1 }"),
        "expected the packed storage order, got:\n{ir}"
    );
    assert!(
        !ir.contains("{ i1, i64, i8, double, i1, i32 }"),
        "the declared order is still being used as the layout:\n{ir}"
    );
}

#[test]
fn mapped_readonly_objects_keep_their_source_interface_storage_layout() {
    let source = r#"
        interface Address {
            city: string;
        }

        interface User {
            name: string;
            tags: string[];
            address: Address;
        }

        const makeUser = (): User => ({
            name: "Ada",
            tags: ["admin", "dev"],
            address: { city: "Rio" },
        });

        function main(): void {
            const frozen: Readonly<User> = makeUser();
            console.log(frozen.tags.length, frozen.name);
        }
    "#;
    let ir = compile_to_ir(source, "readonly-object-storage-order");
    let storage = "{ { ptr, i64, i64 }, { ptr, i64 }, { { ptr, i64 } } }";
    assert!(
        ir.contains(storage),
        "mapped readonly objects must use the interface storage order, got:\n{ir}"
    );
    assert!(
        !ir.contains("{ { ptr, i64 }, { ptr, i64, i64 }, { { ptr, i64 } } }"),
        "mapped readonly objects used their declared order instead of the ABI layout:\n{ir}"
    );
}

#[test]
fn a_native_interface_keeps_the_field_order_the_runtime_agrees_on() {
    let context = Context::create();
    let mut codegen = Codegen::new(&context, "native-layout", false, CompileTarget::Native);

    // `BatteryInfo` is `{ level: f32, charging: boolean, timeToFull: u32?,
    // timeToEmpty: u32? }`, and sorting it would put the two pointers first.
    // The `#[repr(C)]` struct the runtime writes through this pointer would
    // then be read one field out of step, silently, because a struct read at
    // the wrong offset does not fail, it just answers with the neighbour.
    let native: Vec<String> = codegen.interfaces["BatteryInfo"]
        .iter()
        .map(|field| field.name.clone())
        .collect();
    assert_eq!(native, ["level", "charging", "timeToFull", "timeToEmpty"]);

    let fields = codegen.interfaces["BatteryInfo"].clone();
    codegen.register_interface("UserBattery".to_string(), fields);
    let permuted: Vec<String> = codegen.interfaces["UserBattery"]
        .iter()
        .map(|field| field.name.clone())
        .collect();
    assert_eq!(permuted, ["timeToFull", "timeToEmpty", "level", "charging"]);

    let declared: Vec<String> = codegen
        .declared_fields(&Type::Named("UserBattery".to_string()))
        .unwrap()
        .iter()
        .map(|field| field.name.clone())
        .collect();
    assert_eq!(declared, ["level", "charging", "timeToFull", "timeToEmpty"]);
}

#[test]
fn emits_toggle_native_call() {
    let ir = compile_to_ir(
        r#"
            function main(): void {
                let values: i32[] = [1, 2, 3];
                const result: i32[] = values.toggle(4);
                console.log(result.length);
            }
        "#,
        "toggle_basic",
    );
    assert!(
        ir.contains("tyrus_array_toggle"),
        "toggle must call tyrus_array_toggle:\n{ir}"
    );
}

#[test]
fn emits_toggle_with_key_native_call() {
    let ir = compile_to_ir(
        r#"
            interface Item {
                id: i32;
                name: string;
            }
            function main(): void {
                let items: Item[] = [];
                const result: Item[] = items.toggle({ id: 1, name: "a" }, "id");
                console.log(result.length);
            }
        "#,
        "toggle_key",
    );
    assert!(
        ir.contains("tyrus_array_toggle"),
        "toggle with key must call tyrus_array_toggle:\n{ir}"
    );
}

#[test]
fn emits_toggle_all_native_call() {
    let ir = compile_to_ir(
        r#"
            function main(): void {
                let values: i32[] = [1, 2, 3];
                const result: i32[] = values.toggleAll([2, 4]);
                console.log(result.length);
            }
        "#,
        "toggle_all",
    );
    assert!(
        ir.contains("tyrus_array_toggle_all"),
        "toggleAll must call tyrus_array_toggle_all:\n{ir}"
    );
}

#[test]
fn toggle_does_not_mutate_receiver() {
    let ir = compile_to_ir(
        r#"
            function main(): void {
                let values: i32[] = [1, 2, 3];
                const result: i32[] = values.toggle(2);
                console.log(values.length);
                console.log(result.length);
            }
        "#,
        "toggle_no_mutate",
    );
    assert!(
        ir.contains("tyrus_array_toggle"),
        "toggle must call tyrus_array_toggle:\n{ir}"
    );
    assert!(
        !ir.contains("tyrus_array_push"),
        "toggle must not use push (non-mutating):\n{ir}"
    );
    assert!(
        !ir.contains("tyrus_array_pop"),
        "toggle must not use pop (non-mutating):\n{ir}"
    );
}

#[test]
fn encoding_guards_only_throwing_targets_and_bounds_uncaught_messages() {
    let encoders = compile_to_ir(
        r#"
        import { base64, hex, radix, utf8 } from "tyrus:encoding";
        function main(): void {
            const bytes: u8[] = utf8.encode("abc");
            console.log(hex.encode(bytes));
            console.log(base64.encode(bytes));
            console.log(utf8.decodeLossy(bytes));
            console.log(radix.format(255, 16));
        }
    "#,
        "encoding-non-throwing",
    );
    assert!(encoders.contains("tyrus_encoding_hex_encode"), "{encoders}");
    assert!(
        !encoders.contains("tyrus_encoding_take_error")
            && !encoders.contains("tyrus_encoding_clear_error"),
        "encoders, lossy decoders, and radix.format never throw, so they must not read the error slot:\n{encoders}"
    );

    let strict = compile_to_ir(
        r#"
        import { EncodingError, utf8 } from "tyrus:encoding";
        function decode(bytes: u8[]): Result<string, EncodingError> {
            return utf8.decode(bytes);
        }
        function main(): void {
            match (decode([255])) {
                Ok(text) => { console.log(text); }
                Err(error) => { console.log(error.code); }
            }
        }
    "#,
        "encoding-strict-throw",
    );
    assert!(
        strict.contains("tyrus_encoding_utf8_decode_result"),
        "{strict}"
    );
    assert!(!strict.contains("tyrus_encoding_take_error"), "{strict}");
    assert!(!strict.contains("tyrus_encoding_clear_error"), "{strict}");
}

#[test]
fn uncaught_native_errors_print_their_code_or_message() {
    let source = r#"
        import { OsError, allowSleep, openUrl, preventSleep, trash } from "tyrus:os";
        import { CryptoError, rand } from "tyrus:crypto";

        import { FsError, readFileSync } from "tyrus:fs";
        function main(): void {
            match (openUrl("https://example.com")) {
                Ok() => {}
                Err(error) => { console.log(error.code); }
            }
            match (trash("/tmp/tyrus-missing-trash-target")) {
                Ok() => { console.log("unexpected success"); }
                Err(error) => { console.log(error.code); }
            }
            match (preventSleep()) {
                Ok() => {}
                Err(error) => { console.log(error.code); }
            }
            match (allowSleep()) {
                Ok() => {}
                Err(error) => { console.log(error.code); }
            }
            match (rand.randomBytes(4)) {
                Ok(bytes) => { console.log(bytes.length); }
                Err(error) => { console.log(error.code); }
            }
            match (readFileSync("missing.txt")) {
                Ok(text) => { console.log(text); }
                Err(error) => { console.log(error.code); }
            }
        }
    "#;
    let root = std::env::temp_dir().join(format!(
        "tyrus-uncaught-native-errors-{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).unwrap();
    std::fs::write(root.join("main.tyr"), source).unwrap();
    let program = tyrus_core::module_loader::load_program(&root.join("main.tyr"), None).unwrap();
    let output = root.join("uncaught_native_errors");
    compile(
        &program,
        "uncaught_native_errors",
        output.to_str().unwrap(),
        false,
    )
    .unwrap();
    let ir = std::fs::read_to_string(output.with_extension("ll")).unwrap();
    let _ = std::fs::remove_dir_all(&root);
    assert!(
        ir.contains("tyrus_os_trash_result"),
        "OS errors must be returned as Result values:\n{ir}"
    );
    assert!(
        ir.contains("tyrus_fs_read_file_sync_result"),
        "filesystem errors must be returned as Result values:\n{ir}"
    );
    assert!(!ir.contains("tyrus_os_take_error"), "{ir}");
    assert!(!ir.contains("tyrus_fs_take_error"), "{ir}");
}

#[test]
fn clipboard_read_emits_the_structured_os_result_abi() {
    let ir = compile_module_to_ir(
        r#"
            import { OsError, clipboardRead } from "tyrus:os";

            function readClipboard(): Result<string?, OsError> {
                return clipboardRead();
            }
        "#,
        "clipboard-read-result",
    );
    assert!(ir.contains("tyrus_os_clipboard_read_result"), "{ir}");
    assert!(!ir.contains("tyrus_os_clipboard_read()"), "{ir}");
}

#[test]
fn emits_stream_capability_methods_through_the_native_abi() {
    let ir = compile_to_ir(
        r#"
        import { DuplexStream, ReadableStream, StreamError, TransformStream, WritableStream } from "tyrus:stream";

        function read(source: ReadableStream<u8[]>): Promise<u8[]?> {
            return source.read();
        }

        function write(destination: WritableStream<u8[]>, chunk: u8[]): Promise<void> {
            return destination.write(chunk);
        }

        function writeNumbers(destination: WritableStream<i32[]>, chunk: i32[]): Promise<void> {
            return destination.write(chunk);
        }

        function writeText(destination: WritableStream<string>, chunk: string): Promise<void> {
            return destination.write(chunk);
        }

        function pipe(source: ReadableStream<u8[]>, destination: WritableStream<u8[]>): Promise<void> {
            return source.pipeTo(destination);
        }

        function closeRead(source: ReadableStream<u8[]>): Promise<void> {
            return source.close();
        }

        function closeWrite(destination: WritableStream<u8[]>): Promise<void> {
            return destination.close();
        }

        function inspectDuplex(stream: DuplexStream<u8[], string>): Promise<void> {
            const readable: ReadableStream<u8[]> = stream.readable;
            const writable: WritableStream<string> = stream.writable;
            return stream.close();
        }

        function inspectTransform(stream: TransformStream<string, u8[]>): Promise<void> {
            const readable: ReadableStream<u8[]> = stream.readable;
            const writable: WritableStream<string> = stream.writable;
            return stream.close();
        }

        async function readOptional(source: ReadableStream<u8[]>): Promise<void> {
            const chunk: u8[]? = await source.read();
            if (chunk) { console.log(chunk.length); }
        }

        async function captureStreamFailure(source: ReadableStream<u8[]>): Promise<void> {
            try { await source.read(); }
            catch (error: StreamError) { console.error(error.code, error.message); }
        }

        async function retainDuplexAcrossAwait(stream: DuplexStream<u8[], string>): Promise<void> {
            await stream.close();
        }

        async function retainTransformAcrossAwait(stream: TransformStream<string, u8[]>): Promise<void> {
            await stream.close();
        }

        function main(): void {}
        "#,
        "stream_capability_methods",
    );
    for symbol in [
        "tyrus_readable_stream_read",
        "tyrus_writable_stream_write",
        "tyrus_readable_stream_pipe_to",
        "tyrus_readable_stream_close",
        "tyrus_writable_stream_close",
        "tyrus_duplex_stream_readable",
        "tyrus_duplex_stream_writable",
        "tyrus_duplex_stream_close",
        "tyrus_transform_stream_readable",
        "tyrus_transform_stream_writable",
        "tyrus_transform_stream_close",
        "tyrus_readable_stream_free",
        "tyrus_writable_stream_free",
        "tyrus_duplex_stream_retain",
        "tyrus_duplex_stream_free",
        "tyrus_transform_stream_retain",
        "tyrus_transform_stream_free",
        "tyrus_promise_await",
    ] {
        assert!(ir.contains(symbol), "missing `{symbol}` dispatch:\n{ir}");
    }
    assert!(
        ir.contains("stream.write.array_bytes") && ir.contains("stream.write.array_length"),
        "array writes must account capacity from length times element size:\n{ir}"
    );
    assert!(
        ir.contains("str_byte_len"),
        "string writes must account capacity from their UTF-8 byte length:\n{ir}"
    );
}

#[test]
fn emits_stream_adapters_and_methods_through_import_aliases() {
    let ir = compile_module_to_ir(
        r#"
        import { FsError, readFileStream as openInput, writeFileStream as openOutput } from "tyrus:fs";
        import { ReadableStream as InputStream, WritableStream as OutputStream } from "tyrus:stream";

        async function copy(sourcePath: string, destinationPath: string): Promise<Result<void, FsError>> {
            const source: InputStream<u8[]> = openInput(sourcePath)?;
            const destination: OutputStream<u8[]> = openOutput(destinationPath)?;
            await source.pipeTo(destination);
            return Ok();
        }

        function main(): void {}
        "#,
        "stream_import_aliases",
    );
    for symbol in [
        "tyrus_fs_read_file_stream",
        "tyrus_fs_write_file_stream",
        "tyrus_readable_stream_pipe_to",
    ] {
        assert!(ir.contains(symbol), "missing `{symbol}` dispatch:\n{ir}");
    }
}

#[test]
fn emits_child_process_result_and_lifecycle_through_the_native_abi() {
    let ir = compile_module_to_ir(
        r#"
        import { ChildProcess, exec, ProcessError } from "tyrus:process";
        import { ReadableStream, WritableStream } from "tyrus:stream";

        async function run(): Promise<Result<Result<i32, ProcessError>, ProcessError>> {
            const child: ChildProcess = exec("tool", ["--version"])?;
            const input: WritableStream<u8[]> = child.stdin;
            const output: ReadableStream<u8[]> = child.stdout;
            const errors: ReadableStream<u8[]> = child.stderr;
            await input.close();
            const result: Result<i32, ProcessError> = await child.wait();
            return Ok(result);
        }

        function main(): void {}
        "#,
        "process_exec_streams",
    );
    for symbol in [
        "tyrus_process_exec_result",
        "tyrus_child_process_stdin",
        "tyrus_child_process_stdout",
        "tyrus_child_process_stderr",
        "tyrus_child_process_wait",
    ] {
        assert!(ir.contains(symbol), "missing `{symbol}` dispatch:\n{ir}");
    }
    assert!(
        ir.contains("declare void @tyrus_process_exec_result({ ptr, i64 }, ptr, ptr)"),
        "process arguments must cross the native ABI by pointer:\n{ir}"
    );
}

#[test]
fn emits_stream_adapters_through_a_native_module_namespace() {
    let ir = compile_module_to_ir(
        r#"
        import { FsError } from "tyrus:fs";
        import * as fs from "tyrus:fs";
        import { ReadableStream, WritableStream } from "tyrus:stream";

        async function copy(sourcePath: string, destinationPath: string): Promise<Result<void, FsError>> {
            const source: ReadableStream<u8[]> = fs.readFileStream(sourcePath)?;
            const destination: WritableStream<u8[]> = fs.writeFileStream(destinationPath)?;
            await source.pipeTo(destination);
            return Ok();
        }

        function main(): void {}
        "#,
        "stream_module_namespace",
    );
    for symbol in [
        "tyrus_fs_read_file_stream",
        "tyrus_fs_write_file_stream",
        "tyrus_readable_stream_pipe_to",
    ] {
        assert!(ir.contains(symbol), "missing `{symbol}` dispatch:\n{ir}");
    }
}

#[test]
fn thrown_type_tags_that_collide_are_reported_by_name() {
    let by_length = |spelling: &str| spelling.len() as u64;
    assert_eq!(
        crate::statements::first_thrown_tag_collision(
            ["ReadError", "string", "FsError", "OsError"],
            by_length
        ),
        Some(("FsError".to_string(), "OsError".to_string()))
    );
    assert_eq!(
        crate::statements::first_thrown_tag_collision(
            ["ReadError", "WriteError", "string", "FsError"],
            crate::statements::thrown_type_tag_of
        ),
        None
    );
}

#[test]
fn discarding_void_expression_results_skips_value_cleanup() {
    let ir = compile_to_ir(
        r#"
            function main(): void {
                console.log("ready");
            }
        "#,
        "discard-void-expression-result",
    );
    assert!(!ir.is_empty());
    assert!(!ir.contains("native.cleanup.owned"));
}
