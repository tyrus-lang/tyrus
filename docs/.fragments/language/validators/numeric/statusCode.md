```tyrus
interface Example {
    @statusCode()
    responseCode: u16;
}

const valid: Example = { responseCode: 200 };
```

::: danger error
```tyrus
const invalid: Example = { responseCode: 99 }; // [!code error]
```

```text
error[validator-decorations-failed]: `@statusCode()` failed for `responseCode`: value must be a valid HTTP status code between 100 and 599
at /src/main.tyr:6:42
const invalid: Example = { responseCode: 99 };
                                         ^^
```
:::
