```tyrus
import { ArchiveEntry, ArchiveError, list } from "tyrus:archive";

async function inspect(): Promise<Result<void, ArchiveError>> {
    const entries: ArchiveEntry[] = await list("./upload.zip")?;
    print(entries.length); // 3
    print(entries[0].name); // report.pdf
    return Ok();
}
```
