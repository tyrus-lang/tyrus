```tyrus
async function main(): Promise<void> {
    let total: i32 = 0;
    while (true) { // [!code warning]
        total += 1;
    }
}
```
