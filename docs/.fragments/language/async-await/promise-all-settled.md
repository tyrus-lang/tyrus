```tyrus
async function main(): Promise<void> {
    const results: PromiseSettledResult<i32>[] = await Promise.allSettled<i32>([
        Promise.resolve(42),
        Promise.reject<i32>({ message: "expected failure", code: "ERR_EXPECTED_FAILURE" }),
    ]);

    for (const result of results) {
        if (result.status === "fulfilled") {
            print(`resolved: ${result.value}`); // "resolved: 42"
        } else {
            print(`rejected: ${result.reason.message}`); // "rejected: expected failure"
        }
    }
}
```
