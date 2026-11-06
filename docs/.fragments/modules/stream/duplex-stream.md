```tyrus
async function relay(stream: DuplexStream<u8[], u8[]>, bytes: u8[]): Promise<void> {
    await stream.writable.write(bytes);
    const received: u8[]? = await stream.readable.read();
    await stream.close();
}
```

::: danger error
```tyrus
async function invalid(stream: DuplexStream<u8[], string>): Promise<void> {
await stream.writable.write([1, 2, 3]); // [!code error]
}
```
:::
