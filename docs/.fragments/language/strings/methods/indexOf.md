```tyrus
const text: string = "hello world";
const position: i64 = text.indexOf("world"); // 6
const missing: i64 = text.indexOf("rust"); // -1
```

::: danger error
```tyrus
const text: string = "hello world";
const position: i64 = text.indexOf(["hello"]); // [!code error]
```

```text
error[E0308]: type mismatch for `indexOf`: expected `string`, found `string[]`
at /src/main.tyr:2:23
const position: i64 = text.indexOf(["hello"]);
                      ^^^^^^^^^^^^^^^^^^^^^^^
help: change the value or the annotation so their types match; convert numbers explicitly with `value as <type>`
```
:::
