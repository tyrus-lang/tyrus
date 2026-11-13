```tyrus
async function primarySource(): Promise<string> {
    const error: Error = { message: "primary down", code: "ERR_PRIMARY_DOWN" };
    throw error;
}

async function fallbackSource(): Promise<string> {
    return "fallback-result";
}

async function main(): Promise<void> {
    const result: string = await Promise.any<string>([
        primarySource(),
        fallbackSource(),
    ]);
    print(result); // "fallback-result"
}
```
