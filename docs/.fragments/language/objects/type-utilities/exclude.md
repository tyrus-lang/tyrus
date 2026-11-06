```tyrus
type T = Exclude<"a" | "b" | "c", "a">;
const v: T = "b";
```

::: danger error
```tyrus
type T = Exclude<"a" | "b" | "c", "a">;
const v: T = "a"; // [!code error]
```

```text
error[E0308]: type mismatch for `v`: expected `"b" | "c"`, found `string`
at /src/main.tyr:2:7
const v: T = "a";
      ^
help: change the value or the annotation so their types match; convert numbers explicitly with `value as <type>`
```
:::
