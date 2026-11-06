```tyrus
import { ArchiveError, ArchiveSummary, unpack } from "tyrus:archive";

async function extract(): Promise<Result<ArchiveSummary, ArchiveError>> {
    return await unpack("./upload.zip", "./workspace/upload");
}
```
