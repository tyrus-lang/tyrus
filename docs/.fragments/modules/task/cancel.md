```tyrus
import { cancel, sleepAsync } from "tyrus:task";

const transfer = async (): Promise<void> => {
    try {
        await sleepAsync(1_000);
    } catch (error: Error) {
        // A cancellation never arrives here.
        print("failed");
    } finally {
        print("released");
        print("assert:", "released");
    }
};

const main = async (): Promise<void> => {
    const running: Promise<void> = transfer();
    await sleepAsync(50);
    const asked: boolean = cancel(running, "Cancelled by caller");
    print(asked);
    print("assert:", true);

    try {
        // Returns once `transfer` has finished releasing what it held.
        await running;
    } catch (error: Error) {
        print(error.message);
        print("assert:", "Cancelled by caller");
    }
};
```
