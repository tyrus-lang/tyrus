::: danger error
```tyrus
async function value(): Promise<i32> {
    return 7;
}

async function main(): Promise<void> {
    const pending = value();
    const first = await pending;
    const second = await pending; // [!code error]
    print(first + second);
}
```

```text
error[async-promise-already-awaited]: `pending` was already consumed by an earlier `await`; a `Promise<T>` can only be awaited once
at /src/main.tyr:8:26
    const second = await pending;
                         ^^^^^^^
help: store the awaited value in a binding and read that instead of awaiting `pending` again
```
:::
