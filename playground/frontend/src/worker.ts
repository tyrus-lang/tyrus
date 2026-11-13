import type { WorkerRequest, WorkerResponse, ExecResult, InitRequest, RunRequest } from './protocol.js';

const DEFAULT_TIMEOUT_MS = 10000;
// 5 s of execution plus a second of slack for module instantiation, matching
// the limit documented in docs/tooling/playground.md.
export const RUN_TIMEOUT_MS = 6000;

type WorkerResult = ExecResult | { ready: true };

interface PendingEntry {
    resolve: (value: WorkerResult) => void;
    reject: (reason: Error) => void;
    timer: ReturnType<typeof setTimeout>;
}

export interface TyrusWorkerHandle {
    postMessage(msg: Omit<RunRequest, 'id'>, transfer: Transferable[], timeoutMs?: number): Promise<ExecResult>;
    postMessage(msg: Omit<InitRequest, 'id'>, transfer: Transferable[], timeoutMs?: number): Promise<{ ready: true }>;
    terminate(): void;
    reset(): void;
}

export function init(): TyrusWorkerHandle {
    let worker: Worker | null = null;
    let messageId = 0;
    const pending: Record<number, PendingEntry> = {};

    function rejectAllPending(reason: unknown): void {
        for (const id of Object.keys(pending)) {
            const entry = pending[Number(id)];
            if (!entry) continue;
            clearTimeout(entry.timer);
            delete pending[Number(id)];
            entry.reject(reason instanceof Error ? reason : new Error(String(reason)));
        }
    }

    function spawn(): void {
        worker = new Worker(new URL('./worker-runtime.js', import.meta.url), { type: 'module' });

        worker.addEventListener('message', (e: MessageEvent<WorkerResponse>) => {
            const data = e.data;
            const entry = pending[data.id];

            if (entry) {
                delete pending[data.id];
                clearTimeout(entry.timer);

                if ('error' in data) {
                    entry.reject(new Error(data.error));
                } else {
                    entry.resolve(data.result);
                }
            }
        });

        worker.addEventListener('error', (e: ErrorEvent) => {
            console.error('Worker error:', e.message);
            rejectAllPending(new Error('Worker crashed: ' + (e.message || 'unknown error')));
        });
    }

    function respawn(): void {
        if (worker) worker.terminate();
        spawn();
    }

    spawn();

    const postMessage = ((msg: Omit<RunRequest, 'id'> | Omit<InitRequest, 'id'>, transfer: Transferable[], timeoutMs?: number) => {
        const effectiveTimeout = timeoutMs || DEFAULT_TIMEOUT_MS;

        return new Promise<WorkerResult>((resolve, reject) => {
            const id = ++messageId;
            const timer = setTimeout(() => {
                delete pending[id];
                
                respawn();
                reject(new Error('Execution timed out after ' + (effectiveTimeout / 1000) + ' seconds.'));
            }, effectiveTimeout);

            pending[id] = { resolve, reject, timer };

            const msgToSend = { id, ...msg } as WorkerRequest;
            worker!.postMessage(msgToSend, transfer);
        });
    }) as TyrusWorkerHandle['postMessage'];

    return {
        postMessage,
        terminate() {
            rejectAllPending(new Error('Worker terminated'));
            if (worker) {
                worker.terminate();
                worker = null;
            }
        },
        reset() {
            rejectAllPending(new Error('Execution cancelled'));
            respawn();
        },
    };
}
