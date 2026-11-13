const API_BASE = '/api/playground';
const MAX_CODE_SIZE = 8192;

type Encoding = 'gzip' | 'none';

interface CompressedCode {
    data: string;
    encoding: Encoding;
}

export async function share(code: string): Promise<string> {
    // Bytes, not UTF-16 units: the backend measures `Buffer.byteLength`, so
    // measuring `.length` here let anything with accents or emoji pass the
    // client check and fail at the server.
    if (new TextEncoder().encode(code).length > MAX_CODE_SIZE) {
        throw new Error('Code exceeds maximum shareable size (8KB)');
    }

    const compressed = await compressCode(code);

    const response = await fetch(API_BASE, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({
            code: compressed.data,
            permanent: false,
            encoding: compressed.encoding,
        }),
    });

    if (!response.ok) {
        const body = await response.json().catch(() => ({}));
        throw new Error(body.error || 'Failed to create share link');
    }

    const result = await response.json();
    // The server builds the canonical link (it knows the deployment's origin);
    // the `?id=` form is only a fallback for a response without one.
    return new URL(
        result.url ?? `/?id=${result.shortId}`,
        window.location.origin,
    ).toString();
}

export async function load(shortId: string): Promise<string> {
    const response = await fetch(API_BASE + '/' + shortId);
    if (!response.ok) {
        throw new Error('Snippet not found');
    }

    const data = await response.json();
    let code: string = data.code;

    if (data.encoding === 'gzip') {
        const binary = Uint8Array.from(atob(code), (c) => c.charCodeAt(0));
        const body = new Response(binary).body;
        if (!body) {
            throw new Error('Failed to read compressed snippet');
        }
        const decompressed = body.pipeThrough(new DecompressionStream('gzip'));
        code = await readBounded(decompressed);
    }

    return code;
}

/// Read a decompressed snippet, refusing one that expands past the limit.
///
/// The server enforces the same bound on the way in, but a link is opened by
/// whoever receives it, so the browser does not take the stored size on trust.
/// Aborting mid-stream is what keeps a crafted payload from allocating first
/// and being measured second.
async function readBounded(stream: ReadableStream<Uint8Array>): Promise<string> {
    const reader = stream.getReader();
    const chunks: Uint8Array[] = [];
    let total = 0;

    try {
        for (;;) {
            const { done, value } = await reader.read();
            if (done) break;
            total += value.length;
            if (total > MAX_CODE_SIZE) {
                throw new Error('Shared snippet is larger than the 8KB limit');
            }
            chunks.push(value);
        }
    } finally {
        reader.releaseLock();
    }

    const joined = new Uint8Array(total);
    let offset = 0;
    for (const chunk of chunks) {
        joined.set(chunk, offset);
        offset += chunk.length;
    }

    return new TextDecoder().decode(joined);
}

async function compressCode(code: string): Promise<CompressedCode> {
    const byteLength = new TextEncoder().encode(code).length;
    if (byteLength < 100) {
        return { data: code, encoding: 'none' };
    }

    try {
        const encoder = new TextEncoder();
        const data = encoder.encode(code);

        const cs = new CompressionStream('gzip');
        const writer = cs.writable.getWriter();
        writer.write(data);
        writer.close();

        const reader = cs.readable.getReader();
        const chunks: Uint8Array[] = [];

        while (true) {
            const chunk = await reader.read();
            if (chunk.done) break;
            chunks.push(chunk.value);
        }

        const totalLength = chunks.reduce((acc, c) => acc + c.length, 0);
        const result = new Uint8Array(totalLength);
        let offset = 0;

        for (const chunk of chunks) {
            result.set(chunk, offset);
            offset += chunk.length;
        }

        const CHUNK_SIZE = 8192;
        let binaryStr = '';
        for (let chunkStart = 0; chunkStart < result.length; chunkStart += CHUNK_SIZE) {
            binaryStr += String.fromCharCode(...result.subarray(chunkStart, chunkStart + CHUNK_SIZE));
        }
        const base64 = btoa(binaryStr);
        return { data: base64, encoding: 'gzip' };
    } catch {
        return { data: code, encoding: 'none' };
    }
}
