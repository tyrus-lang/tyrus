```tyrus
const main = (): void => {
    const colors: string[] = ["red", "green", "blue"];
    const last: string? = colors.at(-1);
    print(last ?? "none"); // blue
};
```

::: danger error
```tyrus
const main = (): void => {
    const colors: string[] = ["red", "green", "blue"];
    const last: string? = colors.at("x"); // [!code error]
};
```

```text
error[E0308]: type mismatch for `at`: expected `i64`, found `string`
at /src/main.tyr:3:27
    const last: string? = colors.at("x"); // [!code error]
                          ^^^^^^^^^^^^^^
help: change the value or the annotation so their types match; convert numbers explicitly with `value as <type>`
```
:::
