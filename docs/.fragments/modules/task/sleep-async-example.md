```tyrus
import { sleepAsync } from "tyrus:task";

async function main(): Promise<void> {
    print("before delay");
    await sleepAsync(25);
    print("after delay");
}
```
