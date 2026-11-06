```tyrus
async function consume(stream: ReadableStream<u8[]>, destination: WritableStream<u8[]>): Promise<void> {
    const first: u8[]? = await stream.read();
    if (first) {
        await destination.write(first);
    }
    await stream.pipeTo(destination);
}
```

::: danger error
```tyrus
async function invalid(readable: ReadableStream<u8[]>, bytes: u8[]): Promise<void> {
await readable.write(bytes); // [!code error]
}
```
:::
