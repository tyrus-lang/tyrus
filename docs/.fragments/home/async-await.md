```tyrus
async function loadName(): Promise<string> {
    return "Ada";
}

async function main(): Promise<void> {
    const name = await loadName();
    
    print(`Hello, ${name}`); // Hello, Ada
}
```
