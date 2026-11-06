```tyrus
interface Example {
    @notNull()
    requiredField: string?;
}

const valid: Example = { requiredField: "hello" };
```

::: danger error
```tyrus
const invalid: Example = { requiredField: null }; // [!code error]
```

```text
error[unhandled-throw]: validation of field `requiredField` may throw `ValidationError`; catch it or add `throws ValidationError` to the enclosing function
at /src/main.tyr:6:43
const invalid: Example = { requiredField: null };
                                          ^^^^
error[validator-decorations-failed]: `@notNull()` failed for `requiredField`: value must not be null
at /src/main.tyr:6:43
const invalid: Example = { requiredField: null };
                                          ^^^^
error[E0308]: type mismatch for `requiredField`: expected `string?`, found `null`
at /src/main.tyr:6:28
const invalid: Example = { requiredField: null };
                           ^^^^^^^^^^^^^
help: change the value or the annotation so their types match; convert numbers explicitly with `value as <type>`
```
:::
