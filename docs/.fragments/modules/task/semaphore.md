```tyrus
import { Semaphore, spawn } from "tyrus:task";

async function runLimited(): Promise<void> {
    const limit = Semaphore(2);
    const handle = spawn(async (): Promise<void> => {
        // Holding a permit across an `await` is expected and allowed, a
        // semaphore limits how many tasks enter a region, it does not
        // protect a value the way `Mutex<T>`/`RwLock<T>` do.
        const permit = await limit.acquire();
        print("running");
    });
    await handle.join();
}
```
