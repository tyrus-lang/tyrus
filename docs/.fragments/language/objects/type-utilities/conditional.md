```tyrus
type IsString<T> = T extends string ? boolean : i32;
type FromString = IsString<string>;
const flag: FromString = true;
```

::: danger error
```tyrus
type IsString<T> = T extends string ? boolean : i32;
type FromNumber = IsString<i32>;
const count: FromNumber = "zero"; // [!code error]
```

```text
error[E0308]: type mismatch for `count`: expected `i32`, found `string`
at /src/main.tyr:3:7
const count: FromNumber = "zero";
      ^^^^^
help: change the value or the annotation so their types match; convert numbers explicitly with `value as <type>`
```
:::
