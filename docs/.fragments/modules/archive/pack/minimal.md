```tyrus
import { ArchiveError, ArchiveSummary, pack } from "tyrus:archive";

async function packRelease(): Promise<Result<ArchiveSummary, ArchiveError>> {
    return await pack("./release", "./release.tar.gz");
}
```
