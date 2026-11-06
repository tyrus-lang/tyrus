```tyrus
async function riskyOperation(): Promise<string> {
    const error: Error = { message: "network timeout", code: "ERR_NETWORK_TIMEOUT" };
    throw error;
}

async function completeAfterCleanup(): Promise<i32> {
    try {
        return 42;
    } catch (error: Error) {
        return 0;
    } finally {
        await Promise.resolve();
        print("cleanup before return");
    }
}

async function main(): Promise<void> {
    try {
        const result: string = await riskyOperation();
        print(result); // "result value"
    } catch (error: Error) {
        printError(error.message); // "network timeout"
    } finally {
        print("cleanup complete");
    }
    print(await completeAfterCleanup()); // 42
}
```
