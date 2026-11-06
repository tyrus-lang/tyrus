```tyrus
interface Example {
    @url()
    website: string;
}

const valid: Example = { website: "https://example.com" };
```

::: danger error
```tyrus
const invalid: Example = { website: "not-a-url" }; // [!code error]
```

```text
error[validator-decorations-failed]: `@url()` failed for `website`: value must be a valid URL
at /src/main.tyr:6:37
const invalid: Example = { website: "not-a-url" };
                                    ^^^^^^^^^^^
```
:::
