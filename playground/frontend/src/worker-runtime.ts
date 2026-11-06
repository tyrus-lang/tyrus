import type { WorkerRequest, WorkerResponse, ExecResult } from './protocol.js';

const WASM_PAGE_BYTES = 64 * 1024;
// Must match --initial-memory in the compiler's wasm-ld invocation: the host
// memory has to be at least as large as the module's static data or
// instantiation fails.
const WASM_MEMORY_PAGES_INITIAL = 16; // 1 MB
const WASM_MEMORY_PAGES_MAXIMUM = 128; // 8 MB
const MEMORY_LIMIT_MB = (WASM_MEMORY_PAGES_MAXIMUM * WASM_PAGE_BYTES) / (1024 * 1024);

interface TyrusWasmExports {
    memory?: WebAssembly.Memory;
    main: () => number | void;
    tyrus_playground_stdout_ptr?: () => number;
    tyrus_playground_stdout_len?: () => number;
    tyrus_playground_stderr_ptr?: () => number;
    tyrus_playground_stderr_len?: () => number;
}

function readWasmString(memory: WebAssembly.Memory, ptr: number, len: number): string {
    const bytes = new Uint8Array(memory.buffer, ptr, len);
    return new TextDecoder('utf-8').decode(bytes);
}

function postResponse(response: WorkerResponse): void {
    self.postMessage(response);
}

function describeTrap(err: unknown, stderrText: string): string {
    // Only the *last* stderr line describes what stopped the program. Earlier
    // lines are ordinary diagnostics the program emitted while still running,
    // an unavailable-API notice, for instance, and reporting one of those as
    // the cause blamed the wrong thing entirely.
    const lastLine = stderrText.split('\n').filter((line) => line.trim()).pop() ?? '';

    if (lastLine) {
        return lastLine.startsWith('out of memory')
            ? `${lastLine} (the playground allows ${MEMORY_LIMIT_MB} MB per run)`
            : lastLine;
    }

    const raw = err instanceof Error ? err.message : String(err);

    if (err instanceof RangeError || /call stack/i.test(raw)) {
        return 'Stack overflow: the program recursed too deeply before returning.';
    }

    if (/unreachable/i.test(raw)) {
        return 'The program stopped unexpectedly (WASM trap).';
    }

    return 'Runtime error: ' + raw;
}

async function handleRun(id: number, request: WorkerRequest & { type: 'run' }): Promise<void> {
    const memory = new WebAssembly.Memory({
        initial: WASM_MEMORY_PAGES_INITIAL,
        maximum: WASM_MEMORY_PAGES_MAXIMUM,
    });

    const imports = {
        env: {
            memory,
            // A WASM instance has no clock of its own, so `instantNow()` reads
            // this instead of a syscall. It is the only host function the
            // runtime imports; everything else in `tyrus:temporal` is
            // arithmetic over values and needs no help.
            tyrus_host_now_millis: (): number => Date.now(),
        },
    };

    let instance: WebAssembly.Instance;
    try {
        const { stream, binary } = request;

        let module: WebAssembly.Module;
        if (stream) {
            const response = new Response(stream, { headers: { 'Content-Type': 'application/wasm' } });
            module = await WebAssembly.compileStreaming(response);
        } else if (binary instanceof Uint8Array) {
            module = await WebAssembly.compile(binary);
        } else {
            throw new Error('No WASM binary or stream provided');
        }

        instance = await WebAssembly.instantiate(module, imports);
    } catch (err) {
        const message = err instanceof Error ? err.message : String(err);
        throw new Error('WASM instantiation failed: ' + message, { cause: err });
    }

    const exports = instance.exports as unknown as TyrusWasmExports;
    const wasmMemory = exports.memory ?? memory;

    try {
        const result = exports.main();

        let stdout = '';
        let stderr = '';

        if (exports.tyrus_playground_stdout_len && exports.tyrus_playground_stdout_ptr) {
            const len = exports.tyrus_playground_stdout_len();
            if (len > 0) {
                stdout = readWasmString(wasmMemory, exports.tyrus_playground_stdout_ptr(), len);
            }
        }

        if (exports.tyrus_playground_stderr_len && exports.tyrus_playground_stderr_ptr) {
            const len = exports.tyrus_playground_stderr_len();
            if (len > 0) {
                stderr = readWasmString(wasmMemory, exports.tyrus_playground_stderr_ptr(), len);
            }
        }

        const execResult: ExecResult = {
            stdout,
            stderr,
            error: null,
            exitCode: (result as number) || 0,
        };
        postResponse({ id, result: execResult });
    } catch (err) {
        let stderrText = '';
        try {
            if (exports.tyrus_playground_stderr_len && exports.tyrus_playground_stderr_ptr) {
                const stderrLen = exports.tyrus_playground_stderr_len();
                if (stderrLen > 0) {
                    stderrText = readWasmString(wasmMemory, exports.tyrus_playground_stderr_ptr(), stderrLen).trim();
                }
            }
        } catch {}

        let stdout = '';
        try {
            if (exports.tyrus_playground_stdout_len && exports.tyrus_playground_stdout_ptr) {
                const len = exports.tyrus_playground_stdout_len();
                if (len > 0) {
                    stdout = readWasmString(wasmMemory, exports.tyrus_playground_stdout_ptr(), len);
                }
            }
        } catch {}

        // Everything but the final line is diagnostics the program produced
        // before it stopped; those belong in the output, not discarded.
        const lines = stderrText.split('\n').filter((line) => line.trim());
        const precedingDiagnostics = lines.slice(0, -1).join('\n');

        const execResult: ExecResult = {
            stdout,
            stderr: precedingDiagnostics,
            error: describeTrap(err, stderrText),
            exitCode: 1,
        };
        postResponse({ id, result: execResult });
    }
}

self.addEventListener('message', async (e: MessageEvent<WorkerRequest>) => {
    const { id, type } = e.data;

    try {
        if (type === 'init') {
            postResponse({ id, result: { ready: true } });
            return;
        }

        if (type === 'run') {
            await handleRun(id, e.data as WorkerRequest & { type: 'run' });
            return;
        }

        throw new Error('Unknown message type: ' + String(type));
    } catch (err) {
        const message = err instanceof Error ? err.message : String(err);
        postResponse({ id, error: message });
    }
});
