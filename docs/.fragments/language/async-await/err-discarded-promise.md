```tyrus
async function fetchData(): Promise<i32> {
    return 42;
}

async function main(): Promise<void> {
    fetchData(); // [!code warning]
}
```
