```tyrus
const text: string = "Hello, World!";
const short: string = text.truncateEllipsis(8); // "Hello..."
const fits: string = "hello".truncateEllipsis(10); // "hello"
```

::: danger error
```tyrus
const text: string = "hello";
const bad: string = text.truncateEllipsis("ten"); // [!code error]
```

```text
error[E0308]: type mismatch for `truncateEllipsis`: expected `usize`, found `string`
at /src/main.tyr:2:21
const bad: string = text.truncateEllipsis("ten");
                    ^^^^^^^^^^^^^^^^^^^^^^^^^^^^
help: change the value or the annotation so their types match; convert numbers explicitly with `value as <type>`
```
:::
