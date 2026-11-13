```tyrus
async function stop(child: ChildProcess): Promise<Result<i32, ProcessError>> {
    const output: ReadableStream<u8[]> = child.stdout;
    await child.stdin.close();
    await child.kill()?;
    return await child.wait();
}
```

::: danger error
```tyrus
async function invalid(child: ChildProcess): Promise<void> {
await child.stdout.write([1, 2, 3]); // [!code error]
}
```
:::
