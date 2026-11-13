```tyrus
interface Example {
    @isUppercase()
    countryCode: string;
}

const valid: Example = { countryCode: "BR" };
```

::: danger error
```tyrus
const invalid: Example = { countryCode: "br" }; // [!code error]
```

```text
error[validator-decorations-failed]: `@isUppercase()` failed for `countryCode`: value must already be uppercase
at /src/main.tyr:6:41
const invalid: Example = { countryCode: "br" };
                                        ^^^^
```
:::
