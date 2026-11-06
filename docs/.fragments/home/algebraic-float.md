```tyrus
const main = (): void => {
    const left: f64 = 6.0;
    const right: f64 = 4.0;

    // Explicit compiler permission for fast-math reassociation:
    const sum: f64 = left.algebraicAdd(right);
    const diff: f64 = left.algebraicSub(right);
    const prod: f64 = left.algebraicMul(right);
    const quot: f64 = left.algebraicDiv(right);
    const rem: f64 = left.algebraicRem(right);

    print(prod); // 24.0
};
```

