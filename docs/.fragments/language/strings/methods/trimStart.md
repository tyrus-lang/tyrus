```tyrus
const padded: string = "  hello  ";
const left: string = padded.trimStart(); // "hello  "
```

::: danger error
```tyrus
const value: i32 = 42;
const left: string = value.trimStart(); // [!code error]
```

```text
error[E0599]: no method named `trimStart` for type `i32`
at /src/main.tyr:2:22
const left: string = value.trimStart();
                     ^^^^^^^^^^^^^^^^^
help: check the member name and the receiver type; hover the value to see its type
```
:::
