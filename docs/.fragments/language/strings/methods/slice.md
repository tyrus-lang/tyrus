```tyrus
const code: string = "tyrus-lang";
const lang: string = code.slice(6); // "lang"
const sub: string = code.slice(0, 5); // "tyrus"
```

::: danger error
```tyrus
const code: string = "tyrus-lang";
const result: string = code.slice("6"); // [!code error]
```

```text
error[E0308]: type mismatch for `slice`: expected `i64`, found `string`
at /src/main.tyr:2:24
const result: string = code.slice("6");
                       ^^^^^^^^^^^^^^^
help: change the value or the annotation so their types match; convert numbers explicitly with `value as <type>`
```
:::
