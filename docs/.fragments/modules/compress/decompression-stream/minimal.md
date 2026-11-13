```tyrus
import { CompressError, decompressionStream } from "tyrus:compress";
import { FsError, readFileStream, writeFileStream } from "tyrus:fs";
import { TransformStream } from "tyrus:stream";

async function restoreLog(): Promise<void> {
    try {
        const gunzip: TransformStream<u8[], u8[]> = decompressionStream();
        const feeding = readFileStream("./app.log.gz").pipeTo(gunzip.writable);
        const draining = gunzip.readable.pipeTo(writeFileStream("./restored.log"));
        await feeding;
        await draining;
        print("restored"); // restored
    } catch (error: FsError | CompressError) {
        printError(error.code);
    }
}
```
