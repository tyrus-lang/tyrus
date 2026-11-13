```tyrus
import { ArchiveError, ArchiveFormat, PackOptions, packStream } from "tyrus:archive";
import { ReadableStream } from "tyrus:stream";

async function firstChunk(): Promise<void> {
    const archive: ReadableStream<u8[]> = packStream("./site", ArchiveFormat.Zip, {
        level: 9, // 0 (store only) to 9 (smallest output); default 6; not allowed for Tar
        exclude: ["**/*.map"], // glob patterns relative to the source to leave out; default none
        prefix: "site-1.0.0", // directory every entry is placed under; default none
        reproducible: true, // fixed timestamps and normalized permissions; default false
        timeoutMs: 60_000, // fail the stream after this many milliseconds; default 600000 (10 minutes)
    });
    try {
        const first: u8[]? = await archive.read();
        print((first ?? []).length > 0); // true
    } catch (error: ArchiveError) {
        printError(error.code);
    }
}
```
