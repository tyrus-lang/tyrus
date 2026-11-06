```tyrus
type Fn = (i32) => string;
type Return<T> = T extends (arg: i32) => infer R ? R : never;
type Result = Return<Fn>;
const value: Result = "hello";
```

::: danger error
```tyrus
type Fn = (i32) => string;
type Return<T> = T extends (arg: i32) => infer R ? R : never;
type Result = Return<Fn>;
const n: i32 = 42;
const value: Result = n; // [!code error]
```

```text
error[E0308]: type mismatch for `value`: expected `string`, found `i32`
at /src/main.tyr:5:7
const value: Result = n;
      ^^^^^
help: change the value or the annotation so their types match; convert numbers explicitly with `value as <type>`
```
:::
