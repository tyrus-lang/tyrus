```tyrus
async function fetchUser(): Promise<string> {
    return "Ada";
}

async function main(): Promise<void> {
    const name: string = await fetchUser();
    print(name); // "Ada"
}
```
