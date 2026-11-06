```tyrus
async function logMessage(message: string): Promise<void> {
    print(message); // "hello from tyrus"
}

async function main(): Promise<void> {
    await logMessage("hello from tyrus");
}
```
