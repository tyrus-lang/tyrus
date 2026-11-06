```tyrus
import { timeout, sleepAsync } from "tyrus:task";

async function slowTask(): Promise<i32> {
    await sleepAsync(300);
    return 42;
}

async function fastTask(): Promise<i32> {
    await sleepAsync(5);
    return 7;
}

async function main(): Promise<void> {
    // The slow task never gets to finish: `timeout` rejects and
    // cooperatively cancels it once the deadline elapses first.
    try {
        const value: i32 = await timeout(50, slowTask());
        print("unexpected", value);
    } catch (error: Error) {
        print(error.message);
    }

    // The fast task settles well within the deadline, so `timeout` simply
    // adopts its outcome.
    const value: i32 = await timeout(500, fastTask());
    print(value);
}
```
