```tyrus
interface Example {
    @cnpj()
    companyTaxId: string;
}

const valid: Example = { companyTaxId: "11.444.777/0001-61" };
```

::: danger error
```tyrus
const invalid: Example = { companyTaxId: "12345678901234" }; // [!code error]
```

```text
error[validator-decorations-failed]: `@cnpj()` failed for `companyTaxId`: value must be a valid Brazilian CNPJ
at /src/main.tyr:6:42
const invalid: Example = { companyTaxId: "12345678901234" };
                                         ^^^^^^^^^^^^^^^^
```
:::
