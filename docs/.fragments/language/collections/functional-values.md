```tyrus
const items: i32[] = [10, 20, 30];
let total: i32 = 0;
for (const value of items.values()) {
    total += value;
}
print(total); // 60
```

::: danger error
```tyrus
const items: i32[] = [10, 20, 30];
const vals: string[] = items.values(); // [!code error]
```

```text
error[E0308]: type mismatch for `vals`: expected `string[]`, found `i32[]`
at /src/main.tyr:2:7
const vals: string[] = items.values();
      ^^^^
help: change the value or the annotation so their types match; convert numbers explicitly with `value as <type>`
```
:::