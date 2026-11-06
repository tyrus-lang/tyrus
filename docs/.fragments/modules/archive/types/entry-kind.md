```tyrus
import { ArchiveEntry, ArchiveEntryKind, ArchiveError, list } from "tyrus:archive";

async function hasLinks(): Promise<Result<boolean, ArchiveError>> {
    const entries: ArchiveEntry[] = await list("./upload.tar")?;
    return Ok(entries.some((entry: ArchiveEntry): boolean => entry.kind === ArchiveEntryKind.Symlink));
}
```
