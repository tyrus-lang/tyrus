```tyrus
import { ArchiveError, ArchiveSummary, unpack } from "tyrus:archive";

async function extractUpload(): Promise<void> {
    match (await unpack("./untrusted.zip", "./workspace/upload")) {
        Ok(summary) => { print(summary.entries); }
        Err(error) => {
            printError(error.code); // ERR_ARCHIVE_PATH_ESCAPE
            printError(error.entry ?? ""); // ../../.bashrc
        }
    }
}
```
