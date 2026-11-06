```tyrus
const main = (): void => {
    const letter: string = "A";
    const code: i32 = letter.charCodeAt(0); // 65
    print(code); // 65
};
```

::: danger error
```tyrus
const main = (): void => {
    const letter: string = "A";
    const code: i32 = letter.charCodeAt("0"); // [!code error]
};
```

```text
error[E0308]: type mismatch for `charCodeAt`: expected `usize`, found `string`
at /src/main.tyr:3:23
    const code: i32 = letter.charCodeAt("0"); // [!code error]
                      ^^^^^^^^^^^^^^^^^^^^^^
help: change the value or the annotation so their types match; convert numbers explicitly with `value as <type>`
```
:::
