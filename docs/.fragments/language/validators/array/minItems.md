```tyrus
interface Example {
    @minItems(1)
    tags: string[];
}

const valid: Example = { tags: ["hello"] };
```

::: danger error
```tyrus
const invalid: Example = { tags: [] }; // [!code error]
```

```text
error[validator-decorations-failed]: `@minItems(1)` failed for `tags`: array must contain at least 1 items
at /src/main.tyr:6:34
const invalid: Example = { tags: [] };
                                 ^^
```
:::
