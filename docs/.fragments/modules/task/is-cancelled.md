```tyrus
import { cancel, isCancelled, sleepAsync } from "tyrus:task";

const compute = async (): Promise<i64> => {
    let total: i64 = 0;
    for (let round: i32 = 0; round < 500; round += 1) {
        for (let i: i64 = 0; i < 200_000; i += 1) {
            total += i;
        }
        // Checked after the expensive part and before suspending again, which
        // is where a loop can stop itself rather than be interrupted.
        if (isCancelled()) {
            console.log("stopped itself");
            console.log("assert:", "stopped itself");
            return total;
        }
        await sleepAsync(1);
    }
    return total;
};

const main = async (): Promise<void> => {
    const work: Promise<i64> = compute();
    cancel(work, "stop");
    try {
        await work;
    } catch (error: Error) {
        console.log(error.message);
        console.log("assert:", "stop");
    }
};
```
