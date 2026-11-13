```tyrus
interface Example {
    @cep()
    postalCode: string;
}

const valid: Example = { postalCode: "01310-100" };
```

::: danger error
```tyrus
const invalid: Example = { postalCode: "12345" }; // [!code error]
```

```text
error[validator-decorations-failed]: `@cep()` failed for `postalCode`: value must be a valid Brazilian postal code
at /src/main.tyr:6:40
const invalid: Example = { postalCode: "12345" };
                                       ^^^^^^^
```
:::
