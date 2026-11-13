```tyrus
import { ArchiveError, ArchiveSummary, unpack } from "tyrus:archive";

async function extract(): Promise<void> {
    const result: Result<ArchiveSummary, ArchiveError> = await unpack("./upload.zip", "./out");
    match (result) {
        Ok(summary) => { print(summary.entries); }
        Err(error) => {
            printError(error.code); // ERR_ARCHIVE_UNSAFE_LINK
            printError(error.entry ?? ""); // docs/latest
        }
    }
}
```
