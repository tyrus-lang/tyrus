```tyrus
const padded: string = "  hello  ";
const clean: string = padded.trim(); // "hello"
```

::: danger error
```tyrus
const value: i32 = 42;
const clean: string = value.trim(); // [!code error]
```

```text
error[E0599]: no method named `trim` for type `i32`
at /src/main.tyr:2:23
const clean: string = value.trim();
                      ^^^^^^^^^^^^
help: check the member name and the receiver type; hover the value to see its type
```
:::
