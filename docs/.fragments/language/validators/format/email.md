```tyrus
interface Example {
    @email()
    contactEmail: string;
}

const valid: Example = { contactEmail: "user@example.com" };
```

::: danger error
```tyrus
const invalid: Example = { contactEmail: "not-an-email" }; // [!code error]
```

```text
error[validator-decorations-failed]: `@email()` failed for `contactEmail`: value must be a valid e-mail address
at /src/main.tyr:6:42
const invalid: Example = { contactEmail: "not-an-email" };
                                         ^^^^^^^^^^^^^^
```
:::
