#!/usr/bin/env bash
# Security test runner for Tyrus Playground.
#
# Checks that `tyrus build --sandbox` and the backend reject code that must not
# run in the playground.
#
# The local checks here previously ran `tyrus build` without ever placing the
# snippet under test, then declared success if the output merely *contained*
# the word "error", which "error: no tyrus.json found" satisfies. The suite
# reported "All tests passed!" from an empty directory. Every local check now
# builds a real project around the snippet and asserts on the specific
# rejection, so a pass means the rule held.
#
# Usage:
#   ./run-all.sh                    # test all snippets against local compiler
#   ./run-all.sh --backend URL      # test against running backend
#   ./run-all.sh --nsjail           # test with nsjail (requires nsjail installed)

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
TYRUS_BIN="${TYRUS_BIN:-tyrus}"
BACKEND_URL=""
USE_NSJAIL=false
PASS=0
FAIL=0

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[0;33m'
NC='\033[0m'

usage() {
    echo "Usage: $0 [--backend URL] [--nsjail]"
    echo ""
    echo "Options:"
    echo "  --backend URL   Test against a running backend instead of local compiler"
    echo "  --nsjail        Test with nsjail isolation"
    echo ""
    echo "Environment variables:"
    echo "  TYRUS_BIN       Path to tyrus binary (default: tyrus)"
    exit 0
}

log_pass() {
    echo -e "  ${GREEN}PASS${NC}: $1"
    PASS=$((PASS + 1))
}

log_fail() {
    echo -e "  ${RED}FAIL${NC}: $1"
    FAIL=$((FAIL + 1))
}

log_skip() {
    echo -e "  ${YELLOW}SKIP${NC}: $1"
}

# ── Parse arguments ──────────────────────────────────────────────────────────

while [[ $# -gt 0 ]]; do
    case $1 in
        --backend)
            BACKEND_URL="$2"
            shift 2
            ;;
        --nsjail)
            USE_NSJAIL=true
            shift
            ;;
        --help|-h)
            usage
            ;;
        *)
            echo "Unknown option: $1"
            usage
            ;;
    esac
done


# ── Helper: compile one snippet in a throwaway project ───────────────────────
#
# Returns the compiler's combined output; the caller asserts on it. The project
# is real (tyrus.json + src/main.tyr) so the compiler reaches the snippet
# instead of failing on a missing project.

compile_local() {
    local file="$1"
    local workdir
    workdir=$(mktemp -d)
    mkdir -p "$workdir/src"
    cp "$file" "$workdir/src/main.tyr"
    printf '%s\n' '{"project":{"name":"sectest","version":"0.1.0"},"scripts":{},"dependencies":{}}' \
        > "$workdir/tyrus.json"
    ( cd "$workdir" && "$TYRUS_BIN" build --compile-target wasm --sandbox 2>&1 ) || true
    rm -rf "$workdir"
}

# Assert that a snippet is refused, and refused for the stated reason.
expect_local_rejection() {
    local label="$1" file="$2" needle="$3"
    local out
    out=$(compile_local "$file")
    if grep -qi -- "$needle" <<< "$out"; then
        log_pass "$label"
    else
        log_fail "$label, expected a rejection mentioning '$needle', got: ${out//$'\n'/ }"
    fi
}

# ── Helper: compile via backend with proper JSON encoding ────────────────────

compile_backend() {
    local file="$1"
    jq -n --rawfile code "$file" '{code: $code}' | \
        curl -s -o /dev/null -w "%{http_code}" \
        -X POST "$BACKEND_URL/api/playground/compile" \
        -H "Content-Type: application/json" \
        --data-binary @-
}

compile_backend_body() {
    local file="$1"
    jq -n --rawfile code "$file" '{code: $code}' | \
        curl -s -X POST "$BACKEND_URL/api/playground/compile" \
        -H "Content-Type: application/json" \
        --data-binary @-
}

# ── Test: Compile fails with --sandbox (import tyrus:fs) ─────────────────────

echo "=== Snippet D: Import tyrus:fs (must fail with --sandbox) ==="
if [ -n "$BACKEND_URL" ]; then
    RESULT=$(compile_backend "$SCRIPT_DIR/D_import_fs.tyr")
    if [ "$RESULT" = "403" ]; then
        log_pass "Backend rejected import tyrus:fs"
    else
        log_fail "Backend returned $RESULT, expected 403"
    fi
else
    expect_local_rejection \
        "Compiler rejected import tyrus:fs with --sandbox" \
        "$SCRIPT_DIR/D_import_fs.tyr" \
        "not available in a sandboxed build"
fi

# ── Test: Compile fails with --sandbox (import tyrus:net) ────────────────────

echo "=== Snippet E: Import tyrus:net (must fail with --sandbox) ==="
if [ -n "$BACKEND_URL" ]; then
    RESULT=$(compile_backend "$SCRIPT_DIR/E_import_net.tyr")
    if [ "$RESULT" = "403" ]; then
        log_pass "Backend rejected import tyrus:net"
    else
        log_fail "Backend returned $RESULT, expected 403"
    fi
else
    expect_local_rejection \
        "Compiler rejected import tyrus:net with --sandbox" \
        "$SCRIPT_DIR/E_import_net.tyr" \
        "not available in a sandboxed build"
fi

# ── Test: Compile fails with --sandbox (import tyrus:process) ────────────────

echo "=== Snippet F: Import tyrus:os (must fail with --sandbox) ==="
if [ -n "$BACKEND_URL" ]; then
    RESULT=$(compile_backend "$SCRIPT_DIR/F_import_os.tyr")
    if [ "$RESULT" = "403" ]; then
        log_pass "Backend rejected import tyrus:os"
    else
        log_fail "Backend returned $RESULT, expected 403"
    fi
else
    expect_local_rejection \
        "Compiler rejected import tyrus:os with --sandbox" \
        "$SCRIPT_DIR/F_import_os.tyr" \
        "not available in a sandboxed build"
fi

# ── Test: Runtime infinite loop times out ─────────────────────────────────────

echo "=== Snippet G: Runtime infinite loop (must timeout) ==="
if [ -n "$BACKEND_URL" ]; then
    HTTP_CODE=$(compile_backend "$SCRIPT_DIR/G_runtime_loop.tyr")
    if [ "$HTTP_CODE" = "200" ]; then
        log_pass "Compiled infinite loop WASM (timeout test requires browser Worker)"
    else
        log_fail "Failed to compile infinite loop (HTTP $HTTP_CODE)"
    fi
else
    TMPDIR_TEST=$(mktemp -d)
    mkdir -p "$TMPDIR_TEST/src"
    cp "$SCRIPT_DIR/G_runtime_loop.tyr" "$TMPDIR_TEST/src/main.tyr"
    echo '{"project":{"name":"test","version":"0.1.0","target":"script"},"scripts":{},"dependencies":{}}' > "$TMPDIR_TEST/tyrus.json"
    pushd "$TMPDIR_TEST" > /dev/null
    RESULT=$("$TYRUS_BIN" build --compile-target wasm --sandbox 2>&1 && echo "BUILD_OK" || echo "BUILD_FAIL")
    popd > /dev/null
    if echo "$RESULT" | grep -q "BUILD_OK"; then
        log_pass "Compiled infinite loop WASM (timeout test requires browser Worker)"
    else
        log_fail "Failed to compile infinite loop"
    fi
    rm -rf "$TMPDIR_TEST"
fi

# ── Test: Large code rejected ────────────────────────────────────────────────

echo "=== Snippet H: Large code (must be rejected by size limit) ==="
LARGE_SIZE=$(wc -c < "$SCRIPT_DIR/H_large_code.tyr")
if [ -n "$BACKEND_URL" ]; then
    RESULT=$(compile_backend "$SCRIPT_DIR/H_large_code.tyr")
    if [ "$RESULT" = "400" ] || [ "$RESULT" = "413" ]; then
        log_pass "Backend rejected ${LARGE_SIZE} byte code"
    else
        log_fail "Backend returned $RESULT, expected 400 or 413"
    fi
else
    log_skip "Large code test requires backend (no backend URL provided)"
fi

# ── Test: a disguised forbidden import is still refused ──────────────────────

echo "=== Snippet I: Forbidden import disguised by a comment ==="
if [ -n "$BACKEND_URL" ]; then
    RESULT=$(compile_backend "$SCRIPT_DIR/I_disguised_import.tyr")
    if [ "$RESULT" = "403" ] || [ "$RESULT" = "422" ]; then
        log_pass "Backend refused the disguised import (HTTP $RESULT)"
    else
        log_fail "Backend returned $RESULT, the comment defeated the check again"
    fi
else
    expect_local_rejection \
        "Compiler rejected the disguised import with --sandbox" \
        "$SCRIPT_DIR/I_disguised_import.tyr" \
        "not available in a sandboxed build"
fi

# ── Test: the size limit applies to the code, not to its encoding ────────────

echo "=== Compressed payload: size limit must apply to the decoded source ==="
if [ -n "$BACKEND_URL" ]; then
    # ~1 MB of source, gzipped and base64-encoded: about 4 KB on the wire, which
    # sailed past an 8 KB limit measured on the payload instead of the code.
    PAYLOAD=$(node -e '
        const { gzipSync } = require("node:zlib");
        const code = "const main = (): void => { console.log(1); };\n".repeat(23000);
        process.stdout.write(JSON.stringify({
            code: gzipSync(Buffer.from(code)).toString("base64"),
            encoding: "gzip",
            permanent: false,
        }));
    ')
    RESULT=$(printf '%s' "$PAYLOAD" | curl -s -o /dev/null -w "%{http_code}" \
        -X POST "$BACKEND_URL/api/playground" \
        -H "Content-Type: application/json" --data-binary @-)
    if [ "$RESULT" = "400" ]; then
        log_pass "Backend rejected 1 MB of source hidden in a 4 KB payload"
    else
        log_fail "Backend returned $RESULT, expected 400, the limit measures the wrong thing"
    fi
else
    log_skip "Compressed size test requires backend (no backend URL provided)"
fi

# ── Test: comptime work is bounded ───────────────────────────────────────────

echo "=== Snippet A: Compile-time loop (must not hang the server) ==="
if [ -n "$BACKEND_URL" ]; then
    START=$(date +%s)
    RESULT=$(compile_backend "$SCRIPT_DIR/A_comptime_loop.tyr")
    ELAPSED=$(( $(date +%s) - START ))
    if [ "$ELAPSED" -le 15 ]; then
        log_pass "Compilation returned in ${ELAPSED}s (HTTP $RESULT)"
    else
        log_fail "Compilation took ${ELAPSED}s, the compile timeout is not holding"
    fi
else
    log_skip "Comptime bound test requires backend (no backend URL provided)"
fi

# ── Summary ──────────────────────────────────────────────────────────────────

echo ""
echo "=== Results ==="
echo -e "  ${GREEN}PASSED${NC}: $PASS"
echo -e "  ${RED}FAILED${NC}: $FAIL"
echo ""

if [ "$FAIL" -gt 0 ]; then
    echo -e "${RED}Some tests failed!${NC}"
    exit 1
else
    echo -e "${GREEN}All tests passed!${NC}"
    exit 0
fi
