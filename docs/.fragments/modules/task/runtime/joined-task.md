```tyrus
import { sleepAsync, spawn } from "tyrus:task";

async function main(): Promise<void> {
    const handle = spawn(async (): Promise<void> => {
        await sleepAsync(10);
        print("finished");
    });
    await handle.join();
}
```
