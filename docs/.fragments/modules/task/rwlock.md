```tyrus
import { RwLock, spawn } from "tyrus:task";

async function main(): Promise<void> {
    const config = RwLock<string>("production");
    const handle = spawn(async (): Promise<void> => {
        // Read access is shared: multiple `read()` guards may be held at
        // once. `read.value` is read-only, assigning through it is a
        // compile error.
        const read = await config.read();
        print(read.value);
    });
    await handle.join();
}
```
