```tyrus
import { cancel, sleepAsync } from "tyrus:task";

async function longRunning(): Promise<string> {
    await sleepAsync(5000);
    return "done";
}

async function main(): Promise<void> {
    const pending: Promise<string> = longRunning();
    const wasCancelled: boolean = cancel(pending, "user abort");
    print(wasCancelled); // true

    try {
        const result: string = await pending;
    } catch (error: Error) {
        printError(error.message); // "user abort"
    }
}
```
