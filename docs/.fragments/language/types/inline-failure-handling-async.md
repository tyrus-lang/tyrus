```tyrus
import { ArchiveError, ArchiveSummary, pack } from "tyrus:archive";

const main = async (): Promise<void> => {
    const summary = await pack("./data", "./backup.zip") onfail {
        printError(summary.code);
    };

    print(summary.entries);
};
```
