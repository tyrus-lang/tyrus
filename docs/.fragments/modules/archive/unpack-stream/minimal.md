```tyrus
import { ArchiveError, ArchiveSummary, unpackStream } from "tyrus:archive";
import { FsError, readFileStream } from "tyrus:fs";
import { ReadableStream } from "tyrus:stream";

async function restore(): Promise<void> {
    try {
        const input: ReadableStream<u8[]> = readFileStream("./site.tgz");
        const summary = await unpackStream(input, "./restored") onfail {
            printError(summary.code);
            return;
        };
        print(summary.entries); // 3
    } catch (error: FsError) {
        printError(error.code);
    }
}
```
