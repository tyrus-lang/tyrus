```tyrus
import { CompressFormat, CompressOptions, compressionStream } from "tyrus:compress";
import { TransformStream } from "tyrus:stream";

async function compressChunks(): Promise<void> {
    const brotli: TransformStream<u8[], u8[]> = compressionStream(CompressFormat.Brotli, {
        level: 9, // 0 to 11 for Brotli, 0 to 9 for the others; default 6
    });
    await brotli.writable.write([116, 121, 114, 117, 115]);
    await brotli.writable.close();
    const first: u8[]? = await brotli.readable.read();
    print((first ?? []).length > 0); // true
}
```
