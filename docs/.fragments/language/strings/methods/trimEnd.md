```tyrus
const padded: string = "  hello  ";
const right: string = padded.trimEnd(); // "  hello"
```

::: danger error
```tyrus
const value: i32 = 42;
const right: string = value.trimEnd(); // [!code error]
```

```text
error[E0599]: no method named `trimEnd` for type `i32`
at /src/main.tyr:2:23
const right: string = value.trimEnd();
                      ^^^^^^^^^^^^^^^
help: check the member name and the receiver type; hover the value to see its type
```
:::
