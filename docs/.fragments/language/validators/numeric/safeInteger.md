```tyrus
interface Example {
    @safeInteger()
    largeId: f64;
}

const valid: Example = { largeId: 9007199254740992 };
```

::: danger error
```tyrus
const invalid: Example = { largeId: 9007199254740993 }; // [!code error]
```

```text
error[E0308]: type mismatch for `largeId`: expected `f64`, found `9007199254740993`
at /src/main.tyr:6:28
const invalid: Example = { largeId: 9007199254740993 };
                           ^^^^^^^
help: change the value or the annotation so their types match; convert numbers explicitly with `value as <type>`
```
:::
