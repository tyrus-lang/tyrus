```tyrus
import { CompressFormat, compressionStream } from "tyrus:compress";
import { readFileStream, writeFileStream } from "tyrus:fs";
import { TransformStream } from "tyrus:stream";

async function compressLog(): Promise<void> {
    try {
        const gzip: TransformStream<u8[], u8[]> = compressionStream(CompressFormat.Gzip);
        const feeding = readFileStream("./app.log").pipeTo(gzip.writable);
        const draining = gzip.readable.pipeTo(writeFileStream("./app.log.gz"));
        await feeding;
        await draining;
        print("compressed"); // compressed
    } catch (error: Error) {
        printError(error.message);
    }
}
```
