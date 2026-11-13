```tyrus
async function main(): Promise<void> {
    const value: i32 = await Promise.resolve(42);
    print(value); // 42

    const [left, right] = await Promise.all<i32>([
        Promise.resolve(20),
        Promise.resolve(22),
    ]);
    print(left + right); // 42

    const winner: i32 = await Promise.race<i32>([
        Promise.resolve(42),
        Promise.resolve(7),
    ]);
    print(winner); // 42

    const { promise, resolve } = Promise.withResolvers<i32>();
    resolve(99);
    const result: i32 = await promise;
    print(result); // 99
}
```
