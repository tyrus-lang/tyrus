#!/usr/bin/env bash
#
# Shared helper for the Rust-unit-test-based leak checks
# (`scripts/leak-check-repl`, `scripts/leak-check-jit`). Not runnable on its
# own, source it and call `run_rust_leak_test <test_name>`.
#
# Appends to the caller's `failures` array and expects the caller to have
# already `cd`'d into the project root.

run_rust_leak_test() {
    local test_name="$1"
    echo "    running   ${test_name}"

    # Build the test binary (no-run builds faster without execution)
    if ! cargo test --no-run --bin tyrus -- "${test_name}" 2>/dev/null; then
        echo "    BUILD FAILED  ${test_name}"
        failures+=("${test_name} (build failed)")
        return
    fi

    # Find the test binary (newest matching the test name via deps/)
    local test_binary
    test_binary="$(ls -t target/debug/deps/tyrus-* 2>/dev/null | head -1)"
    if [[ -z ${test_binary} ]]; then
        echo "    NOT FOUND     ${test_name}: no test binary"
        failures+=("${test_name} (test binary not found)")
        return
    fi

    local report
    report="$(valgrind --leak-check=full --show-leak-kinds=definite,indirect --error-exitcode=0 \
        "${test_binary}" "${test_name}" --nocapture --test-threads=1 2>&1 >/dev/null)"
    local test_result=$?

    # Check test passed
    if [[ ${test_result} -ne 0 ]]; then
        echo "    TEST FAILED   ${test_name}"
        failures+=("${test_name} (test exited ${test_result})")
        return
    fi

    # Corruption check
    if printf '%s\n' "${report}" | grep -qE 'Invalid free|Invalid read|Invalid write|mismatched'; then
        echo "    CORRUPTED     ${test_name}: invalid free or invalid access"
        failures+=("${test_name} corrupted memory")
        return
    fi

    # Extract definitely-lost bytes
    local lost
    lost="$(printf '%s\n' "${report}" |
        grep 'definitely lost:' |
        sed 's/.*definitely lost: //; s/ bytes.*//; s/,//g')"
    lost="${lost:-0}"

    if [[ ${lost} == "0" ]]; then
        echo "    ok            ${test_name}"
    else
        echo "    REGRESSED     ${test_name}: ${lost} bytes definitely lost"
        failures+=("${test_name} leaked ${lost} bytes")
    fi
}
