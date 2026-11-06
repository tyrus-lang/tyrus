```tyrus
interface Example {
    @notEmptyItems()
    members: string[];
}

const valid: Example = { members: ["user1"] };
```

::: danger error
```tyrus
const invalid: Example = { members: [] }; // [!code error]
```

```text
error[validator-decorations-failed]: `@notEmptyItems()` failed for `members`: array must not be empty
at /src/main.tyr:6:37
const invalid: Example = { members: [] };
                                    ^^
```
:::
