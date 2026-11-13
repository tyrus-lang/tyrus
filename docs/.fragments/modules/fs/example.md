```tyrus
import { FsError, readFile, writeFile, mkdir, rm } from "tyrus:fs";

const run = async (): Promise<Result<void, FsError>> => {
    await writeFile("message.txt", "hello from tyrus")?;
    const message = await readFile("message.txt")?;
    print(message); // hello from tyrus
    mkdir("subdir")?;
    rm("message.txt")?;
    return Ok();
};
```
