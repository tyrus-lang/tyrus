```tyrus
comptime function fibonacci(index: i32): i64 {
    let previous: i64 = 0;
    let current: i64 = 1;

    for (let pos: i32 = 0; pos < index; pos++) {
        const next: i64 = previous + current;
        previous = current;
        current = next;
    }

    return previous;
}

// Evaluated once at compile time, zero runtime calculation:
const maxLimit: i64 = comptime(fibonacci(50));

const main = (): void => {
    print(maxLimit); // 12586269025
};
```

