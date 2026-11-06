```tyrus
async function produce(stream: WritableStream<u8[]>, bytes: u8[]): Promise<void> {
    await stream.write(bytes);
    await stream.close();
}
```

::: danger error
```tyrus
async function invalid(writable: WritableStream<u8[]>): Promise<void> {
const text: string = "not bytes";
await writable.write(text); // [!code error]
}
```
:::
