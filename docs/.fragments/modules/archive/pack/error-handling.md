```tyrus
import { ArchiveError, ArchiveSummary, pack } from "tyrus:archive";

async function packTwice(): Promise<void> {
    const first = await pack("./release", "./release.zip");
    match (await pack("./release", "./release.zip")) {
        Ok(summary) => { print(summary.entries); }
        Err(error) => { printError(error.code); } // ERR_ARCHIVE_DESTINATION_EXISTS
    }
}
```
