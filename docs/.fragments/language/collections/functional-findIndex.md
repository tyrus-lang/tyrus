```tyrus
const items: string[] = ["apple", "banana", "cherry"];
const index: i64 = items.findIndex(item => item === "banana");
// 1
```

::: danger error
```tyrus
const items: string[] = ["apple", "banana", "cherry"];
const index: i64 = items.findIndex(item => item.length); // [!code error]
```

```text
error[E0308]: type mismatch for `return`: expected `boolean`, found `usize`
at /src/main.tyr:2:44
const index: i64 = items.findIndex(item => item.length);
                                           ^^^^^^^^^^^
help: change the value or the annotation so their types match; convert numbers explicitly with `value as <type>`
```
:::
