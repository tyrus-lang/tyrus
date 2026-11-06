```tyrus
import { Mutex, spawn } from "tyrus:task";

async function main(): Promise<void> {
    const counter = Mutex<u32>(0);
    const handle = spawn(async (): Promise<void> => {
        // `guard.value` reads and writes the protected value in place; the
        // lock releases automatically once `guard` goes out of scope.
        const guard = await counter.lock();
        guard.value += 1;
    });
    await handle.join();
}
```
