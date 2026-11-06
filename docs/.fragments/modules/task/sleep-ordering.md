```tyrus
import { sleep } from "tyrus:task";

function main(): void {
    sleep(1000);
    print("first"); // Prints first.
    print("second"); // Prints second.
}
```

```tyrus
import { sleepAsync, spawn } from "tyrus:task";

async function first(): Promise<void> {
    await sleepAsync(1000);
    print("first"); // Prints second, after sleepAsync completes.
}

async function second(): Promise<void> {
    print("second"); // Prints first, while the first task waits.
}

async function main(): Promise<void> {
    const firstTask = spawn(first);
    const secondTask = spawn(second);
    await firstTask.join();
    await secondTask.join();
}
```
