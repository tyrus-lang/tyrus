```tyrus
async function process(enabled: boolean): Promise<i32> {
    let total: i32 = 0;
    let outer: i32 = 0;
    while (outer < 2) {
        outer += 1;
        let inner: i32 = 0;
        while (inner < 3) {
            inner += 1;
            if (inner === 2) {
                await Promise.resolve();
                continue;
            }
            if (outer === 2 && inner === 3) {
                await Promise.resolve();
                break;
            }
            total += outer * 10 + inner;
        }
    }
    while (enabled) {
        if (total > 0) {
            const label: string = await Promise.resolve<string>("selected");
            await Promise.resolve<void>();
            total += await Promise.resolve<i32>(7);
            print(label);
            break;
        } else {
            const value: i32 = await Promise.resolve<i32>(9);
            await Promise.resolve<void>();
            total += value;
            break;
        }
    }
    return total;
}
```
