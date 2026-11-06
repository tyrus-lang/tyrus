```tyrus
comptime function fibonacci(index: i32): i64 {
    let previous: i64 = 0;
    let current: i64 = 1;

    for (let position: i32 = 0; position < index; position++) {
        const next: i64 = previous + current;
        previous = current;
        current = next;
    }

    return previous;
}

const fibonacci50: i64 = comptime(fibonacci(50));

const main = (): void => {
    print(fibonacci50); // 12586269025
};
```
