```tyrus
import { ArchiveError, ArchiveSummary, UnpackOptions, unpack } from "tyrus:archive";

async function extract(): Promise<Result<void, ArchiveError>> {
    const summary: ArchiveSummary = await unpack("./tool-v2.tar.gz", "./vendor/tool", {
        overwrite: true, // replace existing files; default false
        stripComponents: 1, // drop leading path components; default 0
        maxEntries: 10_000, // reject archives with more entries; default 100000
        maxBytes: 2_000_000_000, // reject more extracted bytes; default 4 GiB
        maxRatio: 500, // reject entries that expand more than this; default 200
        timeoutMs: 120_000, // give up after this many milliseconds; default 600000 (10 minutes)
    })?;
    print(summary.entries); // 57
    print(summary.bytes); // 3145728
    return Ok();
}
```
