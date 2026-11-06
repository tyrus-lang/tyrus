```tyrus
async function writeChunk(stream: WritableStream<u8[]>, bytes: u8[]): Promise<void> throws StreamError {
    await stream.write(bytes);
}
```

::: danger error
```tyrus
async function invalid(stream: WritableStream<u8[]>): Promise<void> {
    await stream.write("not bytes"); // [!code error]
}
```
:::
