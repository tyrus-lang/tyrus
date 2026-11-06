```tyrus
function createDeferred(): Promise<string> {
    const { promise, resolve, reject } = Promise.withResolvers<string>();
    resolve("done");
    return promise;
}

async function main(): Promise<void> {
    const result: string = await createDeferred();
    print(result); // "done"
}
```
