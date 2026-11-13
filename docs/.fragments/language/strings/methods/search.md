```tyrus
const main = (): void => {
    const email: string = "user@example.com";
    const atSign: i32 = email.search(/@/); // 4
    print(atSign); // 4
};
```

::: danger error
```tyrus
const main = (): void => {
    const email: string = "user@example.com";
    const atSign: i32 = email.search("@"); // [!code error]
};
```

```text
error[E0308]: type mismatch for `search`: expected `Regex`, found `string`
at /src/main.tyr:3:25
    const atSign: i32 = email.search("@"); // [!code error]
                        ^^^^^^^^^^^^^^^^^
help: change the value or the annotation so their types match; convert numbers explicitly with `value as <type>`
```
:::
