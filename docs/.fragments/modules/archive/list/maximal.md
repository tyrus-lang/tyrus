```tyrus
import { ArchiveEntry, ArchiveEntryKind, ArchiveError, UnpackOptions, list } from "tyrus:archive";

async function inspect(): Promise<Result<void, ArchiveError>> {
    const entries: ArchiveEntry[] = await list("./upload.zip", {
        maxEntries: 1_000, // reject archives with more entries; default 100000
        timeoutMs: 30_000, // give up after this many milliseconds; default 600000 (10 minutes)
    })?;
    const first: ArchiveEntry = entries[0];
    print(first.name); // report.pdf
    print(first.kind === ArchiveEntryKind.File); // true
    print(first.size); // 20480
    print(first.compressedSize ?? 0); // 18311
    return Ok();
}
```
