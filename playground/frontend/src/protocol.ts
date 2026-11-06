export interface ExecResult {
    stdout: string;
    stderr: string;
    error: string | null;
    exitCode: number;
}

export interface RunRequest {
    id: number;
    type: 'run';
    stream?: ReadableStream<Uint8Array>;
    binary?: Uint8Array<ArrayBuffer>;
}

export interface InitRequest {
    id: number;
    type: 'init';
}

export type WorkerRequest = RunRequest | InitRequest;

export interface RunResponse {
    id: number;
    result: ExecResult;
}

export interface InitResponse {
    id: number;
    result: { ready: true };
}

export interface ErrorResponse {
    id: number;
    error: string;
}

export type WorkerResponse = RunResponse | InitResponse | ErrorResponse;
