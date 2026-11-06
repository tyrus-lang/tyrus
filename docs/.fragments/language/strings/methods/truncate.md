```tyrus
const text: string = "Hello, World!";
const short: string = text.truncate(5); // "Hello"
const small: string = text.truncate(3); // "Hel"
```

::: danger error
```tyrus
const text: string = "hello";
const bad: string = text.truncate("ten"); // [!code error]
```

```text
error[E0308]: type mismatch for `truncate`: expected `usize`, found `string`
at /src/main.tyr:2:21
const bad: string = text.truncate("ten");
                    ^^^^^^^^^^^^^^^^^^^^
help: change the value or the annotation so their types match; convert numbers explicitly with `value as <type>`
```
:::
