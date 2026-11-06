```tyrus
async function main(): Promise<void> {
    try {
        await Promise.reject<i32>({
            message: "multi failure",
        });
    } catch (error: Error) {
        printError(error.message); // "multi failure"
    }
}
```
