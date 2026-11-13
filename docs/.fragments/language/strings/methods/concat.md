```tyrus
const first: string = "Hello";
const second: string = " World";
const combined: string = first.concat(second); // "Hello World"
```

::: danger error
```tyrus
const first: string = "Hello";
const combined: string = first.concat(42); // [!code error]
```

```text
error[E0308]: type mismatch for `concat`: expected `string`, found `integer literal`
at /src/main.tyr:2:26
const combined: string = first.concat(42);
                         ^^^^^^^^^^^^^^^^
help: change the value or the annotation so their types match; convert numbers explicitly with `value as <type>`
```
:::
