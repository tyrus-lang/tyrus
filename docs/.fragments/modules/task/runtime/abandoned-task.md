```tyrus
import { sleepAsync, spawn } from "tyrus:task";

async function main(): Promise<void> {
    const abandoned = spawn(async (): Promise<void> => {
        await sleepAsync(60000);
        print("never printed");
    });
    print("done");
}
```
