```tyrus
import { ArchiveError, ArchiveSummary, UnpackOptions, unpackStream } from "tyrus:archive";
import { ReadableStream } from "tyrus:stream";

async function extract(upload: ReadableStream<u8[]>): Promise<Result<void, ArchiveError>> {
    const summary: ArchiveSummary = await unpackStream(upload, "./workspace", {
        overwrite: true, // replace files that already exist; default false
        stripComponents: 1, // drop this many leading path components; default 0
        maxEntries: 1_000, // refuse archives with more entries; default 100000
        maxBytes: 50_000_000, // refuse more extracted bytes; default 4 GiB
        maxRatio: 100, // refuse a TAR.GZ expanding past this ratio; default 200
        timeoutMs: 30_000, // give up after this many milliseconds; default 600000 (10 minutes)
    })?;
    print(summary.entries); // 3
    return Ok();
}
```
