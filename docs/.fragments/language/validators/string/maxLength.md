```tyrus
interface Example {
    @maxLength(50)
    displayName: string;
}

const valid: Example = { displayName: "A".repeat(10) };
```

::: danger error
```tyrus
const invalid: Example = { displayName: "A".repeat(51) }; // [!code error]
```

```text
error[unhandled-throw]: validation of field `displayName` may throw `ValidationError`; catch it or add `throws ValidationError` to the enclosing function
at /src/main.tyr:6:41
const invalid: Example = { displayName: "A".repeat(51) };
                                        ^^^^^^^^^^^^^^
```
:::
