```tyrus
import { ArchiveError, ArchiveFormat, packStream } from "tyrus:archive";
import { FsError, writeFileStream } from "tyrus:fs";
import { ReadableStream, WritableStream } from "tyrus:stream";

async function backup(): Promise<void> {
    try {
        const archive: ReadableStream<u8[]> = packStream("./site", ArchiveFormat.TarGz);
        const file: WritableStream<u8[]> = writeFileStream("./site.tgz");
        await archive.pipeTo(file);
        print("packed"); // packed
    } catch (error: ArchiveError | FsError) {
        printError(error.code);
    }
}
```
