```tyrus
const text: string = "order-123";
const result: string[]? = text.match(/^order-(\d+)$/);
if (result) {
    const orderId: string = result[0]; // "order-123"
    const digits: string = result[1]; // "123"
}
```

::: danger error
```tyrus
const text: string = "order-123";
const result: string[]? = text.match("order"); // [!code error]
```

```text
error[E0308]: type mismatch for `match`: expected `Regex`, found `string`
at /src/main.tyr:2:27
const result: string[]? = text.match("order");
                          ^^^^^^^^^^^^^^^^^^^
help: change the value or the annotation so their types match; convert numbers explicitly with `value as <type>`
```
:::
