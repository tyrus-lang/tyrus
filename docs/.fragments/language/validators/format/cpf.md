```tyrus
interface Example {
    @cpf()
    taxId: string;
}

const valid: Example = { taxId: "529.982.247-25" };
```

::: danger error
```tyrus
const invalid: Example = { taxId: "12345678901" }; // [!code error]
```

```text
error[validator-decorations-failed]: `@cpf()` failed for `taxId`: value must be a valid Brazilian CPF
at /src/main.tyr:6:35
const invalid: Example = { taxId: "12345678901" };
                                  ^^^^^^^^^^^^^
```
:::
