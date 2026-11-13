```tyrus
async function main(): Promise<void> {
    const resolved: Promise<i32> = Promise.resolve(42);
    const value: i32 = await resolved;
    print(value); // 42

    try {
        const rejected: Promise<i32> = Promise.reject<i32>({ message: "boom", code: "ERR_BOOM" });
        await rejected;
    } catch (error: Error) {
        printError(error.message); // "boom"
    }
}
```
