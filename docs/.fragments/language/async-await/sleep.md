```tyrus
import { sleepAsync } from "tyrus:task";

async function countdown(): Promise<void> {
    for (let i: i32 = 3; i > 0; i -= 1) {
        print(i);
        await sleepAsync(1000);
    }
    print("go!");
}

async function main(): Promise<void> {
    await countdown();
}
```
