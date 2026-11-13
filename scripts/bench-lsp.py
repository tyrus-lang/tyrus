#!/usr/bin/env python3
"""Measure warmed LSP request latency over the stdio protocol."""

import argparse
import json
import math
import os
import pathlib
import shutil
import subprocess
import tempfile
import threading
import time
from urllib.parse import quote

PENDING_RESPONSES = {}
DIAGNOSTICS = []


def send(process, payload):
    body = json.dumps(payload, separators=(",", ":")).encode()
    process.stdin.write(f"Content-Length: {len(body)}\r\n\r\n".encode() + body)
    process.stdin.flush()


def receive(process):
    headers = {}
    while True:
        line = process.stdout.readline()
        if not line:
            raise RuntimeError("LSP process closed stdout before replying")
        if line in (b"\r\n", b"\n"):
            break
        key, value = line.decode("ascii").split(":", 1)
        headers[key.lower()] = value.strip()
    length = int(headers["content-length"])
    body = bytearray()
    while len(body) < length:
        chunk = process.stdout.read(length - len(body))
        if not chunk:
            raise RuntimeError("LSP process returned a truncated message")
        body.extend(chunk)
    return json.loads(body)


def response_for(process, request_id):
    while True:
        if request_id in PENDING_RESPONSES:
            message = PENDING_RESPONSES.pop(request_id)
        else:
            message = receive(process)
        if message.get("method") == "textDocument/publishDiagnostics":
            params = message.get("params", {})
            DIAGNOSTICS.append(
                (time.perf_counter(), params.get("uri"), params.get("diagnostics", []))
            )
            continue
        if message.get("id") == request_id:
            if "error" in message:
                raise RuntimeError(f"LSP request failed: {message['error']}")
            return message.get("result")
        if isinstance(message.get("id"), int):
            PENDING_RESPONSES[message["id"]] = message


def wait_for_diagnostic(process, uri, after_count, matching_events=1):
    while True:
        matching = [timestamp for timestamp, published_uri, _ in DIAGNOSTICS[after_count:] if published_uri == uri]
        if len(matching) >= matching_events:
            return matching[matching_events - 1]
        message = receive(process)
        if message.get("method") == "textDocument/publishDiagnostics":
            params = message.get("params", {})
            DIAGNOSTICS.append(
                (time.perf_counter(), params.get("uri"), params.get("diagnostics", []))
            )
        elif isinstance(message.get("id"), int):
            PENDING_RESPONSES[message["id"]] = message


def percentile(values, fraction):
    ordered = sorted(values)
    return ordered[max(0, math.ceil(fraction * len(ordered)) - 1)]


def child_proc_metrics(pid):
    try:
        status = pathlib.Path(f"/proc/{pid}/status").read_text(encoding="utf-8")
        stat = pathlib.Path(f"/proc/{pid}/stat").read_text(encoding="utf-8").split()
        high_water = next(
            (int(line.split()[1]) for line in status.splitlines() if line.startswith("VmHWM:")),
            None,
        )
        ticks = os.sysconf(os.sysconf_names["SC_CLK_TCK"])
        cpu_seconds = (int(stat[13]) + int(stat[14])) / ticks
        io_values = pathlib.Path(f"/proc/{pid}/io").read_text(encoding="utf-8").splitlines()
        io_bytes = {
            key: int(line.split()[1])
            for line in io_values
            for key in (line.split(":", 1)[0],)
            if key in {"read_bytes", "write_bytes"}
        }
        return cpu_seconds, high_water, io_bytes
    except (OSError, KeyError, ValueError, IndexError):
        return None


def latency_budgets(source_lines):
    if source_lines <= 500:
        return {
            "hover": 25,
            "completion": 20,
            "signatureHelp": 5,
            "definition": 5,
            "references": 10,
            "rename": 10,
            "inlayHint": 10,
            "codeLens": 5,
            "codeAction": 10,
            "semanticTokens": 5,
            "formatting": 15,
            "diagnostics": 350,
            "hoverAfterChange": 250,
            "coldCompletion": 75,
        }
    if source_lines <= 2000:
        return {
            "hover": 60,
            "completion": 45,
            "signatureHelp": 5,
            "definition": 5,
            "references": 35,
            "rename": 35,
            "inlayHint": 30,
            "codeLens": 5,
            "codeAction": 25,
            "semanticTokens": 5,
            "formatting": 45,
            "diagnostics": 700,
            "hoverAfterChange": 500,
            "coldCompletion": 200,
        }
    return {
        "hover": 200,
        "completion": 160,
        "signatureHelp": 10,
        "definition": 10,
        "references": 120,
        "rename": 120,
        "inlayHint": 120,
        "codeLens": 10,
        "codeAction": 100,
        "semanticTokens": 10,
        "formatting": 150,
        "diagnostics": 1500,
        "hoverAfterChange": 1000,
        "coldCompletion": 800,
    }


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--binary", type=pathlib.Path, default=pathlib.Path("target/debug/tyrus"))
    parser.add_argument("--workspace", type=pathlib.Path, default=pathlib.Path.cwd())
    parser.add_argument("--iterations", type=int, default=30)
    parser.add_argument("--open-documents", type=int, default=1, help="number of documents to open in a generated workspace")
    parser.add_argument("--trace", action="store_true", help="enable stage timings on LSP stderr")
    parser.add_argument("--fixture-lines", type=int, help="generate a temporary source fixture with this many lines")
    parser.add_argument(
        "--fixture-profile",
        choices=("flat", "nested", "comments", "imports"),
        default="flat",
        help="synthetic source shape used with --fixture-lines",
    )
    parser.add_argument("--fixture-imports", type=int, default=8, help="module count for the imports profile")
    parser.add_argument("--enforce-budget", action="store_true", help="fail when fixture p95 exceeds the initial LSP budget")
    args = parser.parse_args()
    if args.iterations < 1:
        parser.error("--iterations must be positive")
    if args.open_documents < 1:
        parser.error("--open-documents must be positive")
    if args.open_documents > 1 and args.fixture_lines is None:
        parser.error("--open-documents greater than 1 requires --fixture-lines")
    if args.enforce_budget and args.fixture_lines is None:
        parser.error("--enforce-budget requires --fixture-lines")
    binary = args.binary.resolve()
    generated_workspace = None
    imports = []
    if args.fixture_lines is not None:
        if args.fixture_lines < 4:
            parser.error("--fixture-lines must be at least 4")
        generated_workspace = pathlib.Path(tempfile.mkdtemp(prefix="tyrus-lsp-bench-"))
        (generated_workspace / "src").mkdir()
        workspace = generated_workspace
        if args.fixture_profile == "imports":
            if args.fixture_imports < 1:
                parser.error("--fixture-imports must be positive")
            for index in range(args.fixture_imports):
                imports.append(f'import {{ value{index} }} from "./module{index}";')
                (workspace / "src" / f"module{index}.tyr").write_text(
                    f"export const value{index}: i32 = {index};\n", encoding="utf-8"
                )
        lines = imports + [
            "const benchmarkUnused: i32 = 0;",
            "const double = (value: i32): i32 => value * 2;",
            "const main = (): void => {",
        ]
        remaining = args.fixture_lines - len(lines) - 1
        if args.fixture_profile == "comments":
            statement_count = max(1, remaining // 2)
        else:
            statement_count = max(1, remaining)
        for index in range(statement_count):
            expression = f"double({index})"
            if args.fixture_profile == "nested":
                expression = f"double(double(double({index})))"
            line = f"    print({expression});"
            if args.fixture_profile == "comments":
                lines.append(f"    // Benchmark comment {index}")
            lines.append(line)
        if len(lines) < args.fixture_lines - 1:
            lines.extend("    print(double(0));" for _ in range(args.fixture_lines - len(lines) - 1))
        lines.append("};")
        (workspace / "src" / "main.tyr").write_text("\n".join(lines) + "\n", encoding="utf-8")
    else:
        workspace = args.workspace.resolve()
    source_path = workspace / "src" / "main.tyr"
    if not binary.is_file():
        parser.error(f"compiler binary not found: {binary}")
    if not source_path.is_file():
        parser.error(f"benchmark source not found: {source_path}")
    source = source_path.read_text(encoding="utf-8")
    uri = "file://" + quote(str(source_path))
    open_documents = [(source_path, source)]
    for index in range(1, args.open_documents):
        path = workspace / "src" / f"document_{index}.tyr"
        document_source = source.replace("const main =", f"const main_{index} =")
        path.write_text(document_source, encoding="utf-8")
        open_documents.append((path, document_source))
    process_start_started = time.perf_counter()
    environment = os.environ.copy()
    if args.trace:
        environment["TYRUS_LSP_TRACE"] = "1"
    process = subprocess.Popen(
        [str(binary), "lsp"], stdin=subprocess.PIPE, stdout=subprocess.PIPE,
        stderr=None if args.trace else subprocess.DEVNULL, bufsize=0, env=environment,
    )
    child_metrics = {"cpu_seconds": None, "max_rss_kib": None, "read_bytes": None, "write_bytes": None}
    monitor_stop = threading.Event()

    def monitor_child():
        while not monitor_stop.is_set():
            metrics = child_proc_metrics(process.pid)
            if metrics is not None:
                cpu_seconds, max_rss_kib, io_bytes = metrics
                child_metrics["cpu_seconds"] = cpu_seconds
                child_metrics["max_rss_kib"] = max(child_metrics["max_rss_kib"] or 0, max_rss_kib or 0)
                child_metrics["read_bytes"] = io_bytes.get("read_bytes", 0)
                child_metrics["write_bytes"] = io_bytes.get("write_bytes", 0)
            monitor_stop.wait(0.01)

    monitor = threading.Thread(target=monitor_child, daemon=True)
    monitor.start()
    process_spawn_ms = (time.perf_counter() - process_start_started) * 1000
    request_id = 0

    def request_with_response(method, params):
        nonlocal request_id
        request_id += 1
        message = {"jsonrpc": "2.0", "id": request_id, "method": method}
        if params is not None:
            message["params"] = params
        send(process, message)
        started = time.perf_counter()
        response = response_for(process, request_id)
        return (time.perf_counter() - started) * 1000, response

    def request(method, params):
        return request_with_response(method, params)[0]

    try:
        initialize_ms = request("initialize", {
            "processId": None,
            "rootUri": "file://" + quote(str(workspace)),
            "workspaceFolders": [{"uri": "file://" + quote(str(workspace)), "name": workspace.name}],
            "capabilities": {},
            "initializationOptions": {"codeLensQuickFixesEnable": True},
        })
        print(f"processSpawn: {process_spawn_ms:.3f}ms")
        print(f"initializeRequest: {initialize_ms:.3f}ms")
        send(process, {"jsonrpc": "2.0", "method": "initialized", "params": {}})
        open_diagnostics_before = len(DIAGNOSTICS)
        open_started = time.perf_counter()
        for path, document_source in open_documents:
            send(process, {"jsonrpc": "2.0", "method": "textDocument/didOpen", "params": {
                "textDocument": {
                    "uri": "file://" + quote(str(path)),
                    "languageId": "tyrus",
                    "version": 1,
                    "text": document_source,
                }
            }})
        diagnostic_times = {
            "file://" + quote(str(path)): wait_for_diagnostic(process, "file://" + quote(str(path)), open_diagnostics_before, 2)
            for path, _ in open_documents
        }
        open_diagnostics_at = diagnostic_times[uri]
        diagnostics_open_ms = (open_diagnostics_at - open_started) * 1000
        print(f"diagnosticsAfterOpen: {diagnostics_open_ms:.3f}ms")
        open_batch_ms = (max(diagnostic_times.values()) - open_started) * 1000
        print(f"workspaceOpenDiagnostics: documents={len(open_documents)} total={open_batch_ms:.3f}ms")
        document = {"uri": uri}
        symbol_line = len(imports) + 1
        published_diagnostics = next(
            (
                diagnostics
                for _, published_uri, diagnostics in reversed(DIAGNOSTICS)
                if published_uri == uri
            ),
            [],
        )
        requests = {
            "hover": ("textDocument/hover", {"textDocument": document, "position": {"line": symbol_line, "character": 6}}),
            "completion": ("textDocument/completion", {"textDocument": document, "position": {"line": symbol_line + 1, "character": 10}}),
            "signatureHelp": ("textDocument/signatureHelp", {"textDocument": document, "position": {"line": symbol_line + 2, "character": 8}}),
            "definition": ("textDocument/definition", {"textDocument": document, "position": {"line": symbol_line, "character": 6}}),
            "references": ("textDocument/references", {"textDocument": document, "position": {"line": symbol_line, "character": 6}, "context": {"includeDeclaration": True}}),
            "rename": ("textDocument/rename", {"textDocument": document, "position": {"line": symbol_line, "character": 6}, "newName": "renamedMain"}),
            "inlayHint": ("textDocument/inlayHint", {"textDocument": document, "range": {"start": {"line": 0, "character": 0}, "end": {"line": len(source.splitlines()), "character": 0}}}),
            "codeLens": ("textDocument/codeLens", {"textDocument": document}),
            "codeAction": ("textDocument/codeAction", {"textDocument": document, "range": {"start": {"line": 0, "character": 0}, "end": {"line": len(source.splitlines()), "character": 0}}, "context": {"diagnostics": published_diagnostics}}),
            "semanticTokens": ("textDocument/semanticTokens/full", {"textDocument": document}),
            "formatting": ("textDocument/formatting", {"textDocument": document, "options": {"tabSize": 4, "insertSpaces": True}}),
        }
        if imports:
            imported_symbol_position = {"line": 0, "character": 10}
            requests.update(
                {
                    "definitionImported": (
                        "textDocument/definition",
                        {"textDocument": document, "position": imported_symbol_position},
                    ),
                    "referencesImported": (
                        "textDocument/references",
                        {
                            "textDocument": document,
                            "position": imported_symbol_position,
                            "context": {"includeDeclaration": True},
                        },
                    ),
                    "renameImported": (
                        "textDocument/rename",
                        {
                            "textDocument": document,
                            "position": imported_symbol_position,
                            "newName": "renamedValue",
                        },
                    ),
                }
            )
        cold_completion_ms = None
        budgets = latency_budgets(len(source.splitlines())) if args.enforce_budget else None
        for name, (method, params) in requests.items():
            first, first_response = request_with_response(method, params)
            if name == "semanticTokens":
                token_data = first_response.get("data", []) if isinstance(first_response, dict) else []
                if isinstance(token_data, list):
                    print(
                        f"semanticTokensPayload: bytes={len(json.dumps(first_response, separators=(',', ':')).encode())} tokens={len(token_data) // 5}"
                    )
            if name == "codeAction":
                action_count = len(first_response) if isinstance(first_response, list) else 0
                print(
                    f"codeActionResults: diagnostics={len(params.get('context', {}).get('diagnostics', []))} actions={action_count}"
                )
            if name == "definitionImported":
                if not isinstance(first_response, dict) or not first_response.get("uri", "").endswith("module0.tyr"):
                    raise RuntimeError("imported Go to Definition did not resolve to module0.tyr")
            elif name == "referencesImported":
                if not isinstance(first_response, list) or len(first_response) < 2:
                    raise RuntimeError("imported references did not include the module and import sites")
            elif name == "renameImported":
                changes = first_response.get("changes") if isinstance(first_response, dict) else None
                if not isinstance(changes, dict) or len(changes) < 2:
                    raise RuntimeError("imported rename did not edit both the module and import sites")
            samples = [request(method, params) for _ in range(args.iterations)]
            p50 = percentile(samples, 0.50)
            p95 = percentile(samples, 0.95)
            print(f"{name}: first={first:.3f}ms p50={p50:.3f}ms p95={p95:.3f}ms n={len(samples)}")
            if name == "completion":
                cold_completion_ms = first
            budget_name = {
                "definitionImported": "definition",
                "referencesImported": "references",
                "renameImported": "rename",
            }.get(name, name)
            if budgets is not None and p95 > budgets[budget_name]:
                raise RuntimeError(
                    f"{name} p95 {p95:.3f}ms exceeded the {budgets[budget_name]}ms budget"
                )
        change_started = time.perf_counter()
        change_diagnostics_before = len(DIAGNOSTICS)
        send(process, {"jsonrpc": "2.0", "method": "textDocument/didChange", "params": {
            "textDocument": {"uri": uri, "version": 2},
            "contentChanges": [{"text": source}],
        }})
        change_diagnostics_at = wait_for_diagnostic(process, uri, change_diagnostics_before, 2)
        diagnostics_change_ms = (change_diagnostics_at - change_started) * 1000
        print(f"diagnosticsAfterChange: {diagnostics_change_ms:.3f}ms")
        if budgets is not None and max(diagnostics_open_ms, diagnostics_change_ms) > budgets["diagnostics"]:
            raise RuntimeError(f"diagnostics exceeded the {budgets['diagnostics']}ms budget")
        hover_change_started = time.perf_counter()
        hover_diagnostics_before = len(DIAGNOSTICS)
        params = {"textDocument": document, "position": {"line": symbol_line, "character": 6}}
        hover_after_change_samples = []
        for index in range(args.iterations):
            send(process, {"jsonrpc": "2.0", "method": "textDocument/didChange", "params": {
                "textDocument": {"uri": uri, "version": 3 + index},
                "contentChanges": [{"text": source}],
            }})
            hover_after_change_samples.append(request("textDocument/hover", params))
        hover_change_elapsed_ms = (time.perf_counter() - hover_change_started) * 1000
        hover_after_change_p50 = percentile(hover_after_change_samples, 0.50)
        hover_after_change_p95 = percentile(hover_after_change_samples, 0.95)
        print(
            "hoverAfterChange: "
            f"first={hover_after_change_samples[0]:.3f}ms "
            f"p50={hover_after_change_p50:.3f}ms "
            f"p95={hover_after_change_p95:.3f}ms "
            f"n={len(hover_after_change_samples)} elapsed={hover_change_elapsed_ms:.3f}ms"
        )
        if budgets is not None:
            if hover_after_change_p95 > budgets["hoverAfterChange"]:
                raise RuntimeError(f"hover after change p95 exceeded the {budgets['hoverAfterChange']}ms budget")
            if cold_completion_ms is not None and cold_completion_ms > budgets["coldCompletion"]:
                raise RuntimeError(f"cold completion exceeded the {budgets['coldCompletion']}ms budget")
            print("LSP latency budgets: PASS")
        wait_for_diagnostic(process, uri, hover_diagnostics_before, 2)
        request("shutdown", None)
        send(process, {"jsonrpc": "2.0", "method": "exit", "params": None})
    finally:
        if process.poll() is None:
            process.terminate()
        process.wait(timeout=5)
        monitor_stop.set()
        monitor.join(timeout=1)
        cpu = child_metrics["cpu_seconds"]
        max_rss = child_metrics["max_rss_kib"]
        read_bytes = child_metrics["read_bytes"]
        write_bytes = child_metrics["write_bytes"]
        process_metrics = [
            ("cpu", cpu, "s"),
            ("maxRss", max_rss, "KiB"),
            ("readBytes", read_bytes, "B"),
            ("writeBytes", write_bytes, "B"),
        ]
        print("childProcess: " + " ".join(
            f"{name}={value:.3f}{unit}" if name == "cpu" and value is not None else
            f"{name}={value}{unit}" if value is not None else
            f"{name}=unavailable"
            for name, value, unit in process_metrics
        ))
        if generated_workspace is not None:
            shutil.rmtree(generated_workspace, ignore_errors=True)


if __name__ == "__main__":
    main()
