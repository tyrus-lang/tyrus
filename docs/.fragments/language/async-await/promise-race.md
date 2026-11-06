```tyrus
import { sleepAsync } from "tyrus:task";

async function fastService(): Promise<string> {
    await sleepAsync(5);
    return "fast-result";
}

async function slowService(): Promise<string> {
    await sleepAsync(50);
    return "slow-result";
}

async function main(): Promise<void> {
    const winner: string = await Promise.race<string>([
        fastService(),
        slowService(),
    ]);
    print(winner); // "fast-result"
}
```

This example uses `sleepAsync` to model different response times and targets the native runtime. The Playground's WASM runtime does not support async execution or Promise combinators yet.
