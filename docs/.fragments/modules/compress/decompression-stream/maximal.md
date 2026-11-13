```tyrus
import { CompressError, CompressFormat, DecompressOptions, decompressionStream } from "tyrus:compress";
import { ReadableStream, TransformStream } from "tyrus:stream";

async function restore(source: ReadableStream<u8[]>): Promise<void> {
    const brotli: TransformStream<u8[], u8[]> = decompressionStream({
        format: CompressFormat.Brotli, // required for Brotli and DeflateRaw; default detected from the data
        maxOutputBytes: 1_000_000_000, // fail the stream past this much output; default 4 GiB
        maxRatio: 1_000, // fail the stream past this expansion once past 1 MiB; default 200
    });
    try {
        await source.pipeTo(brotli.writable);
        const first: u8[]? = await brotli.readable.read();
        print((first ?? []).length); // 5
    } catch (error: CompressError) {
        printError(error.code);
    }
}
```
