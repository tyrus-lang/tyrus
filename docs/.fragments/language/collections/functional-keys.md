```tyrus
const items: string[] = ["x", "y", "z"];
for (const index of items.keys()) {
    print(index); // 0, 1, 2
}
```

::: danger error
```tyrus
const items: string[] = ["x", "y", "z"];
const keys: i64[] = items.keys(); // [!code error]
```

```text
error[E0308]: type mismatch for `keys`: expected `i64[]`, found `usize[]`
at /src/main.tyr:2:7
const keys: i64[] = items.keys();
      ^^^^
help: change the value or the annotation so their types match; convert numbers explicitly with `value as <type>`
```
:::
