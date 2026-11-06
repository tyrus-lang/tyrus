```tyrus
async function loadUsers(): Promise<string[]> {
    return ["Ada", "Grace"];
}

async function loadScores(): Promise<i32[]> {
    return [95, 87];
}

async function main(): Promise<void> {
    const [users, scores] = await Promise.all<string[] | i32[]>([
        loadUsers(),
        loadScores(),
    ]);
    print(users);  // ["Ada", "Grace"]
    print(scores); // [95, 87]
}
```
