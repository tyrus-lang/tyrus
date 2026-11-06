```tyrus
import { ArchiveError, ArchiveSummary, PackOptions, pack } from "tyrus:archive";

async function packRelease(): Promise<Result<void, ArchiveError>> {
    const summary: ArchiveSummary = await pack("./release", "./release-1.0.0.tar.gz", {
        level: 9, // 0 (store only) to 9 (smallest output); default 6
        overwrite: true, // replace an existing file; default false
        exclude: ["**/*.map", "tmp"], // globs relative to the source; default none
        prefix: "release-1.0.0", // directory that holds every entry; default none
        reproducible: true, // zero timestamps and normalize permissions; default false
        timeoutMs: 120_000, // give up after this many milliseconds; default 600000 (10 minutes)
    })?;
    print(summary.entries); // 42
    print(summary.bytes); // 1048576
    return Ok();
}
```
