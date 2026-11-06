```tyrus
import { ArchiveError, ArchiveSummary, pack, unpack } from "tyrus:archive";

async function backup(): Promise<Result<void, ArchiveError>> {
    const packed: ArchiveSummary = await pack("./documents", "./backup.zip")?;
    print(packed.entries); // 12

    const restored: ArchiveSummary = await unpack("./backup.zip", "./restored")?;
    print(restored.bytes); // 48213
    return Ok();
}

const main = async (): Promise<void> => {
    match (await backup()) {
        Ok() => { print("done"); } // done
        Err(error) => { printError(error.code); } // ERR_ARCHIVE_NOT_FOUND
    }
};
```
