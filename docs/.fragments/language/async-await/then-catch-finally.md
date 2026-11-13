```tyrus
async function main(): Promise<void> {
    Promise.resolve(42)
        .then((value: i32) => {
            print(value); // 42
        })
        .catch((error: Error) => {
            printError(error.message);
        })
        .finally(() => {
            print("settled");
        });
}
```
