```tyrus
async function fetchConfig(): Promise<string> {
    return "config-data";
}

async function main(): Promise<void> {
    print("loading config...");
    const config: string = await fetchConfig();
    print(config); // "config-data"

    print("loading user...");
    const user: string = await fetchUser();
    print(user); // "Ada"
}
```
