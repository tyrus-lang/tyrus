```tyrus
interface Example {
    @minLength(3)
    username: string;
}

const valid: Example = { username: "abc" };
```

::: danger error
```tyrus
const invalid: Example = { username: "ab" }; // [!code error]
```

```text
error[validator-decorations-failed]: `@minLength(3)` failed for `username`: value must contain at least 3 characters
at /src/main.tyr:6:38
const invalid: Example = { username: "ab" };
                                     ^^^^
```
:::
