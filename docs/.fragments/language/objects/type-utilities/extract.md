```tyrus
type T = Extract<"a" | "b" | "c", "a" | "b">;
const v: T = "a";
```

::: danger error
```tyrus
type T = Extract<"a" | "b" | "c", "a" | "b">;
const v: T = "c"; // [!code error]
```

```text
error[E0308]: type mismatch for `v`: expected `"a" | "b"`, found `string`
at /src/main.tyr:2:7
const v: T = "c";
      ^
help: change the value or the annotation so their types match; convert numbers explicitly with `value as <type>`
```
:::
