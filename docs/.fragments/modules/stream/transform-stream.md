```tyrus
async function transform(stream: TransformStream<string, u8[]>): Promise<u8[]?> {
    await stream.writable.write("payload");
    return await stream.readable.read();
}
```

::: danger error
```tyrus
async function invalid(stream: TransformStream<string, u8[]>): Promise<void> {
await stream.writable.write([1, 2, 3]); // [!code error]
}
```
:::
