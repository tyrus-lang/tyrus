```tyrus
async function getData(): Promise<i32> {
    return 42;
}

function sync(): void {
    const value: i32 = await getData(); // [!code error]
}
```
